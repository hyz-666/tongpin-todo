//! Process-wide application state shared across Tauri commands.

use std::sync::Mutex;

use todo_core::CoreHandle;

/// Holds the lazily-opened [`CoreHandle`]. Exactly one per process; renderer
/// reloads reuse it via Tauri managed state.
pub struct AppState {
    pub handle: Mutex<Option<CoreHandle>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            handle: Mutex::new(None),
        }
    }
}
