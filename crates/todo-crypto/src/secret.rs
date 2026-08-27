//! Zeroizing secret-key wrappers.

use zeroize::Zeroize;

/// A secret byte buffer that zeroizes on drop.
pub struct SecretBytes {
    inner: Vec<u8>,
}

impl SecretBytes {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { inner: bytes }
    }

    pub fn from_array<const N: usize>(bytes: [u8; N]) -> Self {
        Self {
            inner: bytes.to_vec(),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.inner
    }

    pub fn to_array<const N: usize>(&self) -> Option<[u8; N]> {
        self.inner.clone().try_into().ok()
    }
}

impl Drop for SecretBytes {
    fn drop(&mut self) {
        self.inner.zeroize();
    }
}
