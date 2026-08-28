//! Version summaries: per-origin frontiers and bounded gap ranges.

use std::collections::BTreeMap;

use todo_domain::ids::DeviceId;

/// A half-open `[start, end)` operation-sequence range for one origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SeqRange {
    pub start: u64,
    pub end: u64,
}

impl SeqRange {
    pub fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    pub fn contains(&self, seq: u64) -> bool {
        seq >= self.start && seq < self.end
    }

    pub fn overlaps(&self, other: &SeqRange) -> bool {
        self.start < other.end && other.start < self.end
    }

    pub fn merge(&self, other: &SeqRange) -> Option<SeqRange> {
        if self.overlaps(other) || self.end == other.start || other.end == self.start {
            Some(SeqRange::new(
                self.start.min(other.start),
                self.end.max(other.end),
            ))
        } else {
            None
        }
    }
}

/// A summary of what one replica has: contiguous frontiers plus gaps.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VersionSummary {
    /// Origin device -> contiguous frontier (highest contiguous sequence + 1).
    pub frontiers: BTreeMap<DeviceId, u64>,
    /// Origin device -> sorted, non-overlapping missing ranges below the frontier.
    pub gaps: BTreeMap<DeviceId, Vec<SeqRange>>,
}

impl VersionSummary {
    /// Normalize gaps so they are sorted and non-overlapping.
    pub fn normalize(&mut self) {
        for ranges in self.gaps.values_mut() {
            ranges.sort();
            let mut merged: Vec<SeqRange> = Vec::new();
            for r in ranges.drain(..) {
                match merged.last().and_then(|m| m.merge(&r)) {
                    Some(m) => *merged.last_mut().unwrap() = m,
                    None => merged.push(r),
                }
            }
            *ranges = merged;
        }
    }

    /// Compute ranges that `self` is missing relative to `remote`.
    pub fn missing_from(&self, remote: &VersionSummary) -> BTreeMap<DeviceId, Vec<SeqRange>> {
        let mut out: BTreeMap<DeviceId, Vec<SeqRange>> = BTreeMap::new();
        for (origin, remote_frontier) in &remote.frontiers {
            let local_frontier = self.frontiers.get(origin).copied().unwrap_or(0);
            if *remote_frontier <= local_frontier {
                // Local already has up to or beyond the remote frontier;
                // only account for local gaps that the remote can fill.
            }
            // The remote has operations [local_frontier, remote_frontier) that we lack.
            if *remote_frontier > local_frontier {
                let ranges = vec![SeqRange::new(local_frontier, *remote_frontier)];
                // Subtract local gaps? No — local gaps are already-missing ranges below
                // local_frontier, so the tail [local_frontier, remote_frontier) is what we need.
                out.insert(*origin, ranges);
            }
        }
        // For each origin where we have gaps below our frontier, ask the remote to fill them.
        for (origin, local_gaps) in &self.gaps {
            if remote.frontiers.contains_key(origin) {
                let ranges = out.entry(*origin).or_default();
                for g in local_gaps {
                    ranges.push(*g);
                }
                // Normalize the combined ranges.
                ranges.sort();
                let mut merged: Vec<SeqRange> = Vec::new();
                for r in ranges.drain(..) {
                    match merged.last().and_then(|m| m.merge(&r)) {
                        Some(m) => *merged.last_mut().unwrap() = m,
                        None => merged.push(r),
                    }
                }
                *ranges = merged;
            }
        }
        out
    }

    /// True if `self` covers everything `other` has.
    pub fn covers(&self, other: &VersionSummary) -> bool {
        self.missing_from(other).is_empty()
    }
}
