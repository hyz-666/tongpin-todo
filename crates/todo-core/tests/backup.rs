//! Encrypted backup: round-trip, tamper, and version handling.

use std::path::Path;
use std::time::Duration;

use tempfile::tempdir;
use todo_core::CoreError;
use todo_core::backup::{create_backup, restore_backup};
use todo_storage::Storage;
use todo_storage::config::{SecretBytes, StorageConfig};

fn config(dir: &Path) -> StorageConfig {
    StorageConfig {
        profile_path: dir.join("profile.db"),
        database_key: SecretBytes::from_bytes(vec![1; 32]),
        busy_timeout: Duration::from_secs(5),
    }
}

fn seed(storage: &Storage) {
    storage
        .conn
        .execute(
            "INSERT INTO field_registers(entity_type, entity_id, generation, field_name, value, physical_millis, logical, device_id, origin_sequence)
             VALUES ('task', X'0102030405060708090A0B0C0D0E0F10', 1, 'title', X'22E4B9B0E7899BE5A5B622', 100, 0, X'0101', 1)",
            [],
        )
        .unwrap();
}

#[test]
fn backup_round_trip_preserves_data() {
    let dir = tempdir().unwrap();
    let storage = Storage::open(config(dir.path())).unwrap();
    seed(&storage);

    let backup = create_backup(&storage.conn, "correct horse").unwrap();
    drop(storage);

    let snapshot = restore_backup(&backup, "correct horse").unwrap();
    assert!(!snapshot.fields.is_empty());
    assert!(snapshot.fields.iter().any(|f| f.field_name == "title"));
    // The value bytes decode to JSON "买牛奶".
    let title_value = snapshot
        .fields
        .iter()
        .find(|f| f.field_name == "title")
        .unwrap();
    let s: String = serde_json::from_slice(&title_value.value).unwrap();
    assert_eq!(s, "买牛奶");
}

#[test]
fn wrong_passphrase_fails() {
    let dir = tempdir().unwrap();
    let storage = Storage::open(config(dir.path())).unwrap();
    seed(&storage);
    let backup = create_backup(&storage.conn, "right").unwrap();
    assert!(matches!(
        restore_backup(&backup, "wrong"),
        Err(CoreError::BadPassphrase)
    ));
}

#[test]
fn truncated_backup_fails() {
    let dir = tempdir().unwrap();
    let storage = Storage::open(config(dir.path())).unwrap();
    seed(&storage);
    let backup = create_backup(&storage.conn, "pass").unwrap();
    let truncated = &backup[..backup.len() / 2];
    assert!(restore_backup(truncated, "pass").is_err());
}

#[test]
fn tampered_backup_fails() {
    let dir = tempdir().unwrap();
    let storage = Storage::open(config(dir.path())).unwrap();
    seed(&storage);
    let mut backup = create_backup(&storage.conn, "pass").unwrap();
    let last = backup.len() - 1;
    backup[last] ^= 0xFF;
    assert!(restore_backup(&backup, "pass").is_err());
}

#[test]
fn unsupported_version_fails() {
    let mut bytes = vec![0u8; 64];
    bytes[0..4].copy_from_slice(b"TPB1");
    bytes[4] = 0xFF;
    assert!(matches!(
        restore_backup(&bytes, "pass"),
        Err(CoreError::UnsupportedBackupVersion)
    ));
}
