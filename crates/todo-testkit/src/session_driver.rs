//! Session driver: replica models and convergence comparison.

use std::collections::BTreeSet;

use todo_domain::ids::OperationId;
use todo_domain::operation::{ReplicaProjection, VerifiedOperation, apply_operation};

/// A replica under test: its projection plus the operations it retains.
/// A `BTreeSet` of operation ids keeps synchronization linear rather than
/// quadratic, so large deterministic suites stay fast.
pub struct Replica {
    pub name: String,
    pub projection: ReplicaProjection,
    pub log: Vec<VerifiedOperation>,
    seen: BTreeSet<OperationId>,
}

impl Replica {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            projection: ReplicaProjection::default(),
            log: Vec::new(),
            seen: BTreeSet::new(),
        }
    }

    /// Apply a locally-originated operation.
    pub fn apply_local(&mut self, op: VerifiedOperation) {
        let id = op.stamp.operation;
        if !self.seen.insert(id) {
            return;
        }
        let _ = apply_operation(&mut self.projection, &op);
        self.log.push(op);
    }

    /// Apply every operation another replica holds. Application is idempotent
    /// and order-insensitive by design, so repeated or reordered delivery
    /// converges.
    pub fn sync_from(&mut self, other: &Replica) {
        for op in &other.log {
            self.apply_local(op.clone());
        }
    }
}

/// Two replicas have converged when their materialized state is identical.
///
/// Only `entities` (registers/clocks, lifecycle generations, tombstones) is
/// compared. The `conflicts` list is a local observation whose *order* depends
/// on when each replica happened to learn an operation, so it is not part of
/// convergence — requiring it would make every partitioned replica look
/// divergent forever.
pub fn converged(a: &Replica, b: &Replica) -> bool {
    a.projection.entities == b.projection.entities
}

/// All replicas in a group have converged. Takes references so callers can
/// keep using the replicas after the check.
pub fn all_converged(replicas: &[&Replica]) -> bool {
    replicas.windows(2).all(|w| converged(w[0], w[1]))
}
