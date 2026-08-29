//! Scheduler: platform-independent intents and per-peer sync decisions.

use std::collections::HashSet;

use todo_domain::ids::DeviceId;

/// The platform's scheduling intent, decoupled from platform execution.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SchedulerIntent {
    #[default]
    ForegroundActive,
    WindowsTray,
    AndroidFgs,
    OsDeferred,
    ProcessStopping,
}

/// The decision for whether and how to sync a given peer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncDecision {
    Sync,
    Pause,
    Block,
    Stop,
    Defer,
}

/// A scheduler that maps platform intent plus per-peer trust/state to a decision.
#[derive(Default)]
pub struct Scheduler {
    intent: SchedulerIntent,
    low_space: bool,
    incompatible: HashSet<DeviceId>,
    revoked: HashSet<DeviceId>,
}

impl Scheduler {
    pub fn new(intent: SchedulerIntent) -> Self {
        Self {
            intent,
            ..Self::default()
        }
    }

    pub fn set_intent(&mut self, intent: SchedulerIntent) {
        self.intent = intent;
    }

    pub fn pause_low_space(&mut self) {
        self.low_space = true;
    }

    pub fn resume_from_low_space(&mut self) {
        self.low_space = false;
    }

    pub fn block_incompatible(&mut self, peer: DeviceId) {
        self.incompatible.insert(peer);
    }

    pub fn stop_revoked(&mut self, peer: DeviceId) {
        self.revoked.insert(peer);
    }

    pub fn decision(&self, peer: &DeviceId) -> SyncDecision {
        if self.intent == SchedulerIntent::ProcessStopping {
            return SyncDecision::Defer;
        }
        if self.revoked.contains(peer) {
            return SyncDecision::Stop;
        }
        if self.incompatible.contains(peer) {
            return SyncDecision::Block;
        }
        if self.low_space {
            return SyncDecision::Pause;
        }
        match self.intent {
            SchedulerIntent::OsDeferred => SyncDecision::Defer,
            _ => SyncDecision::Sync,
        }
    }
}
