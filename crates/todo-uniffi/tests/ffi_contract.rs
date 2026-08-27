//! FFI contract: open/close, commands, queries, error mapping, page bounds.

use tempfile::tempdir;
use todo_uniffi::{Core, CoreErrorCode, FfiCommand, FfiPage, FfiTaskQuery};

fn open(dir: &std::path::Path) -> std::sync::Arc<Core> {
    Core::open(
        dir.join("profile.db").to_string_lossy().to_string(),
        vec![7u8; 32],
        vec![1u8; 32],
    )
    .unwrap()
}

fn create(core: &Core, title: &str) -> String {
    let receipt = core
        .dispatch(FfiCommand::CreateTask {
            title: title.to_string(),
            description: String::new(),
            due_date: None,
            due_time: None,
            priority: "none".to_string(),
            list_id: None,
            tags: vec![],
        })
        .unwrap();
    receipt.affected_entities[0].clone()
}

#[test]
fn open_create_and_query() {
    let dir = tempdir().unwrap();
    let core = open(dir.path());
    let id = create(&core, "买牛奶");

    let page = FfiPage {
        cursor: None,
        limit: 50,
    };
    let q = FfiTaskQuery {
        list: "all".to_string(),
        active_only: true,
    };
    let result = core.list_tasks(q, page, "2026-09-01".to_string()).unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].title, "买牛奶");
    assert_eq!(result.items[0].id, id);
}

#[test]
fn chinese_and_emoji_round_trip() {
    let dir = tempdir().unwrap();
    let core = open(dir.path());
    create(&core, "买牛奶 🥛 和面包");

    let hits = core.search("牛奶".to_string(), 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert!(hits[0].title.contains("🥛"));
}

#[test]
fn empty_title_maps_to_invalid_command() {
    let dir = tempdir().unwrap();
    let core = open(dir.path());
    let result = core.dispatch(FfiCommand::CreateTask {
        title: String::new(),
        description: String::new(),
        due_date: None,
        due_time: None,
        priority: "none".to_string(),
        list_id: None,
        tags: vec![],
    });
    match result {
        Err(todo_uniffi::FfiError::Core { code, .. }) => {
            assert_eq!(code, CoreErrorCode::Domain)
        }
        Ok(_) => panic!("expected error for empty title"),
    }
}

#[test]
fn page_limit_is_bounded() {
    let dir = tempdir().unwrap();
    let core = open(dir.path());
    for i in 0..250 {
        create(&core, &format!("任务{i}"));
    }
    // A request for more than the maximum returns at most the maximum page size.
    let page = FfiPage {
        cursor: None,
        limit: 500,
    };
    let q = FfiTaskQuery {
        list: "all".to_string(),
        active_only: true,
    };
    let result = core.list_tasks(q, page, "2026-09-01".to_string()).unwrap();
    assert_eq!(result.items.len(), 200);
    assert!(result.next_cursor.is_some());
}

#[test]
fn close_is_idempotent_and_blocks_calls() {
    let dir = tempdir().unwrap();
    let core = open(dir.path());
    create(&core, "任务");
    core.close();
    core.close();

    let result = core.dispatch(FfiCommand::CreateList {
        name: "x".to_string(),
    });
    match result {
        Err(todo_uniffi::FfiError::Core { code, .. }) => {
            assert_eq!(code, CoreErrorCode::Closed)
        }
        Ok(_) => panic!("expected closed error"),
    }
}
