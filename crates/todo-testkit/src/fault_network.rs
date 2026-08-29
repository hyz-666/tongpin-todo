//! Deterministic fault injection for the simulated LAN.
//!
//! A `FaultNetwork` sits between two peers and transforms frames in transit:
//! dropping, duplicating, reordering, truncating, corrupting, delaying, or
//! pausing them. All transformations derive from a fixed seed, so a failing
//! scenario can be replayed exactly.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// How frames from a given peer are mangled in transit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// Delivered unchanged.
    Pass,
    /// Silently discarded.
    Drop,
    /// Delivered twice.
    Duplicate,
    /// Delivered out of order.
    Reorder,
    /// Only the first half of the bytes are delivered.
    Truncate,
    /// A byte is flipped.
    Corrupt,
    /// Held until explicitly delivered.
    Delay,
}

impl Fault {
    pub const ALL: [Fault; 7] = [
        Fault::Pass,
        Fault::Drop,
        Fault::Duplicate,
        Fault::Reorder,
        Fault::Truncate,
        Fault::Corrupt,
        Fault::Delay,
    ];
}

/// A deterministic, seed-driven network between numbered peers.
pub struct FaultNetwork {
    seed: u64,
    queues: BTreeMap<u8, VecDeque<Vec<u8>>>,
    paused: BTreeSet<u8>,
    policy: BTreeMap<u8, Fault>,
}

impl FaultNetwork {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            queues: BTreeMap::new(),
            paused: BTreeSet::new(),
            policy: BTreeMap::new(),
        }
    }

    /// Set how frames from `from` are transformed.
    pub fn set_policy(&mut self, from: u8, fault: Fault) {
        self.policy.insert(from, fault);
    }

    /// Pause delivery from a peer (frames queue up but are not delivered).
    pub fn pause(&mut self, from: u8) {
        self.paused.insert(from);
    }

    /// Resume delivery from a peer.
    pub fn resume(&mut self, from: u8) {
        self.paused.remove(&from);
    }

    /// Queue one frame from `from`, applying the configured fault.
    pub fn send(&mut self, from: u8, bytes: Vec<u8>) {
        let fault = self.policy.get(&from).copied().unwrap_or(Fault::Pass);
        let mut batch = match fault {
            Fault::Pass | Fault::Delay => vec![bytes],
            Fault::Drop => {
                if self.next_bool() {
                    vec![bytes]
                } else {
                    vec![]
                }
            }
            Fault::Duplicate => {
                if self.next_bool() {
                    vec![bytes.clone(), bytes]
                } else {
                    vec![bytes]
                }
            }
            Fault::Reorder => vec![bytes],
            Fault::Truncate => {
                let half = bytes.len() / 2;
                vec![bytes[..half].to_vec()]
            }
            Fault::Corrupt => {
                let mut out = bytes;
                if let Some(b) = out.first_mut() {
                    *b ^= 0xFF;
                }
                vec![out]
            }
        };

        let queue = self.queues.entry(from).or_default();
        if fault == Fault::Reorder && queue.len() >= 2 {
            // Insert before the last frame instead of appending.
            let last = queue.pop_back().unwrap();
            queue.extend(batch.drain(..));
            queue.push_back(last);
        } else {
            queue.extend(batch);
        }
    }

    /// Take everything currently deliverable from `from`.
    pub fn deliver_to(&mut self, from: u8) -> Vec<Vec<u8>> {
        if self.paused.contains(&from) {
            return Vec::new();
        }
        let fault = self.policy.get(&from).copied().unwrap_or(Fault::Pass);
        if fault == Fault::Delay {
            return Vec::new();
        }
        let queue = self.queues.entry(from).or_default();
        queue.drain(..).collect()
    }

    /// Deliver queued frames even when delayed or paused (used to resume).
    pub fn force_deliver(&mut self, from: u8) -> Vec<Vec<u8>> {
        let queue = self.queues.entry(from).or_default();
        queue.drain(..).collect()
    }

    pub fn pending(&self, from: u8) -> usize {
        self.queues.get(&from).map(|q| q.len()).unwrap_or(0)
    }

    fn next_bool(&mut self) -> bool {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.seed = x;
        x.is_multiple_of(2)
    }
}
