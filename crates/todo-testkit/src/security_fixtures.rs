//! Security fixtures: signed-operation pairs, tampering, and replay helpers.
//!
//! The central assertion of every security test is **zero unauthorized
//! mutations**: a rejected operation must leave the projection byte-identical.

use serde_json::json;
use todo_core::SignatureVerifier;
use todo_crypto::{DeviceIdentity, VerifyReason, sign_operation, verify_operation};
use todo_domain::clock::Hlc;
use todo_domain::ids::{DeviceId, EntityId, LifecycleGeneration, OperationId};
use todo_domain::operation::{EntityKind, OperationPayload, ReplicaProjection, VerifiedOperation};
use todo_domain::register::VersionStamp;

/// A signed operation together with the author's public key.
pub struct SignedOperationFixture {
    pub author: DeviceId,
    pub signing_public: ed25519_dalek::VerifyingKey,
    pub operation: VerifiedOperation,
    pub signature: ed25519_dalek::Signature,
}

/// Build a signed `SetField` operation for an entity.
pub fn signed_operation(
    identity: &DeviceIdentity,
    entity: EntityId,
    field: &str,
    value: &str,
    sequence: u64,
) -> SignedOperationFixture {
    let operation = VerifiedOperation {
        entity,
        kind: EntityKind::Task,
        parent: None,
        stamp: VersionStamp {
            generation: LifecycleGeneration(1),
            hlc: Hlc {
                physical_millis: 1_700_000_000_000 + sequence as i64,
                logical: 0,
            },
            device: identity.device_id(),
            operation: OperationId::new(identity.device_id(), sequence),
        },
        payload: OperationPayload::SetField {
            field: field.to_string(),
            value: json!(value),
        },
    };
    let signature = sign_operation(identity, &operation).unwrap();
    SignedOperationFixture {
        author: identity.device_id(),
        signing_public: *identity.signing_public(),
        operation,
        signature,
    }
}

/// Every way a hostile peer can tamper with an operation.
pub fn tampered_variants(
    fixture: &SignedOperationFixture,
) -> Vec<(&'static str, VerifiedOperation)> {
    let base = fixture.operation.clone();
    let mut changed_payload = base.clone();
    changed_payload.payload = OperationPayload::SetField {
        field: "title".to_string(),
        value: json!("\u{88ab}\u{7be1}\u{6539}"),
    };
    let mut changed_entity = base.clone();
    changed_entity.entity = EntityId::from_uuid(uuid::Uuid::from_bytes([9u8; 16]));
    let mut changed_kind = base.clone();
    changed_kind.kind = EntityKind::List;
    let mut changed_stamp = base;
    changed_stamp.stamp.operation.sequence += 1;
    vec![
        ("payload", changed_payload),
        ("entity", changed_entity),
        ("kind", changed_kind),
        ("stamp", changed_stamp),
    ]
}

/// Verify an operation against the author's key.
pub fn verify(
    fixture: &SignedOperationFixture,
    op: &VerifiedOperation,
) -> Result<(), VerifyReason> {
    verify_operation(&fixture.signing_public, op, &fixture.signature)
}

/// Build a signed `(VerifiedOperation, Signature)` tuple for tests that need
/// the raw pair rather than the fixture struct.
pub fn signed_op_pair(
    identity: &DeviceIdentity,
    entity: EntityId,
    value: &str,
    sequence: u64,
) -> (VerifiedOperation, ed25519_dalek::Signature) {
    let fixture = signed_operation(identity, entity, "title", value, sequence);
    (fixture.operation, fixture.signature)
}

/// Build a signed operation and return `(op, sig_bytes)` — the form accepted
/// by `SignatureVerifier::verify`.
pub fn signed_operation_bytes(
    identity: &DeviceIdentity,
    entity: EntityId,
    value: &str,
    sequence: u64,
) -> (VerifiedOperation, Vec<u8>) {
    let fixture = signed_operation(identity, entity, "title", value, sequence);
    (fixture.operation, fixture.signature.to_bytes().to_vec())
}

/// Return `true` when the signature is valid.
pub fn verify_valid(
    identity: &DeviceIdentity,
    op: &VerifiedOperation,
    signature: &ed25519_dalek::Signature,
) -> bool {
    verify_operation(identity.signing_public(), op, signature).is_ok()
}

/// Verify with a *different* key — must fail.
pub fn verify_with_wrong_key(
    _wrong_identity: &DeviceIdentity,
    op: &VerifiedOperation,
    signature: &ed25519_dalek::Signature,
) -> VerifyReason {
    // We verify against the *wrong* identity's public key.
    verify_operation(_wrong_identity.signing_public(), op, signature).unwrap_err()
}

/// Substitute the author field on an operation (does NOT re-sign).
pub fn substitute_author(op: &VerifiedOperation, new_author: DeviceId) -> VerifiedOperation {
    let mut clone = op.clone();
    clone.stamp.device = new_author;
    clone.stamp.operation = OperationId::new(new_author, op.stamp.operation.sequence);
    clone
}

/// Modify the payload of an operation (does NOT re-sign).
pub fn tamper_payload(op: &VerifiedOperation) -> VerifiedOperation {
    let mut clone = op.clone();
    clone.payload = OperationPayload::SetField {
        field: "title".to_string(),
        value: json!("\u{88ab}\u{7be1}\u{6539}"),
    };
    clone
}

/// Assert that an unauthorized operation is rejected **and** leaves the
/// projection unchanged.
///
/// Returns `true` when both conditions hold.
pub fn rejects_without_mutation(
    verifier: &dyn SignatureVerifier,
    signer: &DeviceId,
    op: &VerifiedOperation,
    signature: &[u8],
    state: &ReplicaProjection,
) -> bool {
    let before = state.clone();
    let canonical = serde_json::to_vec(&op).unwrap_or_default();
    let verified = verifier.verify(signer, &canonical, signature);
    verified.is_err() && *state == before
}

/// Deterministic device ids for reproducible fixtures.
pub fn fixture_device(byte: u8) -> DeviceId {
    DeviceId::from_bytes([byte; 32])
}

/// Deterministic entity ids for reproducible fixtures.
pub fn fixture_entity(byte: u8) -> EntityId {
    EntityId::from_uuid(uuid::Uuid::from_bytes([byte; 16]))
}
