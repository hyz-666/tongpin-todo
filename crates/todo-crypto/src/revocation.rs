//! Revocation: remove-wins, causal cutoff, and discovery-secret rekey.

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand_core::RngCore;
use sha2::{Digest, Sha256};

use todo_domain::ids::DeviceId;

use crate::error::CryptoError;
use crate::identity::DeviceIdentity;
use crate::secret::SecretBytes;

/// A per-recipient rekey envelope: (device, nonce, ciphertext).
pub struct RekeyEnvelope {
    pub device: DeviceId,
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

/// Encrypt a freshly rotated discovery secret to each remaining member using
/// an X25519 shared secret expanded via HKDF-style hashing.
pub fn create_rekey_envelopes(
    self_identity: &DeviceIdentity,
    new_secret: &[u8; 32],
    recipients: &[(DeviceId, [u8; 32])],
) -> Vec<RekeyEnvelope> {
    recipients
        .iter()
        .filter_map(|(device, noise_public)| {
            let shared = self_identity.shared_secret(noise_public);
            let key = expand_key(&shared);
            let mut nonce = [0u8; 12];
            rand_core::OsRng.fill_bytes(&mut nonce);
            let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
            let ciphertext = cipher
                .encrypt(Nonce::from_slice(&nonce), new_secret.as_slice())
                .ok()?;
            Some(RekeyEnvelope {
                device: *device,
                nonce,
                ciphertext,
            })
        })
        .collect()
}

/// Decrypt a rekey envelope addressed to this device.
pub fn open_rekey_envelope(
    self_identity: &DeviceIdentity,
    sender_noise_public: &[u8; 32],
    nonce: &[u8; 12],
    ciphertext: &[u8],
) -> Result<SecretBytes, CryptoError> {
    let shared = self_identity.shared_secret(sender_noise_public);
    let key = expand_key(&shared);
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| CryptoError::InvalidSignature)?;
    Ok(SecretBytes::new(plaintext))
}

fn expand_key(shared: &[u8; 32]) -> [u8; 32] {
    let digest = Sha256::digest(shared);
    digest.into()
}

/// Determine whether an operation sequence is at or after the revocation
/// causal cutoff (post-cutoff effects must be rejected or rebuilt).
pub fn is_post_cutoff(operation_sequence: u64, cutoff: u64) -> bool {
    operation_sequence > cutoff
}
