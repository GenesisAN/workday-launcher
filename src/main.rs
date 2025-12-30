use anyhow::{anyhow, Context, Result};
use chrono::{Datelike, Local, NaiveDate, Weekday};
use clap::Parser;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

/// 工作日启动器：仅在中国工作日运行指定程序
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    after_help = "Holiday JSON data source: https://unpkg.com/holiday-calendar@1.3.0/data/CN/\n\
This tool only reads local JSON files; please follow the upstream data license/terms."
)]
struct Cli {
    /// 配置文件路径（TOML）
    #[arg(long)]
    config: Option<PathBuf>,

    /// 覆盖日期（YYYY-MM-DD），用于测试
    #[arg(long)]
    date: Option<String>,

    /// 演练模式：只打印决策与命令，不实际执行
    #[arg(long)]
    dry_run: bool,

    /// 输出更多日志
    #[arg(long)]
    verbose: bool,
}

#[derive(Deserialize, Debug)]
struct AppConfig {
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    workdir: Option<String>,
    #[serde(default)]
    env: HashMap<String, String>,

    holiday_json_files: Vec<String>,

    /// 找不到日期时的默认行为："allow" | "deny"
    #[serde(default = "default_missing_policy")]
    missing_policy: String,
}

fn default_missing_policy() -> String {
    "allow".to_string()
}

// ---------- 节假日 JSON 结构 ----------
#[derive(Deserialize, Debug)]
struct HolidayFile {
    year: i32,
    region: String,
    dates: Vec<HolidayDate>,
}

#[derive(Deserialize, Debug, Clone)]
struct HolidayDate {
    date: String, // 日期格式："YYYY-MM-DD"
    name: String,
    #[serde(default)]
    name_cn: Option<String>,
    #[serde(default)]
    name_en: Option<String>,
    #[serde(rename = "type")]
    kind: String, // 类型："public_holiday" | "transfer_workday" | ...
}

// 索引：date -> 节假日条目（类型/名称等）
#[derive(Debug, Default)]
struct HolidayIndex {
    map: HashMap<NaiveDate, HolidayDate>,
}

impl HolidayIndex {
    fn insert_file(&mut self, file: HolidayFile) -> Result<()> {
        if file.region != "CN" {
            return Err(anyhow!("Unsupported region: {}", file.region));
        }
        for d in file.dates {
            let date = NaiveDate::parse_from_str(&d.date, "%Y-%m-%d")
                .with_context(|| format!("Bad date format in json: {}", d.date))?;
            // 后加载的文件覆盖先加载的（便于做覆盖/修正）
            self.map.insert(date, d);
        }
        Ok(())
    }

    fn get(&self, date: NaiveDate) -> Option<&HolidayDate> {
        self.map.get(&date)
    }
}

// ---------- 决策逻辑（是否执行） ----------
#[derive(Debug)]
enum Decision {
    Run { reason: String },
    Skip { reason: String },
}

fn decide(date: NaiveDate, idx: &HolidayIndex, missing_policy: &str) -> Decision {
    if let Some(h) = idx.get(date) {
        match h.kind.as_str() {
            "transfer_workday" => {
                return Decision::Run {
                    reason: format!("transfer_workday: {}", h.name),
                }
            }
            "public_holiday" => {
                return Decision::Skip {
                    reason: format!("public_holiday: {}", h.name),
                }
            }
            other => {
                // 未知类型：继续走周末/工作日规则，但把类型写进原因里
                // 如果你更保守，也可以改成遇到未知类型就直接 Skip
                let base = weekday_rule(date);
                return match base {
                    Decision::Run { reason } => Decision::Run {
                        reason: format!("unknown holiday type ({other}) + {reason}"),
                    },
                    Decision::Skip { reason } => Decision::Skip {
                        reason: format!("unknown holiday type ({other}) + {reason}"),
                    },
                };
            }
        }
    }

    // 当日不在任何 JSON 中
    match missing_policy {
        "deny" => Decision::Skip {
            reason: "date not found in holiday json (missing_policy=deny)".to_string(),
        },
        _ => weekday_rule(date),
    }
}

fn weekday_rule(date: NaiveDate) -> Decision {
    match date.weekday() {
        Weekday::Sat | Weekday::Sun => Decision::Skip {
            reason: "weekend".to_string(),
        },
        _ => Decision::Run {
            reason: "weekday".to_string(),
        },
    }
}

// ---------- 执行外部命令 ----------
fn run_command(cfg: &AppConfig, verbose: bool) -> Result<ExitStatus> {
    let mut cmd = Command::new(&cfg.command);
    cmd.args(&cfg.args);

    if let Some(wd) = &cfg.workdir {
        cmd.current_dir(wd);
    }

    for (k, v) in &cfg.env {
        cmd.env(k, v);
    }

    if verbose {
        eprintln!("[launcher] exec: {:?} {:?}", cfg.command, cfg.args);
        if let Some(wd) = &cfg.workdir {
            eprintln!("[launcher] workdir: {}", wd);
        }
        if !cfg.env.is_empty() {
            eprintln!("[launcher] env: {:?}", cfg.env);
        }
    }

    let status = cmd.status().context("failed to spawn command")?;
    Ok(status)
}

// ---------- 配置加载与入口 ----------
fn load_config(path: &Path) -> Result<AppConfig> {
    let s = fs::read_to_string(path)
        .with_context(|| format!("failed to read config: {}", path.display()))?;
    let cfg: AppConfig = toml::from_str(&s).context("failed to parse config toml")?;
    Ok(cfg)
}

fn find_config_path(cli_config: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = cli_config {
        return Ok(p);
    }

    let filename = PathBuf::from("config.toml");

    // 1) 当前工作目录
    let cwd = std::env::current_dir().context("failed to get current working directory")?;
    let candidate = cwd.join(&filename);
    if candidate.exists() {
        return Ok(candidate);
    }

    // 2) 可执行文件所在目录（"软件根目录"）
    let exe = std::env::current_exe().context("failed to get current exe path")?;
    if let Some(exe_dir) = exe.parent() {
        let candidate = exe_dir.join(&filename);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(anyhow!(
        "config not provided and config.toml not found in cwd ({}) or exe dir ({})",
        cwd.display(),
        exe.parent().map(|p| p.display().to_string()).unwrap_or_else(|| "<unknown>".to_string())
    ))
}

fn resolve_against(base_dir: &Path, p: &str) -> PathBuf {
    let pb = PathBuf::from(p);
    if pb.is_absolute() {
        pb
    } else {
        base_dir.join(pb)
    }
}

fn load_holidays(base_dir: &Path, paths: &[String]) -> Result<HolidayIndex> {
    let mut idx = HolidayIndex::default();
    for p in paths {
        let resolved = resolve_against(base_dir, p);
        let s = fs::read_to_string(&resolved).with_context(|| {
            format!(
                "failed to read holiday json: {} (from config entry: {p})",
                resolved.display()
            )
        })?;
        let hf: HolidayFile = serde_json::from_str(&s)
            .with_context(|| format!("failed to parse holiday json: {}", resolved.display()))?;
        idx.insert_file(hf)
            .with_context(|| format!("failed to index holiday json: {}", resolved.display()))?;
    }
    Ok(idx)
}

fn looks_like_path(s: &str) -> bool {
    s.starts_with('.') || s.contains('\\') || s.contains('/')
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = find_config_path(cli.config)?;

    // 相对路径以“配置文件所在目录”作为基准：
    // - 适合源码仓库（config.toml 与 data/ 同级）
    // - 发布时把 config.toml 放在软件目录即可实现“以软件为准”
    let base_dir = config_path
        .parent()
        .ok_or_else(|| anyhow!("config path has no parent: {}", config_path.display()))?;

    let mut cfg = load_config(&config_path)?;
    if let Some(wd) = cfg.workdir.as_deref() {
        let resolved = resolve_against(base_dir, wd);
        cfg.workdir = Some(resolved.to_string_lossy().to_string());
    }
    if looks_like_path(&cfg.command) {
        let resolved = resolve_against(base_dir, &cfg.command);
        cfg.command = resolved.to_string_lossy().to_string();
    }

    let idx = load_holidays(base_dir, &cfg.holiday_json_files)?;

    let date = if let Some(d) = cli.date.as_deref() {
        NaiveDate::parse_from_str(d, "%Y-%m-%d").context("bad --date format, need YYYY-MM-DD")?
    } else {
        // 本地时间日期
        let now = Local::now();
        NaiveDate::from_ymd_opt(now.year(), now.month(), now.day())
            .ok_or_else(|| anyhow!("failed to get local date"))?
    };

    let decision = decide(date, &idx, &cfg.missing_policy);

    match decision {
        Decision::Run { reason } => {
            println!("[launcher] {} RUN ({})", date, reason);
            if cli.dry_run {
                println!(
                    "[launcher] dry-run command: {} {}",
                    cfg.command,
                    cfg.args.join(" ")
                );
                return Ok(());
            }
            let status = run_command(&cfg, cli.verbose)?;
            println!("[launcher] exit status: {}", status);
            // 把子进程退出码透传（可选）
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Decision::Skip { reason } => {
            println!("[launcher] {} SKIP ({})", date, reason);
        }
    }

    Ok(())
}
