//! Small cancellable compaction batches over the operations table.

/// A compaction plan broken into small cancellable batches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompactionPlan {
    pub total_batches: usize,
    pub batch_size: usize,
}

impl CompactionPlan {
    /// Plan a compaction of `operation_count` rows in batches of `batch_size`.
    /// Returns an empty plan (no work) for a zero or trivial count.
    pub fn plan(operation_count: usize, batch_size: usize) -> Self {
        if batch_size == 0 {
            return Self {
                total_batches: 0,
                batch_size: 0,
            };
        }
        Self {
            total_batches: operation_count.div_ceil(batch_size),
            batch_size,
        }
    }

    /// The row range `[start, end)` for a given batch index.
    pub fn batch_range(&self, index: usize) -> Option<(usize, usize)> {
        if index >= self.total_batches {
            return None;
        }
        let start = index * self.batch_size;
        Some((start, start + self.batch_size))
    }
}

/// Whether the watermark covers an operation so it may be compacted.
pub fn compactable(sequence: u64, watermark: u64) -> bool {
    sequence <= watermark
}
