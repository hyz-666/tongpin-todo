#![forbid(unsafe_code)]

//! Shared test support for the todo workspace.
//!
//! Depends on the production crates for tests only; it ships no product behavior.

pub mod fault_network;
pub mod security_fixtures;
pub mod session_driver;

pub use fault_network::{Fault, FaultNetwork};
pub use security_fixtures::{
    signed_operation, substitute_author, tamper_payload, verify_valid, verify_with_wrong_key,
};
pub use session_driver::{Replica, all_converged, converged};
