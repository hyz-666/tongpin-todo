//! DNS-SD discovery adapter behind an injectable backend trait.

use todo_discovery::{Candidate, Endpoint, is_usable_endpoint};

/// A raw DNS-SD service instance reported by the platform backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawService {
    pub instance: String,
    pub hint: [u8; 16],
    pub endpoints: Vec<(String, u16)>,
    pub capabilities: u32,
    pub ttl_millis: i64,
}

/// Injectable DNS-SD backend. Tests use a fake; production uses mdns-sd.
pub trait DiscoveryBackend {
    /// Browse `_tptodo._tcp.local` and return discovered service instances.
    fn browse(&self) -> Vec<RawService>;
    /// Register our own service instance with the rotating hint only.
    fn register(
        &self,
        instance: &str,
        port: u16,
        hint: [u8; 16],
        capabilities: u32,
    ) -> Result<(), String>;
    /// Stop registration/browsing.
    fn stop(&self);
}

/// Convert raw DNS-SD services into untrusted candidates tagged with the
/// current network generation. Endpoints that are loopback/multicast/invalid
/// are dropped; services with no usable endpoint are discarded entirely.
pub fn to_candidates(services: Vec<RawService>, generation: u64) -> Vec<Candidate> {
    services
        .into_iter()
        .map(|s| {
            let endpoints: Vec<Endpoint> = s
                .endpoints
                .into_iter()
                .filter(|(ip, _)| is_usable_endpoint(ip))
                .map(|(ip, port)| Endpoint { ip, port })
                .collect();
            Candidate {
                opaque_service_instance: s.instance,
                hint: s.hint,
                endpoints,
                capabilities: s.capabilities,
                ttl_millis: s.ttl_millis,
                network_generation: generation,
            }
        })
        .filter(|c| !c.endpoints.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_candidates_drops_invalid_endpoints() {
        let raw = vec![RawService {
            instance: "srv-A".into(),
            hint: [0xAA; 16],
            endpoints: vec![("192.168.1.10".into(), 5353), ("127.0.0.1".into(), 5353)],
            capabilities: 0,
            ttl_millis: 1_000_000,
        }];
        let candidates = to_candidates(raw, 3);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].endpoints.len(), 1);
        assert_eq!(candidates[0].network_generation, 3);
    }

    #[test]
    fn to_candidates_discards_all_invalid() {
        let raw = vec![RawService {
            instance: "bad".into(),
            hint: [0xBB; 16],
            endpoints: vec![("0.0.0.0".into(), 5353)],
            capabilities: 0,
            ttl_millis: 1_000,
        }];
        assert!(to_candidates(raw, 0).is_empty());
    }
}
