//! Verified, idempotent remote batch application.

use todo_domain::ids::DeviceId;
use todo_domain::operation::VerifiedOperation;
use todo_storage::frontier;
use todo_storage::materialize;
use todo_storage::repository::Repository;

use crate::dispatch::{Core, SignatureBytes};
use crate::error::CoreError;

#[derive(Clone)]
pub struct SignedOperation {
    pub signer: DeviceId,
    pub signature: SignatureBytes,
    pub operation: VerifiedOperation,
}

pub struct ApplyBatchReceipt {
    pub applied: usize,
    pub duplicated: usize,
}

impl Core {
    pub fn apply_remote_batch(
        &self,
        operations: Vec<SignedOperation>,
    ) -> Result<ApplyBatchReceipt, CoreError> {
        let mut repo = self.repo.lock().unwrap();
        let mut applied = 0usize;
        let mut duplicated = 0usize;

        for op in &operations {
            if !self.is_member(&op.signer) {
                return Err(CoreError::UnknownMember);
            }
            let canonical = serde_json::to_vec(&op.operation)
                .map_err(|e| CoreError::InvalidCommand(format!("serialize: {e}")))?;
            self.verifier
                .verify(&op.signer, &canonical, &op.signature.0)?;

            let origin = op.operation.stamp.operation.origin;
            let sequence = op.operation.stamp.operation.sequence;
            let frontier_value = Repository::read_frontier(&repo.conn, &origin)?.unwrap_or(0);
            if sequence > frontier_value + 1 {
                return Err(CoreError::OriginGap);
            }

            let tx = repo.conn.transaction()?;
            let inserted = Repository::insert_operation(
                &tx,
                &origin,
                sequence,
                &canonical,
                Some(&op.signature.0),
                now_millis(),
            )?;
            if !inserted {
                duplicated += 1;
                tx.commit()?;
                continue;
            }

            let (current_generation, _) = Repository::read_lifecycle(&tx, &op.operation.entity)?;
            if op.operation.stamp.generation.0 < current_generation {
                // Stale generation: recorded for history, not materialized.
                tx.commit()?;
                continue;
            }

            materialize::apply(&tx, &op.operation)?;
            frontier::advance_frontier(&tx, &origin, sequence)?;
            let revision = Repository::projection_revision(&tx)? + 1;
            Repository::bump_revision(&tx, revision)?;
            tx.commit()?;
            applied += 1;
        }

        Ok(ApplyBatchReceipt {
            applied,
            duplicated,
        })
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
