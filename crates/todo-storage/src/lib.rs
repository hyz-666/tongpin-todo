#![forbid(unsafe_code)]

//! Encrypted SQLite ownership, migrations, and durable projections.

pub mod calendar;
pub mod config;
pub mod connection;
pub mod error;
pub mod frontier;
pub mod materialize;
pub mod migration;
pub mod query;
pub mod repository;
pub mod schema;
pub mod search;
pub mod transaction;

use rusqlite::Connection;

use crate::config::StorageConfig;
use crate::error::StorageError;

pub const API_VERSION: u32 = 1;

/// An open, encrypted replica profile.
pub struct Storage {
    pub conn: Connection,
}

impl Storage {
    pub fn open(config: StorageConfig) -> Result<Self, StorageError> {
        let conn = connection::open_encrypted(&config)?;
        migration::verify_application_id(&conn)?;
        migration::migrate(&conn, migration::builtin_migrations())?;
        migration::set_application_id(&conn)?;
        Ok(Self { conn })
    }
}
