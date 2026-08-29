//! Safe compaction: stable watermarks and membership-aware tombstone rules.

use std::collections::BTreeMap;

use todo_domain::ids::DeviceId;

/// Tombstones require at least this age (days) plus every active member's ack.
pub const TOMBSTONE_MIN_AGE_DAYS: u32 = 30;

/// The per-origin minimum acknowledgement across every currently trusted,
/// non-revoked member. Operations at or below the watermark are safe to
/// compact away behind a pinned snapshot.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StableWatermark {
    pub per_origin: BTreeMap<DeviceId, u64>,
}

/// Compute the stable watermark from per-origin ACK reports.
///
/// `member_acks` maps each origin to the list of `(member, highest_acked_seq)`
/// reported by every active, non-revoked member.
pub fn compute_watermark(
    member_acks: &BTreeMap<DeviceId, Vec<(DeviceId, u64)>>,
) -> StableWatermark {
    let mut per_origin = BTreeMap::new();
    for (origin, acks) in member_acks {
        let min = acks.iter().map(|(_, ack)| *ack).min().unwrap_or(0);
        per_origin.insert(*origin, min);
    }
    StableWatermark { per_origin }
}

impl StableWatermark {
    pub fn covers(&self, origin: &DeviceId, sequence: u64) -> bool {
        self.per_origin
            .get(origin)
            .map(|wm| sequence <= *wm)
            .unwrap_or(false)
    }
}

/// A tombstone may be collected only when it is old enough and every active
/// member has causally acknowledged it.
pub fn tombstone_collectable(age_days: u32, all_active_members_acked: bool) -> bool {
    age_days >= TOMBSTONE_MIN_AGE_DAYS && all_active_members_acked
}
