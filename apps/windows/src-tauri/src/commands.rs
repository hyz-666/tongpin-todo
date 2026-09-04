//! Tauri command layer: exposes `todo-core` to the React frontend.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;

use todo_core::{
    CoreError, CoreHandle, ListScope, MutationReceipt, OperationSigner, Page, SignatureBytes,
    SignatureVerifier, TaskQuery, TaskScope,
};
use todo_domain::clock::{LocalDate, LocalTime};
use todo_domain::command::{
    Command, CreateList, CreateTag, CreateTask, EntityRef, RestoreTask, SetTaskCompleted,
    SetTaskField, SetTaskTag, TaskField,
};
use todo_domain::ids::{DeviceId, EntityId};
use todo_domain::model::Priority;
use todo_storage::config::{SecretBytes, StorageConfig};

use crate::security;
use crate::state::AppState;

// --- signer / verifier (Noop until Task 6 wires real ed25519) ---

struct NoopSigner;
impl OperationSigner for NoopSigner {
    fn sign(&self, _c: &[u8]) -> Result<SignatureBytes, CoreError> {
        Ok(SignatureBytes(vec![0u8; 64]))
    }
}

struct NoopVerifier;
impl SignatureVerifier for NoopVerifier {
    fn verify(&self, _s: &DeviceId, _c: &[u8], _sig: &[u8]) -> Result<(), CoreError> {
        Ok(())
    }
}

// --- DTO types (serde, the FFI boundary for invoke()) ---

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CommandDto {
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskQueryDto {
    pub list: String,
    pub active_only: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDto {
    pub cursor: Option<String>,
    pub limit: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummaryDto {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub due_date: Option<String>,
    pub priority: String,
    pub list_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedTasksDto {
    pub items: Vec<TaskSummaryDto>,
    pub next_cursor: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHitDto {
    pub task_id: String,
    pub title: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationReceiptDto {
    pub operation_ids: Vec<String>,
    pub affected_entities: Vec<String>,
    pub projection_revision: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerStatusDto {
    pub device_id: String,
    pub reachable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatusDto {
    pub replica: String,
    pub peers: Vec<PeerStatusDto>,
}

// --- conversions ---

fn parse_id(s: &str) -> Result<EntityId, String> {
    let u = uuid::Uuid::parse_str(s).map_err(|_| format!("bad id: {s}"))?;
    Ok(EntityId::from_uuid(u))
}

fn parse_date(s: &str) -> Option<LocalDate> {
    let parts: Vec<i64> = s
        .split('-')
        .map(|p| p.parse().ok())
        .collect::<Option<_>>()?;
    if parts.len() != 3 {
        return None;
    }
    LocalDate::new(parts[0] as i32, parts[1] as u8, parts[2] as u8).ok()
}

fn parse_time(s: &str) -> Option<LocalTime> {
    let parts: Vec<i64> = s
        .split(':')
        .map(|p| p.parse().ok())
        .collect::<Option<_>>()?;
    if parts.len() != 2 {
        return None;
    }
    LocalTime::new(parts[0] as u8, parts[1] as u8).ok()
}

fn parse_priority(s: &str) -> Priority {
    match s {
        "low" => Priority::Low,
        "medium" => Priority::Medium,
        "high" => Priority::High,
        _ => Priority::None,
    }
}

fn to_command(cmd: CommandDto) -> Result<Command, String> {
    Ok(match cmd {
        CommandDto::CreateTask {
            title,
            description,
            due_date,
            due_time,
            priority,
            list_id,
            tags,
        } => Command::CreateTask(CreateTask {
            title,
            description,
            due_date: due_date.as_deref().and_then(parse_date),
            due_time: due_time.as_deref().and_then(parse_time),
            priority: parse_priority(&priority),
            list_id: list_id.as_deref().map(parse_id).transpose()?,
            tags: tags
                .iter()
                .map(|t| parse_id(t))
                .collect::<Result<Vec<_>, _>>()?,
        }),
        CommandDto::SetTaskField { task, field, value } => {
            let id = parse_id(&task)?;
            let tf = match field.as_str() {
                "title" => TaskField::Title(value),
                "description" => TaskField::Description(value),
                "priority" => TaskField::Priority(parse_priority(&value)),
                "due_date" => TaskField::DueDate(parse_date(&value)),
                "due_time" => TaskField::DueTime(parse_time(&value)),
                _ => return Err(format!("unknown field: {field}")),
            };
            Command::SetTaskField(SetTaskField {
                task: EntityRef { id },
                field: tf,
            })
        }
        CommandDto::SetTaskCompleted { task, completed } => {
            Command::SetTaskCompleted(SetTaskCompleted {
                task: EntityRef {
                    id: parse_id(&task)?,
                },
                completed,
            })
        }
        CommandDto::DeleteTask { task } => Command::DeleteTask(EntityRef {
            id: parse_id(&task)?,
        }),
        CommandDto::RestoreTask { task } => Command::RestoreTask(RestoreTask {
            task: EntityRef {
                id: parse_id(&task)?,
            },
        }),
        CommandDto::CreateList { name } => Command::CreateList(CreateList { name }),
        CommandDto::DeleteList { list } => Command::DeleteList(EntityRef {
            id: parse_id(&list)?,
        }),
        CommandDto::CreateTag { name } => Command::CreateTag(CreateTag { name }),
        CommandDto::SetTaskTag {
            task,
            tag,
            attached,
        } => Command::SetTaskTag(SetTaskTag {
            task: EntityRef {
                id: parse_id(&task)?,
            },
            tag: EntityRef {
                id: parse_id(&tag)?,
            },
            attached,
        }),
    })
}

fn to_scope(list: &str) -> ListScope {
    match list {
        "inbox" => ListScope::Inbox,
        "today" => ListScope::Today,
        "tomorrow" => ListScope::Tomorrow,
        "next7" => ListScope::Next7Days,
        "completed" => ListScope::Completed,
        "all" => ListScope::All,
        other => uuid::Uuid::parse_str(other)
            .map(EntityId::from_uuid)
            .map(ListScope::List)
            .unwrap_or(ListScope::All),
    }
}

fn task_summary_to_dto(t: todo_core::TaskSummary) -> TaskSummaryDto {
    TaskSummaryDto {
        id: t.id.to_string(),
        title: t.title,
        completed: t.completed,
        due_date: t.due_date.map(|d| format!("{d:?}")),
        priority: format!("{:?}", t.priority).to_lowercase(),
        list_id: t.list_id.map(|l| l.to_string()),
    }
}

fn receipt_to_dto(r: MutationReceipt) -> MutationReceiptDto {
    MutationReceiptDto {
        operation_ids: r.operation_ids.iter().map(|o| format!("{o:?}")).collect(),
        affected_entities: r.affected_entities.iter().map(|e| e.to_string()).collect(),
        projection_revision: r.projection_revision,
    }
}

fn replica_to_str(r: &todo_core::ReplicaState) -> &'static str {
    match r {
        todo_core::ReplicaState::Ready => "ready",
        todo_core::ReplicaState::ReadOnlyLowSpace => "read_only_low_space",
        todo_core::ReplicaState::Recovering(_) => "recovering",
        todo_core::ReplicaState::Unavailable(_) => "unavailable",
    }
}

// --- key management (DPAPI-protected, persisted next to the profile) ---

fn load_or_create_secret32(profile_dir: &Path, filename: &str) -> Result<[u8; 32], String> {
    let path = profile_dir.join(filename);
    if path.exists() {
        let encrypted = std::fs::read(&path).map_err(|e| e.to_string())?;
        let decrypted = security::unprotect(&encrypted)?;
        decrypted
            .as_slice()
            .try_into()
            .map_err(|_| format!("{filename}: expected 32 bytes"))
    } else {
        let secret: [u8; 32] = rand::random();
        let encrypted = security::protect(&secret)?;
        std::fs::write(&path, &encrypted).map_err(|e| e.to_string())?;
        Ok(secret)
    }
}

fn with_handle<T>(
    state: &State<'_, AppState>,
    f: impl FnOnce(&CoreHandle) -> Result<T, String>,
) -> Result<T, String> {
    let guard = state
        .handle
        .lock()
        .map_err(|_| "state lock poisoned".to_string())?;
    let handle = guard.as_ref().ok_or("core not open")?;
    f(handle)
}

// --- commands ---

#[tauri::command]
pub fn open_core(state: State<'_, AppState>, profile_path: String) -> Result<(), String> {
    let mut guard = state
        .handle
        .lock()
        .map_err(|_| "state lock poisoned".to_string())?;
    if guard.is_some() {
        return Ok(());
    }

    let profile_dir = PathBuf::from(&profile_path);
    std::fs::create_dir_all(&profile_dir).map_err(|e| e.to_string())?;

    let device_id_bytes = load_or_create_secret32(&profile_dir, "device_id.dpapi")?;
    let db_key = load_or_create_secret32(&profile_dir, "db_key.dpapi")?;

    let cfg = StorageConfig {
        profile_path: profile_dir,
        database_key: SecretBytes::from_bytes(db_key.to_vec()),
        busy_timeout: std::time::Duration::from_secs(5),
    };

    let handle = CoreHandle::open(
        cfg,
        DeviceId::from_bytes(device_id_bytes),
        Box::new(NoopSigner),
        Box::new(NoopVerifier),
    )
    .map_err(|e| e.to_string())?;

    *guard = Some(handle);
    Ok(())
}

#[tauri::command]
pub fn dispatch(
    state: State<'_, AppState>,
    command: CommandDto,
) -> Result<MutationReceiptDto, String> {
    with_handle(&state, |handle| {
        let cmd = to_command(command)?;
        let receipt = handle.dispatch(cmd).map_err(|e| e.to_string())?;
        Ok(receipt_to_dto(receipt))
    })
}

#[tauri::command]
pub fn list_tasks(
    state: State<'_, AppState>,
    query: TaskQueryDto,
    page: PageDto,
    today: String,
) -> Result<PagedTasksDto, String> {
    with_handle(&state, |handle| {
        let q = TaskQuery {
            list: to_scope(&query.list),
            scope: if query.active_only {
                TaskScope::Active
            } else {
                TaskScope::All
            },
        };
        let p = Page {
            cursor: page.cursor,
            limit: page.limit,
        };
        let result = handle
            .list_tasks(&q, &p, &today)
            .map_err(|e| e.to_string())?;
        Ok(PagedTasksDto {
            items: result.items.into_iter().map(task_summary_to_dto).collect(),
            next_cursor: result.next_cursor,
        })
    })
}

#[tauri::command]
pub fn search(
    state: State<'_, AppState>,
    text: String,
    limit: u32,
) -> Result<Vec<SearchHitDto>, String> {
    with_handle(&state, |handle| {
        let hits = handle
            .search_tasks(&text, limit)
            .map_err(|e| e.to_string())?;
        Ok(hits
            .into_iter()
            .map(|h| SearchHitDto {
                task_id: h.task_id.to_string(),
                title: h.title,
            })
            .collect())
    })
}

#[tauri::command]
pub fn runtime_status(state: State<'_, AppState>) -> Result<RuntimeStatusDto, String> {
    with_handle(&state, |handle| {
        let status = handle.runtime_status();
        Ok(RuntimeStatusDto {
            replica: replica_to_str(&status.replica).to_string(),
            peers: status
                .peers
                .into_iter()
                .map(|p| PeerStatusDto {
                    device_id: format!("{:?}", p.device_id),
                    reachable: p.reachable,
                })
                .collect(),
        })
    })
}

#[tauri::command]
pub fn close_core(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state
        .handle
        .lock()
        .map_err(|_| "state lock poisoned".to_string())?;
    if let Some(handle) = guard.take() {
        handle.close();
    }
    Ok(())
}
