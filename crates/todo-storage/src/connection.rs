//! Encrypted connection setup.

use rusqlite::Connection;

use crate::config::StorageConfig;
use crate::error::StorageError;

/// Plaintext SQLite header that must never appear on an encrypted profile.
const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";

pub fn open_encrypted(config: &StorageConfig) -> Result<Connection, StorageError> {
    // Reject a plaintext database before touching SQLCipher.
    if let Ok(bytes) = std::fs::read(&config.profile_path)
        && bytes.len() >= 16
        && &bytes[..16] == SQLITE_HEADER
    {
        return Err(StorageError::NotEncrypted);
    }

    let conn = Connection::open(&config.profile_path)?;
    // Set the raw key via its hex form; never log the key bytes.
    let hex = config.database_key.hex();
    conn.execute_batch(&format!("PRAGMA key = \"x'{}'\"", hex))?;

    // Reading the schema immediately proves the key is correct.
    verify_key(&conn)?;

    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    conn.busy_timeout(config.busy_timeout)?;
    Ok(conn)
}

fn verify_key(conn: &Connection) -> Result<(), StorageError> {
    match conn.query_row("SELECT count(*) FROM sqlite_master", [], |row| {
        row.get::<_, i64>(0)
    }) {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == rusqlite::ErrorCode::NotADatabase =>
        {
            Err(StorageError::WrongKey)
        }
        Err(e) => Err(e.into()),
    }
}
