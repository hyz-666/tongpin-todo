//! Group genesis: group id and discovery secret.

use rand_core::RngCore;
use uuid::Uuid;

use crate::secret::SecretBytes;

/// A group identity created at genesis.
pub struct GroupIdentity {
    pub group_id: [u8; 16],
    pub discovery_secret: SecretBytes,
}

impl GroupIdentity {
    /// Generate a fresh group id and discovery secret.
    pub fn generate() -> Self {
        let group_id = Uuid::new_v4();
        let mut secret = [0u8; 32];
        rand_core::OsRng.fill_bytes(&mut secret);
        Self {
            group_id: *group_id.as_bytes(),
            discovery_secret: SecretBytes::from_array(secret),
        }
    }
}
