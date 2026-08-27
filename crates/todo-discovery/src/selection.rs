//! Deterministic dialer and session selection.

use todo_domain::ids::DeviceId;

/// The lexicographically lower device id is the preferred dialer.
pub fn preferred_dialer(a: &DeviceId, b: &DeviceId) -> bool {
    a.as_bytes() < b.as_bytes()
}

/// When both peers dial simultaneously, the retained session is keyed by the
/// lexicographically lower device id (both sides compute the same id).
pub fn select_session_owner(a: &DeviceId, b: &DeviceId) -> DeviceId {
    if a.as_bytes() < b.as_bytes() { *a } else { *b }
}
