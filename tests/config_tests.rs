mod common;

use std::path::PathBuf;
use tempfile::tempdir;
use workday_launcher::{load_config, looks_like_path, resolve_against};

#[test]
fn load_config_parses_toml_minimal() {
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
    let base = PathBuf::from(r"C:\base");
    let got = resolve_against(&base, r"data\2025.json");
    assert!(got.to_string_lossy().ends_with(r"base\data\2025.json"));
}

#[test]
fn resolve_against_keeps_absolute_paths() {
    let base = PathBuf::from(r"C:\base");
    let got = resolve_against(&base, r"C:\abs\x.json");
    assert_eq!(got, PathBuf::from(r"C:\abs\x.json"));
}

#[test]
fn looks_like_path_detects_common_cases() {
    assert!(looks_like_path("./bin/tool"));
    assert!(looks_like_path("data/2025.json"));
    assert!(looks_like_path(r"data\2025.json"));
    assert!(!looks_like_path("echo"));
}
