//! Sync runtime: one core/listener set per process with Tokio ownership.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Owns the long-lived sync state for the desktop process. Exactly one instance
/// per process; renderer reloads must reuse it, never duplicate it.
pub struct SyncRuntime {
    network_generation: AtomicU64,
    session_count: AtomicUsize,
}

impl SyncRuntime {
    pub fn new() -> Self {
        Self {
            network_generation: AtomicU64::new(0),
            session_count: AtomicUsize::new(0),
        }
    }

    pub fn network_generation(&self) -> u64 {
        self.network_generation.load(Ordering::Relaxed)
    }

    /// Advance the network generation (Wi-Fi off/on, IP change, sleep/wake).
    pub fn on_network_change(&self) {
        self.network_generation.fetch_add(1, Ordering::Relaxed);
    }

    pub fn session_count(&self) -> usize {
        self.session_count.load(Ordering::Relaxed)
    }

    pub fn on_session_established(&self) {
        self.session_count.fetch_add(1, Ordering::Relaxed);
    }
}

impl Default for SyncRuntime {
    fn default() -> Self {
        Self::new()
    }
}
