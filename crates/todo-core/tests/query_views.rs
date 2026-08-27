//! Smart-list and pagination queries.

use std::path::Path;
use std::time::Duration;

use tempfile::tempdir;
use todo_core::{
    Core, CoreError, ListScope, MutationReceipt, OperationSigner, Page, SignatureBytes,
    SignatureVerifier, TaskQuery, TaskScope,
};
use todo_domain::clock::{LocalDate, LocalTime};
use todo_domain::command::*;
use todo_domain::ids::DeviceId;
use todo_domain::model::Priority;
use todo_storage::config::{SecretBytes, StorageConfig};

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

fn open_core(dir: &Path) -> Core {
    let cfg = StorageConfig {
        profile_path: dir.join("profile.db"),
        database_key: SecretBytes::from_bytes(vec![5; 32]),
        busy_timeout: Duration::from_secs(5),
    };
    Core::open(
        cfg,
        DeviceId::from_bytes([1; 32]),
        Box::new(NoopSigner),
        Box::new(NoopVerifier),
    )
    .unwrap()
}

fn create_task(core: &Core, title: &str) -> MutationReceipt {
    core.dispatch(Command::CreateTask(CreateTask {
        title: title.to_string(),
        description: String::new(),
        due_date: None,
        due_time: None,
        priority: Priority::None,
        list_id: None,
        tags: vec![],
    }))
    .unwrap()
}

#[test]
fn inbox_excludes_completed() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    let a = create_task(&core, "任务A").affected_entities[0];
    let b = create_task(&core, "任务B").affected_entities[0];
    create_task(&core, "任务C");
    core.dispatch(Command::SetTaskCompleted(SetTaskCompleted {
        task: EntityRef { id: b },
        completed: true,
    }))
    .unwrap();

    let page = Page {
        cursor: None,
        limit: 50,
    };
    let q = TaskQuery {
        list: ListScope::Inbox,
        scope: TaskScope::Active,
    };
    let result = core.list_tasks(&q, &page, "2026-09-01").unwrap();
    let ids: Vec<_> = result.items.iter().map(|t| t.id).collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&a));
    assert!(!ids.contains(&b));
}

#[test]
fn today_scope_uses_caller_date() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    let due = core
        .dispatch(Command::CreateTask(CreateTask {
            title: "今天到期".to_string(),
            description: String::new(),
            due_date: Some(LocalDate::new(2026, 9, 1).unwrap()),
            due_time: None,
            priority: Priority::High,
            list_id: None,
            tags: vec![],
        }))
        .unwrap()
        .affected_entities[0];
    create_task(&core, "无日期");

    let q = TaskQuery {
        list: ListScope::Today,
        scope: TaskScope::Active,
    };
    let result = core
        .list_tasks(
            &q,
            &Page {
                cursor: None,
                limit: 50,
            },
            "2026-09-01",
        )
        .unwrap();
    let ids: Vec<_> = result.items.iter().map(|t| t.id).collect();
    assert_eq!(ids, vec![due]);
}

#[test]
fn pagination_is_stable_by_id() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    let mut all = Vec::new();
    for i in 0..25 {
        all.push(create_task(&core, &format!("任务{i}")).affected_entities[0]);
    }

    let q = TaskQuery {
        list: ListScope::All,
        scope: TaskScope::Active,
    };
    let mut seen = Vec::new();
    let mut cursor = None;
    loop {
        let page = Page { cursor, limit: 10 };
        let r = core.list_tasks(&q, &page, "2026-09-01").unwrap();
        seen.extend(r.items.iter().map(|t| t.id));
        cursor = r.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(seen.len(), 25);
    let mut sorted = seen.clone();
    sorted.sort();
    assert_eq!(seen, sorted, "cursor pagination must not skip or duplicate");
}

#[test]
fn task_details_include_tags_and_subtasks() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    let task = create_task(&core, "父任务").affected_entities[0];
    let tag = core
        .dispatch(Command::CreateTag(CreateTag {
            name: "重要".to_string(),
        }))
        .unwrap()
        .affected_entities[0];
    core.dispatch(Command::SetTaskTag(SetTaskTag {
        task: EntityRef { id: task },
        tag: EntityRef { id: tag },
        attached: true,
    }))
    .unwrap();
    let sub = core
        .dispatch(Command::CreateSubtask(CreateSubtask {
            parent: EntityRef { id: task },
            title: "子任务".to_string(),
        }))
        .unwrap()
        .affected_entities[0];

    let details = core.task_details(&task).unwrap();
    assert_eq!(details.title, "父任务");
    assert!(details.tags.contains(&tag));
    assert!(details.subtasks.contains(&sub));
}

#[test]
fn due_time_is_preserved() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    let task = core
        .dispatch(Command::CreateTask(CreateTask {
            title: "带时间".to_string(),
            description: String::new(),
            due_date: Some(LocalDate::new(2026, 9, 1).unwrap()),
            due_time: Some(LocalTime::new(9, 30).unwrap()),
            priority: Priority::Medium,
            list_id: None,
            tags: vec![],
        }))
        .unwrap()
        .affected_entities[0];

    let d = core.task_details(&task).unwrap();
    assert_eq!(d.due_time, Some(LocalTime::new(9, 30).unwrap()));
}
