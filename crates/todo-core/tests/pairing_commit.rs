//! A new peer is only trusted after the pairing commit.

use todo_core::MembershipStore;
use todo_crypto::DeviceIdentity;

#[test]
fn new_peer_not_trusted_before_commit() {
    let store = MembershipStore::new();
    let founder = DeviceIdentity::generate();
    store.genesis(&founder);

    let peer = DeviceIdentity::generate();
    // Mid-pairing: authenticated but not yet committed.
    store.mark_pending(peer.device_id());
    assert!(!store.is_trusted(&peer.device_id()));

    // Commit grants trust.
    store
        .commit_member(
            &founder,
            *peer.signing_public().as_bytes(),
            *peer.noise_public().as_bytes(),
        )
        .unwrap();
    assert!(store.is_trusted(&peer.device_id()));
}

#[test]
fn interrupted_commit_does_not_grant_trust() {
    let store = MembershipStore::new();
    let founder = DeviceIdentity::generate();
    store.genesis(&founder);

    let peer = DeviceIdentity::generate();
    // No commit call happens; trust must remain absent.
    assert!(!store.is_trusted(&peer.device_id()));
    assert_eq!(store.member_count(), 1);
}

#[test]
fn duplicate_commit_is_rejected() {
    let store = MembershipStore::new();
    let founder = DeviceIdentity::generate();
    store.genesis(&founder);
    let peer = DeviceIdentity::generate();
    store
        .commit_member(
            &founder,
            *peer.signing_public().as_bytes(),
            *peer.noise_public().as_bytes(),
        )
        .unwrap();
    // Second commit for the same key fails (already a member).
    assert!(
        store
            .commit_member(
                &founder,
                *peer.signing_public().as_bytes(),
                *peer.noise_public().as_bytes(),
            )
            .is_err()
    );
}
