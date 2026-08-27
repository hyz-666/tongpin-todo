//! Deterministic dialer and session selection.

use todo_discovery::{preferred_dialer, select_session_owner};
use todo_domain::ids::DeviceId;

#[test]
fn lower_device_id_is_preferred_dialer() {
    let a = DeviceId::from_bytes([0x01; 32]);
    let b = DeviceId::from_bytes([0x02; 32]);
    assert!(preferred_dialer(&a, &b));
    assert!(!preferred_dialer(&b, &a));
}

#[test]
fn same_device_is_not_its_own_dialer() {
    let a = DeviceId::from_bytes([0x01; 32]);
    assert!(!preferred_dialer(&a, &a));
}

#[test]
fn simultaneous_dial_keeps_lower_session_owner() {
    let a = DeviceId::from_bytes([0xAA; 32]);
    let b = DeviceId::from_bytes([0x55; 32]);
    // Both peers compute the same owner (the lower id).
    assert_eq!(select_session_owner(&a, &b), select_session_owner(&b, &a));
    assert_eq!(select_session_owner(&a, &b), b);
}
