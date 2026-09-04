#![deny(unsafe_code)]

//! Windows desktop adapter: DNS-SD discovery, listener, and sync runtime.

pub mod commands;
pub mod platform;
pub mod security;
pub mod state;
pub mod sync_runtime;

pub use platform::discovery::{DiscoveryBackend, RawService, to_candidates};
pub use platform::listener::{ListenerHandle, preferred_dialer};
pub use platform::network_monitor::NetworkMonitor;
pub use sync_runtime::SyncRuntime;

pub const API_VERSION: u32 = 1;

/// Tauri application entry point invoked by the `tongpin-windows` binary.
///
/// Registers the command layer and the process-wide `AppState` (which holds the
/// lazily-opened `CoreHandle`).
pub fn run() {
    tauri::Builder::default()
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::open_core,
            commands::dispatch,
            commands::list_tasks,
            commands::search,
            commands::runtime_status,
            commands::close_core,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
