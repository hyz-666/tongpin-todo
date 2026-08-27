//! Pairing state machine and SAS.

use todo_crypto::{PairingSession, PairingState, now_millis};
use uuid::Uuid;

fn offered() -> PairingSession {
    PairingSession::offered(Uuid::new_v4(), now_millis() + 120_000)
}

#[test]
fn full_confirmation_flow_reaches_paired() {
    let mut s = offered();
    assert_eq!(s.state(), PairingState::Offered);
    s.begin_connecting().unwrap();
    assert_eq!(s.state(), PairingState::Connecting);
    s.set_transcript([0xAB; 32]).unwrap();
    assert_eq!(s.state(), PairingState::SasPendingBoth);
    s.confirm_local().unwrap();
    assert_eq!(s.state(), PairingState::LocalConfirmed);
    s.confirm_remote().unwrap();
    assert_eq!(s.state(), PairingState::Paired);
}

#[test]
fn sas_is_derived_from_transcript() {
    let mut a = offered();
    a.begin_connecting().unwrap();
    a.set_transcript([0x12; 32]).unwrap();

    let mut b = offered();
    b.begin_connecting().unwrap();
    b.set_transcript([0x12; 32]).unwrap();

    assert_eq!(a.sas(), b.sas());
    assert_eq!(a.sas().unwrap().len(), 6);
}

#[test]
fn sas_mismatch_means_different_transcripts() {
    let mut a = offered();
    a.begin_connecting().unwrap();
    a.set_transcript([0x12; 32]).unwrap();

    let mut b = offered();
    b.begin_connecting().unwrap();
    b.set_transcript([0x99; 32]).unwrap();

    assert_ne!(a.sas(), b.sas());
}

#[test]
fn one_sided_confirmation_waits() {
    let mut s = offered();
    s.begin_connecting().unwrap();
    s.set_transcript([0x01; 32]).unwrap();
    s.confirm_remote().unwrap();
    assert_eq!(s.state(), PairingState::RemoteConfirmed);
}

#[test]
fn cancel_is_terminal() {
    let mut s = offered();
    s.begin_connecting().unwrap();
    s.set_transcript([0x01; 32]).unwrap();
    s.cancel();
    assert_eq!(s.state(), PairingState::Cancelled);
    // Cancel after paired is a no-op.
    let mut p = offered();
    p.begin_connecting().unwrap();
    p.set_transcript([0x01; 32]).unwrap();
    p.confirm_local().unwrap();
    p.confirm_remote().unwrap();
    p.cancel();
    assert_eq!(p.state(), PairingState::Paired);
}

#[test]
fn expiry_flips_state() {
    let mut s = PairingSession::offered(Uuid::new_v4(), now_millis() + 1_000);
    s.begin_connecting().unwrap();
    s.set_transcript([0x01; 32]).unwrap();
    assert!(!s.is_expired(now_millis()));
    // Past the 1-second expiry.
    assert!(s.is_expired(now_millis() + 2_000));
    s.mark_expired();
    assert_eq!(s.state(), PairingState::Expired);
}
