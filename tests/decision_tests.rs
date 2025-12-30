use chrono::NaiveDate;
use workday_launcher::{decide, Decision, HolidayDate, HolidayIndex};

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
fn weekday_runs_when_missing_and_allow() {
    let idx = HolidayIndex::new();
    let d = NaiveDate::from_ymd_opt(2025, 12, 30).unwrap(); // Tue
    let got = decide(d, &idx, "allow");
    assert_eq!(got, Decision::Run { reason: "weekday".to_string() });
}

#[test]
fn weekend_skips_when_missing_and_allow() {
    let idx = HolidayIndex::new();
    let d = NaiveDate::from_ymd_opt(2026, 1, 3).unwrap(); // Sat
    let got = decide(d, &idx, "allow");
    assert_eq!(got, Decision::Skip { reason: "weekend".to_string() });
}

#[test]
fn missing_policy_deny_always_skips() {
    let idx = HolidayIndex::new();
    let d = NaiveDate::from_ymd_opt(2025, 12, 30).unwrap();
    let got = decide(d, &idx, "deny");
    assert_eq!(
        got,
        Decision::Skip {
            reason: "date not found in holiday json (missing_policy=deny)".to_string()
        }
    );
}

#[test]
fn public_holiday_skips_even_if_weekday() {
    let mut idx = HolidayIndex::new();
    let d = NaiveDate::from_ymd_opt(2025, 10, 1).unwrap();
    idx.insert_entry(d, hd("2025-10-01", "public_holiday", "National Day"));

    let got = decide(d, &idx, "allow");
    assert!(matches!(got, Decision::Skip { .. }));
    if let Decision::Skip { reason } = got {
        assert!(reason.contains("public_holiday"));
    }
}

#[test]
fn transfer_workday_runs_even_if_weekend() {
    let mut idx = HolidayIndex::new();
    let d = NaiveDate::from_ymd_opt(2025, 10, 11).unwrap(); // Sat
    idx.insert_entry(d, hd("2025-10-11", "transfer_workday", "Adjusted Workday"));

    let got = decide(d, &idx, "allow");
    assert!(matches!(got, Decision::Run { .. }));
    if let Decision::Run { reason } = got {
        assert!(reason.contains("transfer_workday"));
    }
}

#[test]
fn unknown_kind_falls_back_to_weekday_rule_with_reason() {
    let mut idx = HolidayIndex::new();
    let d = NaiveDate::from_ymd_opt(2025, 12, 30).unwrap(); // Tue
    idx.insert_entry(d, hd("2025-12-30", "company_event", "Foo"));

    let got = decide(d, &idx, "allow");
    assert!(matches!(got, Decision::Run { .. }));
    if let Decision::Run { reason } = got {
        assert!(reason.contains("unknown holiday type"));
        assert!(reason.contains("weekday"));
    }
}
