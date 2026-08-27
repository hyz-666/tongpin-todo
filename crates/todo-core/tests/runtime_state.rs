//! Runtime status: replica, peer reachability, and trust state.

use std::path::Path;
use std::time::Duration;

use tempfile::tempdir;
use todo_core::{
    CoreError, CoreHandle, OperationSigner, ReplicaState, SignatureBytes, SignatureVerifier,
};
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
fn initial_runtime_status_is_ready() {
    let dir = tempdir().unwrap();
    let handle = open(dir.path());
    let runtime = handle.runtime_status();
    assert_eq!(runtime.replica, ReplicaState::Ready);
    assert!(runtime.peers.is_empty());
}

#[test]
fn peer_reachability_is_tracked() {
    let dir = tempdir().unwrap();
    let handle = open(dir.path());
    let peer = DeviceId::from_bytes([2; 32]);

    handle.update_peer_reachability(peer, true);
    let runtime = handle.runtime_status();
    assert!(
        runtime
            .peers
            .iter()
            .any(|p| p.device_id == peer && p.reachable)
    );

    handle.update_peer_reachability(peer, false);
    let runtime = handle.runtime_status();
    assert!(
        runtime
            .peers
            .iter()
            .any(|p| p.device_id == peer && !p.reachable)
    );
}

#[test]
fn reachability_does_not_imply_durability() {
    let dir = tempdir().unwrap();
    let handle = open(dir.path());
    let peer = DeviceId::from_bytes([3; 32]);
    handle.update_peer_reachability(peer, true);
    // Local durability state is independent of any peer being reachable.
    assert_eq!(handle.replica_state(), ReplicaState::Ready);
    let runtime = handle.runtime_status();
    assert_eq!(runtime.replica, ReplicaState::Ready);
}
