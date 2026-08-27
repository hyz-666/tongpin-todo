//! Merge semantics for version-stamped registers.

use todo_domain::clock::Hlc;
use todo_domain::ids::{DeviceId, LifecycleGeneration, OperationId};
use todo_domain::register::{ApplyDecision, Register, VersionStamp, merge_register};

fn stamp(device: u8, seq: u64, millis: i64, logical: u32, generation: u32) -> VersionStamp {
    VersionStamp {
        generation: LifecycleGeneration(generation),
        hlc: Hlc::new(millis, logical),
        device: DeviceId::from_bytes([device; 32]),
        operation: OperationId::new(DeviceId::from_bytes([device; 32]), seq),
    }
}

fn reg<T>(value: T, stamp: VersionStamp) -> Register<T> {
    Register { value, stamp }
}

#[test]
fn empty_current_applies() {
    let incoming = reg("hello".to_string(), stamp(1, 1, 1000, 0, 1));
    assert_eq!(merge_register(None, &incoming), ApplyDecision::Applied);
}

#[test]
fn newer_stamp_wins() {
    let old = reg("old".to_string(), stamp(1, 1, 1000, 0, 1));
    let new = reg("new".to_string(), stamp(1, 2, 2000, 0, 1));
    assert_eq!(merge_register(Some(&old), &new), ApplyDecision::Applied);
    assert_eq!(
        merge_register(Some(&new), &old),
        ApplyDecision::IgnoredOlderRegister
    );
}

#[test]
fn equal_hlc_uses_device_id() {
    let a = reg("a".to_string(), stamp(1, 1, 1000, 0, 1));
    let b = reg("b".to_string(), stamp(2, 1, 1000, 0, 1)); // same HLC, device 2 > 1
    assert_eq!(merge_register(Some(&a), &b), ApplyDecision::Applied);
    assert_eq!(
        merge_register(Some(&b), &a),
        ApplyDecision::IgnoredOlderRegister
    );
}

#[test]
fn duplicate_is_noop() {
    let s = stamp(1, 1, 1000, 0, 1);
    let r = reg("same".to_string(), s);
    assert_eq!(merge_register(Some(&r), &r), ApplyDecision::Duplicate);
}

#[test]
fn stale_generation_ignored_even_with_newer_hlc() {
    let cur = reg("current".to_string(), stamp(1, 1, 2000, 0, 2)); // generation 2
    let stale = reg("stale".to_string(), stamp(1, 1, 3000, 0, 1)); // generation 1, newer HLC
    assert_eq!(
        merge_register(Some(&cur), &stale),
        ApplyDecision::IgnoredStaleGeneration
    );
}

#[test]
fn same_stamp_different_value_is_rejected() {
    let s = stamp(1, 1, 1000, 0, 1);
    let a = reg("a".to_string(), s);
    let b = reg("b".to_string(), s); // identical stamp, different value
    assert_eq!(merge_register(Some(&a), &b), ApplyDecision::Rejected);
}
