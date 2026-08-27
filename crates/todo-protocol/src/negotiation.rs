//! Capability and version negotiation.

use crate::error::ProtocolError;

/// The highest capability bit marks a required (non-optional) feature.
pub const REQUIRED_FEATURE_FLAG: u64 = 1 << 63;

/// Advertised protocol/version/capability tuple.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HelloInfo {
    pub protocol_major: u16,
    pub protocol_minor: u16,
    pub schema: u16,
    pub capabilities: u64,
}

/// Resource limits that are negotiated to the lower bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceLimits {
    pub frame_limit: usize,
    pub chunk_limit: usize,
    pub in_flight: u32,
}

/// The negotiated outcome: intersected capabilities and lower-bound limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NegotiationOutcome {
    pub protocol_minor: u16,
    pub capabilities: u64,
    pub limits: ResourceLimits,
}

/// Negotiate a session between a local and remote hello.
pub fn negotiate(
    local: &HelloInfo,
    remote: &HelloInfo,
    local_limits: &ResourceLimits,
    remote_limits: &ResourceLimits,
) -> Result<NegotiationOutcome, ProtocolError> {
    if local.protocol_major != remote.protocol_major {
        return Err(ProtocolError::ProtocolIncompatible);
    }
    if local.schema != remote.schema {
        return Err(ProtocolError::ProtocolIncompatible);
    }
    // A required feature the remote demands but the local lacks is fatal.
    let remote_required = remote.capabilities & REQUIRED_FEATURE_FLAG;
    if remote_required & !local.capabilities != 0 {
        return Err(ProtocolError::UnknownRequiredFeature);
    }
    let local_required = local.capabilities & REQUIRED_FEATURE_FLAG;
    if local_required & !remote.capabilities != 0 {
        return Err(ProtocolError::UnknownRequiredFeature);
    }
    Ok(NegotiationOutcome {
        protocol_minor: local.protocol_minor.min(remote.protocol_minor),
        capabilities: local.capabilities & remote.capabilities,
        limits: ResourceLimits {
            frame_limit: local_limits.frame_limit.min(remote_limits.frame_limit),
            chunk_limit: local_limits.chunk_limit.min(remote_limits.chunk_limit),
            in_flight: local_limits.in_flight.min(remote_limits.in_flight),
        },
    })
}
