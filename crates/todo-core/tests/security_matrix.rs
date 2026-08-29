//! Security matrix: signature, tampering, key substitution, and replay.

use todo_crypto::{DeviceIdentity, VerifyReason};
use todo_domain::operation::ReplicaProjection;
use todo_testkit::{
    fixture_entity, signed_operation, substitute_author, tamper_payload, verify_valid,
    verify_with_wrong_key,
};

#[test]
fn valid_signature_verifies() {
    let id = DeviceIdentity::generate();
    let fixture = signed_operation(&id, fixture_entity(1), "title", "\u{5408}\u{6cd5}", 1);
    assert!(verify_valid(&id, &fixture.operation, &fixture.signature));
}

#[test]
fn tampered_payload_is_rejected() {
    let id = DeviceIdentity::generate();
    let fixture = signed_operation(&id, fixture_entity(2), "title", "\u{539f}\u{59cb}", 1);
    let tampered = tamper_payload(&fixture.operation);
    let reason = verify_with_wrong_key(&id, &tampered, &fixture.signature);
    assert_eq!(reason, VerifyReason::BadSignature);
}

#[test]
fn wrong_key_is_rejected() {
    let id = DeviceIdentity::generate();
    let other = DeviceIdentity::generate();
    let fixture = signed_operation(&id, fixture_entity(3), "title", "\u{5185}\u{5bb9}", 1);
    assert_eq!(
        verify_with_wrong_key(&other, &fixture.operation, &fixture.signature),
        VerifyReason::BadSignature
    );
}

#[test]
fn substituted_author_is_rejected() {
    let id = DeviceIdentity::generate();
    let fixture = signed_operation(&id, fixture_entity(4), "title", "\u{5185}\u{5bb9}", 1);
    let substituted = substitute_author(
        &fixture.operation,
        todo_domain::ids::DeviceId::from_bytes([0x99; 32]),
    );
    assert_eq!(
        verify_with_wrong_key(&id, &substituted, &fixture.signature),
        VerifyReason::BadSignature
    );
}

#[test]
fn replay_of_a_valid_operation_is_idempotent() {
    let id = DeviceIdentity::generate();
    let entity = fixture_entity(5);
    let fixture = signed_operation(&id, entity, "title", "\u{91cd}\u{653e}\u{6d4b}\u{8bd5}", 1);
    assert!(verify_valid(&id, &fixture.operation, &fixture.signature));

    // Applying the same verified operation repeatedly converges to one value.
    let mut state = ReplicaProjection::default();
    for _ in 0..3 {
        let _ = todo_domain::operation::apply_operation(&mut state, &fixture.operation);
    }
    assert_eq!(
        state.entities[&entity].fields["title"]
            .value
            .as_str()
            .unwrap(),
        "\u{91cd}\u{653e}\u{6d4b}\u{8bd5}"
    );
}
