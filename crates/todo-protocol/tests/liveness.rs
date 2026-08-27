//! Liveness: heartbeat and death detection with a fake clock.

use todo_protocol::{DEAD_AFTER_MS, HEARTBEAT_INTERVAL_MS, Liveness, SessionState};

#[test]
fn heartbeat_fires_every_interval() {
    let mut l = Liveness::new(0);
    assert!(!l.should_heartbeat(0));
    assert!(l.should_heartbeat(HEARTBEAT_INTERVAL_MS));
    l.on_tx(HEARTBEAT_INTERVAL_MS);
    assert!(!l.should_heartbeat(HEARTBEAT_INTERVAL_MS + 1));
    assert!(l.should_heartbeat(2 * HEARTBEAT_INTERVAL_MS));
}

#[test]
fn authenticated_traffic_resets_idle() {
    let mut l = Liveness::new(0);
    // Traffic at t=20s resets the idle timer.
    l.on_rx(20_000);
    assert!(!l.is_dead(30_000));
    assert!(!l.is_dead(40_000));
    // Dead at t=20s + 30s + 1.
    assert!(l.is_dead(50_001));
}

#[test]
fn dead_after_threshold_without_traffic() {
    let l = Liveness::new(0);
    assert!(!l.is_dead(DEAD_AFTER_MS));
    assert!(l.is_dead(DEAD_AFTER_MS + 1));
}

#[test]
fn state_transitions_are_linear() {
    let mut s = todo_protocol::PeerSession::new();
    assert_eq!(s.state, SessionState::Offline);
    s.transition(SessionState::Dialing);
    assert_eq!(s.state, SessionState::Dialing);
    s.transition(SessionState::Handshaking);
    assert_eq!(s.state, SessionState::Handshaking);
    s.transition(SessionState::Syncing);
    assert_eq!(s.state, SessionState::Syncing);
}
