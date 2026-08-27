//! Lifecycle transitions: delete, restore, and their cascading effects.

use serde_json::Value;

use crate::error::DomainError;
use crate::ids::{EntityId, LifecycleGeneration};
use crate::operation::{
    ApplyReport, EntityKind, ReplicaProjection, VerifiedOperation, ensure_entity,
};
use crate::register::{ApplyDecision, Register};

pub(crate) fn apply_delete(
    state: &mut ReplicaProjection,
    op: &VerifiedOperation,
) -> Result<ApplyReport, DomainError> {
    {
        let entity = ensure_entity(state, op);
        if op.stamp.generation < entity.generation {
            return Ok(ApplyReport {
                decision: ApplyDecision::IgnoredStaleGeneration,
                affected: vec![],
            });
        }
        if entity.deleted {
            return Ok(ApplyReport {
                decision: ApplyDecision::Duplicate,
                affected: vec![op.entity],
            });
        }
        entity.deleted = true;
        entity.tombstone = Some(op.stamp);
    }

    let mut affected = vec![op.entity];
    match op.kind {
        EntityKind::Task => cascade_delete_subtasks(state, op.entity, &mut affected),
        EntityKind::List => move_tasks_to_inbox(state, op.entity, &mut affected),
        EntityKind::Tag => {
            remove_tag_references(state, op.entity, &mut affected);
            crate::operation::rebuild_tag_canonical(state);
        }
        EntityKind::Subtask => {}
    }
    Ok(ApplyReport {
        decision: ApplyDecision::Applied,
        affected,
    })
}

pub(crate) fn apply_restore(
    state: &mut ReplicaProjection,
    op: &VerifiedOperation,
) -> Result<ApplyReport, DomainError> {
    let Some(entity) = state.entities.get_mut(&op.entity) else {
        return Ok(ApplyReport {
            decision: ApplyDecision::Rejected,
            affected: vec![],
        });
    };
    if !entity.deleted {
        return Ok(ApplyReport {
            decision: ApplyDecision::Duplicate,
            affected: vec![op.entity],
        });
    }
    entity.generation = LifecycleGeneration(entity.generation.0 + 1);
    entity.deleted = false;
    entity.tombstone = None;
    Ok(ApplyReport {
        decision: ApplyDecision::Applied,
        affected: vec![op.entity],
    })
}

fn cascade_delete_subtasks(
    state: &mut ReplicaProjection,
    task_id: EntityId,
    affected: &mut Vec<EntityId>,
) {
    let subtask_ids: Vec<EntityId> = state
        .entities
        .iter()
        .filter(|(_, e)| e.kind == EntityKind::Subtask && e.parent == Some(task_id) && !e.deleted)
        .map(|(id, _)| *id)
        .collect();
    for id in subtask_ids {
        if let Some(e) = state.entities.get_mut(&id) {
            e.deleted = true;
            affected.push(id);
        }
    }
}

fn move_tasks_to_inbox(
    state: &mut ReplicaProjection,
    list_id: EntityId,
    affected: &mut Vec<EntityId>,
) {
    let list_str = list_id.to_string();
    for (id, e) in state.entities.iter_mut() {
        if e.kind == EntityKind::Task
            && !e.deleted
            && let Some(reg) = e.fields.get("list_id")
            && reg.value.as_str() == Some(list_str.as_str())
        {
            e.fields.insert(
                "list_id".to_string(),
                Register {
                    value: Value::Null,
                    stamp: reg.stamp,
                },
            );
            affected.push(*id);
        }
    }
}

fn remove_tag_references(
    state: &mut ReplicaProjection,
    tag_id: EntityId,
    affected: &mut Vec<EntityId>,
) {
    let tag_str = tag_id.to_string();
    for (id, e) in state.entities.iter_mut() {
        if e.kind != EntityKind::Task || e.deleted {
            continue;
        }
        if let Some(reg) = e.fields.get("tags")
            && let Value::Array(arr) = &reg.value
            && arr.iter().any(|v| v.as_str() == Some(tag_str.as_str()))
        {
            let filtered = arr
                .iter()
                .filter(|v| v.as_str() != Some(tag_str.as_str()))
                .cloned()
                .collect();
            e.fields.insert(
                "tags".to_string(),
                Register {
                    value: Value::Array(filtered),
                    stamp: reg.stamp,
                },
            );
            affected.push(*id);
        }
    }
}
