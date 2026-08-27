//! Storage error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("wrong database key")]
    WrongKey,
    #[error("database is not encrypted (plaintext header detected)")]
    NotEncrypted,
    #[error("migration failed: {0}")]
    MigrationFailed(String),
    #[error("unsupported downgrade: schema {current} > supported {supported}")]
    UnsupportedDowngrade { current: i32, supported: i32 },
    #[error("unexpected application id {0}")]
    ApplicationIdMismatch(i32),
}
