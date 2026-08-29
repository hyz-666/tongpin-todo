//! Deterministic fault matrix: loss, duplication, reordering, truncation, corruption.

use todo_testkit::{Fault, FaultNetwork};

fn frames(n: usize) -> Vec<Vec<u8>> {
    (0..n).map(|i| vec![i as u8; 8]).collect()
}

#[test]
fn loss_reduces_delivery() {
    let mut net = FaultNetwork::new(1);
    for f in frames(6) {
        net.send(f);
    }
    net.apply(Fault::Drop);
    // Every other frame is dropped, so only half arrive.
    let mut arrived = 0;
    while net.deliver().is_some() {
        arrived += 1;
    }
    assert_eq!(arrived, 3);
}

#[test]
fn duplication_doubles_frames() {
    let mut net = FaultNetwork::new(2);
    for f in frames(4) {
        net.send(f);
    }
    net.apply(Fault::Duplicate);
    let mut arrived = 0;
    while net.deliver().is_some() {
        arrived += 1;
    }
    // Half duplicated -> 2 * 2 + 2 normal = 6.
    assert_eq!(arrived, 6);
}

#[test]
fn truncation_shortens_frames() {
    let mut net = FaultNetwork::new(3);
    net.send(vec![1u8; 8]);
    net.apply(Fault::Truncate);
    let delivered = net.deliver().unwrap();
    assert_eq!(delivered.len(), 4);
}

#[test]
fn corruption_flips_a_byte() {
    let mut net = FaultNetwork::new(4);
    net.send(vec![0u8; 8]);
    net.apply(Fault::Corrupt);
    let delivered = net.deliver().unwrap();
    assert!(delivered.iter().any(|b| *b != 0));
}

#[test]
fn reordering_changes_delivery_order() {
    let mut net = FaultNetwork::new(5);
    net.send(vec![1u8; 4]);
    net.send(vec![2u8; 4]);
    net.send(vec![3u8; 4]);
    net.apply(Fault::Reorder);
    let delivered: Vec<Vec<u8>> = std::iter::from_fn(|| net.deliver()).collect();
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
    for f in frames(3) {
        net.send(f);
    }
    net.apply(Fault::Pass);
    let mut arrived = 0;
    while let Some(f) = net.deliver() {
        assert_eq!(f.len(), 8);
        arrived += 1;
    }
    assert_eq!(arrived, 3);
}
