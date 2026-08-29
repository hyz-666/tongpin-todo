//! End-to-end cross-device consistency: real Core instances syncing through
//! the orchestrator's export/apply path and converging.

use std::path::Path;
use std::time::Duration;

use tempfile::tempdir;
use todo_core::{
    Core, CoreError, OperationSigner, SignatureBytes, SignatureVerifier, SignedOperation,
};
use todo_domain::command::{Command, CreateList, EntityRef, ListField, SetListField};
use todo_domain::ids::{DeviceId, EntityId};
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

fn open_core(dir: &Path, device: u8) -> Core {
    let cfg = StorageConfig {
        profile_path: dir.join(format!("profile-{device}.db")),
        database_key: SecretBytes::from_bytes(vec![7; 32]),
        busy_timeout: Duration::from_secs(5),
    };
    let local = DeviceId::from_bytes([device; 32]);
    Core::open(cfg, local, Box::new(NoopSigner), Box::new(OkVerifier)).unwrap()
}

fn device(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 32])
}

fn create_list(core: &Core, name: &str) -> EntityId {
    core.dispatch(Command::CreateList(CreateList {
        name: name.to_string(),
    }))
    .unwrap()
    .affected_entities[0]
}

/// Make every core trust every other device.
fn trust_all(cores: &[&Core]) {
    for core in cores {
        for b in 1..=3 {
            core.add_member(device(b));
        }
    }
}

/// Pull operations originated by `origin` out of `from` and apply to `to`.
/// `frontier` is the origin's highest contiguous sequence; the export range is
/// `[0, frontier + 1)` to cover every committed operation.
fn sync_one(from: &Core, to: &Core, origin: u8, frontier: u64) {
    let ops = from
        .export_operations(&device(origin), 0, frontier + 1)
        .unwrap();
    if !ops.is_empty() {
        to.apply_remote_batch(ops).unwrap();
    }
}

#[test]
fn three_cores_converge_through_full_sync() {
    let dir = tempdir().unwrap();
    let a = open_core(dir.path(), 1);
    let b = open_core(dir.path(), 2);
    let c = open_core(dir.path(), 3);
    trust_all(&[&a, &b, &c]);

    // Each replica originates its own list.
    let a_list = create_list(&a, "A\u{5217}\u{8868}");
    let b_list = create_list(&b, "B\u{5217}\u{8868}");
    let c_list = create_list(&c, "C\u{5217}\u{8868}");

    // Full bidirectional sync of each origin's single operation.
    sync_one(&a, &b, 1, 1);
    sync_one(&a, &c, 1, 1);
    sync_one(&b, &a, 2, 1);
    sync_one(&b, &c, 2, 1);
    sync_one(&c, &a, 3, 1);
    sync_one(&c, &b, 3, 1);

    // All three replicas now materialize all three lists identically.
    for core in [&a, &b, &c] {
        assert_eq!(
            core.field(&a_list, "name").unwrap(),
            Some(serde_json::json!("A\u{5217}\u{8868}"))
        );
        assert_eq!(
            core.field(&b_list, "name").unwrap(),
            Some(serde_json::json!("B\u{5217}\u{8868}"))
        );
        assert_eq!(
            core.field(&c_list, "name").unwrap(),
            Some(serde_json::json!("C\u{5217}\u{8868}"))
        );
    }
}

#[test]
fn duplicate_delivery_converges() {
    let dir = tempdir().unwrap();
    let a = open_core(dir.path(), 1);
    let b = open_core(dir.path(), 2);
    trust_all(&[&a, &b]);

    let a_list = create_list(&a, "\u{91cd}\u{653e}");

    // Deliver A's operation to B three times; idempotency keeps one copy.
    let ops: Vec<SignedOperation> = a.export_operations(&device(1), 0, 2).unwrap();
    for _ in 0..3 {
        b.apply_remote_batch(ops.clone()).unwrap();
    }

    assert_eq!(
        b.field(&a_list, "name").unwrap(),
        Some(serde_json::json!("\u{91cd}\u{653e}"))
    );
}

#[test]
fn multiple_sync_rounds_keep_converging() {
    let dir = tempdir().unwrap();
    let a = open_core(dir.path(), 1);
    let b = open_core(dir.path(), 2);
    trust_all(&[&a, &b]);

    // Round 1: A creates a list, syncs to B.
    let a_list = create_list(&a, "\u{7b2c}\u{4e00}\u{8f6e}");
    sync_one(&a, &b, 1, 1);
    assert_eq!(
        b.field(&a_list, "name").unwrap(),
        Some(serde_json::json!("\u{7b2c}\u{4e00}\u{8f6e}"))
    );

    // Round 2: B creates a list, syncs to A; both have two lists.
    let b_list = create_list(&b, "\u{7b2c}\u{4e8c}\u{8f6e}");
    sync_one(&b, &a, 2, 1);
    assert_eq!(
        a.field(&b_list, "name").unwrap(),
        Some(serde_json::json!("\u{7b2c}\u{4e8c}\u{8f6e}"))
    );

    // Round 3: A edits its list, syncs again; both agree on the new value.
    a.dispatch(Command::SetListField(SetListField {
        list: EntityRef { id: a_list },
        field: ListField::Name("\u{6539}\u{540d}".to_string()),
    }))
    .unwrap();
    sync_one(&a, &b, 1, 2);

    assert_eq!(
        a.field(&a_list, "name").unwrap(),
        Some(serde_json::json!("\u{6539}\u{540d}"))
    );
    assert_eq!(
        b.field(&a_list, "name").unwrap(),
        Some(serde_json::json!("\u{6539}\u{540d}"))
    );
    assert_eq!(
        b.field(&b_list, "name").unwrap(),
        Some(serde_json::json!("\u{7b2c}\u{4e8c}\u{8f6e}"))
    );
}
