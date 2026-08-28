//! Durable transfer checkpoints.

use todo_domain::ids::DeviceId;
use todo_protocol::{SeqRange, VersionSummary};

/// A persisted transfer checkpoint: what was requested, and how far the peer
/// durably acknowledged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransferCheckpoint {
    pub peer: DeviceId,
    pub transfer_id: [u8; 16],
    pub requested_ranges: Vec<SeqRange>,
    /// Highest contiguous sequence the peer durably acknowledged.
    pub highest_ack: u64,
    pub starting_summary: VersionSummary,
}

impl TransferCheckpoint {
    pub fn new(
        peer: DeviceId,
        transfer_id: [u8; 16],
        requested_ranges: Vec<SeqRange>,
        starting_summary: VersionSummary,
    ) -> Self {
        Self {
            peer,
            transfer_id,
            requested_ranges,
            highest_ack: 0,
            starting_summary,
        }
    }

    /// Ranges still missing after `acknowledged` is persisted.
    pub fn remaining(&self, acknowledged: u64) -> Vec<SeqRange> {
        self.requested_ranges
            .iter()
            .filter_map(|r| {
                let start = r.start.max(acknowledged);
                if start < r.end {
                    Some(SeqRange::new(start, r.end))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Record a durable acknowledgement (monotonic).
    pub fn advance_ack(&mut self, acknowledged: u64) {
        self.highest_ack = self.highest_ack.max(acknowledged);
    }
}
