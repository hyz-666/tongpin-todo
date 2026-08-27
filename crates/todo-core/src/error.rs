//! Core error types.

use thiserror::Error;

use todo_domain::error::DomainError;
use todo_storage::error::StorageError;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("domain error: {0}")]
    Domain(#[from] DomainError),
    #[error("sequence exhausted")]
    SequenceExhausted,
    #[error("bad signature")]
    BadSignature,
    #[error("unknown member")]
    UnknownMember,
    #[error("origin gap")]
    OriginGap,
    #[error("invalid command: {0}")]
    InvalidCommand(String),
}

impl From<rusqlite::Error> for CoreError {
    fn from(e: rusqlite::Error) -> Self {
        CoreError::Storage(StorageError::Sqlite(e))
    }
}
