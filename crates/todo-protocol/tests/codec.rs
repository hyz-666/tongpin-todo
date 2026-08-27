//! Frame codec round-trip and limits.

use todo_domain::ids::DeviceId;
use todo_protocol::{Frame, MessageV1, ProtocolError, decode_frame, encode_frame};

fn device() -> DeviceId {
    DeviceId::from_bytes([3u8; 32])
}

fn frame(message: MessageV1) -> Frame {
    Frame {
        protocol_major: 1,
        session_id: [0xAB; 16],
        sequence: 7,
        message,
    }
}

#[test]
fn round_trip_hello() {
    let f = frame(MessageV1::Hello {
        protocol_minor: 0,
        schema: 1,
        capabilities: 0,
        device_id: device(),
    });
    let bytes = encode_frame(&f).unwrap();
    let decoded = decode_frame(&bytes).unwrap();
    assert_eq!(f, decoded);
}

#[test]
fn round_trip_every_message() {
    let messages = vec![
        MessageV1::Hello {
            protocol_minor: 0,
            schema: 1,
            capabilities: 0,
            device_id: device(),
        },
        MessageV1::HelloAck {
            protocol_minor: 0,
            schema: 1,
            capabilities: 0,
            device_id: device(),
        },
        MessageV1::VersionSummary(vec![1, 2, 3]),
        MessageV1::RangeRequest(vec![4, 5]),
        MessageV1::OperationChunk(vec![6]),
        MessageV1::Ack(vec![7, 8, 9, 10]),
        MessageV1::Nack(vec![]),
        MessageV1::SnapshotOffer(vec![11]),
        MessageV1::SnapshotRequest(vec![12]),
        MessageV1::SnapshotChunk(vec![13, 14]),
        MessageV1::Heartbeat,
        MessageV1::Close,
    ];
    for m in messages {
        let f = frame(m);
        let bytes = encode_frame(&f).unwrap();
        let decoded = decode_frame(&bytes).unwrap();
        assert_eq!(f, decoded);
    }
}

#[test]
fn oversize_frame_rejected() {
    let big = vec![0u8; 70 * 1024]; // > 64 KiB
    let f = frame(MessageV1::OperationChunk(big));
    let bytes = encode_frame(&f).unwrap();
    assert!(matches!(
        decode_frame(&bytes),
        Err(ProtocolError::FrameTooLarge)
    ));
}

#[test]
fn sequence_is_preserved() {
    let f = Frame {
        protocol_major: 1,
        session_id: [0x01; 16],
        sequence: u64::MAX,
        message: MessageV1::Heartbeat,
    };
    let decoded = decode_frame(&encode_frame(&f).unwrap()).unwrap();
    assert_eq!(decoded.sequence, u64::MAX);
}
