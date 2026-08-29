//! Deterministic fault injection for frames in transit.

/// The kinds of faults a hostile or lossy LAN can inflict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// The frame never arrives.
    Drop,
    /// The frame arrives twice.
    Duplicate,
    /// The frame arrives truncated.
    Truncate,
    /// A byte in the frame is flipped.
    Corrupt,
    /// Adjacent frames arrive out of order.
    Reorder,
    /// The frame is delivered unmodified.
    Pass,
}

/// A deterministic, seed-driven network that transforms frames in flight.
pub struct FaultNetwork {
    seed: u64,
    frames: Vec<Vec<u8>>,
    pending: Vec<Vec<u8>>,
}

impl FaultNetwork {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            frames: Vec::new(),
            pending: Vec::new(),
        }
    }

    /// Queue a frame for delivery.
    pub fn send(&mut self, frame: Vec<u8>) {
        self.frames.push(frame);
    }

    fn next_random(&mut self) -> u64 {
        // xorshift64* for reproducible sequences.
        let mut x = self.seed;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.seed = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Drain queued frames into the delivery pipeline, applying a fault
    /// pattern chosen deterministically from the seed.
    pub fn apply(&mut self, fault: Fault) {
        let frames = std::mem::take(&mut self.frames);
        for (index, frame) in frames.into_iter().enumerate() {
            let effective = if index.is_multiple_of(2) { fault } else { Fault::Pass };
            match effective {
                Fault::Drop => {}
                Fault::Duplicate => {
                    self.pending.push(frame.clone());
                    self.pending.push(frame);
                }
                Fault::Truncate => {
                    let keep = frame.len() / 2;
                    self.pending.push(frame[..keep].to_vec());
                }
                Fault::Corrupt => {
                    let mut f = frame.clone();
                    if !f.is_empty() {
                        let pos = (self.next_random() as usize) % f.len();
                        f[pos] ^= 0xFF;
                    }
                    self.pending.push(f);
                }
                Fault::Reorder => {
                    self.pending.push(frame);
                    if self.pending.len() >= 2 {
                        let n = self.pending.len();
                        self.pending.swap(n - 1, n - 2);
                    }
                }
                Fault::Pass => self.pending.push(frame),
            }
        }
    }

    /// Deliver the next transformed frame, if any.
    pub fn deliver(&mut self) -> Option<Vec<u8>> {
        if self.pending.is_empty() {
            return None;
        }
        Some(self.pending.remove(0))
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}
