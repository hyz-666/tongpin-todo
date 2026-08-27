//! Operation signature: valid, tampered, and wrong-key cases.

use serde_json::json;
use todo_crypto::{DeviceIdentity, VerifyReason, sign_operation, verify_operation};
use todo_domain::clock::Hlc;
use todo_domain::ids::{DeviceId, EntityId, LifecycleGeneration, OperationId};
use todo_domain::operation::{EntityKind, OperationPayload, VerifiedOperation};
use todo_domain::register::VersionStamp;

fn sample_op() -> VerifiedOperation {
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
        payload: OperationPayload::SetField {
            field: "title".to_string(),
            value: json!("买牛奶"),
        },
    }
}

#[test]
fn valid_signature_verifies() {
    let id = DeviceIdentity::generate();
    let op = sample_op();
    let sig = sign_operation(&id, &op).unwrap();
    assert_eq!(verify_operation(id.signing_public(), &op, &sig), Ok(()));
}

#[test]
fn changed_payload_fails() {
    let id = DeviceIdentity::generate();
    let op = sample_op();
    let sig = sign_operation(&id, &op).unwrap();

    let mut tampered = op.clone();
    tampered.payload = OperationPayload::SetField {
        field: "title".to_string(),
        value: json!("被篡改"),
    };
    assert_eq!(
        verify_operation(id.signing_public(), &tampered, &sig),
        Err(VerifyReason::BadSignature)
    );
}

#[test]
fn wrong_key_fails() {
    let id = DeviceIdentity::generate();
    let other = DeviceIdentity::generate();
    let op = sample_op();
    let sig = sign_operation(&id, &op).unwrap();
    assert_eq!(
        verify_operation(other.signing_public(), &op, &sig),
        Err(VerifyReason::BadSignature)
    );
}

#[test]
fn changed_stamp_fails() {
    let id = DeviceIdentity::generate();
    let op = sample_op();
    let sig = sign_operation(&id, &op).unwrap();

    let mut tampered = op.clone();
    tampered.stamp.operation = OperationId::new(DeviceId::from_bytes([7u8; 32]), 2);
    assert_eq!(
        verify_operation(id.signing_public(), &tampered, &sig),
        Err(VerifyReason::BadSignature)
    );
}

#[test]
fn different_device_binding_fails() {
    let id = DeviceIdentity::generate();
    let op = sample_op();
    let sig = sign_operation(&id, &op).unwrap();

    // A substituted author device changes the canonical bytes.
    let mut tampered = op.clone();
    tampered.stamp.device = DeviceId::from_bytes([9u8; 32]);
    assert_eq!(
        verify_operation(id.signing_public(), &tampered, &sig),
        Err(VerifyReason::BadSignature)
    );
}
