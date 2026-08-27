//! Quarantine of unrecoverable profile files.

use std::path::{Path, PathBuf};

use crate::error::StorageError;

/// Move a damaged profile aside under a unique quarantine name so it is never
/// silently overwritten. Returns the quarantine path.
pub fn quarantine(profile_path: &Path, reason: &str) -> Result<PathBuf, StorageError> {
    let file_name = profile_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "profile.db".to_string());
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let unique = profile_path.with_file_name(format!("{file_name}.quarantine.{reason}.{stamp}"));
    std::fs::rename(profile_path, &unique).map_err(StorageError::from)?;
    Ok(unique)
}
