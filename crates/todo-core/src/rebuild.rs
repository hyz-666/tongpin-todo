//! Deterministic projection rebuild from retained valid operations.

use todo_domain::operation::{ReplicaProjection, VerifiedOperation, apply_operation};

/// Rebuild the projection by replaying operations in order.
///
/// This is used after a revocation to discard post-cutoff effects while
/// retaining everything before it. The immutable operation history is never
/// mutated; only the derived projection is regenerated.
pub fn rebuild_projection(ops: &[VerifiedOperation]) -> ReplicaProjection {
    let mut state = ReplicaProjection::default();
    for op in ops {
        let _ = apply_operation(&mut state, op);
    }
    state
}

/// Rebuild the projection keeping only operations up to (and including) a
/// causal cutoff per origin.
pub fn rebuild_with_cutoff(
    ops: &[VerifiedOperation],
    cutoff: &std::collections::BTreeMap<todo_domain::ids::DeviceId, u64>,
) -> ReplicaProjection {
    let retained: Vec<VerifiedOperation> = ops
        .iter()
        .filter(|op| {
            cutoff
                .get(&op.stamp.device)
                .map(|c| op.stamp.operation.sequence <= *c)
                .unwrap_or(true)
        })
        .cloned()
        .collect();
    rebuild_projection(&retained)
}
