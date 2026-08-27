//! Bounded, revisioned, idempotently-cancellable subscriptions.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};

use crate::event::{Event, EventKind, SubscriptionKind};

pub type SubscriptionId = u64;

const CHANNEL_BOUND: usize = 16;

/// A live subscription. Dropping it does not cancel; call `cancel`.
pub struct Subscription {
    id: SubscriptionId,
    rx: Receiver<Event>,
    registry: Arc<SubscriptionRegistry>,
}

impl Subscription {
    pub fn recv(&self) -> Result<Event, std::sync::mpsc::RecvError> {
        self.rx.recv()
    }

    pub fn try_recv(&self) -> Result<Event, TryRecvError> {
        self.rx.try_recv()
    }

    /// Cancel this subscription. Idempotent.
    pub fn cancel(&self) {
        self.registry.unsubscribe(self.id);
    }
}

pub(crate) struct SubscriptionRegistry {
    next_id: AtomicU64,
    senders: Mutex<HashMap<SubscriptionId, (SubscriptionKind, SyncSender<Event>)>>,
    revision: AtomicU64,
}

impl SubscriptionRegistry {
    pub(crate) fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            senders: Mutex::new(HashMap::new()),
            revision: AtomicU64::new(0),
        }
    }

    pub(crate) fn subscribe(self: &Arc<Self>, kind: SubscriptionKind) -> Subscription {
        let (tx, rx) = std::sync::mpsc::sync_channel(CHANNEL_BOUND);
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.senders.lock().unwrap().insert(id, (kind, tx));
        Subscription {
            id,
            rx,
            registry: self.clone(),
        }
    }

    fn unsubscribe(&self, id: SubscriptionId) {
        self.senders.lock().unwrap().remove(&id);
    }

    pub(crate) fn notify(&self, kind: SubscriptionKind, event_kind: EventKind) {
        let revision = self.revision.fetch_add(1, Ordering::SeqCst) + 1;
        let event = Event {
            revision,
            kind: event_kind,
        };
        let senders = self.senders.lock().unwrap();
        for (k, tx) in senders.values() {
            if *k == kind {
                // Bounded backpressure: a slow consumer's overflow is dropped,
                // but the monotonic revision lets it detect the gap.
                let _ = tx.try_send(event.clone());
            }
        }
    }
}
