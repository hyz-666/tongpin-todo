//! Remote batch application: verification, idempotency, gaps, and ordering.

use std::path::Path;
use std::time::Duration;

use tempfile::tempdir;
use todo_core::{
    Core, CoreError, OperationSigner, SignatureBytes, SignatureVerifier, SignedOperation,
};
use todo_domain::clock::Hlc;
use todo_domain::command::Command;
use todo_domain::ids::{DeviceId, EntityId, LifecycleGeneration, OperationId};
use todo_domain::operation::{EntityKind, OperationPayload, VerifiedOperation};
use todo_domain::register::VersionStamp;
use todo_storage::config::{SecretBytes, StorageConfig};

struct NoopSigner;
impl OperationSigner for NoopSigner {
    fn sign(&self, _c: &[u8]) -> Result<SignatureBytes, CoreError> {
        Ok(SignatureBytes(vec![0u8; 64]))
    }
}

struct OkVerifier;
impl SignatureVerifier for OkVerifier {
    fn verify(&self, _s: &DeviceId, _c: &[u8], _sig: &[u8]) -> Result<(), CoreError> {
        Ok(())
    }
}

struct RejectVerifier;
impl SignatureVerifier for RejectVerifier {
    fn verify(&self, _s: &DeviceId, _c: &[u8], _sig: &[u8]) -> Result<(), CoreError> {
        Err(CoreError::BadSignature)
    }
}

fn open_core(dir: &Path, verifier: Box<dyn SignatureVerifier>) -> Core {
    let cfg = StorageConfig {
        profile_path: dir.join("profile.db"),
        database_key: SecretBytes::from_bytes(vec![7; 32]),
        busy_timeout: Duration::from_secs(5),
    };
    let local = DeviceId::from_bytes([1; 32]);
    Core::open(cfg, local, Box::new(NoopSigner), verifier).unwrap()
}

fn remote_device(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 32])
}

fn set_field_op(
    entity: EntityId,
    field: &str,
    value: &str,
    sequence: u64,
    device: u8,
) -> SignedOperation {
    let device = remote_device(device);
    let stamp = VersionStamp {
        generation: LifecycleGeneration(1),
        hlc: Hlc::new(100, 0),
        device,
        operation: OperationId::new(device, sequence),
    };
    SignedOperation {
        signer: device,
        signature: SignatureBytes(vec![0u8; 64]),
        operation: VerifiedOperation {
            entity,
            kind: EntityKind::Task,
            parent: None,
            stamp,
            payload: OperationPayload::SetField {
                field: field.to_string(),
                value: serde_json::json!(value),
            },
        },
    }
}

#[test]
fn bad_signature_is_rejected() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path(), Box::new(RejectVerifier));
    core.add_member(remote_device(2));
    let op = set_field_op(EntityId::new_v7_for_test(1), "title", "x", 1, 2);
    let result = core.apply_remote_batch(vec![op]);
    assert!(matches!(result, Err(CoreError::BadSignature)));
}

#[test]
fn duplicate_operation_is_idempotent() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path(), Box::new(OkVerifier));
    core.add_member(remote_device(2));
    let op = set_field_op(EntityId::new_v7_for_test(2), "title", "x", 1, 2);
    let r1 = core.apply_remote_batch(vec![op.clone()]).unwrap();
    assert_eq!(r1.applied, 1);
    let r2 = core.apply_remote_batch(vec![op]).unwrap();
    assert_eq!(r2.applied, 0);
    assert_eq!(r2.duplicated, 1);
}

#[test]
fn out_of_order_is_rejected_with_gap() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path(), Box::new(OkVerifier));
    core.add_member(remote_device(2));
    let op2 = set_field_op(EntityId::new_v7_for_test(3), "title", "two", 2, 2);
    // Sequence 2 with no 1 received first -> origin gap.
    let result = core.apply_remote_batch(vec![op2]);
    assert!(matches!(result, Err(CoreError::OriginGap)));
}

#[test]
fn stale_generation_is_ignored() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path(), Box::new(OkVerifier));
    // Local dispatch creates generation 1, then a remote op at generation 0 is stale.
    let id = core
        .dispatch(Command::CreateList(todo_domain::command::CreateList {
            name: "本地".to_string(),
        }))
        .unwrap()
        .affected_entities[0];
    let device = remote_device(2);
    let stale = SignedOperation {
        signer: device,
        signature: SignatureBytes(vec![0u8; 64]),
        operation: VerifiedOperation {
            entity: id,
            kind: EntityKind::List,
            parent: None,
            stamp: VersionStamp {
                generation: LifecycleGeneration(0),
                hlc: Hlc::new(999, 0),
                device,
                operation: OperationId::new(device, 1),
            },
            payload: OperationPayload::SetField {
                field: "name".to_string(),
                value: serde_json::json!("陈旧"),
            },
        },
    };
    // Register the remote device as known so it isn't a member failure.
    core.add_member(device);
    let r = core.apply_remote_batch(vec![stale]).unwrap();
    assert_eq!(r.applied, 0);
    assert_eq!(
        core.field(&id, "name").unwrap().unwrap(),
        serde_json::json!("本地")
    );
}
