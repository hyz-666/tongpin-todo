//! Six-digit Short Authentication String (SAS) derivation.

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Domain separator for SAS derivation.
pub const SAS_DOMAIN: &[u8] = b"tptodo.sas.v1";

/// Derive a six-digit SAS from the authenticated Noise transcript hash,
/// using an unbiased reduction modulo 1,000,000.
pub fn derive_sas(transcript_hash: &[u8; 32]) -> String {
    let mut mac = HmacSha256::new_from_slice(SAS_DOMAIN).expect("HMAC accepts any key length");
    mac.update(transcript_hash);
    let digest = mac.finalize().into_bytes();

    // Rejection sampling over 4-byte windows to avoid modulo bias.
    let limit = u32::MAX - (u32::MAX % 1_000_000);
    let bytes = digest.as_slice();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        let v = u32::from_be_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
        if v < limit {
            return format!("{:06}", v % 1_000_000);
        }
        i += 4;
    }
    // Fallback (astronomically unlikely given 8 windows).
    let v = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]);
    format!("{:06}", v % 1_000_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sas_is_six_digits() {
        let h = [0xAB; 32];
        let sas = derive_sas(&h);
        assert_eq!(sas.len(), 6);
        assert!(sas.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn sas_is_deterministic() {
        let h = [0x11; 32];
        assert_eq!(derive_sas(&h), derive_sas(&h));
    }

    #[test]
    fn different_transcripts_differ() {
        let a = [0x01; 32];
        let b = [0x02; 32];
        assert_ne!(derive_sas(&a), derive_sas(&b));
    }
}
