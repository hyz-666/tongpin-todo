//! Discovery adapter: candidate conversion and backend abstraction.

use std::sync::Mutex;

use tongpin_windows_lib::{DiscoveryBackend, RawService, to_candidates};

/// A fake DNS-SD backend for tests.
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

    fn register(
        &self,
        _instance: &str,
        _port: u16,
        _hint: [u8; 16],
        _capabilities: u32,
    ) -> Result<(), String> {
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
        ttl_millis: 1_000_000,
    }
}

#[test]
fn candidates_are_tagged_with_generation() {
    let backend = FakeBackend::new(vec![service("a", 1)]);
    let raw = backend.browse();
    let candidates = to_candidates(raw, 7);
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].network_generation, 7);
    assert_eq!(candidates[0].hint, [1; 16]);
}

#[test]
fn stop_halts_browsing() {
    let backend = FakeBackend::new(vec![service("a", 1)]);
    assert_eq!(backend.browse().len(), 1);
    backend.stop();
    assert!(backend.browse().is_empty());
}

#[test]
fn duplicate_callbacks_yield_duplicate_candidates() {
    // The adapter passes raw services through; deduplication is the registry's
    // job, so two callbacks with the same instance produce two rows that the
    // candidate registry collapses by service instance.
    let backend = FakeBackend::new(vec![service("dup", 1), service("dup", 1)]);
    let candidates = to_candidates(backend.browse(), 0);
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].opaque_service_instance, "dup");
}

#[test]
fn invalid_endpoints_are_filtered() {
    let raw = vec![RawService {
        instance: "bad".into(),
        hint: [2; 16],
        endpoints: vec![("0.0.0.0".into(), 1)],
        capabilities: 0,
        ttl_millis: 100,
    }];
    assert!(to_candidates(raw, 0).is_empty());
}
