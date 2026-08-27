//! Versioned authenticated encrypted backups (Argon2id + XChaCha20-Poly1305).

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use rand::RngCore;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;

const MAGIC: &[u8; 4] = b"TPB1";
const VERSION: u8 = 1;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;

/// A serialized business snapshot (excludes device private keys and database keys).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub fields: Vec<FieldRecord>,
    pub lifecycles: Vec<LifecycleRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldRecord {
    pub entity_type: String,
    pub entity_id: Vec<u8>,
    pub generation: i64,
    pub field_name: String,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleRecord {
    pub entity_type: String,
    pub entity_id: Vec<u8>,
    pub generation: i64,
    pub deleted: i64,
}

fn collect_snapshot(conn: &Connection) -> Result<Snapshot, CoreError> {
    let mut fields = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT entity_type, entity_id, generation, field_name, value FROM field_registers",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(FieldRecord {
                entity_type: r.get(0)?,
                entity_id: r.get(1)?,
                generation: r.get(2)?,
                field_name: r.get(3)?,
                value: r.get(4)?,
            })
        })?;
        for row in rows {
            fields.push(row?);
        }
    }
    let mut lifecycles = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT entity_type, entity_id, generation, deleted FROM entity_lifecycle")?;
        let rows = stmt.query_map([], |r| {
            Ok(LifecycleRecord {
                entity_type: r.get(0)?,
                entity_id: r.get(1)?,
                generation: r.get(2)?,
                deleted: r.get(3)?,
            })
        })?;
        for row in rows {
            lifecycles.push(row?);
        }
    }
    Ok(Snapshot { fields, lifecycles })
}

/// Create an encrypted backup of the database as a self-contained byte blob.
pub fn create_backup(conn: &Connection, passphrase: &str) -> Result<Vec<u8>, CoreError> {
    let snapshot = collect_snapshot(conn)?;
    let plaintext = serde_json::to_vec(&snapshot).map_err(|_| CoreError::BackupEncryption)?;

    let salt = random_bytes(SALT_LEN);
    let key = derive_key(passphrase, &salt)?;
    let nonce = random_bytes(NONCE_LEN);
    let cipher = XChaCha20Poly1305::new(key.as_ref().into());
    let ciphertext = cipher
        .encrypt(XNonce::from_slice(&nonce), plaintext.as_ref())
        .map_err(|_| CoreError::BackupEncryption)?;

    let mut out = Vec::with_capacity(5 + SALT_LEN + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypt a backup and return the restored business snapshot.
pub fn restore_backup(bytes: &[u8], passphrase: &str) -> Result<Snapshot, CoreError> {
    if bytes.len() < 5 || &bytes[0..4] != MAGIC {
        return Err(CoreError::InvalidBackup);
    }
    if bytes[4] != VERSION {
        return Err(CoreError::UnsupportedBackupVersion);
    }
    if bytes.len() < 5 + SALT_LEN + NONCE_LEN {
        return Err(CoreError::InvalidBackup);
    }
    let salt = &bytes[5..5 + SALT_LEN];
    let nonce = &bytes[5 + SALT_LEN..5 + SALT_LEN + NONCE_LEN];
    let ciphertext = &bytes[5 + SALT_LEN + NONCE_LEN..];

    let key = derive_key(passphrase, salt)?;
    let cipher = XChaCha20Poly1305::new(key.as_ref().into());
    let plaintext = cipher
        .decrypt(XNonce::from_slice(nonce), ciphertext)
        .map_err(|_| CoreError::BadPassphrase)?;

    serde_json::from_slice(&plaintext).map_err(|_| CoreError::InvalidBackup)
}

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], CoreError> {
    let params = Params::new(19_456, 2, 1, Some(32)).map_err(|_| CoreError::BackupEncryption)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|_| CoreError::BackupEncryption)?;
    Ok(key)
}

fn random_bytes(len: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; len];
    rand::rng().fill_bytes(&mut bytes);
    bytes
}
