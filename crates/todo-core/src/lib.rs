#![forbid(unsafe_code)]

//! The only business-data write entry point and public core service API.

pub mod api;
pub mod apply;
pub mod backup;
pub mod checkpoint;
pub mod dispatch;
pub mod error;
pub mod event;
pub mod membership;
pub mod open;
pub mod pairing;
pub mod query;
pub mod recovery;
pub mod runtime;
pub mod subscription;
pub mod sync;

pub use api::CoreHandle;
pub use apply::{ApplyBatchReceipt, SignedOperation};
pub use checkpoint::TransferCheckpoint;
pub use dispatch::{Core, MutationReceipt, OperationSigner, SignatureBytes, SignatureVerifier};
pub use error::CoreError;
pub use event::{Event, EventKind, SubscriptionKind};
pub use membership::MembershipStore;
pub use open::{PairingPort, SyncPort};
pub use pairing::PairingManager;
pub use query::{
    CodePointRange, ConflictRecord, DayBucket, ListScope, Page, PagedTasks, SearchHit, TaskDetails,
    TaskQuery, TaskScope, TaskSummary, TrashEntry,
};
pub use recovery::{RecoveryReason, ReplicaState, UnavailableReason};
pub use runtime::{PeerStatus, RuntimeStatus};
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
