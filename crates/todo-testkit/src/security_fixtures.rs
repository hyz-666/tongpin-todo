//! Security test fixtures: identity pairs, tampering, and replay detection.

use serde_json::json;
use todo_crypto::{DeviceIdentity, VerifyReason, sign_operation, verify_operation};
use todo_domain::clock::Hlc;
use todo_domain::ids::{DeviceId, EntityId, LifecycleGeneration, OperationId};
use todo_domain::operation::{EntityKind, OperationPayload, VerifiedOperation};
use todo_domain::register::VersionStamp;

/// Build a signed operation from an identity, for a given entity and title.
pub fn signed_operation(
    identity: &DeviceIdentity,
    entity: EntityId,
    title: &str,
    sequence: u64,
) -> (VerifiedOperation, ed25519_dalek::Signature) {
    let op = VerifiedOperation {
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
            field: "title".to_string(),
            value: json!(title),
        },
    };
    let sig = sign_operation(identity, &op).unwrap();
    (op, sig)
}

/// A valid signature verifies under the author's public key.
pub fn verify_valid(
    identity: &DeviceIdentity,
    op: &VerifiedOperation,
    sig: &ed25519_dalek::Signature,
) -> bool {
    verify_operation(identity.signing_public(), op, sig).is_ok()
}

/// Verifying with the wrong key fails with `BadSignature`.
pub fn verify_with_wrong_key(
    wrong: &DeviceIdentity,
    op: &VerifiedOperation,
    sig: &ed25519_dalek::Signature,
) -> VerifyReason {
    verify_operation(wrong.signing_public(), op, sig).unwrap_err()
}

/// Tamper with an operation's payload while keeping the original signature;
/// verification must fail.
pub fn tamper_payload(op: &VerifiedOperation) -> VerifiedOperation {
    let mut t = op.clone();
    t.payload = OperationPayload::SetField {
        field: "title".to_string(),
        value: json!("被篡改的内容"),
    };
    t
}

/// Substitute the author device; the canonical bytes change so the signature
/// no longer verifies.
pub fn substitute_author(op: &VerifiedOperation, other: DeviceId) -> VerifiedOperation {
    let mut t = op.clone();
    t.stamp.device = other;
    t.stamp.operation = OperationId::new(other, t.stamp.operation.sequence);
    t
}
