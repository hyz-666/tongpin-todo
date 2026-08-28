//! Range requests: normalization, deduplication, and resource bounds.

use todo_domain::ids::DeviceId;

use crate::error::ProtocolError;
use crate::version_summary::SeqRange;

/// A request for specific operation ranges from one origin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RangeRequest {
    pub origin: DeviceId,
    pub ranges: Vec<SeqRange>,
}

impl RangeRequest {
    /// Sort, merge, and drop empty ranges so the request is canonical.
    pub fn normalize(&mut self) {
        self.ranges.retain(|r| !r.is_empty());
        self.ranges.sort();
        let mut merged: Vec<SeqRange> = Vec::new();
        for r in self.ranges.drain(..) {
            match merged.last().and_then(|m| m.merge(&r)) {
                Some(m) => *merged.last_mut().unwrap() = m,
                None => merged.push(r),
            }
        }
        self.ranges = merged;
    }

    /// Total number of operations requested.
    pub fn total_operations(&self) -> u64 {
        self.ranges.iter().map(|r| r.len()).sum()
    }

    /// Reject requests that ask for more operations than the bounded limit.
    pub fn validate(&self, max_operations: u64) -> Result<(), ProtocolError> {
        if self.total_operations() > max_operations {
            return Err(ProtocolError::FrameTooLarge);
        }
        Ok(())
    }
}
