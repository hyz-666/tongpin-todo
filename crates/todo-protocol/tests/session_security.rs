//! Transport encryption round-trip and tamper rejection.

use todo_protocol::{NoiseSession, frame_ciphertext, unframe};

fn established_transport() -> (todo_protocol::Transport, todo_protocol::Transport) {
    let ik = [0x11; 32];
    let rk = [0x22; 32];
    let mut initiator = NoiseSession::xx_initiator(&ik, b"p").unwrap();
    let mut responder = NoiseSession::xx_responder(&rk, b"p").unwrap();

    let mut buf = [0u8; 1024];
    let n1 = initiator.write_message(b"", &mut buf).unwrap();
    let mut buf2 = [0u8; 1024];
    let _ = responder.read_message(&buf[..n1], &mut buf2).unwrap();
    let n2 = responder.write_message(b"", &mut buf).unwrap();
    let _ = initiator.read_message(&buf[..n2], &mut buf2).unwrap();
    let n3 = initiator.write_message(b"", &mut buf).unwrap();
    let _ = responder.read_message(&buf[..n3], &mut buf2).unwrap();

    (
        initiator.into_transport().unwrap(),
        responder.into_transport().unwrap(),
    )
}

#[test]
fn encrypted_round_trip() {
    let (mut a, mut b) = established_transport();
    let plaintext = b"hello encrypted world";
    let mut ct = [0u8; 1024];
    let n = a.write(plaintext, &mut ct).unwrap();
    let mut out = [0u8; 1024];
    let m = b.read(&ct[..n], &mut out).unwrap();
    assert_eq!(&out[..m], plaintext);
}

#[test]
fn tampered_ciphertext_rejected() {
    let (mut a, mut b) = established_transport();
    let mut ct = [0u8; 1024];
    let n = a.write(b"secret", &mut ct).unwrap();
    ct[0] ^= 0xFF;
    let mut out = [0u8; 1024];
    assert!(b.read(&ct[..n], &mut out).is_err());
}

#[test]
fn truncated_ciphertext_rejected() {
    let (mut a, mut b) = established_transport();
    let mut ct = [0u8; 1024];
    let n = a.write(b"secret", &mut ct).unwrap();
    let mut out = [0u8; 1024];
    assert!(b.read(&ct[..n - 1], &mut out).is_err());
}

#[test]
fn length_framing_round_trip() {
    let (mut a, mut b) = established_transport();
    let mut ct = [0u8; 1024];
    let n = a.write(b"payload", &mut ct).unwrap();
    let mut wire = Vec::new();
    frame_ciphertext(&ct[..n], &mut wire).unwrap();
    let (ciphertext, rest) = unframe(&wire).unwrap();
    assert!(rest.is_empty());
    let mut out = [0u8; 1024];
    let m = b.read(ciphertext, &mut out).unwrap();
    assert_eq!(&out[..m], b"payload");
}

#[test]
fn truncated_frame_rejected() {
    let mut wire = Vec::new();
    frame_ciphertext(&[1, 2, 3, 4], &mut wire).unwrap();
    // Truncate the ciphertext portion.
    assert!(unframe(&wire[..wire.len() - 1]).is_err());
}
