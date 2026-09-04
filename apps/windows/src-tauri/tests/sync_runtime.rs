//! Sync runtime: generation, sessions, and dial preference.

use tongpin_windows_lib::{SyncRuntime, preferred_dialer};

#[test]
fn network_change_bumps_generation() {
    let rt = SyncRuntime::new();
    assert_eq!(rt.network_generation(), 0);
    rt.on_network_change();
    assert_eq!(rt.network_generation(), 1);
    rt.on_network_change();
    assert_eq!(rt.network_generation(), 2);
}

#[test]
fn sessions_accumulate() {
    let rt = SyncRuntime::new();
    assert_eq!(rt.session_count(), 0);
    rt.on_session_established();
    rt.on_session_established();
    assert_eq!(rt.session_count(), 2);
}

#[test]
fn lower_device_id_is_preferred_dialer() {
    assert!(preferred_dialer(&[0x01; 32], &[0x02; 32]));
    assert!(!preferred_dialer(&[0x02; 32], &[0x01; 32]));
    assert!(!preferred_dialer(&[0x01; 32], &[0x01; 32]));
}

#[test]
fn listener_binds_ephemeral_port() {
    let handle = tongpin_windows_lib::ListenerHandle::bind_loopback().unwrap();
    assert!(handle.port > 0);
}
