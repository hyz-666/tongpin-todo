//! Network monitor: generation bumps on interface changes.

use std::sync::Mutex;

use todo_discovery::NetworkGeneration;

/// Tracks the current network generation and rediscovery delay.
#[derive(Default)]
pub struct NetworkMonitor {
    generation: Mutex<NetworkGeneration>,
}

impl NetworkMonitor {
    pub fn new() -> Self {
        Self {
            generation: Mutex::new(NetworkGeneration::new()),
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation.lock().unwrap().value()
    }

    /// Advance the generation; all stale candidates are discarded by the
    /// discovery registry, and any stale sockets must be closed.
    pub fn on_network_change(&self) {
        let mut g = self.generation.lock().unwrap();
        *g = g.next();
    }

    /// Rediscovery delay (0–2 s) using an injected jitter fraction.
    pub fn rediscovery_delay(&self, jitter: f64) -> u64 {
        let j = jitter.clamp(0.0, 1.0);
        (2_000.0 * j) as u64
    }
}
