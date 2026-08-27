//! Monotonic network generation, incremented on each network change.

/// A monotonically increasing generation counter. A new generation invalidates
/// every candidate discovered under an older generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkGeneration(pub u64);

impl NetworkGeneration {
    pub const fn new() -> Self {
        Self(0)
    }

    pub fn value(&self) -> u64 {
        self.0
    }

    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl Default for NetworkGeneration {
    fn default() -> Self {
        Self::new()
    }
}
