#![deny(unsafe_code)]

//! Windows desktop adapter: DNS-SD discovery, listener, and sync runtime.

use std::sync::Arc;

pub mod background_sync;
pub mod commands;
pub mod platform;
pub mod security;
pub mod state;
pub mod sync_runtime;

pub use background_sync::{SyncService, SyncState, SyncStatus};
pub use platform::discovery::{
    DiscoveryBackend, MdnsBackend, NullBackend, RawService, to_candidates,
};
pub use platform::listener::{ListenerHandle, preferred_dialer};
pub use platform::network_monitor::NetworkMonitor;
pub use sync_runtime::SyncRuntime;
pub use todo_core::{SchedulerIntent, TriggerSource};

pub const API_VERSION: u32 = 1;

/// Tauri application entry point invoked by the `tongpin-windows` binary.
///
/// Registers the command layer and the process-wide `AppState` (which holds the
/// lazily-opened `CoreHandle`), and starts the background sync engine.
pub fn run() {
    // Background sync: prefer real mDNS; fall back to a no-op if the daemon
    // cannot start (the app remains fully functional as a local-first client).
    let backend: Arc<dyn DiscoveryBackend> = match MdnsBackend::new() {
        Ok(b) => Arc::new(b),
        Err(e) => {
            eprintln!("mDNS backend unavailable: {e}; sync disabled");
            Arc::new(NullBackend)
        }
    };
    let sync = SyncService::new(todo_core::SchedulerIntent::ForegroundActive, backend);
    sync.start();
    let sync_for_exit = Arc::clone(&sync);

    tauri::Builder::default()
        .manage(state::AppState::default())
        .manage(sync)
        .invoke_handler(tauri::generate_handler![
            commands::open_core,
            commands::dispatch,
            commands::list_tasks,
            commands::search,
            commands::task_details,
            commands::runtime_status,
            commands::sync_status,
            commands::trigger_sync,
            commands::notify_network_change,
            commands::close_core,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(move |_app, event| {
            if let tauri::RunEvent::Exit = event {
                sync_for_exit.stop();
            }
        });
}
