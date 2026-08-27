#![forbid(unsafe_code)]

//! Canonicalization, identities, signatures, pairing, and membership.

pub mod canonical;
pub mod error;
pub mod group;
pub mod identity;
pub mod membership;
pub mod pairing;
pub mod revocation;
pub mod sas;
pub mod secret;
pub mod signature;

pub use canonical::{PROTOCOL_MAJOR, decode_operation, encode_operation};
pub use error::{CryptoError, VerifyReason};
pub use group::GroupIdentity;
pub use identity::{DeviceIdentity, IdentityBinding, device_id_from_public};
pub use membership::{
    EventHash, MembershipEvent, MembershipGraph, MembershipKind,
    device_id_from_public as membership_device_id,
};
pub use pairing::{
    CandidateEndpoint, PAIRING_EXPIRY_SECS, PairingPayloadV1, PairingSession, PairingState,
    now_millis,
};
pub use revocation::{RekeyEnvelope, create_rekey_envelopes, is_post_cutoff, open_rekey_envelope};
pub use sas::derive_sas;
pub use signature::{DOMAIN_SEPARATOR, sign_operation, verify_operation};

pub const API_VERSION: u32 = 1;
