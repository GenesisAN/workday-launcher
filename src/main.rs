use anyhow::{anyhow, Context, Result};
use chrono::{Datelike, Local, NaiveDate};
use clap::Parser;
use std::path::PathBuf;

use workday_launcher::{
    decide, find_config_path, load_config, load_holidays, looks_like_path, resolve_against,
    run_command, Decision,
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

    let decision = decide(date, &idx, &cfg.missing_policy, cfg.invert);

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
