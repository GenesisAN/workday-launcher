//! 决策逻辑（是否执行）相关测试。
//!
//! 覆盖点：
//! - 日期不在节假日 JSON 中时：按 `missing_policy` 与周末/工作日规则决定
//! - 日期命中节假日条目时：`transfer_workday` 强制运行、`public_holiday` 强制跳过
//! - 未知 `type`：回退到周末/工作日规则，但 reason 中要带上“unknown holiday type”

use chrono::NaiveDate;
use workday_launcher::{decide, Decision, HolidayDate, HolidayIndex};

/// 构造一个 HolidayDate（仅填充当前测试需要的字段）。
///
/// 说明：
/// - `date` 字段是字符串形式，索引时由库内部解析为 `NaiveDate`
/// - `name_cn/name_en` 对决策逻辑不重要，因此置为 `None`
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
    // 用例目的：
    // - 当天不在任何 holiday JSON 中
    // - missing_policy=allow
    // 期望：仅按 weekday_rule 判断：工作日 -> 运行。
    let idx = HolidayIndex::new();
    let d = NaiveDate::from_ymd_opt(2025, 12, 30).unwrap(); // Tue
    let got = decide(d, &idx, "allow");
    assert_eq!(got, Decision::Run { reason: "weekday".to_string() });
}

#[test]
fn weekend_skips_when_missing_and_allow() {
    // 用例目的：
    // - 当天不在任何 holiday JSON 中
    // - missing_policy=allow
    // 期望：仅按 weekday_rule 判断：周末 -> 跳过。
    let idx = HolidayIndex::new();
    let d = NaiveDate::from_ymd_opt(2026, 1, 3).unwrap(); // Sat
    let got = decide(d, &idx, "allow");
    assert_eq!(got, Decision::Skip { reason: "weekend".to_string() });
}

#[test]
fn missing_policy_deny_always_skips() {
    // 用例目的：
    // - 当天不在任何 holiday JSON 中
    // - missing_policy=deny
    // 期望：直接跳过（更保守），不再按周末/工作日规则判断。
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
    // 用例目的：
    // - 当天命中 public_holiday
    // 期望：即使是工作日，也必须跳过。
    // 断言策略：
    // - 类型断言（Decision::Skip）
    // - reason 包含 public_holiday（方便 CLI 输出可读）
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
    // 用例目的：
    // - 当天命中 transfer_workday（调休补班）
    // 期望：即使是周末，也必须运行。
    // 断言策略：
    // - 类型断言（Decision::Run）
    // - reason 包含 transfer_workday
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
    // 用例目的：
    // - JSON 中出现未知 type（例如上游库新增类型或地区自定义类型）
    // 期望：
    // - 决策仍按周末/工作日规则回退（这里选一个工作日 -> Run）
    // - reason 中要带上“unknown holiday type (...) + weekday”，便于排查数据类型变更
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
