//! Background LAN sync: discovery loop + trigger routing + orchestrator drive.
//!
//! [`SyncService`] is the long-lived engine that runs sync in the background.
//! It reuses the platform discovery adapter, the scheduler/orchestrator from
//! `todo-core`, and the candidate registry from `todo-discovery` to:
//!
//! 1. browse the LAN for peers (DNS-SD),
//! 2. route each trigger source (startup / periodic / network change / manual)
//!    through the scheduler,
//! 3. drive the [`SyncOrchestrator`] state machine for each allowed peer, and
//! 4. expose a coarse status snapshot (offline / syncing / connected).
//!
//! The Noise handshake and operation transfer (Plan 2/3 transport) are not part
//! of the MVP: [`Self::sync_peer`] drives the orchestrator through a converged
//! cycle and stops at the transport boundary, which a real transport replaces.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;

use todo_core::{
    Scheduler, SchedulerIntent, SyncDecision, SyncOrchestrator, SyncPhase, TriggerDecision,
    TriggerSource,
};
use todo_discovery::{Candidate, CandidateRegistry};
use todo_domain::ids::DeviceId;
use todo_protocol::VersionSummary;

use crate::platform::discovery::{DiscoveryBackend, hex, now_millis, to_candidates};

/// Background sync pass interval.
pub const PERIODIC_INTERVAL: Duration = Duration::from_secs(30);

/// Coarse sync state surfaced to the UI status indicator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    Offline,
    Syncing,
    Connected,
}

/// A discovered (unauthenticated) LAN peer.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPeer {
    pub instance: String,
    pub hint: String,
    pub endpoint: String,
}

/// Snapshot of the sync engine, returned by the `sync_status` command.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub state: SyncState,
    pub generation: u64,
    pub session_count: usize,
    pub peers: Vec<SyncPeer>,
}

/// The process-wide background sync engine. Exactly one instance per process;
/// share it via `Arc` (Tauri managed state).
pub struct SyncService {
    scheduler: Mutex<Scheduler>,
    registry: Mutex<CandidateRegistry>,
    orchestrator: Mutex<SyncOrchestrator>,
    backend: Arc<dyn DiscoveryBackend>,
    status: Mutex<SyncStatus>,
    sessions: AtomicUsize,
    stopped: AtomicBool,
}

impl SyncService {
    pub fn new(intent: SchedulerIntent, backend: Arc<dyn DiscoveryBackend>) -> Arc<Self> {
        Arc::new(Self {
            scheduler: Mutex::new(Scheduler::new(intent)),
            registry: Mutex::new(CandidateRegistry::new()),
            orchestrator: Mutex::new(SyncOrchestrator::default()),
            backend,
            status: Mutex::new(SyncStatus {
                state: SyncState::Offline,
                generation: 0,
                session_count: 0,
                peers: Vec::new(),
            }),
            sessions: AtomicUsize::new(0),
            stopped: AtomicBool::new(false),
        })
    }

    pub fn set_intent(&self, intent: SchedulerIntent) {
        self.scheduler.lock().unwrap().set_intent(intent);
    }

    pub fn status(&self) -> SyncStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    /// Spawn the background loop: an immediate startup pass, then periodic
    /// passes until [`Self::stop`] is called.
    pub fn start(self: &Arc<Self>) {
        let this = Arc::clone(self);
        std::thread::spawn(move || {
            this.on_trigger(TriggerSource::Startup);
            while !this.is_stopped() {
                std::thread::sleep(PERIODIC_INTERVAL);
                if this.is_stopped() {
                    break;
                }
                this.on_trigger(TriggerSource::Periodic);
            }
        });
    }

    /// Handle a sync trigger: run a discovery pass, then drive the orchestrator
    /// for each peer the scheduler allows.
    pub fn on_trigger(&self, source: TriggerSource) {
        if self.is_stopped() {
            return;
        }
        self.discover();
        let candidates = self.registry.lock().unwrap().candidates();
        for candidate in &candidates {
            let peer = hint_to_device_id(&candidate.hint);
            if self.evaluate(source, &peer) == TriggerDecision::Trigger {
                self.sync_peer(candidate, peer);
            }
        }
        let state = self.compute_state();
        let peers = peer_snapshot(&candidates);
        self.update_status(state, peers);
    }

    /// Advance the network generation, drop stale candidates, and rediscover.
    pub fn on_network_change(&self) {
        if self.is_stopped() {
            return;
        }
        self.registry.lock().unwrap().new_generation();
        self.on_trigger(TriggerSource::NetworkChange);
    }

    /// Stop the background loop and tear down discovery.
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.backend.stop();
        self.update_status(SyncState::Offline, Vec::new());
    }

    fn discover(&self) {
        let raw = self.backend.browse();
        let now = now_millis();
        let mut registry = self.registry.lock().unwrap();
        let generation = registry.generation().value();
        for candidate in to_candidates(raw, generation) {
            registry.upsert(candidate);
        }
        registry.prune_expired(now);
    }

    fn evaluate(&self, source: TriggerSource, peer: &DeviceId) -> TriggerDecision {
        match self.scheduler.lock().unwrap().decision(peer) {
            SyncDecision::Sync => TriggerDecision::Trigger,
            SyncDecision::Pause => TriggerDecision::Defer,
            SyncDecision::Block | SyncDecision::Stop => TriggerDecision::Ignore,
            SyncDecision::Defer => match source {
                TriggerSource::NetworkChange | TriggerSource::Manual => TriggerDecision::Trigger,
                TriggerSource::Startup | TriggerSource::Periodic => TriggerDecision::Defer,
            },
        }
    }

    fn sync_peer(&self, candidate: &Candidate, peer: DeviceId) {
        let mut orch = self.orchestrator.lock().unwrap();
        orch.begin(peer); // → Discovering
        if let Some(endpoint) = candidate.endpoints.first() {
            orch.on_candidate(&endpoint.ip, endpoint.port); // → Dialing
        }
        // Transport boundary: the Noise handshake + operation transfer are not
        // yet implemented (Plan 2/3 transport). Simulate a converged cycle so
        // the state machine reaches `Complete`; a real transport replaces this.
        orch.on_handshake(); // → Handshaking (SendHello)
        orch.on_negotiated(); // → Negotiating (SendSummary)
        orch.on_summary(VersionSummary::default()); // → Complete (no missing ranges)
        self.sessions.fetch_add(1, Ordering::Relaxed);
    }

    fn compute_state(&self) -> SyncState {
        if self.is_stopped() || self.registry.lock().unwrap().is_empty() {
            return SyncState::Offline;
        }
        match self.orchestrator.lock().unwrap().phase() {
            SyncPhase::Idle | SyncPhase::Complete | SyncPhase::Backoff | SyncPhase::Failed => {
                SyncState::Connected
            }
            _ => SyncState::Syncing,
        }
    }

    fn update_status(&self, state: SyncState, peers: Vec<SyncPeer>) {
        let mut status = self.status.lock().unwrap();
        status.state = state;
        status.generation = self.registry.lock().unwrap().generation().value();
        status.session_count = self.sessions.load(Ordering::Relaxed);
        status.peers = peers;
    }
}

/// Synthesize a [`DeviceId`] from a discovery hint. Until Noise/membership
/// authentication yields the real 32-byte id, the 16-byte hint (zero-padded)
/// serves as the peer identity for scheduler decisions.
fn hint_to_device_id(hint: &[u8; 16]) -> DeviceId {
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(hint);
    DeviceId::from_bytes(bytes)
}

fn peer_snapshot(candidates: &[Candidate]) -> Vec<SyncPeer> {
    candidates
        .iter()
        .map(|c| SyncPeer {
            instance: c.opaque_service_instance.clone(),
            hint: hex(&c.hint),
            endpoint: c
                .endpoints
                .first()
                .map(|e| format!("{}:{}", e.ip, e.port))
                .unwrap_or_default(),
        })
        .collect()
}
