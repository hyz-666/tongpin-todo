//! Device identity: separate Ed25519 signing and X25519 Noise static keys.

use std::fmt;

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey, StaticSecret};

use todo_domain::ids::DeviceId;

use crate::error::CryptoError;

/// A device identity with separate signing and Noise static keys.
///
/// The device id is the SHA-256 of the exact canonical Ed25519 public key bytes,
/// so it is stable, collision-resistant, and derives from a public value only.
pub struct DeviceIdentity {
    signing: SigningKey,
    noise_static: StaticSecret,
    signing_public: VerifyingKey,
    noise_public: PublicKey,
    device_id: DeviceId,
}

/// The public material bound together in a membership event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityBinding {
    pub device_id: DeviceId,
    pub signing_public: [u8; 32],
    pub noise_static_public: [u8; 32],
}

impl DeviceIdentity {
    pub fn generate() -> Self {
        let mut rng = rand_core::OsRng;
        let signing = SigningKey::generate(&mut rng);
        let noise_static = StaticSecret::random_from_rng(rng);
        let signing_public = signing.verifying_key();
        let noise_public = PublicKey::from(&noise_static);
        let device_id = device_id_from_public(&signing_public);
        Self {
            signing,
            noise_static,
            signing_public,
            noise_public,
            device_id,
        }
    }

    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }

    pub fn signing_public(&self) -> &VerifyingKey {
        &self.signing_public
    }

    pub fn noise_public(&self) -> &PublicKey {
        &self.noise_public
    }

    /// Public-only binding for membership inclusion.
    pub fn binding(&self) -> IdentityBinding {
        IdentityBinding {
            device_id: self.device_id,
            signing_public: *self.signing_public.as_bytes(),
            noise_static_public: *self.noise_public.as_bytes(),
        }
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        self.signing.sign(message)
    }

    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), CryptoError> {
        self.signing_public
            .verify_strict(message, signature)
            .map_err(|_| CryptoError::InvalidSignature)
    }

    /// Compute the X25519 shared secret with a peer's static public key.
    pub fn shared_secret(&self, peer_noise_public: &[u8; 32]) -> [u8; 32] {
        let peer = PublicKey::from(*peer_noise_public);
        let shared = self.noise_static.diffie_hellman(&peer);
        *shared.as_bytes()
    }
}

/// Derive the stable device id from the canonical Ed25519 public key bytes.
pub fn device_id_from_public(public: &VerifyingKey) -> DeviceId {
    let digest = Sha256::digest(public.as_bytes());
    DeviceId::from_bytes(digest.into())
}

impl fmt::Debug for DeviceIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceIdentity")
            .field("device_id", &self.device_id)
            .finish_non_exhaustive()
    }
}
