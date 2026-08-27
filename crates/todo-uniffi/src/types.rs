//! FFI-safe record and enum types (no internal locks, paths, or secret bytes).

/// A command expressed with FFI-safe primitives. IDs are UUID hex strings.
#[derive(Debug, Clone, uniffi::Enum)]
pub enum FfiCommand {
    CreateTask {
        title: String,
        description: String,
        due_date: Option<String>,
        due_time: Option<String>,
        priority: String,
        list_id: Option<String>,
        tags: Vec<String>,
    },
    SetTaskField {
        task: String,
        field: String,
        value: String,
    },
    SetTaskCompleted {
        task: String,
        completed: bool,
    },
    DeleteTask {
        task: String,
    },
    RestoreTask {
        task: String,
    },
    CreateList {
        name: String,
    },
    DeleteList {
        list: String,
    },
    CreateTag {
        name: String,
    },
    SetTaskTag {
        task: String,
        tag: String,
        attached: bool,
    },
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiTaskQuery {
    pub list: String,
    pub active_only: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiPage {
    pub cursor: Option<String>,
    pub limit: u32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiTaskSummary {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub due_date: Option<String>,
    pub priority: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiPagedTasks {
    pub items: Vec<FfiTaskSummary>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiSearchHit {
    pub task_id: String,
    pub title: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct FfiMutationReceipt {
    pub operation_ids: Vec<String>,
    pub affected_entities: Vec<String>,
    pub projection_revision: u64,
}
