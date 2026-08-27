//! Length-bounded canonical CBOR frame codec.

use std::io::Cursor;

use ciborium::value::{Integer, Value as CborValue};

use todo_domain::ids::DeviceId;

use crate::error::ProtocolError;
use crate::frame::{Frame, MessageV1};
use crate::limits::MAX_FRAME_SIZE;

/// Encode a frame as a fixed-order canonical CBOR array.
pub fn encode_frame(frame: &Frame) -> Result<Vec<u8>, ProtocolError> {
    let message = encode_message(&frame.message);
    let arr = CborValue::Array(vec![
        CborValue::Integer((frame.protocol_major as u64).into()),
        CborValue::Bytes(frame.session_id.to_vec()),
        CborValue::Integer(frame.sequence.into()),
        message,
    ]);
    let mut buf = Vec::new();
    ciborium::into_writer(&arr, &mut buf).map_err(|_| ProtocolError::Decode)?;
    Ok(buf)
}

fn encode_message(m: &MessageV1) -> CborValue {
    match m {
        MessageV1::Hello {
            protocol_minor,
            schema,
            capabilities,
            device_id,
        } => CborValue::Array(vec![
            CborValue::Integer(0.into()),
            CborValue::Integer((*protocol_minor as u64).into()),
            CborValue::Integer((*schema as u64).into()),
            CborValue::Integer((*capabilities).into()),
            CborValue::Bytes(device_id.as_bytes().to_vec()),
        ]),
        MessageV1::HelloAck {
            protocol_minor,
            schema,
            capabilities,
            device_id,
        } => CborValue::Array(vec![
            CborValue::Integer(1.into()),
            CborValue::Integer((*protocol_minor as u64).into()),
            CborValue::Integer((*schema as u64).into()),
            CborValue::Integer((*capabilities).into()),
            CborValue::Bytes(device_id.as_bytes().to_vec()),
        ]),
        MessageV1::VersionSummary(data)
        | MessageV1::RangeRequest(data)
        | MessageV1::OperationChunk(data)
        | MessageV1::Ack(data)
        | MessageV1::Nack(data)
        | MessageV1::SnapshotOffer(data)
        | MessageV1::SnapshotRequest(data)
        | MessageV1::SnapshotChunk(data) => CborValue::Array(vec![
            CborValue::Integer((m.kind_code() as u64).into()),
            CborValue::Bytes(data.clone()),
        ]),
        MessageV1::Heartbeat => CborValue::Array(vec![CborValue::Integer(10.into())]),
        MessageV1::Close => CborValue::Array(vec![CborValue::Integer(11.into())]),
    }
}

/// Decode a frame, rejecting oversized, truncated, and noncanonical inputs.
pub fn decode_frame(bytes: &[u8]) -> Result<Frame, ProtocolError> {
    if bytes.len() > MAX_FRAME_SIZE {
        return Err(ProtocolError::FrameTooLarge);
    }
    let mut cursor = Cursor::new(bytes);
    let value: CborValue =
        ciborium::from_reader(&mut cursor).map_err(|_| ProtocolError::MalformedFrame)?;
    if cursor.position() != bytes.len() as u64 {
        return Err(ProtocolError::MalformedFrame);
    }
    let CborValue::Array(fields) = value else {
        return Err(ProtocolError::MalformedFrame);
    };
    if fields.len() != 4 {
        return Err(ProtocolError::MalformedFrame);
    }
    let protocol_major = expect_u16(&fields[0])?;
    let session_id = expect_16(&fields[1])?;
    let sequence = expect_u64(&fields[2])?;
    let message = decode_message(&fields[3])?;
    Ok(Frame {
        protocol_major,
        session_id,
        sequence,
        message,
    })
}

fn decode_message(field: &CborValue) -> Result<MessageV1, ProtocolError> {
    let CborValue::Array(items) = field else {
        return Err(ProtocolError::MalformedFrame);
    };
    let kind = expect_u8(items.first().ok_or(ProtocolError::MalformedFrame)?)?;
    match kind {
        0 | 1 => {
            if items.len() != 5 {
                return Err(ProtocolError::MalformedFrame);
            }
            let protocol_minor = expect_u16(&items[1])?;
            let schema = expect_u16(&items[2])?;
            let capabilities = expect_u64(&items[3])?;
            let device_id = expect_device(&items[4])?;
            let m = if kind == 0 {
                MessageV1::Hello {
                    protocol_minor,
                    schema,
                    capabilities,
                    device_id,
                }
            } else {
                MessageV1::HelloAck {
                    protocol_minor,
                    schema,
                    capabilities,
                    device_id,
                }
            };
            Ok(m)
        }
        2..=9 => {
            if items.len() != 2 {
                return Err(ProtocolError::MalformedFrame);
            }
            let data = expect_bytes(&items[1])?;
            Ok(match kind {
                2 => MessageV1::VersionSummary(data),
                3 => MessageV1::RangeRequest(data),
                4 => MessageV1::OperationChunk(data),
                5 => MessageV1::Ack(data),
                6 => MessageV1::Nack(data),
                7 => MessageV1::SnapshotOffer(data),
                8 => MessageV1::SnapshotRequest(data),
                _ => MessageV1::SnapshotChunk(data),
            })
        }
        10 => Ok(MessageV1::Heartbeat),
        11 => Ok(MessageV1::Close),
        _ => Err(ProtocolError::MalformedFrame),
    }
}

fn expect_u8(f: &CborValue) -> Result<u8, ProtocolError> {
    match f {
        CborValue::Integer(i) => u8::try_from(*i).map_err(|_| ProtocolError::MalformedFrame),
        _ => Err(ProtocolError::MalformedFrame),
    }
}

fn expect_u16(f: &CborValue) -> Result<u16, ProtocolError> {
    match f {
        CborValue::Integer(i) => u16::try_from(*i).map_err(|_| ProtocolError::MalformedFrame),
        _ => Err(ProtocolError::MalformedFrame),
    }
}

fn expect_u64(f: &CborValue) -> Result<u64, ProtocolError> {
    match f {
        CborValue::Integer(i) => u64::try_from(*i).map_err(|_| ProtocolError::MalformedFrame),
        _ => Err(ProtocolError::MalformedFrame),
    }
}

fn expect_16(f: &CborValue) -> Result<[u8; 16], ProtocolError> {
    match f {
        CborValue::Bytes(b) if b.len() == 16 => Ok(b
            .as_slice()
            .try_into()
            .map_err(|_| ProtocolError::MalformedFrame)?),
        _ => Err(ProtocolError::MalformedFrame),
    }
}

fn expect_device(f: &CborValue) -> Result<DeviceId, ProtocolError> {
    match f {
        CborValue::Bytes(b) if b.len() == 32 => {
            let arr: [u8; 32] = b
                .as_slice()
                .try_into()
                .map_err(|_| ProtocolError::MalformedFrame)?;
            Ok(DeviceId::from_bytes(arr))
        }
        _ => Err(ProtocolError::MalformedFrame),
    }
}

fn expect_bytes(f: &CborValue) -> Result<Vec<u8>, ProtocolError> {
    match f {
        CborValue::Bytes(b) => Ok(b.clone()),
        _ => Err(ProtocolError::MalformedFrame),
    }
}

// Keep the Integer import referenced for value construction clarity.
#[allow(dead_code)]
fn _integer(_: Integer) {}
