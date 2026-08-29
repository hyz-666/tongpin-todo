//! Backoff: ranges, jitter bounds, caps, and resets.

use todo_core::{AttemptCounter, chunk_retry_delay, dial_delay};

#[test]
fn chunk_retry_follows_sequence() {
    let base = [1_000, 2_000, 4_000, 8_000, 16_000];
    for (i, expected) in base.iter().enumerate() {
        // jitter = 0 means exact base value.
        assert_eq!(chunk_retry_delay(i as u32, 0.0), *expected);
    }
}

#[test]
fn dial_backoff_follows_sequence() {
    let base = [
        1_000, 2_000, 5_000, 10_000, 30_000, 60_000, 120_000, 300_000,
    ];
    for (i, expected) in base.iter().enumerate() {
        assert_eq!(dial_delay(i as u32, 0.0), *expected);
    }
}

#[test]
fn full_jitter_bounds() {
    // jitter = 1.0 collapses the delay to 0.
    assert_eq!(chunk_retry_delay(0, 1.0), 0);
    // jitter = 0.5 yields half the base.
    assert_eq!(chunk_retry_delay(0, 0.5), 500);
}

#[test]
fn five_chunk_attempts_then_cap() {
    // Attempts 0..4 map to 1/2/4/8/16s; attempt 5 and beyond stay capped at 16s.
    assert_eq!(chunk_retry_delay(4, 0.0), 16_000);
    assert_eq!(chunk_retry_delay(5, 0.0), 16_000);
    assert_eq!(chunk_retry_delay(100, 0.0), 16_000);
}

#[test]
fn dial_caps_at_five_minutes() {
    assert_eq!(dial_delay(7, 0.0), 300_000);
    assert_eq!(dial_delay(100, 0.0), 300_000);
}

#[test]
fn counter_resets_only_on_auth() {
    let mut c = AttemptCounter::new();
    let d1 = c.record_failure(0.0);
    let d2 = c.record_failure(0.0);
    assert!(d2 > d1, "backoff grows with failures");
    assert_eq!(c.attempts, 2);

    c.reset_on_auth();
    assert_eq!(c.attempts, 0);
    assert!(c.authenticated);
    // After reset, the next failure starts from the base delay again.
    assert_eq!(c.record_failure(0.0), 1_000);
}

#[test]
fn no_overflow_on_extreme_attempts() {
    // A huge attempt count must not overflow; it just clamps.
    let _ = dial_delay(u32::MAX, 0.5);
    let _ = chunk_retry_delay(u32::MAX, 0.5);
    let mut c = AttemptCounter::new();
    c.attempts = u32::MAX;
    let _ = c.record_failure(0.5); // saturating add, no panic
}
