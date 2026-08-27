//! Boundary tests for identifiers, clocks, and deterministic ordering.

use todo_domain::clock::Hlc;
use todo_domain::error::DomainError;
use todo_domain::ids::{DeviceId, EntityId, OperationId};

#[test]
fn entity_ids_are_unique_uuidv7() {
    let a = EntityId::new_v7();
    let b = EntityId::new_v7();
    assert_ne!(a, b);
    assert_eq!(a.as_uuid().get_version_num(), 7);
}

#[test]
fn device_id_roundtrips_bytes() {
    let mut bytes = [0u8; 32];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = i as u8;
    }
    let id = DeviceId::from_bytes(bytes);
    assert_eq!(id.as_bytes(), &bytes);
}

#[test]
fn operation_sequence_zero_and_exhaustion() {
    let a = DeviceId::from_bytes([1u8; 32]);
    let zero = OperationId::new(a, 0);
    assert_eq!(zero.sequence, 0);
    assert!(zero.next_sequence().is_some());
    let max = OperationId::new(a, u64::MAX);
    assert!(max.next_sequence().is_none());
}

#[test]
fn operation_ordering_by_device_then_sequence() {
    let a = DeviceId::from_bytes([1u8; 32]);
    let b = DeviceId::from_bytes([2u8; 32]);
    let op_a0 = OperationId::new(a, 0);
    let op_a1 = OperationId::new(a, 1);
    let op_b0 = OperationId::new(b, 0);
    assert!(op_a0 < op_a1);
    assert!(op_a1 < op_b0); // a < b by device id, so any a precedes any b
    assert!(op_a0 < op_b0);
}

#[test]
fn hlc_tick_advances_with_wall_clock() {
    let h = Hlc::new(1000, 0);
    let next = h.tick(2000).unwrap();
    assert_eq!(next.physical_millis, 2000);
    assert_eq!(next.logical, 0);
    assert!(next > h);
}

#[test]
fn hlc_tick_does_not_decrease_on_rollback() {
    let h = Hlc::new(2000, 0);
    let next = h.tick(1500).unwrap(); // wall clock moved backwards
    assert_eq!(next.physical_millis, 2000);
    assert_eq!(next.logical, 1);
    assert!(next > h);
}

#[test]
fn hlc_logical_overflow_is_typed_error() {
    let h = Hlc::new(2000, u32::MAX);
    assert!(matches!(h.tick(1500), Err(DomainError::HlcLogicalOverflow)));
}

#[test]
fn hlc_observe_merges_remote() {
    let local = Hlc::new(1000, 0);
    let remote = Hlc::new(2000, 3);
    let merged = local.observe(&remote, 1500).unwrap();
    assert_eq!(merged.physical_millis, 2000);
    assert_eq!(merged.logical, 4); // remote.logical + 1
    assert!(merged > local);
    assert!(merged > remote);
}

#[test]
fn hlc_observe_same_physical_takes_max_logical() {
    let local = Hlc::new(2000, 5);
    let remote = Hlc::new(2000, 3);
    let merged = local.observe(&remote, 1000).unwrap();
    assert_eq!(merged.physical_millis, 2000);
    assert_eq!(merged.logical, 6); // max(5, 3) + 1
}
