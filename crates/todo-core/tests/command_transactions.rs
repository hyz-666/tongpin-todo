//! Command dispatch: every command variant persists atomically.

use std::path::Path;
use std::time::Duration;

use tempfile::tempdir;
use todo_core::{
    Core, CoreError, MutationReceipt, OperationSigner, SignatureBytes, SignatureVerifier,
};
use todo_domain::clock::{LocalDate, LocalTime};
use todo_domain::command::*;
use todo_domain::ids::DeviceId;
use todo_domain::model::Priority;
use todo_storage::config::{SecretBytes, StorageConfig};

struct NoopSigner;
impl OperationSigner for NoopSigner {
    fn sign(&self, _canonical: &[u8]) -> Result<SignatureBytes, CoreError> {
        Ok(SignatureBytes(vec![0u8; 64]))
    }
}

struct NoopVerifier;
impl SignatureVerifier for NoopVerifier {
    fn verify(&self, _signer: &DeviceId, _canonical: &[u8], _sig: &[u8]) -> Result<(), CoreError> {
        Ok(())
    }
}

fn open_core(dir: &Path) -> Core {
    let cfg = StorageConfig {
        profile_path: dir.join("profile.db"),
        database_key: SecretBytes::from_bytes(vec![7; 32]),
        busy_timeout: Duration::from_secs(5),
    };
    let device = DeviceId::from_bytes([1; 32]);
    Core::open(cfg, device, Box::new(NoopSigner), Box::new(NoopVerifier)).unwrap()
}

fn reopen_core(dir: &Path) -> Core {
    open_core(dir)
}

fn new_task(title: &str) -> Command {
    Command::CreateTask(CreateTask {
        title: title.to_string(),
        description: String::new(),
        due_date: None,
        due_time: None,
        priority: Priority::None,
        list_id: None,
        tags: vec![],
    })
}

#[test]
fn create_task_persists_after_reopen() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    let receipt: MutationReceipt = core.dispatch(new_task("买牛奶")).unwrap();
    assert!(!receipt.operation_ids.is_empty());

    let reopened = reopen_core(dir.path());
    let title = reopened
        .field(&receipt.affected_entities[0], "title")
        .unwrap()
        .unwrap();
    assert_eq!(title, serde_json::json!("买牛奶"));
}

#[test]
fn set_task_field_persists() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    let r = core.dispatch(new_task("初始")).unwrap();
    let id = r.affected_entities[0];
    core.dispatch(Command::SetTaskField(SetTaskField {
        task: EntityRef { id },
        field: TaskField::Description("补充说明".to_string()),
    }))
    .unwrap();

    let reopened = reopen_core(dir.path());
    assert_eq!(
        reopened.field(&id, "description").unwrap().unwrap(),
        serde_json::json!("补充说明")
    );
}

#[test]
fn set_completed_persists() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    let id = core.dispatch(new_task("完成我")).unwrap().affected_entities[0];
    core.dispatch(Command::SetTaskCompleted(SetTaskCompleted {
        task: EntityRef { id },
        completed: true,
    }))
    .unwrap();

    let reopened = reopen_core(dir.path());
    assert_eq!(
        reopened.field(&id, "completed").unwrap().unwrap(),
        serde_json::json!(true)
    );
}

#[test]
fn delete_and_restore_lifecycle() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    let id = core
        .dispatch(new_task("删了又回"))
        .unwrap()
        .affected_entities[0];
    core.dispatch(Command::DeleteTask(EntityRef { id }))
        .unwrap();
    assert!(core.is_deleted(&id));
    core.dispatch(Command::RestoreTask(RestoreTask {
        task: EntityRef { id },
    }))
    .unwrap();
    assert!(!core.is_deleted(&id));

    let reopened = reopen_core(dir.path());
    assert!(!reopened.is_deleted(&id));
}

#[test]
fn task_with_due_and_priority_persists() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    let r = core
        .dispatch(Command::CreateTask(CreateTask {
            title: "带日期的".to_string(),
            description: String::new(),
            due_date: Some(LocalDate::new(2026, 9, 1).unwrap()),
            due_time: Some(LocalTime::new(9, 30).unwrap()),
            priority: Priority::High,
            list_id: None,
            tags: vec![],
        }))
        .unwrap();
    let id = r.affected_entities[0];

    let reopened = reopen_core(dir.path());
    assert_eq!(
        reopened.field(&id, "due_date").unwrap().unwrap(),
        serde_json::json!("2026-09-01")
    );
    assert_eq!(
        reopened.field(&id, "priority").unwrap().unwrap(),
        serde_json::json!("high")
    );
}

#[test]
fn list_and_tag_commands_persist() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    let list = core
        .dispatch(Command::CreateList(CreateList {
            name: "工作".to_string(),
        }))
        .unwrap()
        .affected_entities[0];
    let tag = core
        .dispatch(Command::CreateTag(CreateTag {
            name: "重要".to_string(),
        }))
        .unwrap()
        .affected_entities[0];

    let reopened = reopen_core(dir.path());
    assert_eq!(
        reopened.field(&list, "name").unwrap().unwrap(),
        serde_json::json!("工作")
    );
    assert_eq!(
        reopened.field(&tag, "name").unwrap().unwrap(),
        serde_json::json!("重要")
    );
}

#[test]
fn subtask_commands_persist() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    let task = core.dispatch(new_task("父任务")).unwrap().affected_entities[0];
    let sub = core
        .dispatch(Command::CreateSubtask(CreateSubtask {
            parent: EntityRef { id: task },
            title: "子任务".to_string(),
        }))
        .unwrap()
        .affected_entities[0];

    let reopened = reopen_core(dir.path());
    assert_eq!(
        reopened.field(&sub, "title").unwrap().unwrap(),
        serde_json::json!("子任务")
    );
    assert_eq!(reopened.parent_of(&sub).unwrap(), Some(task));
}

#[test]
fn failed_dispatch_rolls_back_atomically() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    // A command whose validation fails must leave no partial state.
    let result = core.dispatch(Command::CreateList(CreateList {
        name: String::new(), // empty name -> invalid
    }));
    assert!(result.is_err());

    let reopened = reopen_core(dir.path());
    assert_eq!(reopened.count_entities(), 0);
}
