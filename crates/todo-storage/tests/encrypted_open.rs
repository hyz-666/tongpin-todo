//! Encrypted open: correct/wrong key, plaintext rejection, WAL, and FK.

use std::path::Path;
use std::time::Duration;

use tempfile::tempdir;
use todo_storage::Storage;
use todo_storage::config::{SecretBytes, StorageConfig};
use todo_storage::error::StorageError;

fn key(byte: u8) -> SecretBytes {
    SecretBytes::from_bytes(vec![byte; 32])
}

fn config(dir: &Path, key: SecretBytes) -> StorageConfig {
    StorageConfig {
        profile_path: dir.join("profile.db"),
        database_key: key,
        busy_timeout: Duration::from_secs(5),
    }
}

#[test]
fn correct_key_opens_and_migrates() {
    let dir = tempdir().unwrap();
    let s = Storage::open(config(dir.path(), key(1))).unwrap();
    let tables: i64 = s
        .conn
        .query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get(0))
        .unwrap();
    assert!(tables > 0, "schema was created");
}

#[test]
fn wrong_key_fails() {
    let dir = tempdir().unwrap();
    Storage::open(config(dir.path(), key(1))).unwrap();
    assert!(matches!(
        Storage::open(config(dir.path(), key(2))),
        Err(StorageError::WrongKey)
    ));
}

#[test]
fn plaintext_header_rejected() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("plain.db");
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TABLE t(id INTEGER);").unwrap();
    drop(conn);

    let cfg = config(dir.path(), key(1));
    let mut cfg = cfg;
    cfg.profile_path = path;
    assert!(matches!(
        Storage::open(cfg),
        Err(StorageError::NotEncrypted)
    ));
}

#[test]
fn wal_reopen_preserves_data() {
    let dir = tempdir().unwrap();
    let cfg = config(dir.path(), key(1));
    {
        let s = Storage::open(cfg.clone()).unwrap();
        s.conn
            .execute_batch("CREATE TABLE t(id INTEGER); INSERT INTO t VALUES (42);")
            .unwrap();
    }
    let s2 = Storage::open(cfg).unwrap();
    let v: i64 = s2
        .conn
        .query_row("SELECT id FROM t", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v, 42);
}

#[test]
fn foreign_keys_enforced() {
    let dir = tempdir().unwrap();
    let s = Storage::open(config(dir.path(), key(1))).unwrap();
    s.conn
        .execute_batch(
            "CREATE TABLE p(id INTEGER PRIMARY KEY);
             CREATE TABLE c(id INTEGER, pid INTEGER REFERENCES p(id));",
        )
        .unwrap();
    assert!(
        s.conn
            .execute("INSERT INTO c(id, pid) VALUES (1, 999)", [])
            .is_err()
    );
}
