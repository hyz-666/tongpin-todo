//! Deterministic fault matrix: loss, duplication, reordering, truncation, corruption.

use todo_testkit::{Fault, FaultNetwork};

fn frames(n: usize) -> Vec<Vec<u8>> {
    (0..n).map(|i| vec![i as u8; 8]).collect()
}

#[test]
fn loss_reduces_delivery() {
    let mut net = FaultNetwork::new(1);
    net.set_policy(0, Fault::Drop);
    for f in frames(6) {
        net.send(0, f);
    }
    // Every other frame is dropped (seed-driven coin flip), so roughly half arrive.
    let delivered = net.deliver_to(0);
    assert!(
        delivered.len() < 6,
        "drop must remove at least one frame, got {}",
        delivered.len()
    );
}

#[test]
fn duplication_doubles_frames() {
    let mut net = FaultNetwork::new(2);
    net.set_policy(0, Fault::Duplicate);
    for f in frames(4) {
        net.send(0, f);
    }
    let delivered = net.deliver_to(0);
    // Half duplicated -> at least 4, at most 8.
    assert!(
        delivered.len() >= 4,
        "duplication must increase delivery count, got {}",
        delivered.len()
    );
}

#[test]
fn truncation_shortens_frames() {
    let mut net = FaultNetwork::new(3);
    net.set_policy(0, Fault::Truncate);
    net.send(0, vec![1u8; 8]);
    let delivered = net.deliver_to(0);
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0].len(), 4, "truncation delivers half the bytes");
}

#[test]
fn corruption_flips_a_byte() {
    let mut net = FaultNetwork::new(4);
    net.set_policy(0, Fault::Corrupt);
    net.send(0, vec![0u8; 8]);
    let delivered = net.deliver_to(0);
    assert_eq!(delivered.len(), 1);
    assert!(
        delivered[0].iter().any(|b| *b != 0),
        "corruption must flip at least one byte"
    );
}

#[test]
fn reordering_changes_delivery_order() {
    let mut net = FaultNetwork::new(5);
    net.set_policy(0, Fault::Reorder);
    net.send(0, vec![1u8; 4]);
    net.send(0, vec![2u8; 4]);
    net.send(0, vec![3u8; 4]);
    let delivered = net.deliver_to(0);
    assert_eq!(delivered.len(), 3);
    let order: Vec<u8> = delivered.iter().map(|f| f[0]).collect();
    assert_ne!(
        order,
        vec![1, 2, 3],
        "reordering must change delivery order"
    );
    // Every frame still arrives exactly once.
    let mut sorted = order.clone();
    sorted.sort();
    assert_eq!(sorted, vec![1, 2, 3]);
}

#[test]
fn pass_delivers_everything_unchanged() {
    let mut net = FaultNetwork::new(6);
    net.set_policy(0, Fault::Pass);
    for f in frames(3) {
        net.send(0, f);
    }
    let delivered = net.deliver_to(0);
    assert_eq!(delivered.len(), 3);
    for f in &delivered {
        assert_eq!(f.len(), 8);
    }
}

#[test]
fn pause_and_resume_delivers_queued_frames() {
    let mut net = FaultNetwork::new(7);
    net.set_policy(0, Fault::Pass);
    net.send(0, vec![1u8; 4]);
    net.send(0, vec![2u8; 4]);
    net.pause(0);

    // Nothing arrives while paused.
    assert!(net.deliver_to(0).is_empty());
    assert_eq!(net.pending(0), 2);

    // Resume and force-deliver.
    net.resume(0);
    let delivered = net.force_deliver(0);
    assert_eq!(delivered.len(), 2);
}

#[test]
fn delay_holds_frames_until_force_deliver() {
    let mut net = FaultNetwork::new(8);
    net.set_policy(0, Fault::Delay);
    net.send(0, vec![42u8; 4]);
    assert!(
        net.deliver_to(0).is_empty(),
        "delayed frames are not delivered"
    );
    let delivered = net.force_deliver(0);
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0][0], 42);
}
