//! Protocol error categories (redacted, no wire detail).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("protocol incompatible")]
    ProtocolIncompatible,
    #[error("malformed frame")]
    MalformedFrame,
    #[error("frame too large")]
    FrameTooLarge,
    #[error("unknown required feature")]
    UnknownRequiredFeature,
    #[error("sequence error")]
    SequenceError,
    #[error("bad session")]
    BadSession,
    #[error("decode error")]
    Decode,
}
