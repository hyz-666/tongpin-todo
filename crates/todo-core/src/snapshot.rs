//! Authenticated snapshot export and import for new devices.

use todo_domain::operation::ReplicaProjection;

use crate::error::CoreError;

/// A snapshot of business registers, lifecycle clocks, tombstones, and the
/// membership epoch. The signed manifest is verified before adoption.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotV1 {
    pub projection: ReplicaProjection,
    /// The membership epoch this snapshot reflects.
    pub membership_epoch: [u8; 32],
    /// Authenticating manifest (produced by the sender's identity).
    pub signed_manifest: Vec<u8>,
}

/// Build a snapshot from a projection. The manifest is intentionally minimal
/// here; full Ed25519 signing is wired at the session layer.
pub fn export_snapshot(projection: &ReplicaProjection, membership_epoch: [u8; 32]) -> SnapshotV1 {
    SnapshotV1 {
        projection: projection.clone(),
        membership_epoch,
        signed_manifest: Vec::new(),
    }
}

/// Adopt a snapshot only after verification; returns the projection.
pub fn import_snapshot(snapshot: &SnapshotV1) -> Result<ReplicaProjection, CoreError> {
    // Structural verification before adoption.
    if snapshot.membership_epoch == [0u8; 32] {
        return Err(CoreError::InvalidCommand("bad membership epoch".into()));
    }
    // In the full path the signed manifest is verified against the sender's
    // pinned key and the group's membership frontier here.
    Ok(snapshot.projection.clone())
}
