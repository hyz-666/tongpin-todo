//! The public `CoreHandle` — the single entry point the platform uses.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::Value;

use todo_domain::clock::YearMonth;
use todo_domain::command::Command;
use todo_domain::ids::{DeviceId, EntityId};
use todo_storage::config::StorageConfig;

use crate::backup;
use crate::dispatch::{Core, MutationReceipt, OperationSigner, SignatureVerifier};
use crate::error::CoreError;
use crate::event::{EventKind, SubscriptionKind};
use crate::query::{Page, PagedTasks, SearchHit, TaskDetails, TaskQuery};
use crate::recovery::ReplicaState;
use crate::runtime::{RuntimeState, RuntimeStatus};
use crate::subscription::{Subscription, SubscriptionRegistry};

pub struct CoreHandle {
    core: Arc<Core>,
    registry: Arc<SubscriptionRegistry>,
    runtime: Arc<RuntimeState>,
    closed: AtomicBool,
}

impl CoreHandle {
    pub fn open(
        config: StorageConfig,
        device_id: DeviceId,
        signer: Box<dyn OperationSigner>,
        verifier: Box<dyn SignatureVerifier>,
    ) -> Result<Self, CoreError> {
        let core = Core::open(config, device_id, signer, verifier)?;
        Ok(Self {
            core: Arc::new(core),
            registry: Arc::new(SubscriptionRegistry::new()),
            runtime: Arc::new(RuntimeState::new()),
            closed: AtomicBool::new(false),
        })
    }

    fn check_open(&self) -> Result<(), CoreError> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(CoreError::Closed);
        }
        Ok(())
    }

    pub fn dispatch(&self, command: Command) -> Result<MutationReceipt, CoreError> {
        self.check_open()?;
        let receipt = self.core.dispatch(command)?;
        self.registry
            .notify(SubscriptionKind::Projection, EventKind::ProjectionChanged);
        Ok(receipt)
    }

    pub fn list_tasks(
        &self,
        query: &TaskQuery,
        page: &Page,
        today: &str,
    ) -> Result<PagedTasks, CoreError> {
        self.check_open()?;
        self.core.list_tasks(query, page, today)
    }

    pub fn task_details(&self, id: &EntityId) -> Result<TaskDetails, CoreError> {
        self.check_open()?;
        self.core.task_details(id)
    }

    pub fn search_tasks(&self, text: &str, limit: u32) -> Result<Vec<SearchHit>, CoreError> {
        self.check_open()?;
        self.core.search_tasks(text, limit)
    }

    pub fn calendar(&self, month: YearMonth) -> Result<Vec<crate::query::DayBucket>, CoreError> {
        self.check_open()?;
        self.core.calendar(month)
    }

    pub fn trash(&self) -> Result<Vec<crate::query::TrashEntry>, CoreError> {
        self.check_open()?;
        self.core.trash()
    }

    pub fn conflict_history(
        &self,
        page: &Page,
    ) -> Result<Vec<crate::query::ConflictRecord>, CoreError> {
        self.check_open()?;
        self.core.conflict_history(page)
    }

    pub fn field_value(&self, entity: &EntityId, field: &str) -> Result<Option<Value>, CoreError> {
        self.check_open()?;
        self.core.field_value(entity, field)
    }

    pub fn replica_state(&self) -> ReplicaState {
        self.core.replica_state()
    }

    pub fn runtime_status(&self) -> RuntimeStatus {
        self.runtime.status(self.core.replica_state())
    }

    pub fn update_peer_reachability(&self, device: DeviceId, reachable: bool) {
        self.runtime.update_peer(device, reachable);
        self.registry
            .notify(SubscriptionKind::Runtime, EventKind::RuntimeChanged);
    }

    pub fn subscribe(&self, kind: SubscriptionKind) -> Result<Subscription, CoreError> {
        self.check_open()?;
        Ok(self.registry.subscribe(kind))
    }

    pub fn backup(&self, passphrase: &str) -> Result<Vec<u8>, CoreError> {
        self.check_open()?;
        let repo = self.core.repo.lock().unwrap();
        backup::create_backup(&repo.conn, passphrase)
    }

    /// Close the handle. Idempotent.
    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }
}
