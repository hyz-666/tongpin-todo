//! Membership DAG: genesis, add, revoke, merge, and authorization.

use todo_crypto::{DeviceIdentity, MembershipEvent, MembershipGraph, MembershipKind};

#[test]
fn genesis_has_single_active_founder() {
    let founder = DeviceIdentity::generate();
    let g = MembershipGraph::genesis(&founder);
    assert!(g.is_active(&founder.device_id()));
    assert_eq!(g.event_count(), 1);
}

#[test]
fn add_device_authorized_by_active_member() {
    let founder = DeviceIdentity::generate();
    let mut g = MembershipGraph::genesis(&founder);
    let member = DeviceIdentity::generate();

    g.add_device(
        &founder,
        *member.signing_public().as_bytes(),
        *member.noise_public().as_bytes(),
    )
    .unwrap();
    assert!(g.is_active(&member.device_id()));
}

#[test]
fn unauthorized_signer_is_rejected_on_merge() {
    let founder = DeviceIdentity::generate();
    let mut g = MembershipGraph::genesis(&founder);
    let outsider = DeviceIdentity::generate();
    let victim = DeviceIdentity::generate();

    // An outsider forges an AddDevice event.
    let forged = MembershipEvent::sign(
        &outsider,
        g.heads().to_vec(),
        MembershipKind::AddDevice,
        victim.device_id(),
        Some(*victim.signing_public().as_bytes()),
        Some(*victim.noise_public().as_bytes()),
    );
    assert!(g.merge(forged).is_err());
    assert!(!g.is_active(&victim.device_id()));
}

#[test]
fn duplicate_event_is_idempotent() {
    let founder = DeviceIdentity::generate();
    let mut g = MembershipGraph::genesis(&founder);
    let member = DeviceIdentity::generate();
    let ev = g
        .add_device(
            &founder,
            *member.signing_public().as_bytes(),
            *member.noise_public().as_bytes(),
        )
        .unwrap();
    let before = g.event_count();
    g.merge(ev).unwrap();
    assert_eq!(g.event_count(), before);
}

#[test]
fn revocation_is_remove_wins() {
    let founder = DeviceIdentity::generate();
    let mut g = MembershipGraph::genesis(&founder);
    let member = DeviceIdentity::generate();
    g.add_device(
        &founder,
        *member.signing_public().as_bytes(),
        *member.noise_public().as_bytes(),
    )
    .unwrap();
    assert!(g.is_active(&member.device_id()));

    g.revoke(&founder, member.device_id()).unwrap();
    assert!(!g.is_active(&member.device_id()));
}

#[test]
fn concurrent_events_merge_across_peers() {
    let founder = DeviceIdentity::generate();
    let mut a = MembershipGraph::genesis(&founder);
    let mut b = MembershipGraph::genesis(&founder);

    let m1 = DeviceIdentity::generate();
    let m2 = DeviceIdentity::generate();
    let ev_a = a
        .add_device(
            &founder,
            *m1.signing_public().as_bytes(),
            *m1.noise_public().as_bytes(),
        )
        .unwrap();
    let ev_b = b
        .add_device(
            &founder,
            *m2.signing_public().as_bytes(),
            *m2.noise_public().as_bytes(),
        )
        .unwrap();

    // Merge the concurrent events in either order; both members active.
    a.merge(ev_b).unwrap();
    b.merge(ev_a).unwrap();
    assert!(a.is_active(&m1.device_id()));
    assert!(a.is_active(&m2.device_id()));
    assert_eq!(a.event_count(), b.event_count());
}

#[test]
fn revoked_device_cannot_re_add_without_new_pairing() {
    let founder = DeviceIdentity::generate();
    let mut g = MembershipGraph::genesis(&founder);
    let member = DeviceIdentity::generate();
    g.add_device(
        &founder,
        *member.signing_public().as_bytes(),
        *member.noise_public().as_bytes(),
    )
    .unwrap();
    g.revoke(&founder, member.device_id()).unwrap();

    // Re-adding the same device id is rejected (must re-pair to get a new key).
    assert!(
        g.add_device(
            &founder,
            *member.signing_public().as_bytes(),
            *member.noise_public().as_bytes(),
        )
        .is_err()
    );
}
