//! Discovery error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("invalid hint")]
    InvalidHint,
    #[error("unknown hint")]
    UnknownHint,
    #[error("candidate expired")]
    CandidateExpired,
    #[error("invalid endpoint: {0}")]
    InvalidEndpoint(String),
}
