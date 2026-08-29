#![forbid(unsafe_code)]

//! Windows desktop adapter: DNS-SD discovery, listener, and sync runtime.

pub mod platform;
pub mod sync_runtime;

pub use platform::discovery::{DiscoveryBackend, RawService, to_candidates};
pub use platform::listener::{ListenerHandle, preferred_dialer};
pub use platform::network_monitor::NetworkMonitor;
pub use sync_runtime::SyncRuntime;

pub const API_VERSION: u32 = 1;
