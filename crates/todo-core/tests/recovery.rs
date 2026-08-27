//! Storage recovery: quarantine corrupt profiles, distinct reasons.

use std::path::Path;
use std::time::Duration;

use tempfile::tempdir;
use todo_core::recovery::{RecoveryReason, ReplicaState, UnavailableReason, open_with_recovery};
use todo_storage::config::{SecretBytes, StorageConfig};

fn config(dir: &Path, key_byte: u8) -> StorageConfig {
    StorageConfig {
        profile_path: dir.join("profile.db"),
        database_key: SecretBytes::from_bytes(vec![key_byte; 32]),
        busy_timeout: Duration::from_secs(5),
    }
}

#[test]
fn corrupt_db_is_quarantined() {
    let dir = tempdir().unwrap();
    let cfg = config(dir.path(), 1);
    {
        let s = todo_storage::Storage::open(cfg.clone()).unwrap();
        s.conn.execute_batch("CREATE TABLE x(id INTEGER);").unwrap();
    }
    // Truncate the database to corrupt it.
    let path = dir.path().join("profile.db");
    let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.set_len(64).unwrap();

    let result = open_with_recovery(cfg);
    match result {
        Ok((_storage, state)) => {
            assert!(matches!(
                state,
                ReplicaState::Recovering(RecoveryReason::Corrupt)
            ));
        }
        Err(_) => panic!("expected recovery state, not an error"),
    }
    // Original is quarantined under a unique name.
    let entries: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.contains("quarantine") || n.contains("corrupt"))
        .collect();
    assert!(!entries.is_empty(), "corrupt profile must be preserved");
}

#[test]
fn wrong_key_is_unavailable() {
    let dir = tempdir().unwrap();
    let cfg = config(dir.path(), 1);
    {
        todo_storage::Storage::open(cfg.clone()).unwrap();
    }
    let bad = config(dir.path(), 2);
    let result = open_with_recovery(bad);
    match result {
        Ok((_s, state)) => {
            assert!(matches!(
                state,
                ReplicaState::Unavailable(UnavailableReason::MissingKey)
            ));
        }
        Err(_) => panic!("expected unavailable state, not an error"),
    }
}

#[test]
fn clean_open_is_ready() {
    let dir = tempdir().unwrap();
    let cfg = config(dir.path(), 1);
    let (storage, state) = open_with_recovery(cfg).unwrap();
    assert!(matches!(state, ReplicaState::Ready));
    let _ = storage;
}
