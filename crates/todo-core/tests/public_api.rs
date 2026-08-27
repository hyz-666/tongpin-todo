//! Compile-time and runtime coverage of the public CoreHandle API.

use std::path::Path;
use std::time::Duration;

use tempfile::tempdir;
use todo_core::{
    CoreError, CoreHandle, OperationSigner, Page, ReplicaState, SignatureBytes, SignatureVerifier,
    TaskQuery,
};
use todo_domain::command::*;
use todo_domain::ids::DeviceId;
use todo_storage::config::{SecretBytes, StorageConfig};

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

fn open(dir: &Path) -> CoreHandle {
    let cfg = StorageConfig {
        profile_path: dir.join("profile.db"),
        database_key: SecretBytes::from_bytes(vec![9; 32]),
        busy_timeout: Duration::from_secs(5),
    };
    CoreHandle::open(
        cfg,
        DeviceId::from_bytes([1; 32]),
        Box::new(NoopSigner),
        Box::new(NoopVerifier),
    )
    .unwrap()
}

#[test]
fn full_api_surface() {
    let dir = tempdir().unwrap();
    let handle = open(dir.path());

    let receipt = handle
        .dispatch(Command::CreateTask(CreateTask {
            title: "买牛奶".to_string(),
            description: String::new(),
            due_date: None,
            due_time: None,
            priority: todo_domain::model::Priority::None,
            list_id: None,
            tags: vec![],
        }))
        .unwrap();
    let id = receipt.affected_entities[0];

    let tasks = handle
        .list_tasks(&TaskQuery::default(), &Page::default(), "2026-09-01")
        .unwrap();
    assert_eq!(tasks.items.len(), 1);

    let details = handle.task_details(&id).unwrap();
    assert_eq!(details.title, "买牛奶");

    let hits = handle.search_tasks("牛奶", 10).unwrap();
    assert_eq!(hits.len(), 1);

    let calendar = handle
        .calendar(todo_domain::clock::YearMonth::new(2026, 9).unwrap())
        .unwrap();
    let _ = calendar;

    let trash = handle.trash().unwrap();
    assert!(trash.is_empty());

    let history = handle.conflict_history(&Page::default()).unwrap();
    let _ = history;

    assert_eq!(handle.replica_state(), ReplicaState::Ready);
    let runtime = handle.runtime_status();
    assert_eq!(runtime.replica, ReplicaState::Ready);
}

#[test]
fn backup_and_close() {
    let dir = tempdir().unwrap();
    let handle = open(dir.path());
    handle
        .dispatch(Command::CreateList(CreateList {
            name: "工作".to_string(),
        }))
        .unwrap();

    let backup = handle.backup("passphrase").unwrap();
    assert!(!backup.is_empty());

    handle.close();
    handle.close(); // idempotent
}
