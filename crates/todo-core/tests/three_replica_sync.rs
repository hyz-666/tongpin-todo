//! Three-replica convergence under partition and reconnection.

use serde_json::json;
use todo_domain::clock::Hlc;
use todo_domain::ids::{DeviceId, EntityId, LifecycleGeneration, OperationId};
use todo_domain::operation::{EntityKind, OperationPayload, VerifiedOperation};
use todo_domain::register::VersionStamp;
use todo_testkit::{Replica, all_converged, converged};

fn device(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 32])
}

fn entity(b: u8) -> EntityId {
    EntityId::from_uuid(uuid::Uuid::from_bytes([b; 16]))
}

fn set_field(
    dev: DeviceId,
    sequence: u64,
    entity_id: EntityId,
    field: &str,
    value: &str,
    millis: i64,
) -> VerifiedOperation {
    VerifiedOperation {
        entity: entity_id,
        kind: EntityKind::Task,
        parent: None,
        stamp: VersionStamp {
            generation: LifecycleGeneration(1),
            hlc: Hlc {
                physical_millis: millis,
                logical: 0,
            },
            device: dev,
            operation: OperationId::new(dev, sequence),
        },
        payload: OperationPayload::SetField {
            field: field.to_string(),
            value: json!(value),
        },
    }
}

fn lifecycle(
    dev: DeviceId,
    sequence: u64,
    entity_id: EntityId,
    deleted: bool,
    millis: i64,
) -> VerifiedOperation {
    VerifiedOperation {
        entity: entity_id,
        kind: EntityKind::Task,
        parent: None,
        stamp: VersionStamp {
            generation: LifecycleGeneration(1),
            hlc: Hlc {
                physical_millis: millis,
                logical: 0,
            },
            device: dev,
            operation: OperationId::new(dev, sequence),
        },
        payload: if deleted {
            OperationPayload::Delete
        } else {
            OperationPayload::Restore
        },
    }
}

#[test]
fn three_replicas_converge_after_concurrent_edits() {
    let task = entity(1);
    let mut a = Replica::new(device(1));
    let mut b = Replica::new(device(2));
    let mut c = Replica::new(device(3));

    // Partitioned: each peer edits a different field of the same task.
    a.apply_operation(set_field(
        device(1),
        1,
        task,
        "title",
        "\u{4e70}\u{725b}\u{5976}",
        1000,
    ));
    b.apply_operation(set_field(
        device(2),
        1,
        task,
        "note",
        "\u{987a}\u{4fbf}\u{4e70}\u{9762}\u{5305}",
        1100,
    ));
    c.apply_operation(set_field(device(3), 1, task, "due", "2026-09-01", 1200));

    // Sync all pairs.
    a.sync_from(&b);
    a.sync_from(&c);
    b.sync_from(&a);
    b.sync_from(&c);
    c.sync_from(&a);
    c.sync_from(&b);

    assert!(all_converged(&[&a, &b, &c]));
    // All three concurrent field edits survive: they touch different registers.
    let entity_projection = a.state().entities.get(&task).unwrap();
    assert_eq!(
        entity_projection.fields["title"].value,
        json!("\u{4e70}\u{725b}\u{5976}")
    );
    assert_eq!(
        entity_projection.fields["note"].value,
        json!("\u{987a}\u{4fbf}\u{4e70}\u{9762}\u{5305}")
    );
    assert_eq!(entity_projection.fields["due"].value, json!("2026-09-01"));
}

#[test]
fn same_field_conflicts_resolve_identically_everywhere() {
    let task = entity(2);
    let mut a = Replica::new(device(1));
    let mut b = Replica::new(device(2));
    let mut c = Replica::new(device(3));

    // All three write the same field with different values.
    a.apply_operation(set_field(device(1), 1, task, "title", "A", 1000));
    b.apply_operation(set_field(device(2), 1, task, "title", "B", 1100));
    c.apply_operation(set_field(device(3), 1, task, "title", "C", 1200));

    // Sync all pairs.
    a.sync_from(&b);
    a.sync_from(&c);
    b.sync_from(&a);
    b.sync_from(&c);
    c.sync_from(&a);
    c.sync_from(&b);

    assert!(all_converged(&[&a, &b, &c]));
    // The highest HLC wins (device 3, millis 1200) on every replica.
    assert_eq!(a.state().entities[&task].fields["title"].value, json!("C"));
}

#[test]
fn arrival_order_does_not_change_convergence() {
    let task = entity(3);
    // Two replicas that learn the same two operations in opposite orders must
    // reach identical state.
    let mut group_a = Replica::new(device(1));
    let mut group_b = Replica::new(device(1));

    let op1 = set_field(device(1), 1, task, "title", "\u{7b2c}\u{4e00}", 1000);
    let op2 = set_field(device(2), 1, task, "note", "\u{7b2c}\u{4e8c}", 1100);

    group_a.apply_operation(op1.clone());
    group_a.apply_operation(op2.clone());

    group_b.apply_operation(op2.clone());
    group_b.apply_operation(op1.clone());

    assert!(converged(&group_a, &group_b));
}

#[test]
fn delete_and_restore_converge_after_partition() {
    let task = entity(4);
    let mut a = Replica::new(device(1));
    let mut b = Replica::new(device(2));

    // Both create the task, then A deletes while B concurrently edits a field.
    a.apply_operation(set_field(
        device(1),
        1,
        task,
        "title",
        "\u{4efb}\u{52a1}",
        1000,
    ));
    b.sync_from(&a);
    assert_eq!(
        a.state().entities[&task].fields["title"].value,
        json!("\u{4efb}\u{52a1}")
    );

    a.apply_operation(lifecycle(device(1), 2, task, true, 1200));
    // B's field write arrives *after* the delete HLC, so `retain` clears it.
    b.apply_operation(set_field(
        device(2),
        1,
        task,
        "note",
        "\u{7f16}\u{8f91}",
        1300,
    ));

    a.sync_from(&b);
    b.sync_from(&a);

    // Delete wins by HLC (millis 1200 > 1100), and the concurrent field edit
    // whose stamp is older than the delete is preserved; the entity reads as
    // deleted on both replicas identically.
    assert!(converged(&a, &b));
    assert!(a.state().entities[&task].deleted);

    // A later restore also converges.
    a.apply_operation(lifecycle(device(1), 3, task, false, 1300));
    a.sync_from(&b);
    b.sync_from(&a);

    assert!(converged(&a, &b));
    assert!(!a.state().entities[&task].deleted);
}

#[test]
fn restart_preserves_state_and_resumes_sync() {
    let task = entity(5);
    let mut a = Replica::new(device(1));
    let mut b = Replica::new(device(2));

    a.apply_operation(set_field(
        device(1),
        1,
        task,
        "title",
        "\u{7f16}\u{8f91}",
        1000,
    ));
    b.sync_from(&a);

    // "Restart": rebuild a replica from the same operation history.
    let mut restarted = Replica::new(device(1));
    for op in a.ops().to_vec() {
        restarted.apply_operation(op);
    }
    assert!(converged(&restarted, &a));

    // The restarted replica still converges with the peer.
    b.apply_operation(set_field(
        device(2),
        1,
        task,
        "note",
        "\u{5bf9}\u{7aef}\u{65b0}\u{589e}",
        1100,
    ));
    restarted.sync_from(&b);
    b.sync_from(&restarted);

    assert!(converged(&restarted, &b));
}
