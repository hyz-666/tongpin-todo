//! Search: FTS5 trigram, normalized scan, and highlight ranges.

use std::path::Path;
use std::time::Duration;

use tempfile::tempdir;
use todo_core::{Core, CoreError, OperationSigner, SignatureBytes, SignatureVerifier};
use todo_domain::command::*;
use todo_domain::ids::DeviceId;
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

fn create(core: &Core, title: &str, description: &str) {
    core.dispatch(Command::CreateTask(CreateTask {
        title: title.to_string(),
        description: description.to_string(),
        due_date: None,
        due_time: None,
        priority: todo_domain::model::Priority::None,
        list_id: None,
        tags: vec![],
    }))
    .unwrap();
}

#[test]
fn search_title_substring() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    create(&core, "买牛奶和面包", "");
    create(&core, "写周报", "");
    let hits = core.search_tasks("牛奶", 50).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "买牛奶和面包");
}

#[test]
fn search_description_substring() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    create(&core, "任务一", "记得给张总打电话");
    create(&core, "任务二", "");
    let hits = core.search_tasks("电话", 50).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "任务一");
}

#[test]
fn short_query_scans() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    create(&core, "会议纪要", "");
    create(&core, "会议记录", "");
    let hits = core.search_tasks("会", 50).unwrap(); // single code point
    assert_eq!(hits.len(), 2);
}

#[test]
fn highlights_are_code_point_ranges() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    create(&core, "去超市买牛奶", "");
    let hits = core.search_tasks("牛奶", 50).unwrap();
    assert_eq!(hits.len(), 1);
    let hit = &hits[0];
    assert!(!hit.highlights.is_empty());
    // "去超市买牛奶" -> 牛奶 at code points 4..6 (去=0,超=1,市=2,买=3,牛=4,奶=5)
    assert_eq!(hit.highlights[0].start, 4);
    assert_eq!(hit.highlights[0].end, 6);
}

#[test]
fn no_html_in_results() {
    let dir = tempdir().unwrap();
    let core = open_core(dir.path());
    create(&core, "包含 <b> 标签", "");
    let hits = core.search_tasks("标签", 50).unwrap();
    assert_eq!(hits.len(), 1);
    assert!(!hits[0].title.contains('<'));
}
