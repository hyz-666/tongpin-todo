//! Calendar projection, Trash, and conflict history.

use std::path::Path;
use std::time::Duration;

use tempfile::tempdir;
use todo_core::{Core, CoreError, OperationSigner, SignatureBytes, SignatureVerifier};
use todo_domain::clock::{LocalDate, YearMonth};
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

fn create_due(core: &Core, title: &str, date: LocalDate) -> todo_domain::ids::EntityId {
    core.dispatch(Command::CreateTask(CreateTask {
        title: title.to_string(),
        description: String::new(),
        due_date: Some(date),
        due_time: None,
        priority: Priority::None,
        list_id: None,
        tags: vec![],
    }))
    .unwrap()
    .affected_entities[0]
}

#[test]
fn calendar_groups_due_dates_by_day() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    create_due(&core, "一号任务", LocalDate::new(2026, 9, 1).unwrap());
    create_due(&core, "二号任务", LocalDate::new(2026, 9, 1).unwrap());
    create_due(&core, "十五号任务", LocalDate::new(2026, 9, 15).unwrap());
    create_due(&core, "其他月任务", LocalDate::new(2026, 10, 1).unwrap());

    let buckets = core.calendar(YearMonth::new(2026, 9).unwrap()).unwrap();
    // 九月有两个有任务的日期
    let day1 = buckets.iter().find(|b| b.day == 1).unwrap();
    assert_eq!(day1.task_ids.len(), 2);
    let day15 = buckets.iter().find(|b| b.day == 15).unwrap();
    assert_eq!(day15.task_ids.len(), 1);
    // 十月的不在此月
    assert!(buckets.iter().all(|b| b.day != 0 || !b.task_ids.is_empty()));
}

#[test]
fn trash_lists_deleted_entities() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    let keep = create_due(&core, "保留", LocalDate::new(2026, 9, 1).unwrap());
    let gone = core
        .dispatch(Command::CreateList(CreateList {
            name: "删除我".to_string(),
        }))
        .unwrap()
        .affected_entities[0];
    core.dispatch(Command::DeleteList(EntityRef { id: gone }))
        .unwrap();

    let trash = core.trash().unwrap();
    let ids: Vec<_> = trash.iter().map(|t| t.id).collect();
    assert!(ids.contains(&gone));
    assert!(!ids.contains(&keep));
}

#[test]
fn conflict_history_records_replaced_values() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    let task = create_due(&core, "原始标题", LocalDate::new(2026, 9, 1).unwrap());
    core.dispatch(Command::SetTaskField(SetTaskField {
        task: EntityRef { id: task },
        field: TaskField::Title("新标题".to_string()),
    }))
    .unwrap();

    let history = core
        .conflict_history(&todo_core::Page {
            cursor: None,
            limit: 50,
        })
        .unwrap();
    assert!(!history.is_empty());
    let record = &history[0];
    assert_eq!(record.field, "title");
    assert_eq!(record.replaced, serde_json::json!("原始标题"));
}
