#![forbid(unsafe_code)]

//! Deterministic domain rules: values, commands, operations, and merge semantics.

pub mod clock;
pub mod command;
pub mod error;
pub mod ids;
pub mod lifecycle;
pub mod model;
pub mod operation;
pub mod rank;
pub mod register;
pub mod validation;

pub const API_VERSION: u32 = 1;
