#![forbid(unsafe_code)]

//! Canonical frames, version negotiation, and message types.

pub mod codec;
pub mod error;
pub mod frame;
pub mod limits;
pub mod negotiation;

pub use codec::{decode_frame, encode_frame};
pub use error::ProtocolError;
pub use frame::{Frame, MessageV1};
pub use limits::{
    DEFAULT_CIPHERTEXT_BUDGET, DEFAULT_IN_FLIGHT, MAX_CHUNK_OPERATIONS, MAX_CHUNK_SIZE,
    MAX_FRAME_SIZE,
};
pub use negotiation::{
    HelloInfo, NegotiationOutcome, REQUIRED_FEATURE_FLAG, ResourceLimits, negotiate,
};

pub const API_VERSION: u32 = 1;
