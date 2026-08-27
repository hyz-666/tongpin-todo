#![forbid(unsafe_code)]

//! Canonicalization, identities, and signatures.

pub mod canonical;
pub mod error;
pub mod identity;
pub mod secret;
pub mod signature;

pub use canonical::{PROTOCOL_MAJOR, decode_operation, encode_operation};
pub use error::{CryptoError, VerifyReason};
pub use identity::{DeviceIdentity, IdentityBinding, device_id_from_public};
pub use signature::{DOMAIN_SEPARATOR, sign_operation, verify_operation};

pub const API_VERSION: u32 = 1;
