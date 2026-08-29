//! Scheduler intents and per-peer sync decisions.

use todo_core::{Scheduler, SchedulerIntent, SyncDecision};
use todo_domain::ids::DeviceId;

fn dev(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 32])
}

#[test]
fn foreground_syncs() {
    let s = Scheduler::new(SchedulerIntent::ForegroundActive);
    assert_eq!(s.decision(&dev(1)), SyncDecision::Sync);
}

#[test]
fn tray_and_fgs_sync() {
    assert_eq!(
        Scheduler::new(SchedulerIntent::WindowsTray).decision(&dev(1)),
        SyncDecision::Sync
    );
    assert_eq!(
        Scheduler::new(SchedulerIntent::AndroidFgs).decision(&dev(1)),
        SyncDecision::Sync
    );
}

#[test]
fn os_deferred_defers() {
    let s = Scheduler::new(SchedulerIntent::OsDeferred);
    assert_eq!(s.decision(&dev(1)), SyncDecision::Defer);
}

#[test]
fn process_stopping_defers() {
    let s = Scheduler::new(SchedulerIntent::ProcessStopping);
    assert_eq!(s.decision(&dev(1)), SyncDecision::Defer);
}

#[test]
fn low_space_pauses() {
    let mut s = Scheduler::new(SchedulerIntent::ForegroundActive);
    s.pause_low_space();
    assert_eq!(s.decision(&dev(1)), SyncDecision::Pause);
    s.resume_from_low_space();
    assert_eq!(s.decision(&dev(1)), SyncDecision::Sync);
}

#[test]
fn incompatible_blocks_only_that_peer() {
    let mut s = Scheduler::new(SchedulerIntent::ForegroundActive);
    s.block_incompatible(dev(1));
    assert_eq!(s.decision(&dev(1)), SyncDecision::Block);
    // Other peers are unaffected.
    assert_eq!(s.decision(&dev(2)), SyncDecision::Sync);
}

#[test]
fn revoked_stops_only_that_peer() {
    let mut s = Scheduler::new(SchedulerIntent::ForegroundActive);
    s.stop_revoked(dev(1));
    assert_eq!(s.decision(&dev(1)), SyncDecision::Stop);
    assert_eq!(s.decision(&dev(2)), SyncDecision::Sync);
}

#[test]
fn ordinary_peer_loss_is_non_intrusive() {
    // A peer simply going offline does not affect the scheduler decision for it
    // (the connection state machine handles retry); the scheduler still says Sync.
    let s = Scheduler::new(SchedulerIntent::ForegroundActive);
    assert_eq!(s.decision(&dev(3)), SyncDecision::Sync);
}
