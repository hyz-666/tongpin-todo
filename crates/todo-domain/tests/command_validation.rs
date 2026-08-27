//! Validation boundaries for titles, descriptions, names, dates, and times.

use todo_domain::clock::{LocalDate, LocalTime};
use todo_domain::model::Priority;
use todo_domain::validation::{
    validate_description, validate_due, validate_list_name, validate_tag_name, validate_title,
};

#[test]
fn title_scalar_bounds() {
    assert!(validate_title("a").is_ok()); // 1 scalar
    assert!(validate_title("买牛奶").is_ok());
    assert!(validate_title(&"字".repeat(200)).is_ok()); // 200 scalars
    assert!(validate_title(&"字".repeat(201)).is_err()); // 201 scalars
    assert!(validate_title("").is_err()); // 0 scalars
}

#[test]
fn description_byte_bounds() {
    assert!(validate_description("").is_ok()); // empty is allowed
    assert!(validate_description(&"a".repeat(65_536)).is_ok()); // exactly 65536 bytes
    assert!(validate_description(&"a".repeat(65_537)).is_err()); // 65537 bytes
}

#[test]
fn list_name_bounds() {
    assert!(validate_list_name("工作").is_ok());
    assert!(validate_list_name(&"列".repeat(100)).is_ok());
    assert!(validate_list_name(&"列".repeat(101)).is_err());
    assert!(validate_list_name("").is_err());
}

#[test]
fn tag_name_bounds() {
    assert!(validate_tag_name("重要").is_ok());
    assert!(validate_tag_name(&"标".repeat(64)).is_ok());
    assert!(validate_tag_name(&"标".repeat(65)).is_err());
    assert!(validate_tag_name("").is_err());
}

#[test]
fn priority_ordering() {
    assert!(Priority::None < Priority::Low);
    assert!(Priority::Low < Priority::Medium);
    assert!(Priority::Medium < Priority::High);
}

#[test]
fn invalid_dates_rejected() {
    assert!(LocalDate::new(2024, 2, 29).is_ok()); // leap year
    assert!(LocalDate::new(2026, 2, 29).is_err()); // not a leap year
    assert!(LocalDate::new(2026, 13, 1).is_err()); // month out of range
    assert!(LocalDate::new(2026, 0, 1).is_err());
    assert!(LocalDate::new(2026, 4, 31).is_err()); // April has 30 days
    assert!(LocalDate::new(2026, 1, 31).is_ok());
}

#[test]
fn invalid_time_rejected() {
    assert!(LocalTime::new(24, 0).is_err());
    assert!(LocalTime::new(0, 60).is_err());
    assert!(LocalTime::new(23, 59).is_ok());
}

#[test]
fn time_requires_date() {
    let date = LocalDate::new(2026, 8, 27).ok();
    let time = LocalTime::new(9, 30).ok();
    assert!(validate_due(date.as_ref(), time.as_ref()).is_ok());
    assert!(validate_due(None, time.as_ref()).is_err()); // time without date
    assert!(validate_due(date.as_ref(), None).is_ok()); // date without time is fine
    assert!(validate_due(None, None).is_ok());
}
