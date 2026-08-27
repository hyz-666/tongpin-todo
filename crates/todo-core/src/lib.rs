#![forbid(unsafe_code)]

//! The only business-data write entry point and public core service API.

pub mod apply;
pub mod dispatch;
pub mod error;

pub use apply::{ApplyBatchReceipt, SignedOperation};
pub use dispatch::{Core, MutationReceipt, OperationSigner, SignatureBytes, SignatureVerifier};
pub use error::CoreError;

pub const API_VERSION: u32 = 1;

/// Version triple reported at every platform boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildInfo {
    pub core_api: u32,
    pub schema: u32,
    pub protocol_major: u32,
    pub protocol_minor: u32,
}

impl BuildInfo {
    pub const fn current() -> Self {
        Self {
            core_api: API_VERSION,
            schema: 1,
            protocol_major: 1,
            protocol_minor: 0,
        }
    }
}
