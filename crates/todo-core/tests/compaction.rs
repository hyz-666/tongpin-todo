//! Compaction: stable watermarks and membership-aware collection.

use std::collections::BTreeMap;

use todo_core::{compute_watermark, tombstone_collectable};
use todo_domain::ids::DeviceId;
use todo_storage::{CompactionPlan, compactable};

fn dev(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 32])
}

#[test]
fn watermark_is_minimum_across_members() {
    let mut acks = BTreeMap::new();
    // Origin 1 has two members: one acked 100, the other only 40.
    acks.insert(dev(1), vec![(dev(2), 100), (dev(3), 40)]);
    let wm = compute_watermark(&acks);
    assert_eq!(wm.per_origin[&dev(1)], 40);
    assert!(wm.covers(&dev(1), 40));
    assert!(!wm.covers(&dev(1), 41));
}

#[test]
fn one_offline_member_lowers_watermark() {
    // Origin 1: members acked 100 and 100 -> watermark 100.
    let mut acks = BTreeMap::new();
    acks.insert(dev(1), vec![(dev(2), 100), (dev(3), 100)]);
    let wm = compute_watermark(&acks);
    assert_eq!(wm.per_origin[&dev(1)], 100);

    // Now member 3 has been offline and only acked 0.
    acks.insert(dev(1), vec![(dev(2), 100), (dev(3), 0)]);
    let wm = compute_watermark(&acks);
    assert_eq!(wm.per_origin[&dev(1)], 0);
}

#[test]
fn no_acks_means_zero_watermark() {
    let wm = compute_watermark(&BTreeMap::new());
    assert!(wm.per_origin.is_empty());
}

#[test]
fn tombstone_requires_age_and_all_acks() {
    assert!(!tombstone_collectable(29, true));
    assert!(!tombstone_collectable(30, false));
    assert!(!tombstone_collectable(29, false));
    assert!(tombstone_collectable(30, true));
    assert!(tombstone_collectable(365, true));
}

#[test]
fn compaction_plan_batches() {
    let plan = CompactionPlan::plan(600, 256);
    assert_eq!(plan.total_batches, 3);
    assert_eq!(plan.batch_range(0), Some((0, 256)));
    assert_eq!(plan.batch_range(2), Some((512, 768)));
    assert_eq!(plan.batch_range(3), None);
}

#[test]
fn compactable_under_watermark() {
    assert!(compactable(10, 10));
    assert!(compactable(9, 10));
    assert!(!compactable(11, 10));
}

#[test]
fn zero_batch_size_yields_empty_plan() {
    let plan = CompactionPlan::plan(100, 0);
    assert_eq!(plan.total_batches, 0);
}
