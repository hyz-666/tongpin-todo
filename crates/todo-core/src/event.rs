//! Subscription event types.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubscriptionKind {
    Projection,
    Runtime,
    Pairing,
}

#[derive(Clone, Debug)]
pub struct Event {
    pub revision: u64,
    pub kind: EventKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventKind {
    ProjectionChanged,
    RuntimeChanged,
    Pairing,
}
