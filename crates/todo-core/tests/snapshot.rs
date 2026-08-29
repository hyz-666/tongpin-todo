//! Snapshot export/import and adoption safety.

use serde_json::json;
use todo_core::{export_snapshot, import_snapshot};
use todo_domain::clock::Hlc;
use todo_domain::ids::{DeviceId, EntityId, LifecycleGeneration, OperationId};
use todo_domain::operation::{
    EntityKind, OperationPayload, ReplicaProjection, VerifiedOperation, apply_operation,
};
use todo_domain::register::VersionStamp;

fn op(entity: EntityId, title: &str) -> VerifiedOperation {
    VerifiedOperation {
        entity,
        kind: EntityKind::Task,
        parent: None,
        stamp: VersionStamp {
            generation: LifecycleGeneration(1),
            hlc: Hlc {
                physical_millis: 1_700_000_000_000,
                logical: 0,
            },
            device: DeviceId::from_bytes([7u8; 32]),
            operation: OperationId::new(DeviceId::from_bytes([7u8; 32]), 1),
        },
        payload: OperationPayload::SetField {
            field: "title".to_string(),
            value: json!(title),
        },
    }
}

fn projection() -> ReplicaProjection {
    let mut state = ReplicaProjection::default();
    let _ = apply_operation(
        &mut state,
        &op(EntityId::from_uuid(uuid::Uuid::nil()), "任务A"),
    );
    state
}

#[test]
fn export_import_round_trip() {
    let p = projection();
    let snap = export_snapshot(&p, [0xAB; 32]);
    let restored = import_snapshot(&snap).unwrap();
    assert_eq!(restored.entities, p.entities);
    assert_eq!(restored, p);
}

#[test]
fn zero_membership_epoch_is_rejected() {
    let p = projection();
    let snap = export_snapshot(&p, [0u8; 32]);
    assert!(import_snapshot(&snap).is_err());
}

#[test]
fn snapshot_preserves_entity_fields() {
    let p = projection();
    let snap = export_snapshot(&p, [0x11; 32]);
    let restored = import_snapshot(&snap).unwrap();
    let entity = restored
        .entities
        .get(&EntityId::from_uuid(uuid::Uuid::nil()))
        .unwrap();
    assert_eq!(entity.fields["title"].value.as_str().unwrap(), "任务A");
}
