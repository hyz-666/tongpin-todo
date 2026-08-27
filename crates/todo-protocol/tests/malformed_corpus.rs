//! Malformed-input rejection and fuzz-style no-panic corpus.

use todo_protocol::{Frame, MessageV1, decode_frame, encode_frame};

#[test]
fn empty_input_rejected() {
    assert!(decode_frame(&[]).is_err());
    assert!(decode_frame(&[0x9F]).is_err()); // indefinite array
    assert!(decode_frame(&[0xFF]).is_err()); // invalid initial byte
}

#[test]
fn truncated_frame_rejected() {
    let f = Frame {
        protocol_major: 1,
        session_id: [0xAB; 16],
        sequence: 1,
        message: MessageV1::VersionSummary(vec![1, 2, 3, 4, 5, 6, 7, 8]),
    };
    let bytes = encode_frame(&f).unwrap();
    for n in 1..bytes.len() {
        assert!(decode_frame(&bytes[..n]).is_err(), "len {n} should fail");
    }
}

#[test]
fn trailing_garbage_rejected() {
    let f = Frame {
        protocol_major: 1,
        session_id: [0xAB; 16],
        sequence: 1,
        message: MessageV1::Heartbeat,
    };
    let mut bytes = encode_frame(&f).unwrap();
    bytes.push(0x00);
    assert!(decode_frame(&bytes).is_err());
}

#[test]
fn wrong_field_count_rejected() {
    // A 3-element array is not a valid frame (needs 4).
    let mut bad = Vec::new();
    ciborium::into_writer(
        &ciborium::Value::Array(vec![ciborium::Value::Null; 3]),
        &mut bad,
    )
    .unwrap();
    assert!(decode_frame(&bad).is_err());
}

#[test]
fn invalid_enum_rejected() {
    let f = Frame {
        protocol_major: 1,
        session_id: [0xAB; 16],
        sequence: 1,
        message: MessageV1::Heartbeat,
    };
    let bytes = encode_frame(&f).unwrap();
    let mut value: ciborium::Value = ciborium::from_reader(bytes.as_slice()).unwrap();
    // Rewrite the message kind (index 3 -> message array index 0) to 99.
    if let ciborium::Value::Array(ref mut fields) = value
        && let ciborium::Value::Array(ref mut msg) = fields[3]
    {
        msg[0] = ciborium::Value::Integer(99.into());
    }
    let mut bad = Vec::new();
    ciborium::into_writer(&value, &mut bad).unwrap();
    assert!(decode_frame(&bad).is_err());
}

#[test]
fn thousand_random_inputs_do_not_panic() {
    // Deterministic pseudo-random byte sequences must be rejected, never panic.
    let mut state = 0x1234_5678_9abc_def0u64;
    for _ in 0..1000 {
        let len = (state % 200) as usize;
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            bytes.push((state >> 33) as u8);
        }
        let _ = decode_frame(&bytes); // must not panic
    }
}
