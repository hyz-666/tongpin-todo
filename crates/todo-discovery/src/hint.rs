//! Rotating private discovery hints.

use hmac::{Hmac, Mac};
use sha2::Sha256;

use todo_domain::ids::DeviceId;

type HmacSha256 = Hmac<Sha256>;

/// Domain separator for discovery-hint derivation.
pub const DISCOVERY_DOMAIN: &[u8] = b"tptodo.discovery.v1";

/// A hint is the first 16 bytes of an HMAC-SHA256 over the discovery secret
/// and the protocol/window/device tuple.
pub type Hint = [u8; 16];

/// Compute the discovery hint for a device in a given window.
pub fn derive_hint(
    discovery_secret: &[u8],
    protocol_major: u16,
    window: u64,
    device_id: &DeviceId,
) -> Hint {
    let mut mac = HmacSha256::new_from_slice(discovery_secret).expect("HMAC accepts any key");
    mac.update(DISCOVERY_DOMAIN);
    mac.update(&protocol_major.to_be_bytes());
    mac.update(&window.to_be_bytes());
    mac.update(device_id.as_bytes());
    let digest = mac.finalize().into_bytes();
    let mut out = [0u8; 16];
    out.copy_from_slice(&digest[..16]);
    out
}

/// Compute all hints a trusted replica expects for a known member across the
/// previous, current, and next windows.
pub fn expected_hints(
    discovery_secret: &[u8],
    protocol_major: u16,
    window: u64,
    device_id: &DeviceId,
) -> [Hint; 3] {
    let prev = window.saturating_sub(1);
    [
        derive_hint(discovery_secret, protocol_major, prev, device_id),
        derive_hint(discovery_secret, protocol_major, window, device_id),
        derive_hint(discovery_secret, protocol_major, window + 1, device_id),
    ]
}

/// Constant-time hint comparison.
pub fn hint_eq(a: &Hint, b: &Hint) -> bool {
    let mut acc = 0u8;
    for i in 0..16 {
        acc |= a[i] ^ b[i];
    }
    acc == 0
}
