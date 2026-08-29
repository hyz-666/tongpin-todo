//! Per-peer connection runtime: state and attempt counters.

use std::collections::HashMap;

use todo_domain::ids::DeviceId;
use todo_protocol::SessionState;

use crate::backoff::AttemptCounter;

/// Runtime state for one peer.
#[derive(Clone, Copy, Debug, Default)]
pub struct PeerEntry {
    pub state: SessionState,
    pub attempts: AttemptCounter,
}

/// Per-peer connection state machine.
#[derive(Default)]
pub struct PeerRuntime {
    peers: HashMap<DeviceId, PeerEntry>,
}

impl PeerRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entry(&mut self, peer: DeviceId) -> &mut PeerEntry {
        self.peers.entry(peer).or_default()
    }

    pub fn state(&self, peer: &DeviceId) -> SessionState {
        self.peers
            .get(peer)
            .map(|p| p.state)
            .unwrap_or(SessionState::Offline)
    }

    pub fn transition(&mut self, peer: DeviceId, next: SessionState) {
        self.entry(peer).state = next;
    }

    /// Reset the dial attempt counter after an authenticated session.
    pub fn reset_on_auth(&mut self, peer: DeviceId) {
        self.entry(peer).attempts.reset_on_auth();
    }

    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }
}
