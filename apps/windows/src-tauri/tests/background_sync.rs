//! Background sync engine: discovery loop, trigger routing, and status.

use std::sync::{Arc, Mutex};

use tongpin_windows_lib::{
    DiscoveryBackend, RawService, SchedulerIntent, SyncService, SyncState, TriggerSource,
};

/// A fake DNS-SD backend returning a fixed set of services.
struct FakeBackend {
    services: Mutex<Vec<RawService>>,
    stopped: Mutex<bool>,
}

impl FakeBackend {
    fn new(services: Vec<RawService>) -> Self {
        Self {
            services: Mutex::new(services),
            stopped: Mutex::new(false),
        }
    }
}

impl DiscoveryBackend for FakeBackend {
    fn browse(&self) -> Vec<RawService> {
        if *self.stopped.lock().unwrap() {
            return Vec::new();
        }
        self.services.lock().unwrap().clone()
    }

    fn register(&self, _i: &str, _p: u16, _h: [u8; 16], _c: u32) -> Result<(), String> {
        Ok(())
    }

    fn stop(&self) {
        *self.stopped.lock().unwrap() = true;
    }
}

fn service(instance: &str, hint: u8) -> RawService {
    RawService {
        instance: instance.into(),
        hint: [hint; 16],
        endpoints: vec![("192.168.1.10".into(), 5353)],
        capabilities: 0,
        ttl_millis: i64::MAX,
    }
}

#[test]
fn no_peers_is_offline() {
    let backend = Arc::new(FakeBackend::new(vec![]));
    let sync = SyncService::new(SchedulerIntent::ForegroundActive, backend);
    sync.on_trigger(TriggerSource::Startup);
    assert_eq!(sync.status().state, SyncState::Offline);
    assert!(sync.status().peers.is_empty());
}

#[test]
fn discovered_peer_is_connected() {
    let backend = Arc::new(FakeBackend::new(vec![service("srv-a", 1)]));
    let sync = SyncService::new(SchedulerIntent::ForegroundActive, backend);
    sync.on_trigger(TriggerSource::Startup);
    let status = sync.status();
    assert_eq!(status.state, SyncState::Connected);
    assert_eq!(status.peers.len(), 1);
    assert_eq!(status.peers[0].instance, "srv-a");
    assert_eq!(status.peers[0].endpoint, "192.168.1.10:5353");
    assert_eq!(status.session_count, 1, "one cycle converged per peer");
}

#[test]
fn network_change_bumps_generation_and_clears_candidates() {
    let backend = Arc::new(FakeBackend::new(vec![service("srv-a", 1)]));
    let sync = SyncService::new(SchedulerIntent::ForegroundActive, backend);
    sync.on_trigger(TriggerSource::Startup);
    assert_eq!(sync.status().generation, 0);

    sync.on_network_change();
    // Generation advanced and the registry was cleared; the next browse (still
    // returning the same service) repopulates it under the new generation.
    assert_eq!(sync.status().generation, 1);
}

#[test]
fn os_deferred_defers_background_sources() {
    let backend = Arc::new(FakeBackend::new(vec![service("srv-a", 1)]));
    let sync = SyncService::new(SchedulerIntent::OsDeferred, backend);
    // A periodic pass is deferred: the peer is discovered but no cycle runs.
    sync.on_trigger(TriggerSource::Periodic);
    assert_eq!(sync.status().session_count, 0);
    // A network change still overrides the defer.
    sync.on_trigger(TriggerSource::NetworkChange);
    assert_eq!(sync.status().session_count, 1);
}

#[test]
fn stop_halts_sync() {
    let backend = Arc::new(FakeBackend::new(vec![service("srv-a", 1)]));
    let sync = SyncService::new(SchedulerIntent::ForegroundActive, backend);
    sync.on_trigger(TriggerSource::Startup);
    assert_eq!(sync.status().state, SyncState::Connected);

    sync.stop();
    assert!(sync.is_stopped());
    assert_eq!(sync.status().state, SyncState::Offline);
}
