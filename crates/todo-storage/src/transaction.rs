//! Transaction helpers for the storage layer.

use rusqlite::Transaction;

use crate::error::StorageError;

/// Commit a transaction, mapping errors into `StorageError`.
pub fn commit(tx: Transaction<'_>) -> Result<(), StorageError> {
    tx.commit().map_err(Into::into)
}
