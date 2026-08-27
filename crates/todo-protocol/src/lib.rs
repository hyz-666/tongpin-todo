#![forbid(unsafe_code)]

//! Canonical frames, version negotiation, and message types.

pub mod codec;
pub mod error;
pub mod frame;
pub mod limits;
pub mod liveness;
pub mod negotiation;
pub mod noise;
pub mod session;
pub mod transport;

pub use codec::{decode_frame, encode_frame};
pub use error::ProtocolError;
pub use frame::{Frame, MessageV1};
pub use limits::{
    DEFAULT_CIPHERTEXT_BUDGET, DEFAULT_IN_FLIGHT, MAX_CHUNK_OPERATIONS, MAX_CHUNK_SIZE,
    MAX_FRAME_SIZE,
};
pub use liveness::{
    CONNECT_TIMEOUT_MS, DEAD_AFTER_MS, HANDSHAKE_TIMEOUT_MS, HEARTBEAT_INTERVAL_MS,
    HELLO_TIMEOUT_MS, Liveness,
};
pub use negotiation::{
    HelloInfo, NegotiationOutcome, REQUIRED_FEATURE_FLAG, ResourceLimits, negotiate,
};
pub use noise::{NoiseRole, NoiseSession, Transport};
pub use session::{CloseReason, PeerSession, SessionState};
pub use transport::{frame_ciphertext, unframe};

pub const API_VERSION: u32 = 1;
