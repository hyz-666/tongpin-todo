//! Low-space behavior: read-only transition and hysteresis recovery.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tempfile::tempdir;
use todo_core::{
    Core, CoreError, OperationSigner, ReplicaState, SignatureBytes, SignatureVerifier,
};
use todo_domain::command::*;
use todo_domain::ids::DeviceId;
use todo_storage::config::{SecretBytes, StorageConfig};
use todo_storage::health::SpaceProvider;

struct NoopSigner;
impl OperationSigner for NoopSigner {
    fn sign(&self, _c: &[u8]) -> Result<SignatureBytes, CoreError> {
        Ok(SignatureBytes(vec![0u8; 64]))
    }
}
struct NoopVerifier;
impl SignatureVerifier for NoopVerifier {
    fn verify(&self, _s: &DeviceId, _c: &[u8], _sig: &[u8]) -> Result<(), CoreError> {
        Ok(())
    }
}

#[derive(Clone)]
struct FakeSpace {
    bytes: Arc<Mutex<u64>>,
}
impl SpaceProvider for FakeSpace {
    fn available_bytes(&self) -> u64 {
        *self.bytes.lock().unwrap()
    }
}

fn open_core(dir: &Path, space: FakeSpace, reserve: u64) -> Core {
    let cfg = StorageConfig {
        profile_path: dir.join("profile.db"),
        database_key: SecretBytes::from_bytes(vec![5; 32]),
        busy_timeout: Duration::from_secs(5),
    };
    Core::open_with_space(
        cfg,
        DeviceId::from_bytes([1; 32]),
        Box::new(NoopSigner),
        Box::new(NoopVerifier),
        Box::new(space),
        reserve,
    )
    .unwrap()
}

fn create_task(core: &Core, title: &str) {
    core.dispatch(Command::CreateTask(CreateTask {
        title: title.to_string(),
        description: String::new(),
        due_date: None,
        due_time: None,
        priority: todo_domain::model::Priority::None,
        list_id: None,
        tags: vec![],
    }))
    .unwrap();
}

fn new_space(bytes: u64) -> (FakeSpace, Arc<Mutex<u64>>) {
    let arc = Arc::new(Mutex::new(bytes));
    (FakeSpace { bytes: arc.clone() }, arc)
}

#[test]
fn low_space_transitions_to_read_only() {
    let dir = tempdir().unwrap();
    let (space, bytes) = new_space(1_000_000);
    let core = open_core(dir.path(), space, 10_000);
    create_task(&core, "正常写入");
    assert_eq!(core.replica_state(), ReplicaState::Ready);

    *bytes.lock().unwrap() = 100;
    let result = core.dispatch(Command::CreateList(CreateList {
        name: "会失败".to_string(),
    }));
    assert!(result.is_err());
    assert_eq!(core.replica_state(), ReplicaState::ReadOnlyLowSpace);
}

#[test]
fn hysteresis_recovers_to_ready() {
    let dir = tempdir().unwrap();
    let (space, bytes) = new_space(1_000_000);
    let core = open_core(dir.path(), space, 10_000);
    *bytes.lock().unwrap() = 100;
    assert!(
        core.dispatch(Command::CreateList(CreateList {
            name: "x".to_string()
        }))
        .is_err()
    );
    assert_eq!(core.replica_state(), ReplicaState::ReadOnlyLowSpace);

    *bytes.lock().unwrap() = 1_000_000;
    core.note_space_recovered();
    assert_eq!(core.replica_state(), ReplicaState::Ready);
    create_task(&core, "恢复后写入");
}

#[test]
fn read_only_preserves_existing_data() {
    let dir = tempdir().unwrap();
    let (space, bytes) = new_space(1_000_000);
    let core = open_core(dir.path(), space, 10_000);
    let task = core
        .dispatch(Command::CreateList(CreateList {
            name: "保留".to_string(),
        }))
        .unwrap()
        .affected_entities[0];

    *bytes.lock().unwrap() = 100;
    assert!(
        core.dispatch(Command::CreateList(CreateList {
            name: "失败".to_string()
        }))
        .is_err()
    );

    assert!(!core.is_deleted(&task));
}
