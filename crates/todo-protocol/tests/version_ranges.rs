//! Version summary and missing-range computation.

use std::collections::BTreeMap;

use todo_domain::ids::DeviceId;
use todo_protocol::{SeqRange, VersionSummary};

fn dev(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 32])
}

fn summary(frontiers: &[(u8, u64)], gaps: &[(u8, u64, u64)]) -> VersionSummary {
    let mut s = VersionSummary::default();
    for (d, f) in frontiers {
        s.frontiers.insert(dev(*d), *f);
    }
    for (d, start, end) in gaps {
        s.gaps
            .entry(dev(*d))
            .or_default()
            .push(SeqRange::new(*start, *end));
    }
    s.normalize();
    s
}

#[test]
fn equal_summaries_have_no_missing() {
    let a = summary(&[(1, 10)], &[]);
    let b = summary(&[(1, 10)], &[]);
    assert!(a.missing_from(&b).is_empty());
    assert!(a.covers(&b));
}

#[test]
fn disjoint_frontiers_produce_tail_range() {
    let a = summary(&[(1, 5)], &[]);
    let b = summary(&[(1, 10)], &[]);
    let missing = a.missing_from(&b);
    assert_eq!(missing[&dev(1)], vec![SeqRange::new(5, 10)]);
}

#[test]
fn gap_repair_requests_gaps() {
    // Local has 0..8 but is missing 3..5 (a gap), remote has contiguous 0..8.
    let a = summary(&[(1, 8)], &[(1, 3, 5)]);
    let b = summary(&[(1, 8)], &[]);
    let missing = a.missing_from(&b);
    assert_eq!(missing[&dev(1)], vec![SeqRange::new(3, 5)]);
}

#[test]
fn overlapping_ranges_merge() {
    let a = summary(&[(1, 0)], &[(1, 0, 3), (1, 2, 6), (1, 5, 7)]);
    let b = summary(&[(1, 7)], &[]);
    let missing = a.missing_from(&b);
    // [0,3) U [2,6) U [5,7) merges to [0,7).
    assert_eq!(missing[&dev(1)], vec![SeqRange::new(0, 7)]);
}

#[test]
fn higher_sequence_with_missing_lower_requests_lower_first() {
    // Local frontier is 8, but it is missing 0..2 below its frontier.
    let a = summary(&[(1, 8)], &[(1, 0, 2)]);
    let b = summary(&[(1, 10)], &[]);
    let missing = a.missing_from(&b);
    // Missing [0,2) and [8,10).
    assert_eq!(
        missing[&dev(1)],
        vec![SeqRange::new(0, 2), SeqRange::new(8, 10)]
    );
}

#[test]
fn normalize_sorts_and_merges() {
    let mut s = summary(&[], &[(1, 5, 8), (1, 0, 3), (1, 3, 5)]);
    s.normalize();
    let gaps: Vec<SeqRange> = s.gaps[&dev(1)].clone();
    assert_eq!(gaps, vec![SeqRange::new(0, 8)]);
}

#[test]
fn malicious_huge_range_is_bounded_by_frontier() {
    // A remote claiming frontier 1_000_000 yields a large missing range, but it
    // stays representable and bounded (no overflow).
    let a = summary(&[(1, 0)], &[]);
    let b = summary(&[(1, 1_000_000)], &[]);
    let missing = a.missing_from(&b);
    assert_eq!(missing[&dev(1)], vec![SeqRange::new(0, 1_000_000)]);
}

#[test]
fn missing_map_ignores_unrelated_origins() {
    let a = summary(&[(1, 5)], &[]);
    let b = summary(&[(1, 5), (2, 9)], &[]);
    let missing = a.missing_from(&b);
    // Only origin 2 is missing (9 ops).
    assert!(missing.contains_key(&dev(2)));
    assert!(!missing.contains_key(&dev(1)));
}

#[allow(dead_code)]
fn _btree_used(_: BTreeMap<DeviceId, u64>) {}
