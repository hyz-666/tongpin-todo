//! Noise XX and IK handshake authentication.

use todo_protocol::{NoiseRole, NoiseSession};

fn run_xx(
    initiator_key: &[u8; 32],
    responder_key: &[u8; 32],
    prologue: &[u8],
) -> (NoiseSession, NoiseSession) {
    let mut initiator = NoiseSession::xx_initiator(initiator_key, prologue).unwrap();
    let mut responder = NoiseSession::xx_responder(responder_key, prologue).unwrap();

    let mut buf = [0u8; 1024];
    // 1: initiator -> responder
    let n1 = initiator.write_message(b"", &mut buf).unwrap();
    let mut buf2 = [0u8; 1024];
    let _ = responder.read_message(&buf[..n1], &mut buf2).unwrap();
    // 2: responder -> initiator
    let n2 = responder.write_message(b"", &mut buf).unwrap();
    let _ = initiator.read_message(&buf[..n2], &mut buf2).unwrap();
    // 3: initiator -> responder
    let n3 = initiator.write_message(b"", &mut buf).unwrap();
    let _ = responder.read_message(&buf[..n3], &mut buf2).unwrap();

    assert!(initiator.is_finished());
    assert!(responder.is_finished());
    (initiator, responder)
}

#[test]
fn xx_transcripts_match_and_bind_keys() {
    let ik = [0x11; 32];
    let rk = [0x22; 32];
    let (initiator, responder) = run_xx(&ik, &rk, b"prologue");

    assert_eq!(initiator.handshake_hash(), responder.handshake_hash());
    // Remote static is the peer's static public key.
    assert_eq!(initiator.remote_static(), Some(rk_to_pub(&rk)));
    assert_eq!(responder.remote_static(), Some(rk_to_pub(&ik)));
}

#[test]
fn xx_roles_are_distinct() {
    let (initiator, responder) = run_xx(&[0x11; 32], &[0x22; 32], b"p");
    assert_eq!(initiator.role(), NoiseRole::Initiator);
    assert_eq!(responder.role(), NoiseRole::Responder);
}

#[test]
fn changed_prologue_produces_different_transcript() {
    // Same keys but different prologues yield different authenticated transcripts.
    let ik = [0x11; 32];
    let rk = [0x22; 32];
    let (i1, r1) = run_xx(&ik, &rk, b"prologue-A");
    let (i2, r2) = run_xx(&ik, &rk, b"prologue-B");
    assert_ne!(i1.handshake_hash(), i2.handshake_hash());
    assert_ne!(r1.handshake_hash(), r2.handshake_hash());
}

#[test]
fn ik_handshake_binds_pinned_key() {
    let ik = [0x11; 32];
    let rk = [0x22; 32];
    // IK initiator pins the responder's static public key.
    let mut initiator = NoiseSession::ik_initiator(&ik, &rk_to_pub(&rk), b"ik").unwrap();
    let mut responder = NoiseSession::ik_responder(&rk, b"ik").unwrap();

    let mut buf = [0u8; 1024];
    let n1 = initiator.write_message(b"", &mut buf).unwrap();
    let mut buf2 = [0u8; 1024];
    let _ = responder.read_message(&buf[..n1], &mut buf2).unwrap();
    let n2 = responder.write_message(b"", &mut buf).unwrap();
    let _ = initiator.read_message(&buf[..n2], &mut buf2).unwrap();

    assert!(initiator.is_finished());
    assert!(responder.is_finished());
    assert_eq!(initiator.handshake_hash(), responder.handshake_hash());
}

#[test]
fn ik_with_wrong_pinned_key_fails() {
    let ik = [0x11; 32];
    let rk = [0x22; 32];
    let wrong = [0x99; 32];
    let mut initiator = NoiseSession::ik_initiator(&ik, &wrong, b"ik").unwrap();
    let mut responder = NoiseSession::ik_responder(&rk, b"ik").unwrap();

    let mut buf = [0u8; 1024];
    let n1 = initiator.write_message(b"", &mut buf).unwrap();
    let mut buf2 = [0u8; 1024];
    // The responder rejects the initiator whose pinned key does not match.
    assert!(responder.read_message(&buf[..n1], &mut buf2).is_err());
}

/// Convert an X25519 secret key to its public key (for test fixtures).
fn rk_to_pub(secret: &[u8; 32]) -> [u8; 32] {
    let s = x25519_dalek::StaticSecret::from(*secret);
    let p = x25519_dalek::PublicKey::from(&s);
    *p.as_bytes()
}
