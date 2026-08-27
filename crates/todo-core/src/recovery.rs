//! Storage recovery: detect and quarantine unrecoverable profiles.

use todo_storage::Storage;
use todo_storage::config::StorageConfig;
use todo_storage::error::StorageError;
use todo_storage::quarantine;

use crate::error::CoreError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplicaState {
    Ready,
    ReadOnlyLowSpace,
    Recovering(RecoveryReason),
    Unavailable(UnavailableReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryReason {
    Corrupt,
    DirtyShutdown,
    Locked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnavailableReason {
    MissingKey,
    FailedMigration,
}

/// Attempt to open a profile, mapping failures to distinct recovery states.
/// On corruption the original file is quarantined and no empty profile is
/// created.
pub fn open_with_recovery(
    config: StorageConfig,
) -> Result<(Option<Storage>, ReplicaState), CoreError> {
    match Storage::open(config.clone()) {
        Ok(storage) => Ok((Some(storage), ReplicaState::Ready)),
        Err(StorageError::WrongKey) => {
            // A truncated/undersized file is corruption, not a wrong key.
            let undersized = std::fs::metadata(&config.profile_path)
                .map(|m| m.len() < 4096)
                .unwrap_or(false);
            if undersized {
                let _ = quarantine::quarantine(&config.profile_path, "corrupt");
                Ok((None, ReplicaState::Recovering(RecoveryReason::Corrupt)))
            } else {
                Ok((
                    None,
                    ReplicaState::Unavailable(UnavailableReason::MissingKey),
                ))
            }
        }
        Err(StorageError::NotEncrypted) => Ok((
            None,
            ReplicaState::Unavailable(UnavailableReason::MissingKey),
        )),
        Err(StorageError::UnsupportedDowngrade { .. } | StorageError::MigrationFailed(_)) => Ok((
            None,
            ReplicaState::Unavailable(UnavailableReason::FailedMigration),
        )),
        Err(StorageError::Sqlite(_)) => {
            let _ = quarantine::quarantine(&config.profile_path, "corrupt");
            Ok((None, ReplicaState::Recovering(RecoveryReason::Corrupt)))
        }
        Err(StorageError::Io(_)) => Ok((None, ReplicaState::Recovering(RecoveryReason::Locked))),
        Err(StorageError::ApplicationIdMismatch(_)) => {
            Ok((None, ReplicaState::Recovering(RecoveryReason::Corrupt)))
        }
    }
}
