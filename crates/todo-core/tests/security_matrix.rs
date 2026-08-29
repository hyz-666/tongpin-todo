//! Security matrix: signature, tampering, key substitution, and replay.

use todo_crypto::{DeviceIdentity, VerifyReason};
use todo_domain::ids::{DeviceId, EntityId};
use todo_domain::operation::ReplicaProjection;
use todo_testkit::{
    signed_operation, substitute_author, tamper_payload, verify_valid, verify_with_wrong_key,
};

#[test]
fn valid_signature_verifies() {
    let id = DeviceIdentity::generate();
    let (op, sig) = signed_operation(&id, EntityId::from_uuid(uuid::Uuid::new_v4()), "合法", 1);
    assert!(verify_valid(&id, &op, &sig));
}

#[test]
fn tampered_payload_is_rejected() {
    let id = DeviceIdentity::generate();
    let (op, sig) = signed_operation(&id, EntityId::from_uuid(uuid::Uuid::new_v4()), "原始", 1);
    let tampered = tamper_payload(&op);
    // Verifying the tampered content against the original signature fails.
    let reason = verify_with_wrong_key(&id, &tampered, &sig);
    assert_eq!(reason, VerifyReason::BadSignature);
}

#[test]
fn wrong_key_is_rejected() {
    let id = DeviceIdentity::generate();
    let other = DeviceIdentity::generate();
    let (op, sig) = signed_operation(&id, EntityId::from_uuid(uuid::Uuid::new_v4()), "内容", 1);
    assert_eq!(
        verify_with_wrong_key(&other, &op, &sig),
        VerifyReason::BadSignature
    );
}

#[test]
fn substituted_author_is_rejected() {
    let id = DeviceIdentity::generate();
    let (op, sig) = signed_operation(&id, EntityId::from_uuid(uuid::Uuid::new_v4()), "内容", 1);
    let substituted = substitute_author(&op, DeviceId::from_bytes([0x99; 32]));
    assert_eq!(
        verify_with_wrong_key(&id, &substituted, &sig),
        VerifyReason::BadSignature
    );
}

#[test]
fn replay_of_a_valid_operation_is_idempotent() {
    let id = DeviceIdentity::generate();
    let entity = EntityId::from_uuid(uuid::Uuid::new_v4());
    let (op, sig) = signed_operation(&id, entity, "重放测试", 1);
    assert!(verify_valid(&id, &op, &sig));

    // Applying the same verified operation repeatedly converges to one value.
    let mut state = ReplicaProjection::default();
    for _ in 0..3 {
        let _ = todo_domain::operation::apply_operation(&mut state, &op);
    }
    assert_eq!(
        state.entities[&entity].fields["title"]
            .value
            .as_str()
            .unwrap(),
        "重放测试"
    );
}
