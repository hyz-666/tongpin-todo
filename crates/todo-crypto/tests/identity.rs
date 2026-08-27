//! Device identity: generation, fingerprint, key separation, redacted debug.

use todo_crypto::{DeviceIdentity, device_id_from_public};

#[test]
fn generation_is_stable_and_distinct() {
    let a = DeviceIdentity::generate();
    let b = DeviceIdentity::generate();
    assert_ne!(a.device_id(), b.device_id());
    assert_ne!(a.signing_public(), b.signing_public());
    assert_ne!(a.noise_public(), b.noise_public());
}

#[test]
fn fingerprint_is_sha256_of_signing_public() {
    let id = DeviceIdentity::generate();
    let recomputed = device_id_from_public(id.signing_public());
    assert_eq!(id.device_id(), recomputed);
}

#[test]
fn signing_and_noise_keys_are_separate() {
    let id = DeviceIdentity::generate();
    // The Ed25519 public key and X25519 static public key differ.
    assert_ne!(id.signing_public().as_bytes(), id.noise_public().as_bytes());
}

#[test]
fn binding_contains_public_material_only() {
    let id = DeviceIdentity::generate();
    let binding = id.binding();
    assert_eq!(binding.device_id, id.device_id());
    assert_eq!(binding.signing_public, *id.signing_public().as_bytes());
    assert_eq!(binding.noise_static_public, *id.noise_public().as_bytes());
}

#[test]
fn debug_is_redacted() {
    let id = DeviceIdentity::generate();
    let dbg = format!("{id:?}");
    // Must contain the device id, but never leak secret key material.
    assert!(dbg.contains("DeviceIdentity"));
    assert!(dbg.contains("device_id"));
    assert!(!dbg.contains("noise_static"));
    assert!(!dbg.contains("signing"));
}

#[test]
fn shared_secret_is_symmetric() {
    let a = DeviceIdentity::generate();
    let b = DeviceIdentity::generate();
    let s1 = a.shared_secret(b.noise_public().as_bytes());
    let s2 = b.shared_secret(a.noise_public().as_bytes());
    assert_eq!(s1, s2);
}
