//! Operation application and the in-memory replica projection.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::clock::Hlc;
use crate::error::DomainError;
use crate::ids::{DeviceId, EntityId, LifecycleGeneration};
use crate::register::{ApplyDecision, Register, VersionStamp, merge_register};

pub use crate::model::EntityKind;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum OperationPayload {
    SetField { field: String, value: Value },
    Delete,
    Restore,
}

/// An operation whose origin and version have already been verified.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VerifiedOperation {
    pub entity: EntityId,
    pub kind: EntityKind,
    pub parent: Option<EntityId>,
    pub stamp: VersionStamp,
    pub payload: OperationPayload,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityProjection {
    pub kind: EntityKind,
    pub generation: LifecycleGeneration,
    pub deleted: bool,
    pub parent: Option<EntityId>,
    pub fields: BTreeMap<String, Register<Value>>,
    pub tombstone: Option<VersionStamp>,
}

/// A retained losing value for the conflict-history view.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictEntry {
    pub entity: EntityId,
    pub field: String,
    pub replaced: Value,
    pub stamp: VersionStamp,
}

/// The complete materialized projection of a replica.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicaProjection {
    pub entities: BTreeMap<EntityId, EntityProjection>,
    pub conflicts: Vec<ConflictEntry>,
    /// Normalized tag name -> canonical tag id.
    pub tag_canonical: BTreeMap<String, EntityId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApplyReport {
    pub decision: ApplyDecision,
    pub affected: Vec<EntityId>,
}

pub fn apply_operation(
    state: &mut ReplicaProjection,
    op: &VerifiedOperation,
) -> Result<ApplyReport, DomainError> {
    match &op.payload {
        OperationPayload::SetField { field, value } => apply_set_field(state, op, field, value),
        OperationPayload::Delete => crate::lifecycle::apply_delete(state, op),
        OperationPayload::Restore => crate::lifecycle::apply_restore(state, op),
    }
}

fn apply_set_field(
    state: &mut ReplicaProjection,
    op: &VerifiedOperation,
    field: &str,
    value: &Value,
) -> Result<ApplyReport, DomainError> {
    let (decision, conflict, tag_name_changed) = {
        let entity = ensure_entity(state, op);
        if op.stamp.generation < entity.generation {
            return Ok(ApplyReport {
                decision: ApplyDecision::IgnoredStaleGeneration,
                affected: vec![],
            });
        }
        if entity.deleted {
            return Ok(ApplyReport {
                decision: ApplyDecision::IgnoredOlderRegister,
                affected: vec![op.entity],
            });
        }
        let incoming = Register {
            value: value.clone(),
            stamp: op.stamp,
        };
        let current = entity.fields.get(field).cloned();
        let decision = merge_register(current.as_ref(), &incoming);
        let conflict = if decision == ApplyDecision::Applied {
            let mut conflict = None;
            if let Some(cur) = &current
                && cur.value != *value
            {
                conflict = Some(ConflictEntry {
                    entity: op.entity,
                    field: field.to_string(),
                    replaced: cur.value.clone(),
                    stamp: cur.stamp,
                });
            }
            entity.fields.insert(field.to_string(), incoming);
            conflict
        } else {
            None
        };
        let tag_name_changed =
            decision == ApplyDecision::Applied && op.kind == EntityKind::Tag && field == "name";
        (decision, conflict, tag_name_changed)
    };
    if let Some(entry) = conflict {
        state.conflicts.push(entry);
    }
    if tag_name_changed {
        rebuild_tag_canonical(state);
    }
    Ok(ApplyReport {
        decision,
        affected: vec![op.entity],
    })
}

pub(crate) fn rebuild_tag_canonical(state: &mut ReplicaProjection) {
    let mut canonical: BTreeMap<String, (Hlc, DeviceId, EntityId)> = BTreeMap::new();
    for (id, e) in &state.entities {
        if e.kind != EntityKind::Tag || e.deleted {
            continue;
        }
        let Some(reg) = e.fields.get("name") else {
            continue;
        };
        let Some(name) = reg.value.as_str() else {
            continue;
        };
        let normalized = crate::validation::normalize(name);
        let key = (reg.stamp.hlc, reg.stamp.device, *id);
        match canonical.get(&normalized) {
            Some(&existing) if existing <= key => {}
            _ => {
                canonical.insert(normalized, key);
            }
        }
    }
    state.tag_canonical = canonical
        .into_iter()
        .map(|(name, (_, _, id))| (name, id))
        .collect();
}

pub(crate) fn ensure_entity<'a>(
    state: &'a mut ReplicaProjection,
    op: &VerifiedOperation,
) -> &'a mut EntityProjection {
    state
        .entities
        .entry(op.entity)
        .or_insert_with(|| EntityProjection {
            kind: op.kind,
            generation: LifecycleGeneration(1),
            deleted: false,
            parent: op.parent,
            fields: BTreeMap::new(),
            tombstone: None,
        })
}
