//! Bounded exponential backoff with full jitter.

/// Chunk-retry delays in milliseconds (about 1/2/4/8/16 seconds).
pub const CHUNK_RETRY_MS: [u64; 5] = [1_000, 2_000, 4_000, 8_000, 16_000];

/// Dial backoff steps in milliseconds (1s/2s/5s/10s/30s/60s/2m/5m).
pub const DIAL_BACKOFF_MS: [u64; 8] = [
    1_000, 2_000, 5_000, 10_000, 30_000, 60_000, 120_000, 300_000,
];

/// The dial backoff cap (5 minutes).
pub const DIAL_BACKOFF_CAP_MS: u64 = 300_000;

/// Compute a full-jitter delay from a base value and a jitter fraction in
/// `[0.0, 1.0]`, where `1.0` means uniform in `[0, base]`.
pub fn jittered(base_ms: u64, jitter: f64) -> u64 {
    let j = jitter.clamp(0.0, 1.0);
    let scaled = (base_ms as f64 * j) as u64;
    base_ms.saturating_sub(scaled)
}

/// The chunk-retry delay for a 0-based attempt, capped at the last step.
pub fn chunk_retry_delay(attempt: u32, jitter: f64) -> u64 {
    let idx = (attempt as usize).min(CHUNK_RETRY_MS.len() - 1);
    jittered(CHUNK_RETRY_MS[idx], jitter)
}

/// The dial-backoff delay for a 0-based attempt, capped at 5 minutes.
pub fn dial_delay(attempt: u32, jitter: f64) -> u64 {
    let idx = (attempt as usize).min(DIAL_BACKOFF_MS.len() - 1);
    jittered(DIAL_BACKOFF_MS[idx], jitter)
}

/// A resettable attempt counter for a single peer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AttemptCounter {
    pub attempts: u32,
    /// Only an authenticated session resets the counter.
    pub authenticated: bool,
}

impl AttemptCounter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a failed dial attempt and return the next delay.
    pub fn record_failure(&mut self, jitter: f64) -> u64 {
        let delay = dial_delay(self.attempts, jitter);
        self.attempts = self.attempts.saturating_add(1);
        delay
    }

    /// Reset the counter; only called after an authenticated session.
    pub fn reset_on_auth(&mut self) {
        self.attempts = 0;
        self.authenticated = true;
    }
}
