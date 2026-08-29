//! Sync trigger sources: routing through scheduler and network generation.

use todo_core::{SchedulerIntent, SyncTriggers, TriggerDecision, TriggerSource};
use todo_domain::ids::DeviceId;

fn device(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 32])
}

#[test]
fn foreground_manual_triggers() {
    let triggers = SyncTriggers::new(SchedulerIntent::ForegroundActive);
    assert_eq!(
        triggers.evaluate(TriggerSource::Manual, &device(1)),
        TriggerDecision::Trigger
    );
}

#[test]
fn os_deferred_defers_background_sources() {
    let triggers = SyncTriggers::new(SchedulerIntent::OsDeferred);
    assert_eq!(
        triggers.evaluate(TriggerSource::Periodic, &device(1)),
        TriggerDecision::Defer
    );
    assert_eq!(
        triggers.evaluate(TriggerSource::Startup, &device(1)),
        TriggerDecision::Defer
    );
}

#[test]
fn network_change_overrides_defer() {
    let triggers = SyncTriggers::new(SchedulerIntent::OsDeferred);
    // A network change still triggers rediscovery even when OS-deferred.
    assert_eq!(
        triggers.evaluate(TriggerSource::NetworkChange, &device(1)),
        TriggerDecision::Trigger
    );
    // A manual request likewise overrides defer.
    assert_eq!(
        triggers.evaluate(TriggerSource::Manual, &device(1)),
        TriggerDecision::Trigger
    );
}

#[test]
fn low_space_defers_even_manual() {
    let mut triggers = SyncTriggers::new(SchedulerIntent::ForegroundActive);
    triggers.scheduler_mut().pause_low_space();
    assert_eq!(
        triggers.evaluate(TriggerSource::Manual, &device(1)),
        TriggerDecision::Defer
    );
}

#[test]
fn revoked_peer_is_ignored() {
    let mut triggers = SyncTriggers::new(SchedulerIntent::ForegroundActive);
    triggers.scheduler_mut().stop_revoked(device(1));
    assert_eq!(
        triggers.evaluate(TriggerSource::NetworkChange, &device(1)),
        TriggerDecision::Ignore
    );
}

#[test]
fn incompatible_peer_is_ignored() {
    let mut triggers = SyncTriggers::new(SchedulerIntent::ForegroundActive);
    triggers.scheduler_mut().block_incompatible(device(1));
    assert_eq!(
        triggers.evaluate(TriggerSource::Manual, &device(1)),
        TriggerDecision::Ignore
    );
}

#[test]
fn network_change_advances_generation() {
    let mut triggers = SyncTriggers::new(SchedulerIntent::ForegroundActive);
    let g0 = triggers.generation();
    triggers.on_network_change(0.5);
    let g1 = triggers.generation();
    assert!(g1 > g0, "generation must advance on network change");
}

#[test]
fn rediscovery_delay_is_bounded() {
    let mut triggers = SyncTriggers::new(SchedulerIntent::ForegroundActive);
    let delay = triggers.on_network_change(1.0);
    assert!(delay <= 2_000, "max rediscovery delay is 2 seconds");
    assert_eq!(delay, 2_000);
}
