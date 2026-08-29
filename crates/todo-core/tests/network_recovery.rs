//! Network-change recovery: generation bumps and rediscovery jitter.

use todo_core::NetworkRuntime;

#[test]
fn network_change_bumps_generation() {
    let mut rt = NetworkRuntime::new();
    let g0 = rt.generation();
    rt.on_network_change();
    let g1 = rt.generation();
    assert_ne!(g0, g1);
    assert_eq!(g1.value(), g0.value() + 1);
}

#[test]
fn generation_is_monotonic_across_many_changes() {
    let mut rt = NetworkRuntime::new();
    let mut prev = rt.generation().value();
    for _ in 0..10 {
        rt.on_network_change();
        let cur = rt.generation().value();
        assert_eq!(cur, prev + 1);
        prev = cur;
    }
}

#[test]
fn rediscovery_delay_is_bounded_0_to_2s() {
    let rt = NetworkRuntime::new();
    assert_eq!(rt.rediscovery_delay(0.0), 0);
    assert_eq!(rt.rediscovery_delay(1.0), 2_000);
    assert!(rt.rediscovery_delay(0.5) <= 2_000);
}

#[test]
fn rediscovery_jitter_clamps() {
    let rt = NetworkRuntime::new();
    // Out-of-range jitter clamps to the valid window.
    assert_eq!(rt.rediscovery_delay(-1.0), 0);
    assert_eq!(rt.rediscovery_delay(2.0), 2_000);
}
