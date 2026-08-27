//! Materialize a verified operation into the projected tables.

use rusqlite::Connection;

use todo_domain::operation::{EntityKind, OperationPayload, VerifiedOperation};

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
