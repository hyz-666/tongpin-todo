//! Contiguous frontier and gap tracking.

use rusqlite::Connection;

use todo_domain::ids::DeviceId;

use crate::error::StorageError;
use crate::repository::Repository;

/// Record a gap in an origin's sequence range.
pub fn record_gap(
    conn: &Connection,
    origin: &DeviceId,
    start: u64,
    end: u64,
) -> Result<(), StorageError> {
    conn.execute(
        "INSERT OR IGNORE INTO origin_gaps(origin_device_id, start_sequence, end_sequence) VALUES (?1, ?2, ?3)",
        rusqlite::params![origin.as_bytes().as_slice(), start as i64, end as i64],
    )?;
    Ok(())
}

/// Compute the contiguous frontier after accepting `sequence` for `origin`.
/// Returns the new contiguous value, or the current frontier if a gap remains.
pub fn advance_frontier(
    conn: &Connection,
    origin: &DeviceId,
    sequence: u64,
) -> Result<u64, StorageError> {
    let current = Repository::read_frontier(conn, origin)?.unwrap_or(0);
    if sequence <= current {
        return Ok(current);
    }
    if sequence == current + 1 {
        Repository::upsert_frontier(conn, origin, sequence)?;
        return Ok(sequence);
    }
    record_gap(conn, origin, current + 1, sequence - 1)?;
    Ok(current)
}
