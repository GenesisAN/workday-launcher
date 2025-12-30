mod common;

use chrono::NaiveDate;
use tempfile::tempdir;
use workday_launcher::{load_holidays, HolidayDate, HolidayFile, HolidayIndex};

fn file(region: &str, dates: Vec<HolidayDate>) -> HolidayFile {
    HolidayFile {
        year: 2025,
        region: region.to_string(),
        dates,
    }
}

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
