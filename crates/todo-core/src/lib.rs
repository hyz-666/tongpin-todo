#![forbid(unsafe_code)]

//! The only business-data write entry point and public core service API.

pub mod api;
pub mod apply;
pub mod backoff;
pub mod backup;
pub mod checkpoint;
pub mod compaction;
pub mod dispatch;
pub mod error;
pub mod event;
pub mod member_commands;
pub mod membership;
pub mod network_runtime;
pub mod open;
pub mod orchestrator;
pub mod pairing;
pub mod pairing_flow;
pub mod peer_runtime;
pub mod query;
pub mod rebuild;
pub mod recovery;
pub mod runtime;
pub mod scheduler;
pub mod snapshot;
pub mod subscription;
pub mod sync;

pub use api::CoreHandle;
pub use apply::{ApplyBatchReceipt, SignedOperation};
pub use backoff::{AttemptCounter, chunk_retry_delay, dial_delay};
pub use checkpoint::TransferCheckpoint;
pub use compaction::{StableWatermark, compute_watermark, tombstone_collectable};
pub use dispatch::{Core, MutationReceipt, OperationSigner, SignatureBytes, SignatureVerifier};
pub use error::CoreError;
pub use event::{Event, EventKind, SubscriptionKind};
pub use member_commands::{MemberCommands, MemberInfo, MemberStatus};
pub use membership::MembershipStore;
pub use network_runtime::NetworkRuntime;
pub use open::{PairingPort, SyncPort};
pub use orchestrator::{SyncAction, SyncOrchestrator, SyncPhase, SyncStep};
pub use pairing::PairingManager;
pub use pairing_flow::{PairingAction, PairingFlow, PairingPhase};
pub use peer_runtime::PeerRuntime;
pub use query::{
    CodePointRange, ConflictRecord, DayBucket, ListScope, Page, PagedTasks, SearchHit, TaskDetails,
    TaskQuery, TaskScope, TaskSummary, TrashEntry,
};
pub use rebuild::{rebuild_projection, rebuild_with_cutoff};
pub use recovery::{RecoveryReason, ReplicaState, UnavailableReason};
pub use runtime::{PeerStatus, RuntimeStatus};
pub use scheduler::{Scheduler, SchedulerIntent, SyncDecision};
pub use snapshot::{SnapshotV1, export_snapshot, import_snapshot};
pub use subscription::Subscription;
pub use sync::SyncState;

pub const API_VERSION: u32 = 1;

/// Version triple reported at every platform boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildInfo {
    pub core_api: u32,
    pub schema: u32,
    pub protocol_major: u32,
    pub protocol_minor: u32,
}

impl BuildInfo {
    pub const fn current() -> Self {
        Self {
            core_api: API_VERSION,
            schema: 1,
            protocol_major: 1,
            protocol_minor: 0,
        }
    }
}
