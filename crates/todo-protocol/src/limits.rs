//! Resource and size limits.

/// Maximum encoded frame size (64 KiB).
pub const MAX_FRAME_SIZE: usize = 64 * 1024;

/// Maximum encoded operation chunk (512 KiB).
pub const MAX_CHUNK_SIZE: usize = 512 * 1024;

/// Maximum operations per chunk.
pub const MAX_CHUNK_OPERATIONS: usize = 256;

/// Default in-flight chunk limit.
pub const DEFAULT_IN_FLIGHT: u32 = 32;

/// Default in-flight ciphertext budget (8 MiB).
pub const DEFAULT_CIPHERTEXT_BUDGET: usize = 8 * 1024 * 1024;
