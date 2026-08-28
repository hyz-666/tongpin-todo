//! Restart/resume: checkpoint reconciliation and sending only missing ranges.

use todo_core::{SyncState, TransferCheckpoint};
use todo_domain::ids::DeviceId;
use todo_protocol::{SeqRange, VersionSummary};

fn dev(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 32])
}

fn summary(frontier: u64) -> VersionSummary {
    let mut s = VersionSummary::default();
    s.frontiers.insert(dev(1), frontier);
    s
}

#[test]
fn sends_only_still_missing_ranges() {
    let peer = dev(1);
    let mut state = SyncState::new();
    state.record_checkpoint(TransferCheckpoint::new(
        peer,
        [0; 16],
        vec![SeqRange::new(0, 100)],
        summary(100),
    ));
    // Peer durably acknowledged up to 60 before a crash.
    state.advance_ack(&peer, 60);

    let remote = summary(0); // peer has nothing
    let local = summary(150); // we have 0..150
    let to_send = state.ranges_to_send(peer, &remote, &local);
    // Only [60, 150) remains to send.
    assert_eq!(to_send, vec![SeqRange::new(60, 150)]);
}

#[test]
fn lost_ack_resends_from_checkpoint() {
    let peer = dev(1);
    let mut state = SyncState::new();
    state.record_checkpoint(TransferCheckpoint::new(
        peer,
        [0; 16],
        vec![SeqRange::new(0, 50)],
        summary(50),
    ));
    state.advance_ack(&peer, 20);

    // Peer restarts and reports frontier 0 (it lost its in-memory state but
    // not its persisted checkpoint on our side).
    let remote = summary(0);
    let local = summary(50);
    let to_send = state.ranges_to_send(peer, &remote, &local);
    assert_eq!(to_send, vec![SeqRange::new(20, 50)]);
}

#[test]
fn new_peer_summary_further_ahead_reduces_work() {
    let peer = dev(1);
    let mut state = SyncState::new();
    state.record_checkpoint(TransferCheckpoint::new(
        peer,
        [0; 16],
        vec![SeqRange::new(0, 100)],
        summary(100),
    ));
    state.advance_ack(&peer, 40);

    // Peer actually already has up to 70 (its summary caught up).
    let remote = summary(70);
    let local = summary(100);
    let to_send = state.ranges_to_send(peer, &remote, &local);
    // Only [70, 100) remains.
    assert_eq!(to_send, vec![SeqRange::new(70, 100)]);
}
