//! Network-change runtime: generation bumps and rediscovery jitter.

use todo_discovery::NetworkGeneration;

/// Rediscovery delay window after a network change (0–2 seconds).
pub const REDISCOVERY_MAX_MS: u64 = 2_000;

/// Tracks the current network generation and schedules rediscovery.
#[derive(Default)]
pub struct NetworkRuntime {
    generation: NetworkGeneration,
}

impl NetworkRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn generation(&self) -> NetworkGeneration {
        self.generation
    }

    /// Advance the generation on a network change; stale candidates/sockets are
    /// discarded by the discovery registry.
    pub fn on_network_change(&mut self) {
        self.generation = self.generation.next();
    }

    /// The rediscovery delay after a network change, using a jitter fraction
    /// in `[0.0, 1.0]`.
    pub fn rediscovery_delay(&self, jitter: f64) -> u64 {
        let j = jitter.clamp(0.0, 1.0);
        (REDISCOVERY_MAX_MS as f64 * j) as u64
    }
}
