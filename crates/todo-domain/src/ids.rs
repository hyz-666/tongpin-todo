//! Identity value objects: entity ids, device fingerprints, and operation ids.

use uuid::Uuid;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct EntityId(Uuid);

impl EntityId {
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7())
    }

    /// Deterministic id for tests and fixtures.
    pub fn new_v7_for_test(seed: u128) -> Self {
        Self(Uuid::from_u128(seed))
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_bytes(&self) -> [u8; 16] {
        *self.0.as_bytes()
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct DeviceId(pub [u8; 32]);

impl DeviceId {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// A lifecycle generation identifies one incarnation of an entity across
/// delete/restore cycles.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
pub struct LifecycleGeneration(pub u32);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct OperationId {
    pub origin: DeviceId,
    pub sequence: u64,
}

impl OperationId {
    pub const fn new(origin: DeviceId, sequence: u64) -> Self {
        Self { origin, sequence }
    }

    pub fn next_sequence(&self) -> Option<Self> {
        self.sequence.checked_add(1).map(|sequence| Self {
            origin: self.origin,
            sequence,
        })
    }
}
