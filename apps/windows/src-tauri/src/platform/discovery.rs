//! DNS-SD discovery adapter behind an injectable backend trait.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use todo_discovery::{Candidate, Endpoint, is_usable_endpoint};

/// The DNS-SD service type both peers browse and register.
pub const SERVICE_TYPE: &str = "_tptodo._tcp.local.";

/// How long a discovered service stays valid without a refresh (ms).
const CANDIDATE_TTL_MS: i64 = 120_000;

/// Current time in milliseconds since the Unix epoch.
pub(crate) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Lowercase hex encoding of a byte slice.
pub(crate) fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn encode_hint(hint: &[u8; 16]) -> String {
    hex(hint)
}

fn decode_hint(s: &str) -> Option<[u8; 16]> {
    if s.len() != 32 {
        return None;
    }
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

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
pub trait DiscoveryBackend: Send + Sync {
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

/// A production [`DiscoveryBackend`] built on `mdns-sd`. It browses
/// [`SERVICE_TYPE`] and maintains a pull-able snapshot of currently resolved
/// services; [`DiscoveryBackend::browse`] drains the daemon's event stream and
/// returns the snapshot. Peers advertise a 16-byte discovery `hint` (hex) and a
/// `cap` (capabilities bitmask) as TXT properties.
pub struct MdnsBackend {
    daemon: ServiceDaemon,
    receiver: Mutex<mdns_sd::Receiver<ServiceEvent>>,
    snapshot: Mutex<HashMap<String, RawService>>,
    registered: Mutex<Option<String>>,
}

impl MdnsBackend {
    pub fn new() -> Result<Self, String> {
        let daemon = ServiceDaemon::new().map_err(|e| e.to_string())?;
        let receiver = daemon.browse(SERVICE_TYPE).map_err(|e| e.to_string())?;
        Ok(Self {
            daemon,
            receiver: Mutex::new(receiver),
            snapshot: Mutex::new(HashMap::new()),
            registered: Mutex::new(None),
        })
    }
}

impl DiscoveryBackend for MdnsBackend {
    fn browse(&self) -> Vec<RawService> {
        let receiver = self.receiver.lock().unwrap();
        while let Ok(event) = receiver.try_recv() {
            match event {
                ServiceEvent::ServiceResolved(resolved) => {
                    let hint = resolved
                        .get_property_val_str("hint")
                        .and_then(decode_hint)
                        .unwrap_or([0u8; 16]);
                    let capabilities = resolved
                        .get_property_val_str("cap")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0u32);
                    let endpoints: Vec<(String, u16)> = resolved
                        .get_addresses_v4()
                        .into_iter()
                        .map(|ip| (ip.to_string(), resolved.get_port()))
                        .collect();
                    let raw = RawService {
                        instance: resolved.get_fullname().to_string(),
                        hint,
                        endpoints,
                        capabilities,
                        ttl_millis: now_millis() + CANDIDATE_TTL_MS,
                    };
                    self.snapshot
                        .lock()
                        .unwrap()
                        .insert(raw.instance.clone(), raw);
                }
                ServiceEvent::ServiceRemoved(_ty, fullname) => {
                    self.snapshot.lock().unwrap().remove(&fullname);
                }
                _ => {}
            }
        }
        self.snapshot.lock().unwrap().values().cloned().collect()
    }

    fn register(
        &self,
        instance: &str,
        port: u16,
        hint: [u8; 16],
        capabilities: u32,
    ) -> Result<(), String> {
        let hint_hex = encode_hint(&hint);
        let cap_str = capabilities.to_string();
        let props: [(&str, &str); 2] = [("hint", &hint_hex), ("cap", &cap_str)];
        // Empty host/ip with `enable_addr_auto()` lets the daemon detect our
        // own addresses automatically.
        let info = ServiceInfo::new(SERVICE_TYPE, instance, "", "", port, props.as_slice())
            .map_err(|e| e.to_string())?
            .enable_addr_auto();
        let fullname = info.get_fullname().to_string();
        self.daemon.register(info).map_err(|e| e.to_string())?;
        *self.registered.lock().unwrap() = Some(fullname);
        Ok(())
    }

    fn stop(&self) {
        if let Some(fullname) = self.registered.lock().unwrap().take() {
            let _ = self.daemon.unregister(&fullname);
        }
        let _ = self.daemon.shutdown();
    }
}

/// A no-op backend used as a fallback when the mDNS daemon cannot start, or in
/// environments where LAN discovery is disabled.
pub struct NullBackend;

impl DiscoveryBackend for NullBackend {
    fn browse(&self) -> Vec<RawService> {
        Vec::new()
    }

    fn register(&self, _i: &str, _p: u16, _h: [u8; 16], _c: u32) -> Result<(), String> {
        Ok(())
    }

    fn stop(&self) {}
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
