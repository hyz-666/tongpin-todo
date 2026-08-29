//! TCP listener: binds only local interfaces selected by platform policy.

use std::net::TcpListener;

/// A bound TCP listener handle with its listening port.
pub struct ListenerHandle {
    listener: TcpListener,
    pub port: u16,
}

impl ListenerHandle {
    /// Bind an ephemeral port on the loopback interface for local testing.
    pub fn bind_loopback() -> Result<Self, String> {
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        Ok(Self { listener, port })
    }

    /// The listener is ready to accept connections; the actual accept loop is
    /// owned by the sync runtime.
    pub fn listener(&self) -> &TcpListener {
        &self.listener
    }
}

/// Preferred-dialer rule: the lexicographically lower device id dials.
pub fn preferred_dialer(a: &[u8; 32], b: &[u8; 32]) -> bool {
    a < b
}
