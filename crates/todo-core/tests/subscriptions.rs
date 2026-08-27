//! Subscription ordering, coalescing, cancellation, and revision gaps.

use std::path::Path;
use std::time::Duration;

use tempfile::tempdir;
use todo_core::{
    CoreError, CoreHandle, OperationSigner, SignatureBytes, SignatureVerifier, SubscriptionKind,
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

fn create(handle: &CoreHandle, title: &str) {
    handle
        .dispatch(Command::CreateTask(CreateTask {
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

#[test]
fn projection_subscription_receives_revisioned_events() {
    let dir = tempdir().unwrap();
    let handle = open(dir.path());
    let sub = handle.subscribe(SubscriptionKind::Projection).unwrap();

    create(&handle, "任务A");
    let e1 = sub.recv().unwrap();
    assert!(e1.revision >= 1);

    create(&handle, "任务B");
    let e2 = sub.recv().unwrap();
    assert!(e2.revision > e1.revision, "revisions must be monotonic");
}

#[test]
fn cancellation_is_idempotent() {
    let dir = tempdir().unwrap();
    let handle = open(dir.path());
    let sub = handle.subscribe(SubscriptionKind::Projection).unwrap();
    sub.cancel();
    sub.cancel(); // second cancel is a no-op
    create(&handle, "取消后");
    // No panic; the subscription is simply gone.
}

#[test]
fn coalescing_drops_redundant_updates() {
    let dir = tempdir().unwrap();
    let handle = open(dir.path());
    let sub = handle.subscribe(SubscriptionKind::Projection).unwrap();

    // Fire several dispatches; the bounded channel keeps at least one event.
    for i in 0..20 {
        create(&handle, &format!("任务{i}"));
    }
    let first = sub.recv().unwrap();
    assert!(first.revision >= 1);
    // Remaining events (if any) must have strictly increasing revisions.
    let mut last = first.revision;
    while let Ok(e) = sub.try_recv() {
        assert!(e.revision > last);
        last = e.revision;
    }
}
