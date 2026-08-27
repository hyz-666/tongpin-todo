//! Crypto error types with typed verification reasons.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("invalid signature")]
    InvalidSignature,
    #[error("canonical decode error: {0}")]
    Decode(String),
    #[error("canonical encoding error: {0}")]
    Canonical(String),
    #[error("identity error: {0}")]
    Identity(String),
}

/// A typed reason for rejecting a signed operation, returned before any
/// storage access happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyReason {
    BadSignature,
    UnknownDevice,
    WrongGroup,
    UnknownRequiredKind,
    Malformed,
}
