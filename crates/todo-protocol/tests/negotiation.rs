//! Version and capability negotiation.

use todo_protocol::{HelloInfo, ProtocolError, REQUIRED_FEATURE_FLAG, ResourceLimits, negotiate};

fn limits(frame: usize, chunk: usize, in_flight: u32) -> ResourceLimits {
    ResourceLimits {
        frame_limit: frame,
        chunk_limit: chunk,
        in_flight,
    }
}

#[test]
fn current_with_current_negotiates() {
    let a = HelloInfo {
        protocol_major: 1,
        protocol_minor: 0,
        schema: 1,
        capabilities: 0,
    };
    let b = HelloInfo {
        protocol_major: 1,
        protocol_minor: 0,
        schema: 1,
        capabilities: 0,
    };
    let out = negotiate(&a, &b, &limits(64, 512, 32), &limits(64, 512, 32)).unwrap();
    assert_eq!(out.protocol_minor, 0);
}

#[test]
fn compatible_minor_uses_lower() {
    let a = HelloInfo {
        protocol_major: 1,
        protocol_minor: 2,
        schema: 1,
        capabilities: 0,
    };
    let b = HelloInfo {
        protocol_major: 1,
        protocol_minor: 5,
        schema: 1,
        capabilities: 0,
    };
    let out = negotiate(&a, &b, &limits(64, 512, 32), &limits(64, 512, 32)).unwrap();
    assert_eq!(out.protocol_minor, 2);
}

#[test]
fn capability_intersection() {
    let a = HelloInfo {
        protocol_major: 1,
        protocol_minor: 0,
        schema: 1,
        capabilities: 0b0011,
    };
    let b = HelloInfo {
        protocol_major: 1,
        protocol_minor: 0,
        schema: 1,
        capabilities: 0b0110,
    };
    let out = negotiate(&a, &b, &limits(64, 512, 32), &limits(64, 512, 32)).unwrap();
    assert_eq!(out.capabilities, 0b0010);
}

#[test]
fn different_major_incompatible() {
    let a = HelloInfo {
        protocol_major: 1,
        protocol_minor: 0,
        schema: 1,
        capabilities: 0,
    };
    let b = HelloInfo {
        protocol_major: 2,
        protocol_minor: 0,
        schema: 1,
        capabilities: 0,
    };
    assert!(matches!(
        negotiate(&a, &b, &limits(64, 512, 32), &limits(64, 512, 32)),
        Err(ProtocolError::ProtocolIncompatible)
    ));
}

#[test]
fn schema_non_overlap_incompatible() {
    let a = HelloInfo {
        protocol_major: 1,
        protocol_minor: 0,
        schema: 1,
        capabilities: 0,
    };
    let b = HelloInfo {
        protocol_major: 1,
        protocol_minor: 0,
        schema: 2,
        capabilities: 0,
    };
    assert!(matches!(
        negotiate(&a, &b, &limits(64, 512, 32), &limits(64, 512, 32)),
        Err(ProtocolError::ProtocolIncompatible)
    ));
}

#[test]
fn unknown_required_feature_incompatible() {
    let a = HelloInfo {
        protocol_major: 1,
        protocol_minor: 0,
        schema: 1,
        capabilities: 0,
    };
    let b = HelloInfo {
        protocol_major: 1,
        protocol_minor: 0,
        schema: 1,
        capabilities: REQUIRED_FEATURE_FLAG,
    };
    assert!(matches!(
        negotiate(&a, &b, &limits(64, 512, 32), &limits(64, 512, 32)),
        Err(ProtocolError::UnknownRequiredFeature)
    ));
}

#[test]
fn limits_take_lower_bound() {
    let a = HelloInfo {
        protocol_major: 1,
        protocol_minor: 0,
        schema: 1,
        capabilities: 0,
    };
    let b = HelloInfo {
        protocol_major: 1,
        protocol_minor: 0,
        schema: 1,
        capabilities: 0,
    };
    let out = negotiate(&a, &b, &limits(64, 512, 32), &limits(32, 256, 16)).unwrap();
    assert_eq!(out.limits.frame_limit, 32);
    assert_eq!(out.limits.chunk_limit, 256);
    assert_eq!(out.limits.in_flight, 16);
}
