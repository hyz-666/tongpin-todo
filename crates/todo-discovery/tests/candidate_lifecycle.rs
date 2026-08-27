//! Candidate lifecycle: dedup, TTL, endpoint validation, generation invalidation.

use todo_discovery::{Candidate, CandidateRegistry, Endpoint, is_usable_endpoint};

fn candidate(instance: &str, generation: u64, ttl: i64) -> Candidate {
    Candidate {
        opaque_service_instance: instance.to_string(),
        hint: [0xAA; 16],
        endpoints: vec![Endpoint {
            ip: "192.168.1.10".into(),
            port: 5353,
        }],
        capabilities: 0,
        ttl_millis: ttl,
        network_generation: generation,
    }
}

#[test]
fn duplicate_callbacks_are_idempotent() {
    let mut reg = CandidateRegistry::new();
    reg.upsert(candidate("a", 0, 1_000_000));
    reg.upsert(candidate("a", 0, 1_000_000));
    assert_eq!(reg.len(), 1);
}

#[test]
fn ttl_expiry_prunes_candidates() {
    let mut reg = CandidateRegistry::new();
    reg.upsert(candidate("a", 0, 100));
    reg.upsert(candidate("b", 0, 10_000));
    reg.prune_expired(1_000);
    assert_eq!(reg.len(), 1);
    assert!(reg.find_by_hint(&[0xAA; 16]).is_some());
}

#[test]
fn new_generation_discards_stale_candidates() {
    let mut reg = CandidateRegistry::new();
    reg.upsert(candidate("a", 0, 1_000_000));
    reg.new_generation();
    assert!(reg.is_empty());
    // A stale-generation candidate is ignored.
    reg.upsert(candidate("b", 0, 1_000_000));
    assert!(reg.is_empty());
    // A current-generation candidate is accepted.
    reg.upsert(candidate("c", 1, 1_000_000));
    assert_eq!(reg.len(), 1);
}

#[test]
fn endpoint_deduplication() {
    let mut reg = CandidateRegistry::new();
    let mut c = candidate("a", 0, 1_000_000);
    c.endpoints = vec![
        Endpoint {
            ip: "192.168.1.1".into(),
            port: 1,
        },
        Endpoint {
            ip: "192.168.1.1".into(),
            port: 1,
        },
        Endpoint {
            ip: "192.168.1.2".into(),
            port: 2,
        },
    ];
    reg.upsert(c);
    let stored = reg.candidates();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].endpoints.len(), 2);
}

#[test]
fn endpoint_validation_rejects_loopback_and_multicast() {
    assert!(!is_usable_endpoint("127.0.0.1"));
    assert!(!is_usable_endpoint("0.0.0.0"));
    assert!(!is_usable_endpoint("::1"));
    assert!(!is_usable_endpoint("224.0.0.1"));
    assert!(!is_usable_endpoint("ff02::1"));
    assert!(is_usable_endpoint("192.168.1.10"));
    assert!(is_usable_endpoint("10.0.0.5"));
}
