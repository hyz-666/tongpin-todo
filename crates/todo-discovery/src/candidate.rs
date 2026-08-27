//! Untrusted discovery candidates and their lifecycle.

use std::collections::HashMap;

use crate::hint::Hint;
use crate::network::NetworkGeneration;

/// An IP endpoint (v4 or v6).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Endpoint {
    pub ip: String,
    pub port: u16,
}

/// An untrusted candidate reported by the platform adapter. Never trusted
/// until Noise/membership authentication succeeds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub opaque_service_instance: String,
    pub hint: Hint,
    pub endpoints: Vec<Endpoint>,
    pub capabilities: u32,
    /// Absolute expiry in milliseconds since the Unix epoch.
    pub ttl_millis: i64,
    pub network_generation: u64,
}

/// Reject endpoints that are loopback, multicast, or clearly invalid.
pub fn is_usable_endpoint(ip: &str) -> bool {
    if ip == "0.0.0.0" || ip == "::" || ip == "::1" {
        return false;
    }
    if ip.starts_with("127.") || ip.starts_with("224.") || ip.starts_with("239.") {
        return false;
    }
    if ip.to_lowercase().starts_with("ff") && ip.contains(':') {
        return false; // IPv6 multicast
    }
    true
}

/// A candidate registry keyed by service instance, with TTL expiry and
/// generation-safe invalidation.
pub struct CandidateRegistry {
    candidates: HashMap<String, Candidate>,
    generation: NetworkGeneration,
}

impl CandidateRegistry {
    pub fn new() -> Self {
        Self {
            candidates: HashMap::new(),
            generation: NetworkGeneration::new(),
        }
    }

    /// Advance to a new network generation, discarding every stale candidate.
    pub fn new_generation(&mut self) {
        self.generation = self.generation.next();
        self.candidates.clear();
    }

    pub fn generation(&self) -> NetworkGeneration {
        self.generation
    }

    /// Insert or merge a candidate (duplicate callbacks are idempotent).
    pub fn upsert(&mut self, mut candidate: Candidate) {
        if candidate.network_generation != self.generation.value() {
            return; // stale generation
        }
        // Deduplicate endpoints preserving order.
        let mut seen = std::collections::HashSet::new();
        candidate.endpoints.retain(|e| seen.insert(e.clone()));
        self.candidates
            .insert(candidate.opaque_service_instance.clone(), candidate);
    }

    /// Remove expired candidates and return the surviving ones.
    pub fn prune_expired(&mut self, now_millis: i64) {
        self.candidates.retain(|_, c| c.ttl_millis > now_millis);
    }

    pub fn candidates(&self) -> Vec<Candidate> {
        self.candidates.values().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.candidates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    /// Resolve a hint to a candidate (constant-time hint matching).
    pub fn find_by_hint(&self, hint: &Hint) -> Option<&Candidate> {
        self.candidates
            .values()
            .find(|c| crate::hint::hint_eq(&c.hint, hint))
    }
}

impl Default for CandidateRegistry {
    fn default() -> Self {
        Self::new()
    }
}
