//! Discovery hint derivation vectors.

use todo_discovery::{derive_hint, expected_hints, hint_eq};
use todo_domain::ids::DeviceId;

const SECRET: &[u8] = &[0x42; 32];

fn device() -> DeviceId {
    DeviceId::from_bytes([0x11; 32])
}

#[test]
fn hint_is_16_bytes() {
    let h = derive_hint(SECRET, 1, 0, &device());
    assert_eq!(h.len(), 16);
}

#[test]
fn hint_is_deterministic() {
    let a = derive_hint(SECRET, 1, 5, &device());
    let b = derive_hint(SECRET, 1, 5, &device());
    assert_eq!(a, b);
}

#[test]
fn window_rotation_changes_hint() {
    let a = derive_hint(SECRET, 1, 0, &device());
    let b = derive_hint(SECRET, 1, 1, &device());
    assert_ne!(a, b);
}

#[test]
fn expected_hints_cover_prev_current_next() {
    let cur = derive_hint(SECRET, 1, 7, &device());
    let hints = expected_hints(SECRET, 1, 7, &device());
    assert!(hint_eq(&hints[0], &derive_hint(SECRET, 1, 6, &device())));
    assert!(hint_eq(&hints[1], &cur));
    assert!(hint_eq(&hints[2], &derive_hint(SECRET, 1, 8, &device())));
}

#[test]
fn wrong_secret_rejects_hint() {
    let h = derive_hint(SECRET, 1, 0, &device());
    let other = derive_hint(&[0x99; 32], 1, 0, &device());
    assert_ne!(h, other);
}

#[test]
fn wrong_device_rejects_hint() {
    let a = derive_hint(SECRET, 1, 0, &device());
    let b = derive_hint(SECRET, 1, 0, &DeviceId::from_bytes([0x22; 32]));
    assert_ne!(a, b);
}

#[test]
fn wrong_protocol_rejects_hint() {
    let a = derive_hint(SECRET, 1, 0, &device());
    let b = derive_hint(SECRET, 2, 0, &device());
    assert_ne!(a, b);
}

#[test]
fn hint_eq_is_constant_time() {
    let a = derive_hint(SECRET, 1, 0, &device());
    let mut b = a;
    assert!(hint_eq(&a, &b));
    b[0] ^= 0x01;
    assert!(!hint_eq(&a, &b));
}
