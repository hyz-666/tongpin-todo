//! Clock value objects: hybrid logical clock, instants, and calendar types.

use crate::error::DomainError;

/// Hybrid logical clock timestamp. Never decreases across `tick`/`observe`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Hlc {
    pub physical_millis: i64,
    pub logical: u32,
}

impl Hlc {
    pub const fn new(physical_millis: i64, logical: u32) -> Self {
        Self {
            physical_millis,
            logical,
        }
    }

    /// Advance the local clock for a new event observed at `now_millis`.
    pub fn tick(&self, now_millis: i64) -> Result<Self, DomainError> {
        if now_millis > self.physical_millis {
            return Ok(Self::new(now_millis, 0));
        }
        let logical = self
            .logical
            .checked_add(1)
            .ok_or(DomainError::HlcLogicalOverflow)?;
        Ok(Self::new(self.physical_millis, logical))
    }

    /// Merge a remote timestamp received at `now_millis` without decreasing.
    pub fn observe(&self, remote: &Self, now_millis: i64) -> Result<Self, DomainError> {
        let physical = self
            .physical_millis
            .max(remote.physical_millis)
            .max(now_millis);
        let logical = if physical == self.physical_millis && physical == remote.physical_millis {
            self.logical
                .max(remote.logical)
                .checked_add(1)
                .ok_or(DomainError::HlcLogicalOverflow)?
        } else if physical == self.physical_millis {
            self.logical
                .checked_add(1)
                .ok_or(DomainError::HlcLogicalOverflow)?
        } else if physical == remote.physical_millis {
            remote
                .logical
                .checked_add(1)
                .ok_or(DomainError::HlcLogicalOverflow)?
        } else {
            0
        };
        Ok(Self::new(physical, logical))
    }
}

/// An absolute audit instant in UTC, milliseconds since the Unix epoch.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct UtcInstant {
    pub millis_since_unix_epoch: i64,
}

impl UtcInstant {
    pub const fn from_millis(millis_since_unix_epoch: i64) -> Self {
        Self {
            millis_since_unix_epoch,
        }
    }
}

/// A floating calendar date (no time zone).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct LocalDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl LocalDate {
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, DomainError> {
        if month == 0 || month > 12 {
            return Err(DomainError::InvalidDate(year, month, day));
        }
        if day == 0 || day > days_in_month(year, month) {
            return Err(DomainError::InvalidDate(year, month, day));
        }
        Ok(Self { year, month, day })
    }
}

/// A floating wall-clock time of day.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct LocalTime {
    pub hour: u8,
    pub minute: u8,
}

impl LocalTime {
    pub fn new(hour: u8, minute: u8) -> Result<Self, DomainError> {
        if hour > 23 || minute > 59 {
            return Err(DomainError::InvalidTime(hour, minute));
        }
        Ok(Self { hour, minute })
    }
}

/// A calendar month, used by calendar projections.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct YearMonth {
    pub year: i32,
    pub month: u8,
}

impl YearMonth {
    pub fn new(year: i32, month: u8) -> Result<Self, DomainError> {
        if month == 0 || month > 12 {
            return Err(DomainError::InvalidDate(year, month, 0));
        }
        Ok(Self { year, month })
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}
