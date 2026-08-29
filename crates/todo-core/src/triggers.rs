//! Sync trigger sources: what causes a sync cycle to start, and how each
//! source is routed through the scheduler and network generation.

use todo_discovery::NetworkGeneration;
use todo_domain::ids::DeviceId;

use crate::network_runtime::NetworkRuntime;
use crate::scheduler::{Scheduler, SchedulerIntent, SyncDecision};

/// What caused a sync trigger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerSource {
    /// A network change (new interface, IP change, rejoin Wi-Fi).
    NetworkChange,
    /// A manual user request.
    Manual,
    /// Application startup.
    Startup,
    /// A periodic timer.
    Periodic,
}

/// The outcome of evaluating a trigger against the scheduler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerDecision {
    /// Trigger a sync cycle now.
    Trigger,
    /// Defer (background scheduling).
    Defer,
    /// Ignore entirely (blocked/revoked).
    Ignore,
}

/// Routes sync triggers through the scheduler and tracks network generation.
pub struct SyncTriggers {
    scheduler: Scheduler,
    network: NetworkRuntime,
}

impl SyncTriggers {
    pub fn new(intent: SchedulerIntent) -> Self {
        Self {
            scheduler: Scheduler::new(intent),
            network: NetworkRuntime::new(),
        }
    }

    pub fn scheduler(&self) -> &Scheduler {
        &self.scheduler
    }

    pub fn scheduler_mut(&mut self) -> &mut Scheduler {
        &mut self.scheduler
    }

    /// The current network generation.
    pub fn generation(&self) -> NetworkGeneration {
        self.network.generation()
    }

    /// Advance the generation on a network change and report the rediscovery
    /// delay (using a jitter fraction in `[0.0, 1.0]`).
    pub fn on_network_change(&mut self, jitter: f64) -> u64 {
        self.network.on_network_change();
        self.network.rediscovery_delay(jitter)
    }

    /// Evaluate a trigger source for a peer. Network changes and manual
    /// requests override a background `Defer` (OS-deferred) decision, but never
    /// a `Block` (incompatible) or `Stop` (revoked).
    pub fn evaluate(&self, source: TriggerSource, peer: &DeviceId) -> TriggerDecision {
        match self.scheduler.decision(peer) {
            SyncDecision::Sync => TriggerDecision::Trigger,
            SyncDecision::Pause => TriggerDecision::Defer,
            SyncDecision::Block | SyncDecision::Stop => TriggerDecision::Ignore,
            SyncDecision::Defer => match source {
                TriggerSource::NetworkChange | TriggerSource::Manual => TriggerDecision::Trigger,
                TriggerSource::Startup | TriggerSource::Periodic => TriggerDecision::Defer,
            },
        }
    }
}

impl Default for SyncTriggers {
    fn default() -> Self {
        Self::new(SchedulerIntent::ForegroundActive)
    }
}
