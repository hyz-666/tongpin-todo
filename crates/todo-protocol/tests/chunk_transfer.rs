//! Operation chunking and reassembly.

use todo_domain::ids::DeviceId;
use todo_protocol::{chunk_operations, verify_chunk};

fn dev() -> DeviceId {
    DeviceId::from_bytes([9u8; 32])
}

fn ops(n: u64) -> Vec<(u64, Vec<u8>)> {
    (0..n).map(|i| (i, vec![i as u8; 10])).collect()
}

#[test]
fn small_batch_is_one_chunk() {
    let chunks = chunk_operations([0; 16], dev(), &ops(5), 0, 256, 512 * 1024);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].count, 5);
    assert_eq!(chunks[0].ordinal, 0);
    assert!(verify_chunk(&chunks[0]).is_ok());
}

#[test]
fn large_batch_splits_at_operation_boundary() {
    // 600 operations with max 256 per chunk -> 3 chunks.
    let chunks = chunk_operations([0; 16], dev(), &ops(600), 0, 256, 512 * 1024);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].count, 256);
    assert_eq!(chunks[1].count, 256);
    assert_eq!(chunks[2].count, 88);
}

#[test]
fn byte_limit_forces_split() {
    // Each op is 10 bytes; cap at 25 bytes -> every 2 ops form a chunk.
    let chunks = chunk_operations([0; 16], dev(), &ops(6), 0, 256, 25);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].count, 2);
}

#[test]
fn chunk_ranges_are_contiguous() {
    let chunks = chunk_operations([0; 16], dev(), &ops(10), 0, 256, 25);
    let mut seq = 0u64;
    for c in &chunks {
        assert_eq!(c.range.start, seq);
        seq = c.range.end;
    }
    assert_eq!(seq, 10);
}

#[test]
fn tampered_content_hash_rejected() {
    let mut chunks = chunk_operations([0; 16], dev(), &ops(3), 0, 256, 512 * 1024);
    chunks[0].operations[0] ^= 0xFF;
    assert!(verify_chunk(&chunks[0]).is_err());
}
