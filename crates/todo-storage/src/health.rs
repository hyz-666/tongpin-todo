//! Space provisioning and reserve checks.

/// Reports how many bytes are available on the backing volume.
pub trait SpaceProvider: Send + Sync {
    fn available_bytes(&self) -> u64;
}

/// A provider that always reports unlimited space (the default).
pub struct UnlimitedSpace;

impl SpaceProvider for UnlimitedSpace {
    fn available_bytes(&self) -> u64 {
        u64::MAX
    }
}
