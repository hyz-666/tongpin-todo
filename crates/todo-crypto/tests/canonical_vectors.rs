//! Canonical operation encoding: round-trip and strict rejection.

use serde_json::json;
use todo_crypto::{decode_operation, encode_operation};
use todo_domain::clock::Hlc;
use todo_domain::ids::{DeviceId, EntityId, LifecycleGeneration, OperationId};
use todo_domain::operation::{EntityKind, OperationPayload, VerifiedOperation};
use todo_domain::register::VersionStamp;

fn sample_op(payload: OperationPayload) -> VerifiedOperation {
    VerifiedOperation {
        entity: EntityId::from_uuid(uuid::Uuid::nil()),
        kind: EntityKind::Task,
        parent: None,
        stamp: VersionStamp {
            generation: LifecycleGeneration(1),
            hlc: Hlc {
                physical_millis: 1_700_000_000_000,
                logical: 0,
            },
            device: DeviceId::from_bytes([7u8; 32]),
            operation: OperationId::new(DeviceId::from_bytes([7u8; 32]), 1),
        },
        payload,
    }
}

#[test]
fn round_trip_each_payload_kind() {
    let cases = vec![
        OperationPayload::SetField {
            field: "title".to_string(),
            value: json!("买牛奶 🥛"),
        },
        OperationPayload::SetField {
            field: "tags".to_string(),
            value: json!(["a", "b"]),
        },
        OperationPayload::Delete,
        OperationPayload::Restore,
    ];
    for payload in cases {
        let op = sample_op(payload.clone());
        let bytes = encode_operation(&op).unwrap();
        let decoded = decode_operation(&bytes).unwrap();
        assert_eq!(op.entity, decoded.entity);
        assert_eq!(op.kind, decoded.kind);
        assert_eq!(op.stamp.generation, decoded.stamp.generation);
        assert_eq!(op.stamp.hlc, decoded.stamp.hlc);
        assert_eq!(op.stamp.device, decoded.stamp.device);
        assert_eq!(op.stamp.operation, decoded.stamp.operation);
        assert_eq!(payload, decoded.payload);
    }
}

#[test]
fn encoding_is_deterministic() {
    let op = sample_op(OperationPayload::SetField {
        field: "title".to_string(),
        value: json!("任务"),
    });
    let a = encode_operation(&op).unwrap();
    let b = encode_operation(&op).unwrap();
    assert_eq!(a, b);
}

#[test]
fn truncated_bytes_rejected() {
    let op = sample_op(OperationPayload::Delete);
    let bytes = encode_operation(&op).unwrap();
    for n in [0, 1, bytes.len() / 2] {
        assert!(
            decode_operation(&bytes[..n]).is_err(),
            "len {n} should fail"
        );
    }
}

#[test]
fn trailing_garbage_rejected() {
    let op = sample_op(OperationPayload::Restore);
    let mut bytes = encode_operation(&op).unwrap();
    bytes.push(0xFF);
    // ciborium decodes the first value and ignores trailing bytes only if we
    // explicitly allow it; here we require the whole buffer to be one value.
    assert!(decode_operation(&bytes).is_err());
}

#[test]
fn noncanonical_integer_rejected() {
    // A manually built value with a non-minimal integer encoding (0x1a 00 00 00 01)
    // must be rejected as noncanonical.
    let bad = vec![
        0x9A, // array(10)
        0x01, // protocol major
        0x50, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,    // entity bytes(16)
        0x00, // kind
        0xF6, // null parent
        0x1A, 0x00, 0x00, 0x00, 0x01, // noncanonical generation (u32 as 4-byte int)
        0x1B, 0x00, 0x01, 0x8C, 0x0C, 0x7D, 0x0F, 0x40, 0x00, // hlc physical
        0x00, // logical
        0x58, 0x20, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
        0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
        0x07, 0x07, 0x07, 0x07, // device bytes(32)
        0x01, // sequence
        0x81, 0x01, // payload = [Delete]
    ];
    assert!(decode_operation(&bad).is_err());
}

#[test]
fn indefinite_array_rejected() {
    // 0x9F starts an indefinite-length array.
    let bad = vec![0x9F, 0x01, 0xFF];
    assert!(decode_operation(&bad).is_err());
}

#[test]
fn wrong_field_count_rejected() {
    // A 9-element array is the wrong shape (needs exactly 10 fields).
    let mut bad = Vec::new();
    ciborium::into_writer(
        &ciborium::Value::Array(vec![ciborium::Value::Null; 9]),
        &mut bad,
    )
    .unwrap();
    assert!(decode_operation(&bad).is_err());
}

#[test]
fn unknown_kind_rejected() {
    let op = sample_op(OperationPayload::Delete);
    let bytes = encode_operation(&op).unwrap();
    // Rewrite the kind field (index 2) to an out-of-range code.
    let mut value: ciborium::Value = ciborium::from_reader(bytes.as_slice()).unwrap();
    if let ciborium::Value::Array(ref mut fields) = value {
        fields[2] = ciborium::Value::Integer(9.into());
    }
    let mut bad = Vec::new();
    ciborium::into_writer(&value, &mut bad).unwrap();
    assert!(decode_operation(&bad).is_err());
}
