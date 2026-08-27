//! Calendar projection: floating due dates grouped into day buckets.

use rusqlite::Connection;

use todo_domain::ids::EntityId;

use crate::error::StorageError;

pub struct DayBucket {
    pub day: u8,
    pub task_ids: Vec<EntityId>,
}

/// Return tasks with a due date in `year-month`, grouped by day, ordered by day.
pub fn calendar_days(
    conn: &Connection,
    year: i32,
    month: u8,
) -> Result<Vec<DayBucket>, StorageError> {
    let prefix = format!("{year:04}-{month:02}-");
    let mut stmt = conn.prepare(
        "SELECT due_date, entity_id FROM tasks
         WHERE deleted = 0 AND completed = 0 AND due_date LIKE ?1
         ORDER BY due_date, entity_id",
    )?;
    let mut buckets: Vec<DayBucket> = Vec::new();
    let rows = stmt.query_map([format!("{prefix}%")], |row| {
        let due: String = row.get(0)?;
        let bytes: Vec<u8> = row.get(1)?;
        Ok((
            due,
            EntityId::from_uuid(uuid::Uuid::from_slice(&bytes).unwrap()),
        ))
    })?;
    for r in rows {
        let (due, id) = r?;
        let day: u8 = due
            .rsplit('-')
            .next()
            .and_then(|d| d.parse().ok())
            .unwrap_or(0);
        match buckets.last_mut() {
            Some(b) if b.day == day => b.task_ids.push(id),
            _ => buckets.push(DayBucket {
                day,
                task_ids: vec![id],
            }),
        }
    }
    Ok(buckets)
}
