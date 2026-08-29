//! Sync orchestration: the end-to-end sync loop as a deterministic state machine.
//!
//! [`SyncOrchestrator`] owns the *decision* of what to do next during a sync
//! cycle against one peer. It never touches the network: each event method
//! returns a [`SyncAction`] the platform transport must perform, and advances
//! the internal phase state machine. This is the layer that strings together
//! every Plan 2 primitive — discovery, dialing, Noise handshake, negotiation,
//! version summaries, range requests, chunk transfer, application, and durable
//! acknowledgements — into one end-to-end loop.

use todo_domain::ids::DeviceId;
use todo_protocol::{SeqRange, VersionSummary};

/// A concrete step the transport must perform to advance the loop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncAction {
    /// Begin LAN discovery for candidates.
    Discover,
    /// Dial a candidate endpoint.
    Dial { ip: String, port: u16 },
    /// Send a hello (version + capabilities) to the peer.
    SendHello,
    /// Send our version summary.
    SendSummary,
    /// Request operation ranges the peer has that we lack.
    SendRangeRequest { ranges: Vec<SeqRange> },
    /// Send the next operation chunk.
    SendChunk { sequence: u64 },
    /// Apply a received chunk to local storage.
    ApplyChunk { operations: usize },
    /// Send a durable acknowledgement for a sequence.
    SendAck { sequence: u64 },
    /// Wait for the peer's next inbound message.
    Await,
    /// This sync cycle is complete.
    Complete,
}

/// Fine-grained phases of one sync cycle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SyncPhase {
    #[default]
    Idle,
    Discovering,
    Dialing,
    Handshaking,
    Negotiating,
    ExchangingSummaries,
    RequestingRanges,
    Transferring,
    Applying,
    AwaitingAck,
    Complete,
    Backoff,
    Failed,
}

/// The result of feeding an event to the orchestrator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncStep {
    pub phase: SyncPhase,
    pub action: SyncAction,
}

/// Drives one peer through a full sync cycle.
#[derive(Debug)]
pub struct SyncOrchestrator {
    phase: SyncPhase,
    peer: Option<DeviceId>,
    local_summary: VersionSummary,
    remote_summary: Option<VersionSummary>,
    /// Ranges still pending transfer from the peer, in order.
    pending_ranges: Vec<SeqRange>,
    /// Highest sequence the peer has durably acknowledged.
    highest_ack: u64,
}

impl SyncOrchestrator {
    pub fn new(local_summary: VersionSummary) -> Self {
        Self {
            phase: SyncPhase::Idle,
            peer: None,
            local_summary,
            remote_summary: None,
            pending_ranges: Vec::new(),
            highest_ack: 0,
        }
    }

    pub fn phase(&self) -> SyncPhase {
        self.phase
    }

    pub fn peer(&self) -> Option<DeviceId> {
        self.peer
    }

    /// Highest sequence the peer has durably acknowledged.
    pub fn highest_ack(&self) -> u64 {
        self.highest_ack
    }

    /// Ranges still pending transfer from the peer.
    pub fn pending_ranges(&self) -> &[SeqRange] {
        &self.pending_ranges
    }

    /// Start a sync cycle against a peer by discovering how to reach it.
    pub fn begin(&mut self, peer: DeviceId) -> SyncStep {
        self.peer = Some(peer);
        self.phase = SyncPhase::Discovering;
        SyncStep {
            phase: self.phase,
            action: SyncAction::Discover,
        }
    }

    /// A candidate endpoint was discovered; dial it.
    pub fn on_candidate(&mut self, ip: &str, port: u16) -> SyncStep {
        self.phase = SyncPhase::Dialing;
        SyncStep {
            phase: self.phase,
            action: SyncAction::Dial {
                ip: ip.to_string(),
                port,
            },
        }
    }

    /// The Noise handshake completed; send a hello to negotiate.
    pub fn on_handshake(&mut self) -> SyncStep {
        self.phase = SyncPhase::Handshaking;
        SyncStep {
            phase: self.phase,
            action: SyncAction::SendHello,
        }
    }

    /// Negotiation completed successfully; exchange summaries.
    pub fn on_negotiated(&mut self) -> SyncStep {
        self.phase = SyncPhase::Negotiating;
        SyncStep {
            phase: self.phase,
            action: SyncAction::SendSummary,
        }
    }

    /// The peer's version summary arrived. Compute what we still lack and
    /// either request it or finish the cycle.
    pub fn on_summary(&mut self, summary: VersionSummary) -> SyncStep {
        self.remote_summary = Some(summary.clone());
        self.phase = SyncPhase::ExchangingSummaries;
        self.pending_ranges = self.missing_ranges(&summary);
        if self.pending_ranges.is_empty() {
            self.phase = SyncPhase::Complete;
            return SyncStep {
                phase: self.phase,
                action: SyncAction::Complete,
            };
        }
        SyncStep {
            phase: self.phase,
            action: SyncAction::SendRangeRequest {
                ranges: self.pending_ranges.clone(),
            },
        }
    }

    /// A chunk covering `[sequence, sequence + operations)` arrived and was
    /// applied. If more ranges remain, keep transferring; otherwise await the
    /// peer's final acknowledgement.
    pub fn on_chunk(&mut self, sequence: u64, operations: usize) -> SyncStep {
        self.phase = SyncPhase::Applying;
        let end = sequence + operations as u64;
        // Trim the pending ranges by removing anything `[sequence, end)` covers.
        let mut remaining: Vec<SeqRange> = Vec::new();
        for r in self.pending_ranges.drain(..) {
            if r.end <= end {
                // Fully covered by this chunk.
                continue;
            }
            if r.start < end {
                // Partially covered; keep the uncovered tail.
                remaining.push(SeqRange::new(end, r.end));
            } else {
                // Not yet reached.
                remaining.push(r);
            }
        }
        self.pending_ranges = remaining;
        if self.pending_ranges.is_empty() {
            self.phase = SyncPhase::AwaitingAck;
            SyncStep {
                phase: self.phase,
                action: SyncAction::Await,
            }
        } else {
            SyncStep {
                phase: self.phase,
                action: SyncAction::ApplyChunk { operations },
            }
        }
    }

    /// The peer durably acknowledged a sequence; remember it monotonically.
    pub fn on_ack(&mut self, sequence: u64) -> SyncStep {
        self.highest_ack = self.highest_ack.max(sequence);
        self.phase = SyncPhase::Complete;
        SyncStep {
            phase: self.phase,
            action: SyncAction::Complete,
        }
    }

    /// A recoverable error occurred; enter backoff.
    pub fn on_error(&mut self) -> SyncStep {
        self.phase = SyncPhase::Backoff;
        SyncStep {
            phase: self.phase,
            action: SyncAction::Await,
        }
    }

    /// Mark an unrecoverable failure.
    pub fn fail(&mut self) -> SyncStep {
        self.phase = SyncPhase::Failed;
        SyncStep {
            phase: self.phase,
            action: SyncAction::Await,
        }
    }

    /// Reset the cycle back to idle.
    pub fn reset(&mut self) {
        self.phase = SyncPhase::Idle;
        self.peer = None;
        self.remote_summary = None;
        self.pending_ranges.clear();
    }

    /// Ranges `self` is missing relative to `remote`, ignoring anything the
    /// peer has already durably acknowledged.
    fn missing_ranges(&self, remote: &VersionSummary) -> Vec<SeqRange> {
        let missing = self.local_summary.missing_from(remote);
        let mut out: Vec<SeqRange> = Vec::new();
        for ranges in missing.values() {
            for r in ranges {
                let start = r.start.max(self.highest_ack);
                if start < r.end {
                    out.push(SeqRange::new(start, r.end));
                }
            }
        }
        out.sort();
        out
    }
}

impl Default for SyncOrchestrator {
    fn default() -> Self {
        Self::new(VersionSummary::default())
    }
}
