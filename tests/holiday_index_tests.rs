//! 节假日索引（HolidayIndex）相关测试。
//!
//! 覆盖点：
//! - 多个 JSON 文件索引合并时，region 必须一致
//! - 同一天被多次写入时，后加载的文件覆盖先加载的文件（便于修正/覆盖）
//! - JSON 内日期格式不合法时要报错
//! - 从磁盘读取 JSON 列表并成功建索引（load_holidays）

mod common;

use chrono::NaiveDate;
use tempfile::tempdir;
use workday_launcher::{load_holidays, HolidayDate, HolidayFile, HolidayIndex};

/// 构造 HolidayFile（模拟从 holiday-calendar 的 JSON 反序列化结果）。
///
/// 说明：
/// - `year` 在当前逻辑里主要用于元数据，不影响索引键（索引键是每个条目的 date）
fn file(region: &str, dates: Vec<HolidayDate>) -> HolidayFile {
    HolidayFile {
        year: 2025,
        region: region.to_string(),
        dates,
    }
}

/// 构造 HolidayDate（仅填充测试需要字段）。
fn hd(date: &str, kind: &str, name: &str) -> HolidayDate {
    HolidayDate {
        date: date.to_string(),
        name: name.to_string(),
        name_cn: None,
        name_en: None,
        kind: kind.to_string(),
    }
}

#[test]
fn insert_file_rejects_mixed_region() {
    // 用例目的：防止把不同地区的节假日 JSON 混在一起。
    // 期望：一旦已经加载了某个 region，再加载不同 region 的文件要报错。
    let mut idx = HolidayIndex::new();
    idx.insert_file(file("CN", vec![hd("2025-01-01", "public_holiday", "NY")] ))
        .unwrap();

    let err = idx
        .insert_file(file("US", vec![hd("2025-07-04", "public_holiday", "ID")] ))
        .unwrap_err();

    assert!(err.to_string().contains("mixed regions"));
}

#[test]
fn later_files_override_earlier_dates() {
    // 用例目的：验证“后加载覆盖先加载”的规则。
    // 这个规则让你可以用第二个文件对上游数据做局部修正。
    let mut idx = HolidayIndex::new();
    let d = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();

    idx.insert_file(file(
        "CN",
        vec![hd("2025-01-01", "public_holiday", "Old")],
    ))
    .unwrap();

    idx.insert_file(file(
        "CN",
        vec![hd("2025-01-01", "transfer_workday", "New")],
    ))
    .unwrap();

    let got = idx.get(d).unwrap();
    assert_eq!(got.kind, "transfer_workday");
    assert_eq!(got.name, "New");
}

#[test]
fn insert_file_errors_on_bad_date_format() {
    // 用例目的：JSON 内日期格式必须是 YYYY-MM-DD。
    // 期望：出现不合法格式时返回错误，并包含 "Bad date format" 便于定位。
    let mut idx = HolidayIndex::new();
    let err = idx
        .insert_file(file(
            "CN",
            vec![hd("2025/01/01", "public_holiday", "Bad")],
        ))
        .unwrap_err();

    assert!(err.to_string().contains("Bad date format"));
}

#[test]
fn load_holidays_reads_and_indexes_files() {
    // 用例目的：验证从磁盘读取 JSON 并建立索引的整体流程。
    // - 使用 tempfile 创建隔离目录，避免污染仓库 data/
    // - 写入两个 JSON 文件，分别包含 public_holiday 与 transfer_workday
    // 期望：
    // - load_holidays 返回的索引 region 正确
    // - 两个日期都能查到对应条目
    let dir = tempdir().unwrap();
    let base = dir.path();

    common::write_file(
        &base.join("a.json"),
        r#"{
  "year": 2025,
  "region": "CN",
  "dates": [
    {"date": "2025-10-01", "name": "National Day", "type": "public_holiday"}
  ]
}"#,
    );

    common::write_file(
        &base.join("b.json"),
        r#"{
  "year": 2025,
  "region": "CN",
  "dates": [
    {"date": "2025-10-11", "name": "Adjusted", "type": "transfer_workday"}
  ]
}"#,
    );

    let idx = load_holidays(
        base,
        &vec!["a.json".to_string(), "b.json".to_string()],
    )
    .unwrap();

    assert_eq!(idx.region(), Some("CN"));
    assert!(idx
        .get(NaiveDate::from_ymd_opt(2025, 10, 1).unwrap())
        .is_some());
    assert!(idx
        .get(NaiveDate::from_ymd_opt(2025, 10, 11).unwrap())
        .is_some());
}
