//! Ports for LAN sync and pairing, injected by the host platform.

/// Outbound trigger to the LAN sync transport.
pub trait SyncPort: Send + Sync {
    fn trigger_sync(&self);
}

/// Pairing flow port (implemented by the platform UI).
pub trait PairingPort: Send + Sync {
    fn on_pairing_request(&self);
}
