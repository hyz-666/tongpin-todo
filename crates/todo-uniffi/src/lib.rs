#![forbid(unsafe_code)]

//! FFI-safe boundary exposed to Windows and Android via UniFFI.

mod api;
mod error;
mod types;

pub use api::Core;
pub use error::{CoreErrorCode, FfiError};
pub use types::{
    FfiCommand, FfiMutationReceipt, FfiPage, FfiPagedTasks, FfiSearchHit, FfiTaskQuery,
    FfiTaskSummary,
};

pub const API_VERSION: u32 = 1;

uniffi::setup_scaffolding!();
