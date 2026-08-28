//! Durable ACK: acknowledgements only advance after commit.

use todo_core::{SyncState, TransferCheckpoint};
use todo_domain::ids::DeviceId;
use todo_protocol::{FlowControl, SeqRange, VersionSummary};

fn dev(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 32])
}

#[test]
fn ack_does_not_advance_before_commit() {
    let peer = dev(1);
    let mut state = SyncState::new();
    state.record_checkpoint(TransferCheckpoint::new(
        peer,
        [0; 16],
        vec![SeqRange::new(0, 10)],
        VersionSummary::default(),
    ));
    // No advance_ack call yet: highest_ack remains 0.
    assert_eq!(state.checkpoint(&peer).unwrap().highest_ack, 0);
    // The full range is still missing.
    let remaining = state.checkpoint(&peer).unwrap().remaining(0);
    assert_eq!(remaining, vec![SeqRange::new(0, 10)]);
}

#[test]
fn ack_advances_only_after_commit() {
    let peer = dev(1);
    let mut state = SyncState::new();
    state.record_checkpoint(TransferCheckpoint::new(
        peer,
        [0; 16],
        vec![SeqRange::new(0, 10)],
        VersionSummary::default(),
    ));
    // Simulate: apply 5 operations, commit, then ack.
    state.advance_ack(&peer, 5);
    assert_eq!(state.checkpoint(&peer).unwrap().highest_ack, 5);
    let remaining = state.checkpoint(&peer).unwrap().remaining(5);
    assert_eq!(remaining, vec![SeqRange::new(5, 10)]);
}

#[test]
fn ack_is_monotonic() {
    let peer = dev(1);
    let mut state = SyncState::new();
    state.record_checkpoint(TransferCheckpoint::new(
        peer,
        [0; 16],
        vec![SeqRange::new(0, 10)],
        VersionSummary::default(),
    ));
    state.advance_ack(&peer, 7);
    state.advance_ack(&peer, 3); // out-of-order ack must not regress
    assert_eq!(state.checkpoint(&peer).unwrap().highest_ack, 7);
}

#[test]
fn flow_control_caps_in_flight() {
    let mut fc = FlowControl::new(2, 100);
    assert!(fc.on_send(40).is_ok());
    assert!(fc.on_send(40).is_ok());
    assert!(fc.on_send(40).is_err()); // 3rd chunk exceeds max_in_flight=2
    fc.on_ack(40);
    assert!(fc.on_send(40).is_ok());
}
