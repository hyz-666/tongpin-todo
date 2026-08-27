//! Task search via FTS5 trigram and bounded normalization scans.

use rusqlite::Connection;

use todo_domain::ids::EntityId;

use crate::error::StorageError;

pub struct SearchRow {
    pub task_id: EntityId,
    pub title: String,
    pub description: String,
}

/// Search tasks. Queries of at least 3 code points use the FTS5 trigram index;
/// shorter queries fall back to a bounded normalized scan.
pub fn search(conn: &Connection, text: &str, limit: u32) -> Result<Vec<SearchRow>, StorageError> {
    if text.chars().count() >= 3 {
        search_fts(conn, text, limit)
    } else {
        search_scan(conn, text, limit)
    }
}

fn search_fts(conn: &Connection, text: &str, limit: u32) -> Result<Vec<SearchRow>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT task_id, title, description FROM task_fts WHERE task_fts MATCH ? LIMIT ?",
    )?;
    let rows = stmt
        .query_map(
            rusqlite::params![format!("\"{text}\""), limit as i64],
            |row| {
                let bytes: Vec<u8> = row.get(0)?;
                Ok(SearchRow {
                    task_id: EntityId::from_uuid(uuid::Uuid::from_slice(&bytes).unwrap()),
                    title: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    description: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn search_scan(conn: &Connection, text: &str, limit: u32) -> Result<Vec<SearchRow>, StorageError> {
    // Bounded scan: normalized LIKE over a capped number of rows.
    let pattern = format!("%{text}%");
    let mut stmt = conn.prepare(
        "SELECT entity_id, title, description FROM tasks
         WHERE deleted = 0 AND (title LIKE ?1 OR description LIKE ?1)
         ORDER BY entity_id LIMIT ?2",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![pattern, limit as i64], |row| {
            let bytes: Vec<u8> = row.get(0)?;
            Ok(SearchRow {
                task_id: EntityId::from_uuid(uuid::Uuid::from_slice(&bytes).unwrap()),
                title: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                description: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
