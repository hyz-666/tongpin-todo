//! Input validation and normalization rules.

use unicode_normalization::UnicodeNormalization;

use crate::clock::{LocalDate, LocalTime};
use crate::error::DomainError;

pub const MAX_TITLE_SCALARS: usize = 200;
pub const MAX_DESCRIPTION_BYTES: usize = 65_536;
pub const MAX_LIST_NAME_CHARS: usize = 100;
pub const MAX_TAG_NAME_CHARS: usize = 64;

/// Titles are counted in Unicode scalar values and must be 1..=200.
pub fn validate_title(title: &str) -> Result<(), DomainError> {
    let count = title.chars().count();
    if !(1..=MAX_TITLE_SCALARS).contains(&count) {
        return Err(DomainError::InvalidTitle);
    }
    Ok(())
}

/// Descriptions are counted in encoded UTF-8 bytes and must not exceed 65536.
pub fn validate_description(description: &str) -> Result<(), DomainError> {
    if description.len() > MAX_DESCRIPTION_BYTES {
        return Err(DomainError::InvalidDescription);
    }
    Ok(())
}

pub fn validate_list_name(name: &str) -> Result<(), DomainError> {
    let count = name.chars().count();
    if !(1..=MAX_LIST_NAME_CHARS).contains(&count) {
        return Err(DomainError::InvalidListName);
    }
    Ok(())
}

pub fn validate_tag_name(name: &str) -> Result<(), DomainError> {
    let count = name.chars().count();
    if !(1..=MAX_TAG_NAME_CHARS).contains(&count) {
        return Err(DomainError::InvalidTagName);
    }
    Ok(())
}

/// A due time is only meaningful alongside a due date.
pub fn validate_due(date: Option<&LocalDate>, time: Option<&LocalTime>) -> Result<(), DomainError> {
    if time.is_some() && date.is_none() {
        return Err(DomainError::TimeWithoutDate);
    }
    Ok(())
}

/// NFKC-normalize then apply locale-independent case folding.
pub fn normalize(input: &str) -> String {
    input.nfkc().collect::<String>().to_lowercase()
}
