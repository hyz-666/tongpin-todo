//! Low-level read projections for tasks, lists, tags, and subtasks.

use rusqlite::Connection;

use todo_domain::ids::EntityId;

use crate::error::StorageError;

/// A raw task row from the `tasks` projection.
pub struct TaskRow {
    pub id: EntityId,
    pub title: Option<String>,
    pub description: Option<String>,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub priority: Option<String>,
    pub completed: bool,
    pub list_id: Option<EntityId>,
}

fn id_from_blob(bytes: Vec<u8>) -> Option<EntityId> {
    let arr: [u8; 16] = bytes.try_into().ok()?;
    Some(EntityId::from_uuid(uuid::Uuid::from_bytes(arr)))
}

fn row_to_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRow> {
    let id_bytes: Vec<u8> = row.get(0)?;
    let list_bytes: Option<Vec<u8>> = row.get(6)?;
    Ok(TaskRow {
        id: EntityId::from_uuid(uuid::Uuid::from_slice(&id_bytes).unwrap()),
        title: row.get(1)?,
        description: row.get(2)?,
        due_date: row.get(3)?,
        due_time: row.get(4)?,
        priority: row.get(5)?,
        completed: row.get::<_, i64>(7)? != 0,
        list_id: list_bytes.and_then(id_from_blob),
    })
}

/// Filter fragment for a smart list. Returns `(where_clause, params)`.
pub fn smart_list_filter(list: &str, active_only: bool, today: &str) -> (String, Vec<String>) {
    let mut where_clause = String::from("deleted = 0");
    if active_only {
        where_clause.push_str(" AND completed = 0");
    }
    let mut params: Vec<String> = Vec::new();
    match list {
        "inbox" => {
            where_clause.push_str(" AND list_id IS NULL");
        }
        "today" => {
            where_clause.push_str(" AND due_date = ?");
            params.push(today.to_string());
        }
        "tomorrow" => {
            where_clause.push_str(" AND due_date = ?");
            params.push(tomorrow_of(today));
        }
        "next7" => {
            where_clause.push_str(" AND due_date >= ? AND due_date < ?");
            params.push(today.to_string());
            params.push(plus_days(today, 7));
        }
        "completed" => {
            where_clause.push_str(" AND completed = 1");
        }
        "all" => {}
        _ => {
            // custom list id
            where_clause.push_str(" AND list_id = ?");
            params.push(list.to_string());
        }
    }
    (where_clause, params)
}

fn tomorrow_of(today: &str) -> String {
    plus_days(today, 1)
}

fn plus_days(date: &str, days: i64) -> String {
    // date is YYYY-MM-DD; parse and add days.
    let parts: Vec<i64> = date
        .split('-')
        .map(|p| p.parse::<i64>().unwrap_or(0))
        .collect();
    if parts.len() != 3 {
        return date.to_string();
    }
    let (y, m, d) = (parts[0], parts[1], parts[2]);
    // Convert to days since epoch using a simple algorithm.
    let days_since = y * 365 + y / 4 - y / 100 + y / 400 + (m * 306 + 5) / 10 + d - 1;
    let total = days_since + days;
    // Reverse the conversion.
    let mut y = (total * 400) / 146097;
    let mut rem = total - (y * 146097 + 3) / 4;
    if rem < 0 {
        y -= 1;
        rem = total - (y * 146097 + 3) / 4;
    }
    let m = (rem * 12 + 2) / 367;
    let d = rem - (m * 367 + 5) / 12 + 1;
    let mm = if m < 3 { m + 3 } else { m - 9 };
    let yy = if m < 3 { y + 1 } else { y };
    format!("{yy:04}-{mm:02}-{d:02}")
}

pub fn query_task_rows(
    conn: &Connection,
    where_clause: &str,
    params: &[String],
    cursor: Option<&str>,
    limit: u32,
) -> Result<Vec<TaskRow>, StorageError> {
    let mut sql = format!(
        "SELECT entity_id, title, description, due_date, due_time, priority, list_id, completed
         FROM tasks WHERE {where_clause}"
    );
    let mut args: Vec<rusqlite::types::Value> = params.iter().map(|p| p.clone().into()).collect();
    if let Some(c) = cursor {
        sql.push_str(" AND entity_id > ?");
        args.push(hex_decode(c).into());
    }
    sql.push_str(" ORDER BY entity_id LIMIT ?");
    args.push((limit as i64).into());

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(args.iter()), row_to_task)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0xf) as usize] as char);
    }
    s
}

fn hex_decode(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let hi = (bytes[i] as char).to_digit(16).unwrap_or(0) as u8;
        let lo = (bytes[i + 1] as char).to_digit(16).unwrap_or(0) as u8;
        out.push((hi << 4) | lo);
        i += 2;
    }
    out
}

pub fn task_details(conn: &Connection, id: &EntityId) -> Result<TaskRow, StorageError> {
    conn.query_row(
        "SELECT entity_id, title, description, due_date, due_time, priority, list_id, completed
         FROM tasks WHERE entity_id = ?1",
        [id.as_bytes().as_slice()],
        row_to_task,
    )
    .map_err(Into::into)
}

/// Count non-deleted subtasks of a task.
pub fn subtask_ids(conn: &Connection, task_id: &EntityId) -> Result<Vec<EntityId>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT entity_id FROM subtasks WHERE parent_task_id = ?1 AND deleted = 0 ORDER BY entity_id",
    )?;
    let ids = stmt
        .query_map([task_id.as_bytes().as_slice()], |row| {
            let bytes: Vec<u8> = row.get(0)?;
            Ok(EntityId::from_uuid(uuid::Uuid::from_slice(&bytes).unwrap()))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ids)
}

/// Tag ids attached to a task (from the `tag:<id>` field registers).
pub fn tag_ids(conn: &Connection, task_id: &EntityId) -> Result<Vec<EntityId>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT field_name FROM field_registers WHERE entity_id = ?1 AND field_name LIKE 'tag:%' AND CAST(value AS TEXT) = 'true'",
    )?;
    let mut ids = Vec::new();
    let rows = stmt.query_map([task_id.as_bytes().as_slice()], |row| {
        row.get::<_, String>(0)
    })?;
    for name in rows {
        let name = name?;
        if let Some(uuid_str) = name.strip_prefix("tag:")
            && let Ok(u) = uuid::Uuid::parse_str(uuid_str)
        {
            ids.push(EntityId::from_uuid(u));
        }
    }
    Ok(ids)
}
