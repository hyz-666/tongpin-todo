//! Sync orchestration: missing-range computation and checkpoint reconciliation.

use std::collections::HashMap;

use todo_domain::ids::DeviceId;
use todo_protocol::{SeqRange, VersionSummary};

use crate::checkpoint::TransferCheckpoint;

/// Track per-peer transfer state and compute what to send next.
#[derive(Default)]
pub struct SyncState {
    checkpoints: HashMap<DeviceId, TransferCheckpoint>,
}

impl SyncState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Ranges we must send to a peer: what it lacks, minus what it already
    /// durably acknowledged in a prior transfer.
    pub fn ranges_to_send(
        &self,
        peer: DeviceId,
        remote_summary: &VersionSummary,
        local_summary: &VersionSummary,
    ) -> Vec<SeqRange> {
        let missing = remote_summary.missing_from(local_summary);
        let mut ranges = missing.get(&peer).cloned().unwrap_or_default();
        // Subtract anything a durable checkpoint already covered.
        if let Some(cp) = self.checkpoints.get(&peer)
            && cp.highest_ack > 0
        {
            ranges = ranges
                .into_iter()
                .filter_map(|r| {
                    let start = r.start.max(cp.highest_ack);
                    if start < r.end {
                        Some(SeqRange::new(start, r.end))
                    } else {
                        None
                    }
                })
                .collect();
        }
        ranges
    }

    pub fn checkpoint(&self, peer: &DeviceId) -> Option<&TransferCheckpoint> {
        self.checkpoints.get(peer)
    }

    pub fn record_checkpoint(&mut self, cp: TransferCheckpoint) {
        self.checkpoints.insert(cp.peer, cp);
    }

    /// Advance the durable ack for a peer (monotonic).
    pub fn advance_ack(&mut self, peer: &DeviceId, acknowledged: u64) {
        if let Some(cp) = self.checkpoints.get_mut(peer) {
            cp.advance_ack(acknowledged);
        }
    }
}
