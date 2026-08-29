#![forbid(unsafe_code)]

//! Shared test support for the todo workspace.
//!
//! Depends on the production crates for tests only; it ships no product behavior.

pub mod fault_network;
pub mod security_fixtures;
pub mod session_driver;

pub use fault_network::{Fault, FaultNetwork};
pub use security_fixtures::{
    fixture_device, fixture_entity, rejects_without_mutation, signed_op_pair, signed_operation,
    signed_operation_bytes, substitute_author, tamper_payload, tampered_variants, verify_valid,
    verify_with_wrong_key,
};
pub use session_driver::{Replica, all_converged, converged};
