//! Operation signing and verification with a domain separator.

use ed25519_dalek::{Signature, VerifyingKey};

use todo_domain::operation::VerifiedOperation;

use crate::canonical::encode_operation;
use crate::error::{CryptoError, VerifyReason};
use crate::identity::DeviceIdentity;

/// Domain separator for operation signatures.
pub const DOMAIN_SEPARATOR: &[u8] = b"tptodo.operation.v1";

fn signable_message(op: &VerifiedOperation) -> Result<Vec<u8>, CryptoError> {
    let canonical = encode_operation(op)?;
    let mut msg = Vec::with_capacity(DOMAIN_SEPARATOR.len() + canonical.len());
    msg.extend_from_slice(DOMAIN_SEPARATOR);
    msg.extend_from_slice(&canonical);
    Ok(msg)
}

/// Sign a verified operation with the domain separator prepended.
pub fn sign_operation(
    identity: &DeviceIdentity,
    op: &VerifiedOperation,
) -> Result<Signature, CryptoError> {
    let msg = signable_message(op)?;
    Ok(identity.sign(&msg))
}

/// Verify an operation signature, returning a typed reason before any storage
/// access happens.
pub fn verify_operation(
    public: &VerifyingKey,
    op: &VerifiedOperation,
    signature: &Signature,
) -> Result<(), VerifyReason> {
    let msg = signable_message(op).map_err(|_| VerifyReason::Malformed)?;
    public
        .verify_strict(&msg, signature)
        .map_err(|_| VerifyReason::BadSignature)
}
