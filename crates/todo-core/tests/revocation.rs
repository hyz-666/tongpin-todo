//! Revocation: trust removal, rekey, and post-cutoff rejection.

use todo_core::MembershipStore;
use todo_crypto::{DeviceIdentity, create_rekey_envelopes, is_post_cutoff, open_rekey_envelope};

#[test]
fn revocation_stops_trust_immediately() {
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
    assert!(store.is_trusted(&peer.device_id()));

    store.revoke(&founder, peer.device_id()).unwrap();
    assert!(!store.is_trusted(&peer.device_id()));
}

#[test]
fn revoked_device_cannot_recommit_same_key() {
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
    store.revoke(&founder, peer.device_id()).unwrap();

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

#[test]
fn rekey_round_trips_for_remaining_members() {
    let founder = DeviceIdentity::generate();
    let member = DeviceIdentity::generate();
    let new_secret = [0xAB; 32];
    let envelopes = create_rekey_envelopes(
        &founder,
        &new_secret,
        &[(member.device_id(), *member.noise_public().as_bytes())],
    );
    assert_eq!(envelopes.len(), 1);

    let opened = open_rekey_envelope(
        &member,
        founder.noise_public().as_bytes(),
        &envelopes[0].nonce,
        &envelopes[0].ciphertext,
    )
    .unwrap();
    assert_eq!(opened.as_slice(), new_secret.as_slice());
}

#[test]
fn post_cutoff_operations_are_rejected() {
    assert!(!is_post_cutoff(10, 10), "exactly at cutoff is retained");
    assert!(is_post_cutoff(11, 10), "beyond cutoff is rejected");
    assert!(!is_post_cutoff(9, 10), "before cutoff is retained");
}
