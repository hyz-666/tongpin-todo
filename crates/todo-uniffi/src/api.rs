//! The FFI `Core` object and command/query conversion.

use std::sync::Arc;

use todo_core::{
    CoreError, CoreHandle, ListScope, MutationReceipt, OperationSigner, Page, SignatureBytes,
    SignatureVerifier, TaskQuery, TaskScope,
};
use todo_domain::command::{
    Command, CreateList, CreateTag, CreateTask, EntityRef, RestoreTask, SetTaskCompleted,
    SetTaskField, SetTaskTag, TaskField,
};
use todo_domain::ids::{DeviceId, EntityId};
use todo_domain::model::Priority;
use todo_storage::config::{SecretBytes, StorageConfig};

use crate::error::{CoreErrorCode, FfiError, map_error};
use crate::types::{
    FfiCommand, FfiMutationReceipt, FfiPage, FfiPagedTasks, FfiSearchHit, FfiTaskQuery,
    FfiTaskSummary,
};

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

fn parse_id(s: &str) -> Result<EntityId, FfiError> {
    let u = uuid::Uuid::parse_str(s).map_err(|_| FfiError::Core {
        code: CoreErrorCode::InvalidCommand,
        message: format!("bad id: {s}"),
    })?;
    Ok(EntityId::from_uuid(u))
}

fn parse_date(s: &str) -> Option<todo_domain::clock::LocalDate> {
    let parts: Vec<i64> = s
        .split('-')
        .map(|p| p.parse().ok())
        .collect::<Option<_>>()?;
    if parts.len() != 3 {
        return None;
    }
    todo_domain::clock::LocalDate::new(parts[0] as i32, parts[1] as u8, parts[2] as u8).ok()
}

fn parse_time(s: &str) -> Option<todo_domain::clock::LocalTime> {
    let parts: Vec<i64> = s
        .split(':')
        .map(|p| p.parse().ok())
        .collect::<Option<_>>()?;
    if parts.len() != 2 {
        return None;
    }
    todo_domain::clock::LocalTime::new(parts[0] as u8, parts[1] as u8).ok()
}

fn parse_priority(s: &str) -> Priority {
    match s {
        "low" => Priority::Low,
        "medium" => Priority::Medium,
        "high" => Priority::High,
        _ => Priority::None,
    }
}

fn to_command(cmd: FfiCommand) -> Result<Command, FfiError> {
    match cmd {
        FfiCommand::CreateTask {
            title,
            description,
            due_date,
            due_time,
            priority,
            list_id,
            tags,
        } => Ok(Command::CreateTask(CreateTask {
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
        })),
        FfiCommand::SetTaskField { task, field, value } => {
            let id = parse_id(&task)?;
            let tf = match field.as_str() {
                "title" => TaskField::Title(value),
                "description" => TaskField::Description(value),
                "priority" => TaskField::Priority(parse_priority(&value)),
                "due_date" => TaskField::DueDate(parse_date(&value)),
                "due_time" => TaskField::DueTime(parse_time(&value)),
                _ => {
                    return Err(FfiError::Core {
                        code: CoreErrorCode::InvalidCommand,
                        message: format!("unknown field: {field}"),
                    });
                }
            };
            Ok(Command::SetTaskField(SetTaskField {
                task: EntityRef { id },
                field: tf,
            }))
        }
        FfiCommand::SetTaskCompleted { task, completed } => {
            Ok(Command::SetTaskCompleted(SetTaskCompleted {
                task: EntityRef {
                    id: parse_id(&task)?,
                },
                completed,
            }))
        }
        FfiCommand::DeleteTask { task } => Ok(Command::DeleteTask(EntityRef {
            id: parse_id(&task)?,
        })),
        FfiCommand::RestoreTask { task } => Ok(Command::RestoreTask(RestoreTask {
            task: EntityRef {
                id: parse_id(&task)?,
            },
        })),
        FfiCommand::CreateList { name } => Ok(Command::CreateList(CreateList { name })),
        FfiCommand::DeleteList { list } => Ok(Command::DeleteList(EntityRef {
            id: parse_id(&list)?,
        })),
        FfiCommand::CreateTag { name } => Ok(Command::CreateTag(CreateTag { name })),
        FfiCommand::SetTaskTag {
            task,
            tag,
            attached,
        } => Ok(Command::SetTaskTag(SetTaskTag {
            task: EntityRef {
                id: parse_id(&task)?,
            },
            tag: EntityRef {
                id: parse_id(&tag)?,
            },
            attached,
        })),
    }
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

fn receipt(r: MutationReceipt) -> FfiMutationReceipt {
    FfiMutationReceipt {
        operation_ids: r.operation_ids.iter().map(|o| format!("{o:?}")).collect(),
        affected_entities: r.affected_entities.iter().map(|e| e.to_string()).collect(),
        projection_revision: r.projection_revision,
    }
}

#[derive(uniffi::Object)]
pub struct Core {
    handle: CoreHandle,
}

#[uniffi::export]
impl Core {
    #[uniffi::constructor]
    pub fn open(
        profile_path: String,
        db_key: Vec<u8>,
        device_id: Vec<u8>,
    ) -> Result<Arc<Self>, FfiError> {
        let key_bytes = SecretBytes::from_bytes(db_key);
        let cfg = StorageConfig {
            profile_path: std::path::PathBuf::from(profile_path),
            database_key: key_bytes,
            busy_timeout: std::time::Duration::from_secs(5),
        };
        let dev_arr: [u8; 32] = device_id.try_into().map_err(|_| FfiError::Core {
            code: CoreErrorCode::InvalidCommand,
            message: "device id must be 32 bytes".to_string(),
        })?;
        let handle = CoreHandle::open(
            cfg,
            DeviceId::from_bytes(dev_arr),
            Box::new(NoopSigner),
            Box::new(NoopVerifier),
        )
        .map_err(map_error)?;
        Ok(Arc::new(Self { handle }))
    }

    pub fn dispatch(&self, command: FfiCommand) -> Result<FfiMutationReceipt, FfiError> {
        let cmd = to_command(command)?;
        self.handle.dispatch(cmd).map(receipt).map_err(map_error)
    }

    pub fn list_tasks(
        &self,
        query: FfiTaskQuery,
        page: FfiPage,
        today: String,
    ) -> Result<FfiPagedTasks, FfiError> {
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
        let result = self.handle.list_tasks(&q, &p, &today).map_err(map_error)?;
        Ok(FfiPagedTasks {
            items: result
                .items
                .into_iter()
                .map(|t| FfiTaskSummary {
                    id: t.id.to_string(),
                    title: t.title,
                    completed: t.completed,
                    due_date: t.due_date.map(|d| format!("{d:?}")),
                    priority: format!("{:?}", t.priority).to_lowercase(),
                })
                .collect(),
            next_cursor: result.next_cursor,
        })
    }

    pub fn search(&self, text: String, limit: u32) -> Result<Vec<FfiSearchHit>, FfiError> {
        let hits = self.handle.search_tasks(&text, limit).map_err(map_error)?;
        Ok(hits
            .into_iter()
            .map(|h| FfiSearchHit {
                task_id: h.task_id.to_string(),
                title: h.title,
            })
            .collect())
    }

    pub fn close(&self) {
        self.handle.close();
    }
}
