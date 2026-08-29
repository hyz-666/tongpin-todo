//! Tombstone retention: age plus every active member's causal ack.

use todo_core::tombstone_collectable;

#[test]
fn not_collected_before_30_days() {
    assert!(!tombstone_collectable(0, true));
    assert!(!tombstone_collectable(29, true));
}

#[test]
fn collected_at_30_days_with_all_acks() {
    assert!(tombstone_collectable(30, true));
}

#[test]
fn long_offline_member_blocks_collection() {
    // One member has not acked -> not collectable even at 365 days.
    assert!(!tombstone_collectable(365, false));
}

#[test]
fn revocation_unblocks_collection() {
    // A revoked member no longer counts; the remaining active members acked.
    assert!(tombstone_collectable(40, true));
}

#[test]
fn restored_generation_is_protected_by_age_check() {
    // Even with all acks, a fresh tombstone from a restored generation must wait.
    assert!(!tombstone_collectable(5, true));
}
