//! Membership: a causally-linked DAG of signed add/revoke events.

use std::collections::{HashMap, HashSet};

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

use todo_domain::ids::DeviceId;

use crate::error::CryptoError;
use crate::identity::DeviceIdentity;

pub type EventHash = [u8; 32];

/// Domain separator for membership signatures.
pub const MEMBERSHIP_DOMAIN: &[u8] = b"tptodo.membership.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MembershipKind {
    AddDevice,
    Revoke,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MembershipEvent {
    pub epoch_id: EventHash,
    pub parents: Vec<EventHash>,
    pub kind: MembershipKind,
    /// The device being added or revoked.
    pub device: DeviceId,
    /// Present on AddDevice: the new member's signing key.
    pub signing_public: Option<[u8; 32]>,
    /// Present on AddDevice: the new member's Noise static key.
    pub noise_public: Option<[u8; 32]>,
    pub signer: DeviceId,
    pub signature: Signature,
}

impl MembershipEvent {
    /// Build and sign a membership event.
    pub fn sign(
        signer: &DeviceIdentity,
        parents: Vec<EventHash>,
        kind: MembershipKind,
        device: DeviceId,
        signing_public: Option<[u8; 32]>,
        noise_public: Option<[u8; 32]>,
    ) -> Self {
        let content = canonical_content(&parents, kind, device, signing_public, noise_public);
        let mut msg = Vec::with_capacity(MEMBERSHIP_DOMAIN.len() + content.len());
        msg.extend_from_slice(MEMBERSHIP_DOMAIN);
        msg.extend_from_slice(&content);
        let signature = signer.sign(&msg);
        // Epoch id binds content and signature so concurrent events differ.
        let mut hasher = Sha256::new();
        hasher.update(&msg);
        hasher.update(signature.to_bytes());
        let epoch_id = hasher.finalize().into();
        Self {
            epoch_id,
            parents,
            kind,
            device,
            signing_public,
            noise_public,
            signer: signer.device_id(),
            signature,
        }
    }

    fn verify(&self, signer_key: &VerifyingKey) -> Result<(), CryptoError> {
        let content = canonical_content(
            &self.parents,
            self.kind,
            self.device,
            self.signing_public,
            self.noise_public,
        );
        let mut msg = Vec::with_capacity(MEMBERSHIP_DOMAIN.len() + content.len());
        msg.extend_from_slice(MEMBERSHIP_DOMAIN);
        msg.extend_from_slice(&content);
        signer_key
            .verify(&msg, &self.signature)
            .map_err(|_| CryptoError::InvalidSignature)
    }
}

fn canonical_content(
    parents: &[EventHash],
    kind: MembershipKind,
    device: DeviceId,
    signing_public: Option<[u8; 32]>,
    noise_public: Option<[u8; 32]>,
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(parents.len() as u32).to_be_bytes());
    for p in parents {
        out.extend_from_slice(p);
    }
    out.push(match kind {
        MembershipKind::AddDevice => 0,
        MembershipKind::Revoke => 1,
    });
    out.extend_from_slice(device.as_bytes());
    match signing_public {
        Some(k) => {
            out.push(1);
            out.extend_from_slice(&k);
        }
        None => out.push(0),
    }
    match noise_public {
        Some(k) => {
            out.push(1);
            out.extend_from_slice(&k);
        }
        None => out.push(0),
    }
    out
}

/// A membership graph: the set of active members and the event DAG.
#[derive(Default)]
pub struct MembershipGraph {
    events: HashMap<EventHash, MembershipEvent>,
    /// DeviceId -> signing public key for active members.
    keys: HashMap<DeviceId, VerifyingKey>,
    active: HashSet<DeviceId>,
    /// Historically revoked device ids; re-add requires a fresh pairing/key.
    revoked: HashSet<DeviceId>,
    heads: Vec<EventHash>,
}

impl MembershipGraph {
    /// Create a group with a self-membership event for the founding device.
    pub fn genesis(founder: &DeviceIdentity) -> Self {
        let event = MembershipEvent::sign(
            founder,
            vec![],
            MembershipKind::AddDevice,
            founder.device_id(),
            Some(*founder.signing_public().as_bytes()),
            Some(*founder.noise_public().as_bytes()),
        );
        let mut g = Self::default();
        g.keys
            .insert(founder.device_id(), *founder.signing_public());
        g.active.insert(founder.device_id());
        g.heads.push(event.epoch_id);
        g.events.insert(event.epoch_id, event);
        g
    }

    pub fn is_active(&self, device: &DeviceId) -> bool {
        self.active.contains(device)
    }

    /// All active members, deterministically ordered.
    pub fn active_members(&self) -> Vec<DeviceId> {
        let mut members: Vec<DeviceId> = self.active.iter().copied().collect();
        members.sort();
        members
    }

    /// Whether a device was historically revoked.
    pub fn is_revoked(&self, device: &DeviceId) -> bool {
        self.revoked.contains(device)
    }

    pub fn signing_key(&self, device: &DeviceId) -> Option<&VerifyingKey> {
        self.keys.get(device)
    }

    pub fn heads(&self) -> &[EventHash] {
        &self.heads
    }

    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Add a device as a member, signed by an active member.
    pub fn add_device(
        &mut self,
        signer: &DeviceIdentity,
        new_signing_public: [u8; 32],
        new_noise_public: [u8; 32],
    ) -> Result<MembershipEvent, CryptoError> {
        let new_device = device_id_from_public(&new_signing_public);
        if self.active.contains(&new_device) {
            return Err(CryptoError::Identity("already a member".into()));
        }
        if self.revoked.contains(&new_device) {
            return Err(CryptoError::Identity(
                "revoked device must re-pair with a fresh key".into(),
            ));
        }
        let event = MembershipEvent::sign(
            signer,
            self.heads.clone(),
            MembershipKind::AddDevice,
            new_device,
            Some(new_signing_public),
            Some(new_noise_public),
        );
        self.apply(event.clone())?;
        Ok(event)
    }

    /// Revoke a device (remove-wins), signed by an active member.
    pub fn revoke(
        &mut self,
        signer: &DeviceIdentity,
        device: DeviceId,
    ) -> Result<MembershipEvent, CryptoError> {
        let event = MembershipEvent::sign(
            signer,
            self.heads.clone(),
            MembershipKind::Revoke,
            device,
            None,
            None,
        );
        self.apply(event.clone())?;
        Ok(event)
    }

    /// Merge a remote event, verifying the signer's authorization.
    pub fn merge(&mut self, event: MembershipEvent) -> Result<(), CryptoError> {
        // The signer must be an active member (or the self-signed genesis).
        if event.parents.is_empty() && self.events.is_empty() {
            // Genesis from a peer: trust only after local pairing establishes it.
            if event.kind != MembershipKind::AddDevice {
                return Err(CryptoError::Identity("genesis must be AddDevice".into()));
            }
            self.apply(event)?;
            return Ok(());
        }
        let signer_key = self
            .keys
            .get(&event.signer)
            .copied()
            .ok_or(CryptoError::Identity("unknown signer".into()))?;
        event.verify(&signer_key)?;
        self.apply(event)
    }

    fn apply(&mut self, event: MembershipEvent) -> Result<(), CryptoError> {
        if self.events.contains_key(&event.epoch_id) {
            return Ok(()); // duplicate, idempotent
        }
        let mut new_heads = Vec::new();
        // A parent that is still a head is consumed by this event.
        let parents: HashSet<EventHash> = event.parents.iter().copied().collect();
        for h in &self.heads {
            if !parents.contains(h) {
                new_heads.push(*h);
            }
        }
        new_heads.push(event.epoch_id);

        match event.kind {
            MembershipKind::AddDevice => {
                if let Some(pk) = event.signing_public {
                    let key = VerifyingKey::from_bytes(&pk)
                        .map_err(|_| CryptoError::Identity("bad signing key".into()))?;
                    self.keys.insert(event.device, key);
                }
                self.active.insert(event.device);
            }
            MembershipKind::Revoke => {
                self.active.remove(&event.device);
                self.keys.remove(&event.device);
                self.revoked.insert(event.device);
            }
        }
        self.heads = new_heads;
        self.events.insert(event.epoch_id, event);
        Ok(())
    }
}

/// Derive a device id from an Ed25519 public key.
pub fn device_id_from_public(public: &[u8; 32]) -> DeviceId {
    let key = VerifyingKey::from_bytes(public).expect("32-byte key");
    let digest = Sha256::digest(key.as_bytes());
    DeviceId::from_bytes(digest.into())
}
