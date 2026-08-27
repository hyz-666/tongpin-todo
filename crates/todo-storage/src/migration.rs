//! Versioned migration registry and transactional application.

use rusqlite::Connection;

use crate::error::StorageError;
use crate::schema::{APPLICATION_ID, SCHEMA_VERSION};

pub struct Migration {
    pub version: i32,
    pub sql: &'static str,
}

pub fn builtin_migrations() -> &'static [Migration] {
    &[
        Migration {
            version: 1,
            sql: include_str!("../migrations/0001_initial.sql"),
        },
        Migration {
            version: 2,
            sql: include_str!("../migrations/0002_fts.sql"),
        },
    ]
}

pub fn user_version(conn: &Connection) -> Result<i32, StorageError> {
    let v: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    Ok(v as i32)
}

pub fn verify_application_id(conn: &Connection) -> Result<(), StorageError> {
    let id: i64 = conn.query_row("PRAGMA application_id", [], |row| row.get(0))?;
    if id != 0 && id != APPLICATION_ID as i64 {
        return Err(StorageError::ApplicationIdMismatch(id as i32));
    }
    Ok(())
}

pub fn set_application_id(conn: &Connection) -> Result<(), StorageError> {
    conn.execute_batch(&format!("PRAGMA application_id = {};", APPLICATION_ID))?;
    Ok(())
}

/// Apply pending migrations, each in its own transaction. A failure rolls the
/// current migration back and leaves the profile reopenable at its prior version.
pub fn migrate(conn: &Connection, migrations: &[Migration]) -> Result<(), StorageError> {
    let current = user_version(conn)?;
    if current > SCHEMA_VERSION {
        return Err(StorageError::UnsupportedDowngrade {
            current,
            supported: SCHEMA_VERSION,
        });
    }
    for m in migrations.iter().filter(|m| m.version > current) {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(m.sql)
            .map_err(|e| StorageError::MigrationFailed(e.to_string()))?;
        tx.pragma_update(None, "user_version", m.version)?;
        tx.commit()?;
    }
    Ok(())
}
