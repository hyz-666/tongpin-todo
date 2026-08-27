//! Core-side membership: trusted members only after a committed pairing.

use std::collections::HashMap;
use std::sync::Mutex;

use todo_crypto::{DeviceIdentity, MembershipGraph};
use todo_domain::ids::DeviceId;

use crate::error::CoreError;

/// A trust store that only grants membership after a pairing transaction
/// completes and commits.
#[derive(Default)]
pub struct MembershipStore {
    graph: Mutex<MembershipGraph>,
    /// Devices authenticated but not yet committed (mid-pairing).
    pending: Mutex<HashMap<DeviceId, ()>>,
}

impl MembershipStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create the group with a self-membership event for the founder.
    pub fn genesis(&self, founder: &DeviceIdentity) {
        let mut graph = self.graph.lock().unwrap();
        *graph = MembershipGraph::genesis(founder);
    }

    pub fn is_trusted(&self, device: &DeviceId) -> bool {
        self.graph.lock().unwrap().is_active(device)
    }

    pub fn member_count(&self) -> usize {
        let g = self.graph.lock().unwrap();
        g.event_count()
    }

    /// Mark a device as authenticated but not yet committed.
    pub fn mark_pending(&self, device: DeviceId) {
        self.pending.lock().unwrap().insert(device, ());
    }

    /// Commit a new member only after snapshot/frontier transaction commits.
    pub fn commit_member(
        &self,
        signer: &DeviceIdentity,
        new_signing_public: [u8; 32],
        new_noise_public: [u8; 32],
    ) -> Result<(), CoreError> {
        let new_device = todo_crypto::membership_device_id(&new_signing_public);
        let mut graph = self.graph.lock().unwrap();
        graph
            .add_device(signer, new_signing_public, new_noise_public)
            .map_err(|e| CoreError::InvalidCommand(e.to_string()))?;
        self.pending.lock().unwrap().remove(&new_device);
        Ok(())
    }

    /// Revoke a member (remove-wins).
    pub fn revoke(&self, signer: &DeviceIdentity, device: DeviceId) -> Result<(), CoreError> {
        let mut graph = self.graph.lock().unwrap();
        graph
            .revoke(signer, device)
            .map_err(|e| CoreError::InvalidCommand(e.to_string()))?;
        Ok(())
    }
}
