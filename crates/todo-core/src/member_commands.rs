//! Member management commands and status.
//!
//! A thin command layer over [`MembershipStore`] exposing the operations a
//! platform UI needs: listing members with their lifecycle status, looking up a
//! single member, and revoking a device.

use todo_crypto::DeviceIdentity;
use todo_domain::ids::DeviceId;

use crate::error::CoreError;
use crate::membership::MembershipStore;

/// A member's lifecycle status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberStatus {
    /// Active and trusted.
    Active,
    /// Authenticated but not yet committed (mid-pairing).
    Pending,
    /// Historically revoked; re-add requires a fresh key.
    Revoked,
    /// Unknown to this replica.
    Unknown,
}

/// A member together with its status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberInfo {
    pub device: DeviceId,
    pub status: MemberStatus,
}

/// High-level member management commands over a membership store.
pub struct MemberCommands<'a> {
    store: &'a MembershipStore,
}

impl<'a> MemberCommands<'a> {
    pub fn new(store: &'a MembershipStore) -> Self {
        Self { store }
    }

    /// List every known member (active, pending, and revoked) deterministically.
    pub fn list_members(&self) -> Vec<MemberInfo> {
        let mut infos: Vec<MemberInfo> = self
            .store
            .active_members()
            .into_iter()
            .map(|device| MemberInfo {
                device,
                status: MemberStatus::Active,
            })
            .collect();

        // Pending devices are those marked but not yet in the graph.
        let pending = self.store_pending_members();
        for device in pending {
            if !infos.iter().any(|m| m.device == device) {
                infos.push(MemberInfo {
                    device,
                    status: MemberStatus::Pending,
                });
            }
        }

        infos.sort_by_key(|m| m.device);
        infos
    }

    /// Look up a single member's status.
    pub fn member_status(&self, device: &DeviceId) -> MemberStatus {
        if self.store.is_trusted(device) {
            MemberStatus::Active
        } else if self.store.is_pending(device) {
            MemberStatus::Pending
        } else if self.store.is_revoked(device) {
            MemberStatus::Revoked
        } else {
            MemberStatus::Unknown
        }
    }

    /// Revoke a member (remove-wins), signed by an active member.
    pub fn revoke_device(
        &self,
        signer: &DeviceIdentity,
        device: DeviceId,
    ) -> Result<(), CoreError> {
        self.store.revoke(signer, device)
    }

    /// Commit a new member after a pairing transaction commits.
    pub fn commit_member(
        &self,
        signer: &DeviceIdentity,
        new_signing_public: [u8; 32],
        new_noise_public: [u8; 32],
    ) -> Result<(), CoreError> {
        self.store
            .commit_member(signer, new_signing_public, new_noise_public)
    }

    fn store_pending_members(&self) -> Vec<DeviceId> {
        self.store.pending_members()
    }
}
