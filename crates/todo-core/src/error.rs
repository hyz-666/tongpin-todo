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
    #[error("read-only: low space")]
    ReadOnlyLowSpace,
    #[error("bad backup passphrase")]
    BadPassphrase,
    #[error("unsupported backup version")]
    UnsupportedBackupVersion,
    #[error("invalid backup container")]
    InvalidBackup,
    #[error("backup encryption failure")]
    BackupEncryption,
    #[error("core is closed")]
    Closed,
}

impl From<rusqlite::Error> for CoreError {
    fn from(e: rusqlite::Error) -> Self {
        CoreError::Storage(StorageError::Sqlite(e))
    }
}

impl From<std::io::Error> for CoreError {
    fn from(e: std::io::Error) -> Self {
        CoreError::Storage(StorageError::Io(e))
    }
}
