//! Fixed-field canonical CBOR encoding for signed operations.

use ciborium::value::{Integer, Value as CborValue};
use serde_json::Value as JsonValue;

use todo_domain::clock::Hlc;
use todo_domain::ids::{DeviceId, EntityId, LifecycleGeneration, OperationId};
use todo_domain::operation::{EntityKind, OperationPayload, VerifiedOperation};
use todo_domain::register::VersionStamp;

use crate::error::CryptoError;

/// Protocol major version bound into the canonical form.
pub const PROTOCOL_MAJOR: u16 = 1;

/// Encode a verified operation as a fixed-order canonical CBOR array.
pub fn encode_operation(op: &VerifiedOperation) -> Result<Vec<u8>, CryptoError> {
    let payload = encode_payload(&op.payload);
    let arr = CborValue::Array(vec![
        CborValue::Integer(PROTOCOL_MAJOR.into()),
        CborValue::Bytes(op.entity.as_bytes().to_vec()),
        CborValue::Integer((kind_code(op.kind) as u64).into()),
        op.parent
            .map(|p| CborValue::Bytes(p.as_bytes().to_vec()))
            .unwrap_or(CborValue::Null),
        CborValue::Integer((op.stamp.generation.0 as u64).into()),
        CborValue::Integer(op.stamp.hlc.physical_millis.into()),
        CborValue::Integer((op.stamp.hlc.logical as u64).into()),
        CborValue::Bytes(op.stamp.device.as_bytes().to_vec()),
        CborValue::Integer(op.stamp.operation.sequence.into()),
        payload,
    ]);
    let mut buf = Vec::new();
    ciborium::into_writer(&arr, &mut buf).map_err(|e| CryptoError::Canonical(e.to_string()))?;
    Ok(buf)
}

fn encode_payload(payload: &OperationPayload) -> CborValue {
    match payload {
        OperationPayload::SetField { field, value } => CborValue::Array(vec![
            CborValue::Integer(0.into()),
            CborValue::Text(field.clone()),
            json_to_cbor(value),
        ]),
        OperationPayload::Delete => CborValue::Array(vec![CborValue::Integer(1.into())]),
        OperationPayload::Restore => CborValue::Array(vec![CborValue::Integer(2.into())]),
    }
}

/// Strictly decode a canonical operation, rejecting noncanonical inputs.
pub fn decode_operation(bytes: &[u8]) -> Result<VerifiedOperation, CryptoError> {
    let mut cursor = std::io::Cursor::new(bytes);
    let value: CborValue =
        ciborium::from_reader(&mut cursor).map_err(|e| CryptoError::Decode(e.to_string()))?;
    if cursor.position() != bytes.len() as u64 {
        return Err(CryptoError::Decode("trailing bytes".into()));
    }
    let CborValue::Array(fields) = value else {
        return Err(CryptoError::Decode("not an array".into()));
    };
    if fields.len() != 10 {
        return Err(CryptoError::Decode(format!(
            "wrong field count {}",
            fields.len()
        )));
    }

    let major = expect_u16(&fields[0])?;
    if major != PROTOCOL_MAJOR {
        return Err(CryptoError::Decode(format!("bad protocol major {major}")));
    }
    let entity = EntityId::from_uuid(uuid_from_field(&fields[1])?);
    let kind = code_kind(expect_u8(&fields[2])?)?;
    let parent = match &fields[3] {
        CborValue::Null => None,
        CborValue::Bytes(_) => Some(EntityId::from_uuid(uuid_from_field(&fields[3])?)),
        _ => return Err(CryptoError::Decode("bad parent".into())),
    };
    let generation = LifecycleGeneration(expect_u32(&fields[4])?);
    let hlc_physical = expect_i64(&fields[5])?;
    let hlc_logical = expect_u32(&fields[6])?;
    let device = device_from_field(&fields[7])?;
    let sequence = expect_u64(&fields[8])?;
    let payload = decode_payload(&fields[9])?;

    Ok(VerifiedOperation {
        entity,
        kind,
        parent,
        stamp: VersionStamp {
            generation,
            hlc: Hlc {
                physical_millis: hlc_physical,
                logical: hlc_logical,
            },
            device,
            operation: OperationId {
                origin: device,
                sequence,
            },
        },
        payload,
    })
}

fn decode_payload(field: &CborValue) -> Result<OperationPayload, CryptoError> {
    let CborValue::Array(items) = field else {
        return Err(CryptoError::Decode("payload not array".into()));
    };
    match expect_u8(&items[0])? {
        0 => {
            let name = expect_text(&items[1])?;
            let value = cbor_to_json(&items[2])?;
            Ok(OperationPayload::SetField { field: name, value })
        }
        1 => Ok(OperationPayload::Delete),
        2 => Ok(OperationPayload::Restore),
        other => Err(CryptoError::Decode(format!("unknown payload kind {other}"))),
    }
}

fn kind_code(kind: EntityKind) -> u8 {
    match kind {
        EntityKind::Task => 0,
        EntityKind::Subtask => 1,
        EntityKind::List => 2,
        EntityKind::Tag => 3,
    }
}

fn code_kind(code: u8) -> Result<EntityKind, CryptoError> {
    match code {
        0 => Ok(EntityKind::Task),
        1 => Ok(EntityKind::Subtask),
        2 => Ok(EntityKind::List),
        3 => Ok(EntityKind::Tag),
        other => Err(CryptoError::Decode(format!("bad kind code {other}"))),
    }
}

fn uuid_from_field(field: &CborValue) -> Result<uuid::Uuid, CryptoError> {
    match field {
        CborValue::Bytes(b) if b.len() == 16 => {
            let arr: [u8; 16] = b
                .as_slice()
                .try_into()
                .map_err(|_| CryptoError::Decode("bad entity bytes".into()))?;
            Ok(uuid::Uuid::from_bytes(arr))
        }
        _ => Err(CryptoError::Decode("bad entity".into())),
    }
}

fn device_from_field(field: &CborValue) -> Result<DeviceId, CryptoError> {
    match field {
        CborValue::Bytes(b) if b.len() == 32 => {
            let arr: [u8; 32] = b
                .as_slice()
                .try_into()
                .map_err(|_| CryptoError::Decode("bad device bytes".into()))?;
            Ok(DeviceId::from_bytes(arr))
        }
        _ => Err(CryptoError::Decode("bad device".into())),
    }
}

fn expect_u16(f: &CborValue) -> Result<u16, CryptoError> {
    match f {
        CborValue::Integer(i) => {
            u16::try_from(*i).map_err(|_| CryptoError::Decode("bad u16".into()))
        }
        _ => Err(CryptoError::Decode("not integer".into())),
    }
}

fn expect_u8(f: &CborValue) -> Result<u8, CryptoError> {
    match f {
        CborValue::Integer(i) => u8::try_from(*i).map_err(|_| CryptoError::Decode("bad u8".into())),
        _ => Err(CryptoError::Decode("not integer".into())),
    }
}

fn expect_u32(f: &CborValue) -> Result<u32, CryptoError> {
    match f {
        CborValue::Integer(i) => {
            u32::try_from(*i).map_err(|_| CryptoError::Decode("bad u32".into()))
        }
        _ => Err(CryptoError::Decode("not integer".into())),
    }
}

fn expect_u64(f: &CborValue) -> Result<u64, CryptoError> {
    match f {
        CborValue::Integer(i) => {
            u64::try_from(*i).map_err(|_| CryptoError::Decode("bad u64".into()))
        }
        _ => Err(CryptoError::Decode("not integer".into())),
    }
}

fn expect_i64(f: &CborValue) -> Result<i64, CryptoError> {
    match f {
        CborValue::Integer(i) => {
            i64::try_from(*i).map_err(|_| CryptoError::Decode("bad i64".into()))
        }
        _ => Err(CryptoError::Decode("not integer".into())),
    }
}

fn expect_text(f: &CborValue) -> Result<String, CryptoError> {
    match f {
        CborValue::Text(s) => Ok(s.clone()),
        _ => Err(CryptoError::Decode("not text".into())),
    }
}

fn json_to_cbor(v: &JsonValue) -> CborValue {
    match v {
        JsonValue::Null => CborValue::Null,
        JsonValue::Bool(b) => CborValue::Bool(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                CborValue::Integer(i.into())
            } else if let Some(u) = n.as_u64() {
                CborValue::Integer(u.into())
            } else {
                CborValue::Float(n.as_f64().unwrap_or(0.0))
            }
        }
        JsonValue::String(s) => CborValue::Text(s.clone()),
        JsonValue::Array(a) => CborValue::Array(a.iter().map(json_to_cbor).collect()),
        JsonValue::Object(m) => CborValue::Map(
            m.iter()
                .map(|(k, v)| (CborValue::Text(k.clone()), json_to_cbor(v)))
                .collect(),
        ),
    }
}

fn cbor_to_json(v: &CborValue) -> Result<JsonValue, CryptoError> {
    Ok(match v {
        CborValue::Null => JsonValue::Null,
        CborValue::Bool(b) => JsonValue::Bool(*b),
        CborValue::Integer(i) => {
            if let Ok(v) = i64::try_from(*i) {
                JsonValue::Number(v.into())
            } else if let Ok(v) = u64::try_from(*i) {
                JsonValue::Number(v.into())
            } else {
                return Err(CryptoError::Decode("integer out of range".into()));
            }
        }
        CborValue::Float(f) => serde_json::Number::from_f64(*f)
            .map(JsonValue::Number)
            .ok_or_else(|| CryptoError::Decode("bad float".into()))?,
        CborValue::Text(s) => JsonValue::String(s.clone()),
        CborValue::Bytes(b) => {
            JsonValue::Array(b.iter().map(|x| JsonValue::Number((*x).into())).collect())
        }
        CborValue::Array(a) => {
            let mut out = Vec::with_capacity(a.len());
            for item in a {
                out.push(cbor_to_json(item)?);
            }
            JsonValue::Array(out)
        }
        CborValue::Map(m) => {
            let mut map = serde_json::Map::new();
            for (k, val) in m {
                let CborValue::Text(key) = k else {
                    return Err(CryptoError::Decode("non-text map key".into()));
                };
                map.insert(key.clone(), cbor_to_json(val)?);
            }
            JsonValue::Object(map)
        }
        _ => return Err(CryptoError::Decode("unsupported cbor value".into())),
    })
}

// Silence an unused-import lint on the Integer re-export when only built in
// some configurations.
#[allow(dead_code)]
fn _integer_type_used(_: Integer) {}
