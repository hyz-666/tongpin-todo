//! Materialize a verified operation into the projected tables.

use rusqlite::Connection;
use serde_json::Value;

use todo_domain::operation::{EntityKind, OperationPayload, VerifiedOperation};
use todo_domain::validation::normalize;

use crate::error::StorageError;
use crate::repository::Repository;

pub fn entity_type(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Task => "task",
        EntityKind::Subtask => "subtask",
        EntityKind::List => "list",
        EntityKind::Tag => "tag",
    }
}

/// Apply one verified operation to the materialized projections.
pub fn apply(conn: &Connection, op: &VerifiedOperation) -> Result<(), StorageError> {
    let etype = entity_type(op.kind);
    match &op.payload {
        OperationPayload::SetField { field, value } => {
            if let Some(old) = Repository::read_field(conn, &op.entity, field)?
                && old != *value
            {
                let replaced = serde_json::to_vec(&old).unwrap_or_default();
                conn.execute(
                    "INSERT INTO conflict_history(entity_type, entity_id, field_name, replaced_value, physical_millis, logical, device_id, origin_sequence, observed_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        etype,
                        op.entity.as_bytes().as_slice(),
                        field,
                        replaced,
                        op.stamp.hlc.physical_millis,
                        op.stamp.hlc.logical as i64,
                        op.stamp.device.as_bytes().as_slice(),
                        op.stamp.operation.sequence as i64,
                        op.stamp.hlc.physical_millis
                    ],
                )?;
            }
            Repository::upsert_field_register(
                conn,
                etype,
                &op.entity,
                op.stamp.generation.0,
                field,
                value,
                &op.stamp,
            )?;
            if op.kind == EntityKind::Subtask
                && let Some(parent) = op.parent
            {
                Repository::upsert_subtask_parent(conn, &op.entity, &parent)?;
            }
            sync_business_column(conn, op, field, value)?;
        }
        OperationPayload::Delete => {
            Repository::upsert_entity_lifecycle(
                conn,
                etype,
                &op.entity,
                op.stamp.generation.0,
                true,
                None,
            )?;
            mark_deleted(conn, op.kind, &op.entity)?;
        }
        OperationPayload::Restore => {
            let (generation, _) = Repository::read_lifecycle(conn, &op.entity)?;
            Repository::upsert_entity_lifecycle(
                conn,
                etype,
                &op.entity,
                generation + 1,
                false,
                None,
            )?;
        }
    }
    Ok(())
}

fn mark_deleted(
    conn: &Connection,
    kind: EntityKind,
    id: &todo_domain::ids::EntityId,
) -> Result<(), StorageError> {
    let table = match kind {
        EntityKind::Task => "tasks",
        EntityKind::Subtask => "subtasks",
        EntityKind::List => "lists",
        EntityKind::Tag => "tags",
    };
    conn.execute(
        &format!("UPDATE {table} SET deleted = 1 WHERE entity_id = ?1"),
        rusqlite::params![id.as_bytes().as_slice()],
    )?;
    if kind == EntityKind::Task {
        conn.execute(
            "DELETE FROM task_fts WHERE task_id = ?1",
            rusqlite::params![id.as_bytes().as_slice()],
        )?;
    }
    Ok(())
}

fn sync_business_column(
    conn: &Connection,
    op: &VerifiedOperation,
    field: &str,
    value: &Value,
) -> Result<(), StorageError> {
    let id = op.entity.as_bytes();
    let text = value.as_str().map(|s| s.to_string());
    match op.kind {
        EntityKind::Task => sync_task(conn, op, field, value)?,
        EntityKind::Subtask => match field {
            "title" => update_text(conn, "subtasks", id, "title", text.as_deref())?,
            "completed" => update_bool(conn, "subtasks", id, "completed", value.as_bool())?,
            _ => {}
        },
        EntityKind::List => match field {
            "name" => upsert_text(conn, "lists", id, "name", text.as_deref())?,
            "color" => upsert_text(conn, "lists", id, "color", text.as_deref())?,
            "icon" => upsert_text(conn, "lists", id, "icon", text.as_deref())?,
            _ => {}
        },
        EntityKind::Tag => {
            if field == "name"
                && let Some(name) = &text
            {
                upsert_text(conn, "tags", id, "name", Some(name))?;
                upsert_text(conn, "tags", id, "normalized_name", Some(&normalize(name)))?;
            }
        }
    }
    Ok(())
}

fn sync_task(
    conn: &Connection,
    op: &VerifiedOperation,
    field: &str,
    value: &Value,
) -> Result<(), StorageError> {
    let id = op.entity.as_bytes();
    match field {
        "title" => upsert_text(conn, "tasks", id, "title", value.as_str())?,
        "description" => upsert_text(conn, "tasks", id, "description", value.as_str())?,
        "due_date" => upsert_text(conn, "tasks", id, "due_date", value.as_str())?,
        "due_time" => upsert_text(conn, "tasks", id, "due_time", value.as_str())?,
        "priority" => upsert_text(conn, "tasks", id, "priority", value.as_str())?,
        "completed" => upsert_bool(conn, "tasks", id, "completed", value.as_bool())?,
        "list_id" => upsert_text(conn, "tasks", id, "list_id", value.as_str())?,
        _ => {}
    }
    if matches!(field, "title" | "description" | "tags") {
        refresh_fts(conn, &op.entity)?;
    }
    Ok(())
}

fn refresh_fts(conn: &Connection, id: &todo_domain::ids::EntityId) -> Result<(), StorageError> {
    conn.execute(
        "DELETE FROM task_fts WHERE task_id = ?1",
        rusqlite::params![id.as_bytes().as_slice()],
    )?;
    let row = conn.query_row(
        "SELECT title, description FROM tasks WHERE entity_id = ?1",
        [id.as_bytes().as_slice()],
        |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
            ))
        },
    );
    if let Ok((title, description)) = row {
        conn.execute(
            "INSERT INTO task_fts(task_id, title, description, tags) VALUES (?1, ?2, ?3, '')",
            rusqlite::params![
                id.as_bytes().as_slice(),
                title.as_deref().unwrap_or(""),
                description.as_deref().unwrap_or("")
            ],
        )?;
    }
    Ok(())
}

fn upsert_text(
    conn: &Connection,
    table: &str,
    id: [u8; 16],
    column: &str,
    value: Option<&str>,
) -> Result<(), StorageError> {
    conn.execute(
        &format!(
            "INSERT INTO {table}(entity_id, generation, deleted, {column}) VALUES (?1, 1, 0, ?2)
             ON CONFLICT(entity_id) DO UPDATE SET {column}=excluded.{column}"
        ),
        rusqlite::params![id.as_slice(), value],
    )?;
    Ok(())
}

fn update_text(
    conn: &Connection,
    table: &str,
    id: [u8; 16],
    column: &str,
    value: Option<&str>,
) -> Result<(), StorageError> {
    conn.execute(
        &format!("UPDATE {table} SET {column} = ?2 WHERE entity_id = ?1"),
        rusqlite::params![id.as_slice(), value],
    )?;
    Ok(())
}

fn update_bool(
    conn: &Connection,
    table: &str,
    id: [u8; 16],
    column: &str,
    value: Option<bool>,
) -> Result<(), StorageError> {
    conn.execute(
        &format!("UPDATE {table} SET {column} = ?2 WHERE entity_id = ?1"),
        rusqlite::params![id.as_slice(), value.map(|b| b as i64)],
    )?;
    Ok(())
}

fn upsert_bool(
    conn: &Connection,
    table: &str,
    id: [u8; 16],
    column: &str,
    value: Option<bool>,
) -> Result<(), StorageError> {
    conn.execute(
        &format!(
            "INSERT INTO {table}(entity_id, generation, deleted, {column}) VALUES (?1, 1, 0, ?2)
             ON CONFLICT(entity_id) DO UPDATE SET {column}=excluded.{column}"
        ),
        rusqlite::params![id.as_slice(), value.map(|b| b as i64)],
    )?;
    Ok(())
}
