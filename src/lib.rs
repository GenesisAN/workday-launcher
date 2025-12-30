use anyhow::{anyhow, Context, Result};
use chrono::{Datelike, NaiveDate, Weekday};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

#[derive(Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub workdir: Option<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,

    pub holiday_json_files: Vec<String>,

    /// 找不到日期时的默认行为："allow" | "deny"
    #[serde(default = "default_missing_policy")]
    pub missing_policy: String,
}

fn default_missing_policy() -> String {
    "allow".to_string()
}

// ---------- 节假日 JSON 结构 ----------
#[derive(Deserialize, Debug, Clone)]
pub struct HolidayFile {
    pub year: i32,
    pub region: String,
    pub dates: Vec<HolidayDate>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct HolidayDate {
    pub date: String, // 日期格式："YYYY-MM-DD"
    pub name: String,
    #[serde(default)]
    pub name_cn: Option<String>,
    #[serde(default)]
    pub name_en: Option<String>,
    #[serde(rename = "type")]
    pub kind: String, // 类型："public_holiday" | "transfer_workday" | ...
}

// 索引：date -> 节假日条目（类型/名称等）
#[derive(Debug, Default)]
pub struct HolidayIndex {
    map: HashMap<NaiveDate, HolidayDate>,
    region: Option<String>,
}

impl HolidayIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_file(&mut self, file: HolidayFile) -> Result<()> {
        match self.region.as_deref() {
            None => self.region = Some(file.region.clone()),
            Some(r) if r == file.region => {}
            Some(r) => {
                return Err(anyhow!(
                    "mixed regions in holiday json files: already loaded {r}, but got {}",
                    file.region
                ));
            }
        }
        for d in file.dates {
            let date = NaiveDate::parse_from_str(&d.date, "%Y-%m-%d")
                .with_context(|| format!("Bad date format in json: {}", d.date))?;
            // 后加载的文件覆盖先加载的（便于做覆盖/修正）
            self.map.insert(date, d);
        }
        Ok(())
    }

    pub fn insert_entry(&mut self, date: NaiveDate, entry: HolidayDate) {
        self.map.insert(date, entry);
    }

    pub fn get(&self, date: NaiveDate) -> Option<&HolidayDate> {
        self.map.get(&date)
    }

    pub fn region(&self) -> Option<&str> {
        self.region.as_deref()
    }
}

// ---------- 决策逻辑（是否执行） ----------
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Run { reason: String },
    Skip { reason: String },
}

pub fn decide(date: NaiveDate, idx: &HolidayIndex, missing_policy: &str) -> Decision {
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

pub fn weekday_rule(date: NaiveDate) -> Decision {
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
pub fn run_command(cfg: &AppConfig, verbose: bool) -> Result<ExitStatus> {
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

// ---------- 配置加载与路径处理 ----------
pub fn load_config(path: &Path) -> Result<AppConfig> {
    let s = fs::read_to_string(path)
        .with_context(|| format!("failed to read config: {}", path.display()))?;
    let cfg: AppConfig = toml::from_str(&s).context("failed to parse config toml")?;
    Ok(cfg)
}

pub fn find_config_path(cli_config: Option<PathBuf>) -> Result<PathBuf> {
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
        exe.parent()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "<unknown>".to_string())
    ))
}

pub fn resolve_against(base_dir: &Path, p: &str) -> PathBuf {
    let pb = PathBuf::from(p);
    if pb.is_absolute() {
        pb
    } else {
        base_dir.join(pb)
    }
}

pub fn load_holidays(base_dir: &Path, paths: &[String]) -> Result<HolidayIndex> {
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

pub fn looks_like_path(s: &str) -> bool {
    s.starts_with('.') || s.contains('\\') || s.contains('/')
}
