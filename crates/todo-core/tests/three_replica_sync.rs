//! Three-replica convergence: partition, divergent edits, reconnect in any order.

use serde_json::json;
use todo_domain::clock::Hlc;
use todo_domain::ids::{DeviceId, EntityId, LifecycleGeneration, OperationId};
use todo_domain::operation::{EntityKind, OperationPayload, VerifiedOperation};
use todo_domain::register::VersionStamp;
use todo_testkit::{Replica, all_converged};

fn dev(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 32])
}

fn entity(b: u8) -> EntityId {
    EntityId::from_uuid(uuid::Uuid::from_bytes([b; 16]))
}

fn set_field(
    device: DeviceId,
    seq: u64,
    ent: EntityId,
    field: &str,
    value: &str,
    millis: i64,
) -> VerifiedOperation {
    VerifiedOperation {
        entity: ent,
        kind: EntityKind::Task,
        parent: None,
        stamp: VersionStamp {
            generation: LifecycleGeneration(1),
            hlc: Hlc {
                physical_millis: millis,
                logical: 0,
            },
            device,
            operation: OperationId::new(device, seq),
        },
        payload: OperationPayload::SetField {
            field: field.to_string(),
            value: json!(value),
        },
    }
}

fn lifecycle(
    device: DeviceId,
    seq: u64,
    ent: EntityId,
    deleted: bool,
    millis: i64,
) -> VerifiedOperation {
    VerifiedOperation {
        entity: ent,
        kind: EntityKind::Task,
        parent: None,
        stamp: VersionStamp {
            generation: LifecycleGeneration(1),
            hlc: Hlc {
                physical_millis: millis,
                logical: 0,
            },
            device,
            operation: OperationId::new(device, seq),
        },
        payload: if deleted {
            OperationPayload::Delete
        } else {
            OperationPayload::Restore
        },
    }
}

#[test]
fn different_field_edits_converge() {
    let mut a = Replica::new("A");
    let mut b = Replica::new("B");
    let mut c = Replica::new("C");

    // Partitioned: each replica edits a different field of the same entity.
    a.apply_local(set_field(dev(1), 1, entity(9), "title", "来自A", 1_000));
    b.apply_local(set_field(dev(2), 1, entity(9), "note", "来自B", 1_100));
    c.apply_local(set_field(dev(3), 1, entity(9), "priority", "high", 1_200));

    // Reconnect in one order.
    a.sync_from(&b);
    a.sync_from(&c);
    b.sync_from(&a);
    c.sync_from(&a);

    assert!(all_converged(&[&a, &b, &c]), "three replicas must converge");
}

#[test]
fn same_field_edits_converge_by_hlc() {
    let mut a = Replica::new("A");
    let mut b = Replica::new("B");

    // Both edit the same field; the later HLC wins everywhere.
    a.apply_local(set_field(dev(1), 1, entity(9), "title", "A先", 2_000));
    b.apply_local(set_field(dev(2), 1, entity(9), "title", "B后", 2_500));

    a.sync_from(&b);
    b.sync_from(&a);

    assert!(all_converged(&[&a, &b]));
    let winner = a.projection.entities[&entity(9)].fields["title"]
        .value
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(winner, "B后");
}

#[test]
fn reconnect_order_does_not_matter() {
    let mut a = Replica::new("A");
    let mut b = Replica::new("B");
    let mut c = Replica::new("C");

    a.apply_local(set_field(dev(1), 1, entity(1), "title", "一", 3_000));
    b.apply_local(set_field(dev(2), 1, entity(1), "title", "二", 3_100));
    c.apply_local(set_field(dev(3), 1, entity(1), "note", "三", 3_200));

    // A different sync order from the first test: C first, then B.
    a.sync_from(&c);
    a.sync_from(&b);
    b.sync_from(&c);
    b.sync_from(&a);
    c.sync_from(&b);
    c.sync_from(&a);

    assert!(all_converged(&[&a, &b, &c]), "sync order must not matter");
}

#[test]
fn create_delete_restore_converges() {
    let mut a = Replica::new("A");
    let mut b = Replica::new("B");

    let e = entity(7);
    a.apply_local(set_field(dev(1), 1, e, "title", "任务", 4_000));
    a.sync_from(&b);
    b.sync_from(&a);
    assert!(all_converged(&[&a, &b]));

    // A deletes; B has not seen it yet.
    a.apply_local(lifecycle(dev(1), 2, e, true, 4_100));
    b.apply_local(set_field(dev(2), 1, e, "note", "删除后写入", 4_150));

    a.sync_from(&b);
    b.sync_from(&a);
    assert!(all_converged(&[&a, &b]));
    // The delete stands: writes after delete do not resurrect the entity.
    assert!(a.projection.entities[&e].deleted);

    // Restore converges too.
    a.apply_local(lifecycle(dev(1), 3, e, false, 4_200));
    a.sync_from(&b);
    b.sync_from(&a);
    assert!(all_converged(&[&a, &b]));
    assert!(!a.projection.entities[&e].deleted);
}

#[test]
fn repeated_sync_is_idempotent() {
    let mut a = Replica::new("A");
    let mut b = Replica::new("B");
    a.apply_local(set_field(dev(1), 1, entity(9), "title", "幂等", 5_000));

    for _ in 0..5 {
        a.sync_from(&b);
        b.sync_from(&a);
    }
    assert!(all_converged(&[&a, &b]));
    assert_eq!(
        b.log.len(),
        1,
        "duplicate delivery must not duplicate the log"
    );
}
