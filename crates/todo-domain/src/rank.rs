//! Bounded fractional rank allocation with an origin tiebreaker.

use serde::{Deserialize, Serialize};

use crate::error::DomainError;
use crate::ids::DeviceId;

/// A fractional sort key. `position` orders lexicographically; `origin` breaks
/// ties deterministically.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RankKey {
    pub position: Vec<u8>,
    pub origin: DeviceId,
}

fn to_u128(position: &[u8]) -> u128 {
    position
        .iter()
        .fold(0u128, |acc, &b| (acc << 8) | b as u128)
}

fn from_u128(mut v: u128) -> Vec<u8> {
    let mut bytes = [0u8; 16];
    for byte in bytes.iter_mut().rev() {
        *byte = (v & 0xFF) as u8;
        v >>= 8;
    }
    bytes.to_vec()
}

/// The midpoint of the whole space, used for a first element.
pub fn initial(origin: DeviceId) -> RankKey {
    RankKey {
        position: from_u128(1u128 << 127),
        origin,
    }
}

/// Allocate a key strictly between `before` and `after`. Fails with
/// `RankExhausted` when no room remains, signalling a rebalance.
pub fn between(
    before: Option<&RankKey>,
    after: Option<&RankKey>,
    origin: DeviceId,
) -> Result<RankKey, DomainError> {
    let lo = before.map(|k| to_u128(&k.position)).unwrap_or(0);
    let hi = after.map(|k| to_u128(&k.position)).unwrap_or(u128::MAX);
    if hi <= lo {
        return Err(DomainError::RankExhausted);
    }
    let mid = lo + (hi - lo) / 2;
    if mid <= lo || mid >= hi {
        return Err(DomainError::RankExhausted);
    }
    Ok(RankKey {
        position: from_u128(mid),
        origin,
    })
}
