//! Stable error code categories and the FFI error type.

/// The 14 stable error categories mapped from `CoreError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum CoreErrorCode {
    Storage,
    Domain,
    SequenceExhausted,
    BadSignature,
    UnknownMember,
    OriginGap,
    InvalidCommand,
    ReadOnlyLowSpace,
    BadPassphrase,
    UnsupportedBackupVersion,
    InvalidBackup,
    BackupEncryption,
    Closed,
    Unknown,
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiError {
    #[error("{code:?}: {message}")]
    Core {
        code: CoreErrorCode,
        message: String,
    },
}

pub fn map_error(e: todo_core::CoreError) -> FfiError {
    use todo_core::CoreError as E;
    let code = match &e {
        E::Storage(_) => CoreErrorCode::Storage,
        E::Domain(_) => CoreErrorCode::Domain,
        E::SequenceExhausted => CoreErrorCode::SequenceExhausted,
        E::BadSignature => CoreErrorCode::BadSignature,
        E::UnknownMember => CoreErrorCode::UnknownMember,
        E::OriginGap => CoreErrorCode::OriginGap,
        E::InvalidCommand(_) => CoreErrorCode::InvalidCommand,
        E::ReadOnlyLowSpace => CoreErrorCode::ReadOnlyLowSpace,
        E::BadPassphrase => CoreErrorCode::BadPassphrase,
        E::UnsupportedBackupVersion => CoreErrorCode::UnsupportedBackupVersion,
        E::InvalidBackup => CoreErrorCode::InvalidBackup,
        E::BackupEncryption => CoreErrorCode::BackupEncryption,
        E::Closed => CoreErrorCode::Closed,
    };
    FfiError::Core {
        code,
        message: e.to_string(),
    }
}
