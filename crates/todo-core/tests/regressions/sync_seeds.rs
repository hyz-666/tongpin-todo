//! Deterministic large-scale convergence suite (10,000 operations).

use serde_json::json;
use todo_domain::clock::Hlc;
use todo_domain::ids::{DeviceId, EntityId, LifecycleGeneration, OperationId};
use todo_domain::operation::{EntityKind, OperationPayload, VerifiedOperation};
use todo_domain::register::VersionStamp;
use todo_testkit::{Replica, all_converged};

const SEED: u64 = 0x2545_F491_4F6C_DD1D;
const OP_COUNT: usize = 10_000;

/// Deterministic operation generator (LCG).
fn generate(count: usize, seed: u64) -> Vec<VerifiedOperation> {
    let mut rng = seed;
    let mut next = || {
        rng = rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        rng
    };

    let mut ops = Vec::with_capacity(count);
    for i in 0..count {
        let r = next();
        let device = DeviceId::from_bytes([(r >> 33) as u8 % 3 + 1; 32]);
        let entity = EntityId::from_uuid(uuid::Uuid::from_bytes(
            [(r >> 40) as u8 % 16; 16], // 16 distinct entities
        ));
        let millis = 1_700_000_000_000 + i as i64;
        let op = VerifiedOperation {
            entity,
            kind: EntityKind::Task,
            parent: None,
            stamp: VersionStamp {
                generation: LifecycleGeneration(1),
                hlc: Hlc {
                    physical_millis: millis,
                    logical: 0,
                },
                device,
                operation: OperationId::new(device, i as u64),
            },
            payload: OperationPayload::SetField {
                field: "title".to_string(),
                value: json!(format!("op-{i}")),
            },
        };
        ops.push(op);
    }
    ops
}

#[test]
fn two_replicas_converge_over_10k_operations() {
    let ops = generate(OP_COUNT, SEED);
    let mut a = Replica::new("A");
    let mut b = Replica::new("B");

    // Split the work: A originates the even indices, B the odd ones.
    for (i, op) in ops.iter().enumerate() {
        if i % 2 == 0 {
            a.apply_local(op.clone());
        } else {
            b.apply_local(op.clone());
        }
    }
    a.sync_from(&b);
    b.sync_from(&a);

    assert!(
        all_converged(&[&a, &b]),
        "two replicas must converge over {OP_COUNT} deterministic operations"
    );
}

#[test]
fn three_replicas_converge_over_10k_operations() {
    let ops = generate(OP_COUNT, SEED);
    let mut a = Replica::new("A");
    let mut b = Replica::new("B");
    let mut c = Replica::new("C");

    // Round-robin origin across three replicas.
    for (i, op) in ops.iter().enumerate() {
        match i % 3 {
            0 => a.apply_local(op.clone()),
            1 => b.apply_local(op.clone()),
            _ => c.apply_local(op.clone()),
        }
    }

    // Reconnect in an arbitrary order.
    a.sync_from(&c);
    a.sync_from(&b);
    b.sync_from(&a);
    b.sync_from(&c);
    c.sync_from(&b);
    c.sync_from(&a);

    assert!(
        all_converged(&[&a, &b, &c]),
        "three replicas must converge over {OP_COUNT} deterministic operations"
    );
}

#[test]
fn generation_is_deterministic() {
    let first = generate(100, SEED);
    let second = generate(100, SEED);
    assert_eq!(first, second, "same seed must produce the same operations");
}
