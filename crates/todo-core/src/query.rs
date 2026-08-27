//! Public read views: smart lists, search, calendar, Trash, and history.

use serde_json::Value;

use todo_domain::clock::{LocalDate, LocalTime, YearMonth};
use todo_domain::ids::EntityId;
use todo_domain::model::Priority;
use todo_storage::calendar;
use todo_storage::query as sq;
use todo_storage::repository::Repository;
use todo_storage::search;

use crate::dispatch::Core;
use crate::error::CoreError;

pub const MAX_PAGE_LIMIT: u32 = 200;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ListScope {
    Inbox,
    Today,
    Tomorrow,
    Next7Days,
    Completed,
    List(EntityId),
    #[default]
    All,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TaskScope {
    #[default]
    Active,
    All,
}

#[derive(Clone, Debug, Default)]
pub struct TaskQuery {
    pub list: ListScope,
    pub scope: TaskScope,
}

#[derive(Clone, Debug, Default)]
pub struct Page {
    pub cursor: Option<String>,
    pub limit: u32,
}

#[derive(Clone, Debug)]
pub struct TaskSummary {
    pub id: EntityId,
    pub title: String,
    pub completed: bool,
    pub due_date: Option<LocalDate>,
    pub priority: Priority,
    pub list_id: Option<EntityId>,
}

#[derive(Clone, Debug)]
pub struct TaskDetails {
    pub id: EntityId,
    pub title: String,
    pub description: String,
    pub due_date: Option<LocalDate>,
    pub due_time: Option<LocalTime>,
    pub priority: Priority,
    pub completed: bool,
    pub list_id: Option<EntityId>,
    pub tags: Vec<EntityId>,
    pub subtasks: Vec<EntityId>,
}

#[derive(Clone, Debug)]
pub struct PagedTasks {
    pub items: Vec<TaskSummary>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CodePointRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug)]
pub struct SearchHit {
    pub task_id: EntityId,
    pub title: String,
    pub highlights: Vec<CodePointRange>,
}

#[derive(Clone, Debug)]
pub struct DayBucket {
    pub day: u8,
    pub task_ids: Vec<EntityId>,
}

#[derive(Clone, Debug)]
pub struct TrashEntry {
    pub id: EntityId,
    pub deleted: bool,
}

#[derive(Clone, Debug)]
pub struct ConflictRecord {
    pub entity: EntityId,
    pub field: String,
    pub replaced: Value,
    pub observed_at: i64,
}

impl Core {
    pub fn list_tasks(
        &self,
        query: &TaskQuery,
        page: &Page,
        today: &str,
    ) -> Result<PagedTasks, CoreError> {
        let repo = self.repo.lock().unwrap();
        let (list, active_only) = match query.list {
            ListScope::Inbox => ("inbox".to_string(), true),
            ListScope::Today => ("today".to_string(), true),
            ListScope::Tomorrow => ("tomorrow".to_string(), true),
            ListScope::Next7Days => ("next7".to_string(), true),
            ListScope::Completed => ("completed".to_string(), false),
            ListScope::List(id) => (id.to_string(), true),
            ListScope::All => ("all".to_string(), query.scope == TaskScope::Active),
        };
        let (where_clause, params) = sq::smart_list_filter(&list, active_only, today);
        let limit = page.limit.clamp(1, MAX_PAGE_LIMIT);
        let rows = sq::query_task_rows(
            &repo.conn,
            &where_clause,
            &params,
            page.cursor.as_deref(),
            limit,
        )?;

        let mut items = Vec::new();
        let mut last = None;
        for r in rows {
            items.push(TaskSummary {
                id: r.id,
                title: r.title.clone().unwrap_or_default(),
                completed: r.completed,
                due_date: r.due_date.as_deref().and_then(parse_date),
                priority: r
                    .priority
                    .as_deref()
                    .and_then(parse_priority)
                    .unwrap_or_default(),
                list_id: r.list_id,
            });
            last = Some(sq::hex_encode(&r.id.as_bytes()));
        }
        let next_cursor = if items.len() == limit as usize {
            last
        } else {
            None
        };
        Ok(PagedTasks { items, next_cursor })
    }

    pub fn task_details(&self, id: &EntityId) -> Result<TaskDetails, CoreError> {
        let repo = self.repo.lock().unwrap();
        let r = sq::task_details(&repo.conn, id)?;
        Ok(TaskDetails {
            id: r.id,
            title: r.title.unwrap_or_default(),
            description: r.description.unwrap_or_default(),
            due_date: r.due_date.as_deref().and_then(parse_date),
            due_time: r.due_time.as_deref().and_then(parse_time),
            priority: r
                .priority
                .as_deref()
                .and_then(parse_priority)
                .unwrap_or_default(),
            completed: r.completed,
            list_id: r.list_id,
            tags: sq::tag_ids(&repo.conn, id)?,
            subtasks: sq::subtask_ids(&repo.conn, id)?,
        })
    }

    pub fn search_tasks(&self, text: &str, limit: u32) -> Result<Vec<SearchHit>, CoreError> {
        let repo = self.repo.lock().unwrap();
        let rows = search::search(&repo.conn, text, limit.clamp(1, MAX_PAGE_LIMIT))?;
        Ok(rows
            .into_iter()
            .map(|r| {
                let highlights = highlight_ranges(&r.title, text);
                SearchHit {
                    task_id: r.task_id,
                    title: html_escape(&r.title),
                    highlights,
                }
            })
            .collect())
    }

    pub fn calendar(&self, month: YearMonth) -> Result<Vec<DayBucket>, CoreError> {
        let repo = self.repo.lock().unwrap();
        Ok(
            calendar::calendar_days(&repo.conn, month.year, month.month)?
                .into_iter()
                .map(|b| DayBucket {
                    day: b.day,
                    task_ids: b.task_ids,
                })
                .collect(),
        )
    }

    pub fn trash(&self) -> Result<Vec<TrashEntry>, CoreError> {
        let repo = self.repo.lock().unwrap();
        let mut stmt = repo.conn.prepare(
            "SELECT entity_id FROM entity_lifecycle WHERE deleted = 1 ORDER BY entity_id",
        )?;
        let ids = stmt
            .query_map([], |row| {
                let bytes: Vec<u8> = row.get(0)?;
                Ok(EntityId::from_uuid(uuid::Uuid::from_slice(&bytes).unwrap()))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ids
            .into_iter()
            .map(|id| TrashEntry { id, deleted: true })
            .collect())
    }

    pub fn conflict_history(&self, _page: &Page) -> Result<Vec<ConflictRecord>, CoreError> {
        let repo = self.repo.lock().unwrap();
        let mut stmt = repo.conn.prepare(
            "SELECT entity_id, field_name, replaced_value, observed_at FROM conflict_history ORDER BY observed_at DESC",
        )?;
        let records = stmt
            .query_map([], |row| {
                let bytes: Vec<u8> = row.get(0)?;
                let value_bytes: Vec<u8> = row.get(2)?;
                Ok(ConflictRecord {
                    entity: EntityId::from_uuid(uuid::Uuid::from_slice(&bytes).unwrap()),
                    field: row.get(1)?,
                    replaced: serde_json::from_slice(&value_bytes).unwrap_or(Value::Null),
                    observed_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(records)
    }

    pub fn field_value(&self, entity: &EntityId, field: &str) -> Result<Option<Value>, CoreError> {
        let repo = self.repo.lock().unwrap();
        Ok(Repository::read_field(&repo.conn, entity, field)?)
    }
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

fn parse_priority(s: &str) -> Option<Priority> {
    match s {
        "none" => Some(Priority::None),
        "low" => Some(Priority::Low),
        "medium" => Some(Priority::Medium),
        "high" => Some(Priority::High),
        _ => None,
    }
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn highlight_ranges(title: &str, text: &str) -> Vec<CodePointRange> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut ranges = Vec::new();
    let title_chars: Vec<char> = title.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();
    if text_chars.len() > title_chars.len() {
        return Vec::new();
    }
    for start in 0..=(title_chars.len() - text_chars.len()) {
        if title_chars[start..start + text_chars.len()] == text_chars[..] {
            ranges.push(CodePointRange {
                start,
                end: start + text_chars.len(),
            });
        }
    }
    ranges
}
