#![forbid(unsafe_code)]

//! Canonical frames, version negotiation, and message types.

pub mod codec;
pub mod error;
pub mod flow_control;
pub mod frame;
pub mod limits;
pub mod liveness;
pub mod negotiation;
pub mod noise;
pub mod range;
pub mod session;
pub mod transfer;
pub mod transport;
pub mod version_summary;

pub use codec::{decode_frame, encode_frame};
pub use error::ProtocolError;
pub use flow_control::FlowControl;
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
pub use range::RangeRequest;
pub use session::{CloseReason, PeerSession, SessionState};
pub use transfer::{OperationChunk, chunk_operations, verify_chunk};
pub use transport::{frame_ciphertext, unframe};
pub use version_summary::{SeqRange, VersionSummary};

pub const API_VERSION: u32 = 1;
