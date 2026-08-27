//! Length framing around Noise ciphertext.

use crate::error::ProtocolError;
use crate::limits::MAX_FRAME_SIZE;

/// Write a 2-byte big-endian length prefix followed by the ciphertext.
pub fn frame_ciphertext(ciphertext: &[u8], out: &mut Vec<u8>) -> Result<(), ProtocolError> {
    let len = ciphertext.len();
    if len > u16::MAX as usize {
        return Err(ProtocolError::FrameTooLarge);
    }
    if len + 2 > MAX_FRAME_SIZE {
        return Err(ProtocolError::FrameTooLarge);
    }
    out.extend_from_slice(&(len as u16).to_be_bytes());
    out.extend_from_slice(ciphertext);
    Ok(())
}

/// Parse a length-prefixed ciphertext from a buffer, returning (ciphertext, rest).
pub fn unframe(buf: &[u8]) -> Result<(&[u8], &[u8]), ProtocolError> {
    if buf.len() < 2 {
        return Err(ProtocolError::MalformedFrame);
    }
    let len = u16::from_be_bytes([buf[0], buf[1]]) as usize;
    if buf.len() < 2 + len {
        return Err(ProtocolError::MalformedFrame);
    }
    Ok((&buf[2..2 + len], &buf[2 + len..]))
}
