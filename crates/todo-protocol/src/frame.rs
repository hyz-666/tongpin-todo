//! Message and frame definitions.

use todo_domain::ids::DeviceId;

/// The twelve message kinds of the sync protocol.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageV1 {
    Hello {
        protocol_minor: u16,
        schema: u16,
        capabilities: u64,
        device_id: DeviceId,
    },
    HelloAck {
        protocol_minor: u16,
        schema: u16,
        capabilities: u64,
        device_id: DeviceId,
    },
    VersionSummary(Vec<u8>),
    RangeRequest(Vec<u8>),
    OperationChunk(Vec<u8>),
    Ack(Vec<u8>),
    Nack(Vec<u8>),
    SnapshotOffer(Vec<u8>),
    SnapshotRequest(Vec<u8>),
    SnapshotChunk(Vec<u8>),
    Heartbeat,
    Close,
}

impl MessageV1 {
    pub fn kind_code(&self) -> u8 {
        match self {
            MessageV1::Hello { .. } => 0,
            MessageV1::HelloAck { .. } => 1,
            MessageV1::VersionSummary(_) => 2,
            MessageV1::RangeRequest(_) => 3,
            MessageV1::OperationChunk(_) => 4,
            MessageV1::Ack(_) => 5,
            MessageV1::Nack(_) => 6,
            MessageV1::SnapshotOffer(_) => 7,
            MessageV1::SnapshotRequest(_) => 8,
            MessageV1::SnapshotChunk(_) => 9,
            MessageV1::Heartbeat => 10,
            MessageV1::Close => 11,
        }
    }
}

/// An encrypted frame envelope: version, session, monotonic sequence, message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub protocol_major: u16,
    pub session_id: [u8; 16],
    pub sequence: u64,
    pub message: MessageV1,
}
