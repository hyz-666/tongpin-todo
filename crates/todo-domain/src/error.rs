//! Domain error types surfaced at the value and command boundaries.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    #[error("title must contain 1 to 200 Unicode scalar values")]
    InvalidTitle,
    #[error("description exceeds 65536 bytes")]
    InvalidDescription,
    #[error("list name must contain 1 to 100 characters")]
    InvalidListName,
    #[error("tag name must contain 1 to 64 characters")]
    InvalidTagName,
    #[error("invalid date {0}-{1:02}-{2:02}")]
    InvalidDate(i32, u8, u8),
    #[error("invalid time {0:02}:{1:02}")]
    InvalidTime(u8, u8),
    #[error("a due time requires a due date")]
    TimeWithoutDate,
    #[error("HLC logical counter overflow")]
    HlcLogicalOverflow,
}
