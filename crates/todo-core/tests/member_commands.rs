//! Member management commands: list, status, commit, revoke.

use todo_core::{MemberCommands, MemberStatus, MembershipStore};
use todo_crypto::DeviceIdentity;
use todo_domain::ids::DeviceId;

#[test]
fn genesis_lists_founder_as_active() {
    let store = MembershipStore::new();
    let founder = DeviceIdentity::generate();
    store.genesis(&founder);

    let cmds = MemberCommands::new(&store);
    let members = cmds.list_members();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].device, founder.device_id());
    assert_eq!(members[0].status, MemberStatus::Active);
}

#[test]
fn pending_member_surfaces_as_pending() {
    let store = MembershipStore::new();
    let founder = DeviceIdentity::generate();
    store.genesis(&founder);

    let new_device = DeviceIdentity::generate();
    store.mark_pending(new_device.device_id());

    let cmds = MemberCommands::new(&store);
    assert_eq!(
        cmds.member_status(&new_device.device_id()),
        MemberStatus::Pending
    );
    let members = cmds.list_members();
    assert!(
        members
            .iter()
            .any(|m| { m.device == new_device.device_id() && m.status == MemberStatus::Pending })
    );
}

#[test]
fn commit_moves_pending_to_active() {
    let store = MembershipStore::new();
    let founder = DeviceIdentity::generate();
    store.genesis(&founder);

    let new_device = DeviceIdentity::generate();
    store.mark_pending(new_device.device_id());

    let cmds = MemberCommands::new(&store);
    cmds.commit_member(
        &founder,
        *new_device.signing_public().as_bytes(),
        *new_device.noise_public().as_bytes(),
    )
    .unwrap();

    assert_eq!(
        cmds.member_status(&new_device.device_id()),
        MemberStatus::Active
    );
    assert_eq!(cmds.list_members().len(), 2);
}

#[test]
fn revoke_moves_active_to_revoked() {
    let store = MembershipStore::new();
    let founder = DeviceIdentity::generate();
    store.genesis(&founder);

    let new_device = DeviceIdentity::generate();
    store
        .commit_member(
            &founder,
            *new_device.signing_public().as_bytes(),
            *new_device.noise_public().as_bytes(),
        )
        .unwrap();

    let cmds = MemberCommands::new(&store);
    cmds.revoke_device(&founder, new_device.device_id())
        .unwrap();

    assert_eq!(
        cmds.member_status(&new_device.device_id()),
        MemberStatus::Revoked
    );
    assert!(!store.is_trusted(&new_device.device_id()));
}

#[test]
fn unknown_device_is_unknown() {
    let store = MembershipStore::new();
    let founder = DeviceIdentity::generate();
    store.genesis(&founder);

    let cmds = MemberCommands::new(&store);
    let unknown = DeviceId::from_bytes([0xEE; 32]);
    assert_eq!(cmds.member_status(&unknown), MemberStatus::Unknown);
}
