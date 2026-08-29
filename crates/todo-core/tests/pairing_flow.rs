//! Pairing flow: the end-to-end pairing orchestration.

use todo_core::{PairingAction, PairingFlow, PairingPhase};
use todo_domain::ids::DeviceId;
use uuid::Uuid;

fn device(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 32])
}

const TRANSCRIPT: [u8; 32] = [0x42; 32];

#[test]
fn full_pairing_flow_reaches_committed_member() {
    let id = Uuid::new_v4();
    let mut flow = PairingFlow::new(id);

    // Offer begins; show QR.
    let action = flow.start_offering();
    assert_eq!(flow.phase(), PairingPhase::Offering);
    assert!(matches!(action, PairingAction::ShowQr { session_id } if session_id == id));

    // Peer detected; discover.
    let action = flow.on_peer_detected();
    assert_eq!(flow.phase(), PairingPhase::Discovering);
    assert_eq!(action, PairingAction::Discover);

    // Candidate found; connect.
    let action = flow.on_discovered("192.168.1.20", 5353);
    assert_eq!(flow.phase(), PairingPhase::Connecting);
    assert_eq!(
        action,
        PairingAction::Connect {
            ip: "192.168.1.20".to_string(),
            port: 5353,
        }
    );

    // Handshake completes; show SAS.
    let action = flow.on_connected(TRANSCRIPT, device(2));
    assert_eq!(flow.phase(), PairingPhase::ConfirmingSas);
    assert!(matches!(action, PairingAction::ShowSas { .. }));
    assert_eq!(flow.remote_device(), Some(device(2)));
    assert!(flow.sas().is_some());

    // Local confirms first; await peer.
    let action = flow.confirm_local();
    assert_eq!(flow.phase(), PairingPhase::ConfirmingSas);
    assert_eq!(action, PairingAction::AwaitPeerConfirmation);

    // Peer confirms; both confirmed -> commit member.
    let action = flow.confirm_remote();
    assert_eq!(flow.phase(), PairingPhase::Committing);
    assert_eq!(action, PairingAction::CommitMember { device: device(2) });

    // Member committed; done.
    let action = flow.on_member_committed();
    assert_eq!(flow.phase(), PairingPhase::Paired);
    assert_eq!(action, PairingAction::Done);
}

#[test]
fn remote_confirms_before_local_still_converges() {
    let mut flow = PairingFlow::new(Uuid::new_v4());
    flow.start_offering();
    flow.on_peer_detected();
    flow.on_discovered("192.168.1.20", 5353);
    flow.on_connected(TRANSCRIPT, device(2));

    // Remote confirms first.
    let action = flow.confirm_remote();
    assert_eq!(flow.phase(), PairingPhase::ConfirmingSas);
    assert_eq!(action, PairingAction::AwaitPeerConfirmation);

    // Then local confirms; both confirmed -> commit.
    let action = flow.confirm_local();
    assert_eq!(flow.phase(), PairingPhase::Committing);
    assert_eq!(action, PairingAction::CommitMember { device: device(2) });
}

#[test]
fn handshake_failure_marks_failed() {
    let mut flow = PairingFlow::new(Uuid::new_v4());
    flow.start_offering();

    // set_transcript fails when not connecting; our flow maps it to Failed.
    let mut broken = PairingFlow::new(Uuid::new_v4());
    // Jump straight to on_connected without connecting; transcript is rejected.
    let action = broken.on_connected(TRANSCRIPT, device(2));
    assert_eq!(broken.phase(), PairingPhase::Failed);
    assert_eq!(action, PairingAction::Done);
    let _ = flow;
}

#[test]
fn cancel_marks_cancelled() {
    let mut flow = PairingFlow::new(Uuid::new_v4());
    flow.start_offering();
    flow.on_peer_detected();

    flow.cancel();
    assert_eq!(flow.phase(), PairingPhase::Cancelled);
}

#[test]
fn expiry_is_detected() {
    let mut flow = PairingFlow::new(Uuid::new_v4());
    flow.start_offering();

    // The offer has a 120s expiry from construction; epoch 0 is not expired.
    assert!(!flow.is_expired(0));
    // 2030 (about 1.9e12 ms) is well past any offer's expiry.
    assert!(flow.is_expired(1_900_000_000_000));
}
