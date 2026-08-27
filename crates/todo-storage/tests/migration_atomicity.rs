//! Migration atomicity: failures roll back and downgrades are rejected.

use std::path::Path;
use std::time::Duration;

use tempfile::tempdir;
use todo_storage::Storage;
use todo_storage::config::{SecretBytes, StorageConfig};
use todo_storage::error::StorageError;
use todo_storage::migration::{Migration, migrate, user_version};

fn config(dir: &Path) -> StorageConfig {
    StorageConfig {
        profile_path: dir.join("profile.db"),
        database_key: SecretBytes::from_bytes(vec![1; 32]),
        busy_timeout: Duration::from_secs(5),
    }
}

#[test]
fn failed_migration_rolls_back() {
    let dir = tempdir().unwrap();
    let cfg = config(dir.path());
    let s = Storage::open(cfg).unwrap();

    // A migration whose second statement is malformed, after a valid one.
    let bad = vec![Migration {
        version: 2,
        sql: "CREATE TABLE should_rollback(id INTEGER); CREATE TABLE broken(id",
    }];
    let result = migrate(&s.conn, &bad);
    assert!(matches!(result, Err(StorageError::MigrationFailed(_))));

    // Version stays at 1 and the partial DDL was rolled back.
    assert_eq!(user_version(&s.conn).unwrap(), 1);
    let count: i64 = s
        .conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name='should_rollback'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0, "partial migration must be rolled back");
}

#[test]
fn unsupported_downgrade_rejected() {
    let dir = tempdir().unwrap();
    let cfg = config(dir.path());
    let s = Storage::open(cfg.clone()).unwrap();
    // Simulate a profile written by a newer version.
    s.conn.execute_batch("PRAGMA user_version = 2;").unwrap();
    drop(s);

    assert!(matches!(
        Storage::open(cfg),
        Err(StorageError::UnsupportedDowngrade {
            current: 2,
            supported: 1
        })
    ));
}
