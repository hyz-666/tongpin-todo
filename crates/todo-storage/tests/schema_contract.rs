//! Schema contract: tables, constraints, indexes, and version markers.

use std::path::Path;
use std::time::Duration;

use tempfile::tempdir;
use todo_storage::Storage;
use todo_storage::config::{SecretBytes, StorageConfig};
use todo_storage::schema::{APPLICATION_ID, SCHEMA_VERSION, TABLES};

fn open(dir: &Path) -> Storage {
    let cfg = StorageConfig {
        profile_path: dir.join("profile.db"),
        database_key: SecretBytes::from_bytes(vec![1; 32]),
        busy_timeout: Duration::from_secs(5),
    };
    Storage::open(cfg).unwrap()
}

#[test]
fn all_tables_exist() {
    let dir = tempdir().unwrap();
    let s = open(dir.path());
    for table in TABLES {
        let count: i64 = s
            .conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "table `{}` is missing", table);
    }
}

#[test]
fn user_version_and_application_id() {
    let dir = tempdir().unwrap();
    let s = open(dir.path());
    let v: i64 = s
        .conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, SCHEMA_VERSION as i64);
    let id: i64 = s
        .conn
        .query_row("PRAGMA application_id", [], |r| r.get(0))
        .unwrap();
    assert_eq!(id, APPLICATION_ID as i64);
}

#[test]
fn unique_constraints_are_enforced() {
    let dir = tempdir().unwrap();
    let s = open(dir.path());
    let insert = "INSERT INTO operations(origin_device_id, origin_sequence, canonical_bytes, committed_at) VALUES (?1, ?2, ?3, ?4)";
    s.conn
        .execute(insert, rusqlite::params![vec![1u8], 0i64, vec![3u8], 0i64])
        .unwrap();
    assert!(
        s.conn
            .execute(insert, rusqlite::params![vec![1u8], 0i64, vec![3u8], 0i64])
            .is_err()
    );
}

#[test]
fn search_indexes_exist() {
    let dir = tempdir().unwrap();
    let s = open(dir.path());
    for index in [
        "idx_tasks_due_date",
        "idx_tasks_list_id",
        "idx_tags_normalized",
    ] {
        let count: i64 = s
            .conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='index' AND name=?1",
                [index],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "index `{}` is missing", index);
    }
}
