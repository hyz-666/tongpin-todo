//! Version-stamped registers and their deterministic merge semantics.

use serde::{Deserialize, Serialize};

use crate::clock::Hlc;
use crate::ids::{DeviceId, LifecycleGeneration, OperationId};

/// Identifies the version of a value: which lifecycle generation, clock, device,
/// and operation produced it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VersionStamp {
    pub generation: LifecycleGeneration,
    pub hlc: Hlc,
    pub device: DeviceId,
    pub operation: OperationId,
}

/// A last-write-wins register pairing a value with its version stamp.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Register<T> {
    pub value: T,
    pub stamp: VersionStamp,
}

/// The outcome of applying an incoming register against the current one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyDecision {
    Applied,
    Duplicate,
    IgnoredStaleGeneration,
    IgnoredOlderRegister,
    Rejected,
}

/// Deterministically merge `incoming` over `current`. Generation wins first, then
/// the full stamp; equal stamps with equal values are duplicates.
pub fn merge_register<T: PartialEq>(
    current: Option<&Register<T>>,
    incoming: &Register<T>,
) -> ApplyDecision {
    let Some(cur) = current else {
        return ApplyDecision::Applied;
    };
    if incoming.stamp.generation < cur.stamp.generation {
        return ApplyDecision::IgnoredStaleGeneration;
    }
    if incoming.stamp.generation > cur.stamp.generation {
        return ApplyDecision::Applied;
    }
    if incoming.stamp == cur.stamp {
        if incoming.value == cur.value {
            return ApplyDecision::Duplicate;
        }
        return ApplyDecision::Rejected;
    }
    if incoming.stamp < cur.stamp {
        return ApplyDecision::IgnoredOlderRegister;
    }
    ApplyDecision::Applied
}
