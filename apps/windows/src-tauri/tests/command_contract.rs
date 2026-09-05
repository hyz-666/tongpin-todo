//! Command-layer contract: pins the FFI boundary between the React frontend and
//! the `#[tauri::command]` layer against `contracts/core-api-version.json`.

use serde_json::json;

use tongpin_windows_lib::commands::{CommandDto, TaskSummaryDto};

/// The command layer exposes `coreApi = 1` (see `contracts/core-api-version.json`).
#[test]
fn api_version_matches_contract() {
    assert_eq!(tongpin_windows_lib::API_VERSION, 1);
}

/// The frontend sends command JSON with a camelCase `type` tag and camelCase
/// field names; the serde boundary must map them onto the Rust DTO.
#[test]
fn create_task_command_deserializes_from_camel_case() {
    let json = json!({
        "type": "createTask",
        "title": "Buy milk",
        "description": "2% from the corner store",
        "dueDate": "2026-09-07",
        "dueTime": "09:30",
        "priority": "high",
        "listId": "00000000-0000-0000-0000-000000000001",
        "tags": ["00000000-0000-0000-0000-000000000002"]
    });

    let cmd: CommandDto = serde_json::from_value(json).expect("createTask deserializes");
    match cmd {
        CommandDto::CreateTask {
            title,
            description,
            due_date,
            due_time,
            priority,
            list_id,
            tags,
        } => {
            assert_eq!(title, "Buy milk");
            assert_eq!(description, "2% from the corner store");
            assert_eq!(due_date.as_deref(), Some("2026-09-07"));
            assert_eq!(due_time.as_deref(), Some("09:30"));
            assert_eq!(priority, "high");
            assert_eq!(
                list_id.as_deref(),
                Some("00000000-0000-0000-0000-000000000001")
            );
            assert_eq!(tags, vec!["00000000-0000-0000-0000-000000000002"]);
        }
        other => panic!("expected CreateTask, got {other:?}"),
    }
}

#[test]
fn set_task_completed_command_deserializes() {
    let json = json!({ "type": "setTaskCompleted", "task": "00000000-0000-0000-0000-000000000003", "completed": true });
    let cmd: CommandDto = serde_json::from_value(json).expect("setTaskCompleted deserializes");
    match cmd {
        CommandDto::SetTaskCompleted { task, completed } => {
            assert_eq!(task, "00000000-0000-0000-0000-000000000003");
            assert!(completed);
        }
        other => panic!("expected SetTaskCompleted, got {other:?}"),
    }
}

/// Command-layer responses serialize with camelCase field names, matching the
/// TypeScript `TaskSummary` / `PagedTasks` types.
#[test]
fn task_summary_serializes_to_camel_case() {
    let dto = TaskSummaryDto {
        id: "00000000-0000-0000-0000-000000000004".into(),
        title: "Write report".into(),
        completed: false,
        due_date: Some("2026-09-08".into()),
        priority: "medium".into(),
        list_id: Some("00000000-0000-0000-0000-000000000005".into()),
    };
    let value = serde_json::to_value(dto).expect("serializes");
    assert_eq!(value["id"], "00000000-0000-0000-0000-000000000004");
    assert_eq!(value["title"], "Write report");
    assert_eq!(value["dueDate"], "2026-09-08");
    assert_eq!(value["listId"], "00000000-0000-0000-0000-000000000005");
    assert_eq!(value["priority"], "medium");
    assert_eq!(value["completed"], false);
}
