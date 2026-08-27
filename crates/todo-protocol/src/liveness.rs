//! Liveness: heartbeats, idle reset, and death detection.

/// Heartbeat interval in milliseconds.
pub const HEARTBEAT_INTERVAL_MS: i64 = 10_000;

/// A peer is considered dead after this many milliseconds without traffic.
pub const DEAD_AFTER_MS: i64 = 30_000;

/// Connect timeout (ms).
pub const CONNECT_TIMEOUT_MS: i64 = 5_000;

/// Handshake timeout (ms).
pub const HANDSHAKE_TIMEOUT_MS: i64 = 10_000;

/// Authenticated HELLO timeout (ms).
pub const HELLO_TIMEOUT_MS: i64 = 5_000;

/// Track a peer's last send/receive times against a fake (injected) clock.
#[derive(Clone, Copy, Debug)]
pub struct Liveness {
    last_rx: i64,
    last_tx: i64,
}

impl Liveness {
    pub fn new(now: i64) -> Self {
        Self {
            last_rx: now,
            last_tx: now,
        }
    }

    pub fn on_rx(&mut self, now: i64) {
        self.last_rx = now;
    }

    pub fn on_tx(&mut self, now: i64) {
        self.last_tx = now;
    }

    /// Any authenticated traffic resets idle; only receive time gates death.
    pub fn is_dead(&self, now: i64) -> bool {
        now - self.last_rx > DEAD_AFTER_MS
    }

    pub fn should_heartbeat(&self, now: i64) -> bool {
        now - self.last_tx >= HEARTBEAT_INTERVAL_MS
    }
}

impl Default for Liveness {
    fn default() -> Self {
        Self::new(0)
    }
}
