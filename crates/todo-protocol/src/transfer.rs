//! Operation chunking: split into bounded chunks with content hashes.

use sha2::{Digest, Sha256};

use todo_domain::ids::DeviceId;

use crate::error::ProtocolError;
use crate::version_summary::SeqRange;

/// One chunk of encoded operations for a transfer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationChunk {
    pub transfer_id: [u8; 16],
    pub origin: DeviceId,
    pub range: SeqRange,
    pub ordinal: u32,
    pub count: u32,
    /// Concatenated encoded operations.
    pub operations: Vec<u8>,
    /// SHA-256 of `operations`.
    pub content_hash: [u8; 32],
}

/// Split a contiguous run of encoded operations into bounded chunks.
///
/// `ops` is `(sequence, encoded)` pairs in ascending sequence order; `start_seq`
/// is the sequence of the first operation. Chunks never exceed `max_ops`
/// operations or `max_bytes` encoded bytes.
pub fn chunk_operations(
    transfer_id: [u8; 16],
    origin: DeviceId,
    ops: &[(u64, Vec<u8>)],
    start_seq: u64,
    max_ops: usize,
    max_bytes: usize,
) -> Vec<OperationChunk> {
    let mut chunks = Vec::new();
    let mut current: Vec<u8> = Vec::new();
    let mut count = 0u32;
    let mut range_start = start_seq;
    let mut ordinal = 0u32;

    for (seq, encoded) in ops {
        let would_overflow =
            count > 0 && (count as usize >= max_ops || current.len() + encoded.len() > max_bytes);
        if would_overflow {
            let range = SeqRange::new(range_start, *seq);
            chunks.push(build_chunk(
                transfer_id,
                origin,
                range,
                ordinal,
                count,
                std::mem::take(&mut current),
            ));
            ordinal += 1;
            count = 0;
            range_start = *seq;
        }
        current.extend_from_slice(encoded);
        count += 1;
    }
    if count > 0 {
        let range = SeqRange::new(range_start, range_start + count as u64);
        chunks.push(build_chunk(
            transfer_id,
            origin,
            range,
            ordinal,
            count,
            current,
        ));
    }
    chunks
}

fn build_chunk(
    transfer_id: [u8; 16],
    origin: DeviceId,
    range: SeqRange,
    ordinal: u32,
    count: u32,
    operations: Vec<u8>,
) -> OperationChunk {
    let content_hash: [u8; 32] = Sha256::digest(&operations).into();
    OperationChunk {
        transfer_id,
        origin,
        range,
        ordinal,
        count,
        operations,
        content_hash,
    }
}

/// Verify a chunk's content hash.
pub fn verify_chunk(chunk: &OperationChunk) -> Result<(), ProtocolError> {
    let digest: [u8; 32] = Sha256::digest(&chunk.operations).into();
    if digest != chunk.content_hash {
        return Err(ProtocolError::MalformedFrame);
    }
    Ok(())
}
