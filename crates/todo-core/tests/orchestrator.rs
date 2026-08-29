//! Sync orchestrator: the end-to-end sync loop state machine.

use todo_core::{SyncAction, SyncOrchestrator, SyncPhase};
use todo_domain::ids::DeviceId;
use todo_protocol::{SeqRange, VersionSummary};

fn device(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 32])
}

fn summary(frontier: u64) -> VersionSummary {
    let mut s = VersionSummary::default();
    s.frontiers.insert(device(1), frontier);
    s
}

#[test]
fn full_cycle_drives_every_phase_in_order() {
    let local = summary(0); // we have nothing
    let mut orch = SyncOrchestrator::new(local);

    // Begin against a peer.
    let step = orch.begin(device(2));
    assert_eq!(step.phase, SyncPhase::Discovering);
    assert_eq!(step.action, SyncAction::Discover);

    // A candidate is discovered.
    let step = orch.on_candidate("192.168.1.10", 5353);
    assert_eq!(step.phase, SyncPhase::Dialing);
    assert_eq!(
        step.action,
        SyncAction::Dial {
            ip: "192.168.1.10".to_string(),
            port: 5353,
        }
    );

    // Handshake completes.
    let step = orch.on_handshake();
    assert_eq!(step.phase, SyncPhase::Handshaking);
    assert_eq!(step.action, SyncAction::SendHello);

    // Negotiation completes.
    let step = orch.on_negotiated();
    assert_eq!(step.phase, SyncPhase::Negotiating);
    assert_eq!(step.action, SyncAction::SendSummary);

    // Peer summary arrives: it has 5 operations we lack.
    let step = orch.on_summary(summary(5));
    assert_eq!(step.phase, SyncPhase::ExchangingSummaries);
    assert_eq!(
        step.action,
        SyncAction::SendRangeRequest {
            ranges: vec![SeqRange::new(0, 5)],
        }
    );

    // Chunks arrive; the first three advance the pending range.
    let step = orch.on_chunk(0, 3);
    assert_eq!(step.phase, SyncPhase::Applying);
    assert_eq!(step.action, SyncAction::ApplyChunk { operations: 3 });
    assert_eq!(orch.pending_ranges(), &[SeqRange::new(3, 5)]);

    // The final chunk consumes the range and waits for ACK.
    let step = orch.on_chunk(3, 2);
    assert_eq!(step.phase, SyncPhase::AwaitingAck);
    assert_eq!(step.action, SyncAction::Await);

    // The peer acknowledges; cycle completes.
    let step = orch.on_ack(4);
    assert_eq!(step.phase, SyncPhase::Complete);
    assert_eq!(step.action, SyncAction::Complete);
    assert_eq!(orch.highest_ack(), 4);
}

#[test]
fn empty_summary_completes_immediately() {
    // We already have everything the peer has.
    let local = summary(5);
    let mut orch = SyncOrchestrator::new(local);
    orch.begin(device(2));
    orch.on_candidate("192.168.1.10", 5353);
    orch.on_handshake();
    orch.on_negotiated();

    let step = orch.on_summary(summary(5));
    assert_eq!(step.phase, SyncPhase::Complete);
    assert_eq!(step.action, SyncAction::Complete);
}

#[test]
fn ack_is_monotonic() {
    let local = summary(0);
    let mut orch = SyncOrchestrator::new(local);

    orch.on_ack(3);
    orch.on_ack(1); // a lower ack must not regress
    assert_eq!(orch.highest_ack(), 3);

    orch.on_ack(7);
    assert_eq!(orch.highest_ack(), 7);
}

#[test]
fn error_enters_backoff() {
    let mut orch = SyncOrchestrator::new(summary(0));
    orch.begin(device(2));

    let step = orch.on_error();
    assert_eq!(step.phase, SyncPhase::Backoff);
    assert_eq!(step.action, SyncAction::Await);
}

#[test]
fn fail_is_unrecoverable() {
    let mut orch = SyncOrchestrator::new(summary(0));
    orch.begin(device(2));

    let step = orch.fail();
    assert_eq!(step.phase, SyncPhase::Failed);
}

#[test]
fn reset_returns_to_idle() {
    let mut orch = SyncOrchestrator::new(summary(0));
    orch.begin(device(2));
    orch.on_candidate("192.168.1.10", 5353);

    orch.reset();
    assert_eq!(orch.phase(), SyncPhase::Idle);
    assert!(orch.peer().is_none());
    assert!(orch.pending_ranges().is_empty());
}

#[test]
fn already_acknowledged_ranges_are_skipped() {
    // We have a durable checkpoint at sequence 3, so a peer with 5 operations
    // only owes us [3, 5).
    let mut orch = SyncOrchestrator::new(summary(0));
    orch.on_ack(3);
    orch.begin(device(2));
    orch.on_candidate("192.168.1.10", 5353);
    orch.on_handshake();
    orch.on_negotiated();

    let step = orch.on_summary(summary(5));
    assert_eq!(
        step.action,
        SyncAction::SendRangeRequest {
            ranges: vec![SeqRange::new(3, 5)],
        }
    );
}
