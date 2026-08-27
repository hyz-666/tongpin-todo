//! Lifecycle rules: delete/restore, generations, and cascading effects.

use todo_domain::clock::Hlc;
use todo_domain::ids::{DeviceId, EntityId, LifecycleGeneration, OperationId};
use todo_domain::operation::{
    EntityKind, OperationPayload, ReplicaProjection, VerifiedOperation, apply_operation,
};
use todo_domain::register::VersionStamp;

fn stamp(device: u8, seq: u64, millis: i64, generation: u32) -> VersionStamp {
    VersionStamp {
        generation: LifecycleGeneration(generation),
        hlc: Hlc::new(millis, 0),
        device: DeviceId::from_bytes([device; 32]),
        operation: OperationId::new(DeviceId::from_bytes([device; 32]), seq),
    }
}

fn set_field(
    id: EntityId,
    kind: EntityKind,
    field: &str,
    value: serde_json::Value,
    stamp: VersionStamp,
) -> VerifiedOperation {
    VerifiedOperation {
        entity: id,
        kind,
        parent: None,
        stamp,
        payload: OperationPayload::SetField {
            field: field.to_string(),
            value,
        },
    }
}

fn delete(id: EntityId, kind: EntityKind, stamp: VersionStamp) -> VerifiedOperation {
    VerifiedOperation {
        entity: id,
        kind,
        parent: None,
        stamp,
        payload: OperationPayload::Delete,
    }
}

fn restore(id: EntityId, kind: EntityKind, stamp: VersionStamp) -> VerifiedOperation {
    VerifiedOperation {
        entity: id,
        kind,
        parent: None,
        stamp,
        payload: OperationPayload::Restore,
    }
}

fn task_id(n: u128) -> EntityId {
    // Deterministic ids via a fixed seed; EntityId is a UUIDv7 wrapper, so build
    // a synthetic uuid through the constructor used by tests.
    todo_domain::ids::EntityId::new_v7_for_test(n)
}

#[test]
fn delete_wins_within_generation() {
    let mut s = ReplicaProjection::default();
    let id = task_id(1);
    apply_operation(
        &mut s,
        &set_field(
            id,
            EntityKind::Task,
            "title",
            "task".into(),
            stamp(1, 1, 100, 1),
        ),
    )
    .unwrap();
    apply_operation(&mut s, &delete(id, EntityKind::Task, stamp(1, 2, 200, 1))).unwrap();
    assert!(s.entities[&id].deleted);
    // Editing after delete within the same generation is ignored.
    let r = apply_operation(
        &mut s,
        &set_field(
            id,
            EntityKind::Task,
            "title",
            "edited".into(),
            stamp(1, 3, 300, 1),
        ),
    )
    .unwrap();
    assert!(!matches!(
        r.decision,
        todo_domain::register::ApplyDecision::Applied
    ));
}

#[test]
fn restore_creates_new_generation() {
    let mut s = ReplicaProjection::default();
    let id = task_id(2);
    apply_operation(
        &mut s,
        &set_field(
            id,
            EntityKind::Task,
            "title",
            "task".into(),
            stamp(1, 1, 100, 1),
        ),
    )
    .unwrap();
    apply_operation(&mut s, &delete(id, EntityKind::Task, stamp(1, 2, 200, 1))).unwrap();
    apply_operation(&mut s, &restore(id, EntityKind::Task, stamp(1, 3, 300, 1))).unwrap();
    assert!(!s.entities[&id].deleted);
    assert_eq!(s.entities[&id].generation, LifecycleGeneration(2));
}

#[test]
fn stale_generation_operations_are_ignored() {
    let mut s = ReplicaProjection::default();
    let id = task_id(3);
    apply_operation(
        &mut s,
        &set_field(
            id,
            EntityKind::Task,
            "title",
            "gen1".into(),
            stamp(1, 1, 100, 1),
        ),
    )
    .unwrap();
    // Restore pushes the entity to generation 2.
    apply_operation(&mut s, &delete(id, EntityKind::Task, stamp(1, 2, 150, 1))).unwrap();
    apply_operation(&mut s, &restore(id, EntityKind::Task, stamp(1, 3, 200, 1))).unwrap();
    // A late-arriving operation from generation 1 is ignored, even with a newer HLC.
    let r = apply_operation(
        &mut s,
        &set_field(
            id,
            EntityKind::Task,
            "title",
            "late".into(),
            stamp(2, 1, 500, 1),
        ),
    )
    .unwrap();
    assert!(matches!(
        r.decision,
        todo_domain::register::ApplyDecision::IgnoredStaleGeneration
    ));
    assert_eq!(
        s.entities[&id].fields["title"].value,
        serde_json::json!("gen1")
    );
}

#[test]
fn task_deletion_cascades_to_subtasks() {
    let mut s = ReplicaProjection::default();
    let task = task_id(4);
    let sub = task_id(5);
    apply_operation(
        &mut s,
        &set_field(
            task,
            EntityKind::Task,
            "title",
            "parent".into(),
            stamp(1, 1, 100, 1),
        ),
    )
    .unwrap();
    let mut sub_op = set_field(
        sub,
        EntityKind::Subtask,
        "title",
        "child".into(),
        stamp(1, 2, 200, 1),
    );
    sub_op.parent = Some(task);
    apply_operation(&mut s, &sub_op).unwrap();
    apply_operation(&mut s, &delete(task, EntityKind::Task, stamp(1, 3, 300, 1))).unwrap();
    assert!(s.entities[&task].deleted);
    assert!(s.entities[&sub].deleted, "subtask is deleted with its task");
}

#[test]
fn list_deletion_moves_tasks_to_inbox() {
    let mut s = ReplicaProjection::default();
    let list = task_id(6);
    let task = task_id(7);
    apply_operation(
        &mut s,
        &set_field(
            list,
            EntityKind::List,
            "name",
            "work".into(),
            stamp(1, 1, 100, 1),
        ),
    )
    .unwrap();
    apply_operation(
        &mut s,
        &set_field(
            task,
            EntityKind::Task,
            "list_id",
            list.to_string().into(),
            stamp(1, 2, 200, 1),
        ),
    )
    .unwrap();
    apply_operation(&mut s, &delete(list, EntityKind::List, stamp(1, 3, 300, 1))).unwrap();
    // Task is reassigned to Inbox (list_id cleared).
    let list_id = s.entities[&task]
        .fields
        .get("list_id")
        .map(|r| r.value.clone());
    assert_eq!(list_id, Some(serde_json::Value::Null));
}

#[test]
fn tag_delete_removes_reference_without_hidden_reattachment() {
    let mut s = ReplicaProjection::default();
    let tag = task_id(8);
    let task = task_id(9);
    apply_operation(
        &mut s,
        &set_field(
            tag,
            EntityKind::Tag,
            "name",
            "important".into(),
            stamp(1, 1, 100, 1),
        ),
    )
    .unwrap();
    apply_operation(
        &mut s,
        &set_field(
            task,
            EntityKind::Task,
            "tags",
            serde_json::json!([tag.to_string()]),
            stamp(1, 2, 200, 1),
        ),
    )
    .unwrap();
    apply_operation(&mut s, &delete(tag, EntityKind::Tag, stamp(1, 3, 300, 1))).unwrap();
    // The reference is removed from the task.
    let tags = s.entities[&task]
        .fields
        .get("tags")
        .map(|r| r.value.clone());
    assert_eq!(tags, Some(serde_json::json!([])));
}
