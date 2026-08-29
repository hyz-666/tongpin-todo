//! Database access primitives for the serialized core executor.
//!
//! All operations take a `&Connection` so callers can pass either the open
//! connection or an active `Transaction` (which derefs to `Connection`).

use rusqlite::Connection;
use serde_json::Value;

use todo_domain::ids::{DeviceId, EntityId};
use todo_domain::register::VersionStamp;

use crate::error::StorageError;

/// A stored operation row: `(sequence, canonical_bytes, signature)`.
pub type StoredOperation = (u64, Vec<u8>, Option<Vec<u8>>);

pub struct Repository {
    pub conn: Connection,
}

impl Repository {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    // --- operations table ---
    pub fn insert_operation(
        conn: &Connection,
        origin: &DeviceId,
        sequence: u64,
        canonical: &[u8],
        signature: Option<&[u8]>,
        committed_at: i64,
    ) -> Result<bool, StorageError> {
        let changed = conn.execute(
            "INSERT OR IGNORE INTO operations(origin_device_id, origin_sequence, canonical_bytes, signature, committed_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                origin.as_bytes().as_slice(),
                sequence as i64,
                canonical,
                signature,
                committed_at
            ],
        )?;
        Ok(changed > 0)
    }

    /// Read the operations of one origin in the half-open `[from_seq, to_seq)`
    /// range, in ascending sequence order. Returns `(sequence, canonical, signature)`.
    pub fn read_operations(
        conn: &Connection,
        origin: &DeviceId,
        from_seq: u64,
        to_seq: u64,
    ) -> Result<Vec<StoredOperation>, StorageError> {
        let mut stmt = conn.prepare(
            "SELECT origin_sequence, canonical_bytes, signature FROM operations
             WHERE origin_device_id = ?1 AND origin_sequence >= ?2 AND origin_sequence < ?3
             ORDER BY origin_sequence ASC",
        )?;
        let rows = stmt.query_map(
            rusqlite::params![origin.as_bytes().as_slice(), from_seq as i64, to_seq as i64],
            |row| {
                Ok((
                    row.get::<_, i64>(0)? as u64,
                    row.get::<_, Vec<u8>>(1)?,
                    row.get::<_, Option<Vec<u8>>>(2)?,
                ))
            },
        )?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    // --- field_registers table ---
    pub fn upsert_field_register(
        conn: &Connection,
        entity_type: &str,
        entity_id: &EntityId,
        generation: u32,
        field_name: &str,
        value: &Value,
        stamp: &VersionStamp,
    ) -> Result<(), StorageError> {
        let value_bytes = serde_json::to_vec(value).unwrap_or_default();
        conn.execute(
            "INSERT INTO field_registers(entity_type, entity_id, generation, field_name, value, physical_millis, logical, device_id, origin_sequence)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(entity_type, entity_id, generation, field_name) DO UPDATE SET
               value=excluded.value, physical_millis=excluded.physical_millis, logical=excluded.logical,
               device_id=excluded.device_id, origin_sequence=excluded.origin_sequence",
            rusqlite::params![
                entity_type,
                entity_id.as_bytes().as_slice(),
                generation as i64,
                field_name,
                value_bytes,
                stamp.hlc.physical_millis,
                stamp.hlc.logical as i64,
                stamp.device.as_bytes().as_slice(),
                stamp.operation.sequence as i64
            ],
        )?;
        Ok(())
    }

    // --- entity_lifecycle table ---
    pub fn upsert_entity_lifecycle(
        conn: &Connection,
        entity_type: &str,
        entity_id: &EntityId,
        generation: u32,
        deleted: bool,
        tombstone: Option<&[u8]>,
    ) -> Result<(), StorageError> {
        conn.execute(
            "INSERT INTO entity_lifecycle(entity_type, entity_id, generation, deleted, tombstone_operation)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(entity_type, entity_id) DO UPDATE SET
               generation=excluded.generation, deleted=excluded.deleted, tombstone_operation=excluded.tombstone_operation",
            rusqlite::params![
                entity_type,
                entity_id.as_bytes().as_slice(),
                generation as i64,
                deleted as i64,
                tombstone
            ],
        )?;
        Ok(())
    }

    pub fn read_lifecycle(
        conn: &Connection,
        entity_id: &EntityId,
    ) -> Result<(u32, bool), StorageError> {
        let r = conn.query_row(
            "SELECT generation, deleted FROM entity_lifecycle WHERE entity_id = ?1",
            [entity_id.as_bytes().as_slice()],
            |row| Ok((row.get::<_, i64>(0)? as u32, row.get::<_, i64>(1)? != 0)),
        );
        match r {
            Ok(v) => Ok(v),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok((1, false)),
            Err(e) => Err(e.into()),
        }
    }

    // --- subtasks parent ---
    pub fn upsert_subtask_parent(
        conn: &Connection,
        entity_id: &EntityId,
        parent: &EntityId,
    ) -> Result<(), StorageError> {
        conn.execute(
            "INSERT INTO subtasks(entity_id, parent_task_id, generation, deleted, title, completed, rank)
             VALUES (?1, ?2, 1, 0, NULL, NULL, NULL)
             ON CONFLICT(entity_id) DO UPDATE SET parent_task_id=excluded.parent_task_id",
            rusqlite::params![entity_id.as_bytes().as_slice(), parent.as_bytes().as_slice()],
        )?;
        Ok(())
    }

    // --- frontiers ---
    pub fn read_frontier(
        conn: &Connection,
        origin: &DeviceId,
    ) -> Result<Option<u64>, StorageError> {
        let r = conn.query_row(
            "SELECT contiguous_sequence FROM origin_frontiers WHERE origin_device_id = ?1",
            [origin.as_bytes().as_slice()],
            |row| row.get::<_, i64>(0),
        );
        match r {
            Ok(v) => Ok(Some(v as u64)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn upsert_frontier(
        conn: &Connection,
        origin: &DeviceId,
        sequence: u64,
    ) -> Result<(), StorageError> {
        conn.execute(
            "INSERT INTO origin_frontiers(origin_device_id, contiguous_sequence) VALUES (?1, ?2)
             ON CONFLICT(origin_device_id) DO UPDATE SET contiguous_sequence=excluded.contiguous_sequence",
            rusqlite::params![origin.as_bytes().as_slice(), sequence as i64],
        )?;
        Ok(())
    }

    // --- queries ---
    pub fn read_field(
        conn: &Connection,
        entity_id: &EntityId,
        field: &str,
    ) -> Result<Option<Value>, StorageError> {
        let r = conn.query_row(
            "SELECT value FROM field_registers WHERE entity_id = ?1 AND field_name = ?2 ORDER BY generation DESC LIMIT 1",
            rusqlite::params![entity_id.as_bytes().as_slice(), field],
            |row| row.get::<_, Vec<u8>>(0),
        );
        match r {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes).ok()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn is_deleted(conn: &Connection, entity_id: &EntityId) -> Result<bool, StorageError> {
        Ok(Self::read_lifecycle(conn, entity_id)?.1)
    }

    pub fn read_parent(
        conn: &Connection,
        entity_id: &EntityId,
    ) -> Result<Option<EntityId>, StorageError> {
        let r = conn.query_row(
            "SELECT parent_task_id FROM subtasks WHERE entity_id = ?1",
            [entity_id.as_bytes().as_slice()],
            |row| row.get::<_, Vec<u8>>(0),
        );
        match r {
            Ok(bytes) => {
                let arr: [u8; 16] = bytes.try_into().map_err(|_| {
                    StorageError::Sqlite(rusqlite::Error::InvalidParameterName(
                        "parent id".to_string(),
                    ))
                })?;
                Ok(Some(EntityId::from_uuid(uuid::Uuid::from_bytes(arr))))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn count_entities(conn: &Connection) -> Result<usize, StorageError> {
        let v: i64 = conn.query_row("SELECT count(*) FROM entity_lifecycle", [], |row| {
            row.get(0)
        })?;
        Ok(v as usize)
    }

    // --- projection revision ---
    pub fn projection_revision(conn: &Connection) -> Result<u64, StorageError> {
        let r = conn.query_row(
            "SELECT value FROM meta WHERE key = 'projection_revision'",
            [],
            |row| row.get::<_, String>(0),
        );
        match r {
            Ok(v) => Ok(v.parse().unwrap_or(0)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
            Err(e) => Err(e.into()),
        }
    }

    pub fn bump_revision(conn: &Connection, revision: u64) -> Result<(), StorageError> {
        conn.execute(
            "INSERT INTO meta(key, value) VALUES ('projection_revision', ?1)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            rusqlite::params![revision.to_string()],
        )?;
        Ok(())
    }
}
