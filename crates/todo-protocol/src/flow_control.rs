//! Flow control: bounded in-flight chunks and ciphertext budget.

use crate::error::ProtocolError;
use crate::limits::{DEFAULT_CIPHERTEXT_BUDGET, DEFAULT_IN_FLIGHT};

/// Track in-flight chunks and bytes to cap outstanding data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlowControl {
    pub max_in_flight: u32,
    pub max_bytes: usize,
    in_flight: u32,
    in_flight_bytes: usize,
}

impl FlowControl {
    pub fn new(max_in_flight: u32, max_bytes: usize) -> Self {
        Self {
            max_in_flight,
            max_bytes,
            in_flight: 0,
            in_flight_bytes: 0,
        }
    }

    pub fn in_flight(&self) -> u32 {
        self.in_flight
    }

    pub fn in_flight_bytes(&self) -> usize {
        self.in_flight_bytes
    }

    /// Whether another chunk of `bytes` can be sent now.
    pub fn can_send(&self, bytes: usize) -> bool {
        self.in_flight < self.max_in_flight && self.in_flight_bytes + bytes <= self.max_bytes
    }

    pub fn on_send(&mut self, bytes: usize) -> Result<(), ProtocolError> {
        if !self.can_send(bytes) {
            return Err(ProtocolError::FrameTooLarge);
        }
        self.in_flight += 1;
        self.in_flight_bytes += bytes;
        Ok(())
    }

    pub fn on_ack(&mut self, bytes: usize) {
        self.in_flight = self.in_flight.saturating_sub(1);
        self.in_flight_bytes = self.in_flight_bytes.saturating_sub(bytes);
    }
}

impl Default for FlowControl {
    fn default() -> Self {
        Self::new(DEFAULT_IN_FLIGHT, DEFAULT_CIPHERTEXT_BUDGET)
    }
}
