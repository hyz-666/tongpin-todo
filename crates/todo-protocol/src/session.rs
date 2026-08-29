//! Per-peer session state machine.

/// Per-peer connection state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SessionState {
    #[default]
    Offline,
    Dialing,
    Handshaking,
    Syncing,
    Backoff,
    Incompatible,
    Revoked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseReason {
    Timeout,
    Revoked,
    Incompatible,
    Local,
    Remote,
}

/// A peer connection with its current state.
#[derive(Clone, Debug)]
pub struct PeerSession {
    pub state: SessionState,
}

impl PeerSession {
    pub fn new() -> Self {
        Self {
            state: SessionState::Offline,
        }
    }

    pub fn transition(&mut self, next: SessionState) {
        self.state = next;
    }
}

impl Default for PeerSession {
    fn default() -> Self {
        Self::new()
    }
}
