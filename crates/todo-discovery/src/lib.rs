#![forbid(unsafe_code)]

//! Rotating discovery hints and untrusted candidate lifecycle.

pub mod candidate;
pub mod error;
pub mod hint;
pub mod network;
pub mod selection;

pub use candidate::{Candidate, CandidateRegistry, Endpoint, is_usable_endpoint};
pub use error::DiscoveryError;
pub use hint::{DISCOVERY_DOMAIN, Hint, derive_hint, expected_hints, hint_eq};
pub use network::NetworkGeneration;
pub use selection::{preferred_dialer, select_session_owner};

pub const API_VERSION: u32 = 1;
