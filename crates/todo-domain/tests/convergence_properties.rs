//! Deterministic convergence: any delivery order yields the same projection.

use proptest::prelude::*;

use todo_domain::clock::Hlc;
use todo_domain::ids::{DeviceId, EntityId, LifecycleGeneration, OperationId};
use todo_domain::operation::{
    EntityKind, OperationPayload, ReplicaProjection, VerifiedOperation, apply_operation,
};
use todo_domain::register::VersionStamp;

fn op_strategy() -> impl Strategy<Value = Vec<VerifiedOperation>> {
    let field = prop_oneof![Just("title"), Just("completed"), Just("note")];
    let value = prop_oneof![
        Just(serde_json::json!("alpha")),
        Just(serde_json::json!("beta")),
        Just(serde_json::json!(true)),
        Just(serde_json::json!(false)),
    ];
    let entity_idx = 0u8..4u8;
    prop::collection::vec((entity_idx, field, value), 0..24).prop_map(|ops| {
        ops.into_iter()
            .enumerate()
            .map(|(i, (entity_idx, field, value))| {
                let device = DeviceId::from_bytes([1; 32]);
                let stamp = VersionStamp {
                    generation: LifecycleGeneration(1),
                    hlc: Hlc::new(i as i64, 0),
                    device,
                    operation: OperationId::new(device, i as u64),
                };
                VerifiedOperation {
                    entity: EntityId::new_v7_for_test(entity_idx as u128),
                    kind: EntityKind::Task,
                    parent: None,
                    stamp,
                    payload: OperationPayload::SetField {
                        field: field.to_string(),
                        value,
                    },
                }
            })
            .collect()
    })
}

fn apply_in_order(ops: &[VerifiedOperation]) -> ReplicaProjection {
    let mut s = ReplicaProjection::default();
    for op in ops {
        apply_operation(&mut s, op).unwrap();
    }
    s
}

// Conflict history is an append-only diagnostic log whose order depends on
// delivery order; only the business projection must converge.
fn assert_converged(a: &ReplicaProjection, b: &ReplicaProjection) {
    assert_eq!(a.entities, b.entities);
    assert_eq!(a.tag_canonical, b.tag_canonical);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn replicas_converge_regardless_of_order(ops in op_strategy()) {
        let forward = apply_in_order(&ops);
        let reversed: Vec<VerifiedOperation> = ops.iter().cloned().rev().collect();
        let backward = apply_in_order(&reversed);
        assert_converged(&forward, &backward);
    }

    #[test]
    fn replicas_converge_with_duplicates(ops in op_strategy()) {
        let mut doubled = ops.clone();
        doubled.extend(ops.iter().cloned());
        let a = apply_in_order(&doubled);
        let b = apply_in_order(&ops);
        assert_converged(&a, &b);
    }
}
