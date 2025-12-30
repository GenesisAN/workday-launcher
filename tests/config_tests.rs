//! 配置解析与路径处理相关测试。
//!
//! 覆盖点：
//! - 最小 TOML 配置能被解析（并验证默认值，如 missing_policy=allow）
//! - 相对路径按 base_dir 拼接、绝对路径保持不变
//! - looks_like_path 对常见输入的判断（用于决定是否做相对路径解析）

mod common;

use std::env;
use tempfile::tempdir;
use workday_launcher::{load_config, looks_like_path, resolve_against};

#[test]
fn load_config_parses_toml_minimal() {
    // 用例目的：验证 load_config 能解析最小配置，并补齐 serde 默认值。
    // - 这里只写 command 与 holiday_json_files
    // - missing_policy 未显式配置，期望默认是 allow
    let dir = tempdir().unwrap();
    let cfg_path = dir.path().join("config.toml");

    common::write_file(
        &cfg_path,
        r#"command = "echo"
holiday_json_files = ["2025.json"]
"#,
    );

    let cfg = load_config(&cfg_path).unwrap();
    assert_eq!(cfg.command, "echo");
    assert_eq!(cfg.holiday_json_files, vec!["2025.json".to_string()]);
    assert_eq!(cfg.missing_policy, "allow");
}

#[test]
fn resolve_against_joins_relative_paths() {
    // 用例目的：相对路径需要按 base_dir 进行拼接。
    // 说明：这里用 tempfile 生成真实存在的 base_dir，保证跨平台一致。
    let dir = tempdir().unwrap();
    let base = dir.path();

    let got = resolve_against(base, "data/2025.json");
    assert_eq!(got, base.join("data/2025.json"));
}

#[test]
fn resolve_against_keeps_absolute_paths() {
    // 用例目的：绝对路径不应该再拼接 base_dir。
    // 说明：用 env::temp_dir() 构造一个在当前平台上一定是“绝对路径”的值。
    let dir = tempdir().unwrap();
    let base = dir.path();

    let abs = env::temp_dir().join("workday-launcher-test").join("x.json");
    let abs_s = abs.to_string_lossy().to_string();

    let got = resolve_against(base, &abs_s);
    assert_eq!(got, abs);
}

#[test]
fn looks_like_path_detects_common_cases() {
    // 用例目的：looks_like_path 用于判断 command/workdir 是否“看起来像路径”。
    // 期望：
    // - ./xxx、包含 / 或 \ 的字符串 -> true
    // - 纯命令名（例如 echo） -> false
    assert!(looks_like_path("./bin/tool"));
    assert!(looks_like_path("data/2025.json"));
    assert!(looks_like_path(r"data\2025.json"));
    assert!(!looks_like_path("echo"));
}
