//! Fractional rank allocation and tag canonicalization.

use todo_domain::clock::Hlc;
use todo_domain::ids::{DeviceId, EntityId, LifecycleGeneration, OperationId};
use todo_domain::operation::{
    EntityKind, OperationPayload, ReplicaProjection, VerifiedOperation, apply_operation,
};
use todo_domain::rank::{RankKey, between, initial};
use todo_domain::register::VersionStamp;

fn origin(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 32])
}

fn stamp(device: u8, seq: u64, millis: i64) -> VersionStamp {
    VersionStamp {
        generation: LifecycleGeneration(1),
        hlc: Hlc::new(millis, 0),
        device: DeviceId::from_bytes([device; 32]),
        operation: OperationId::new(DeviceId::from_bytes([device; 32]), seq),
    }
}

fn set_tag_name(id: EntityId, name: &str, stamp: VersionStamp) -> VerifiedOperation {
    VerifiedOperation {
        entity: id,
        kind: EntityKind::Tag,
        parent: None,
        stamp,
        payload: OperationPayload::SetField {
            field: "name".to_string(),
            value: serde_json::json!(name),
        },
    }
}

#[test]
fn initial_position_is_deterministic() {
    let a = initial(origin(1));
    let b = initial(origin(2));
    assert_eq!(a.position, b.position, "initial position is deterministic");
    assert_ne!(a, b, "origin still distinguishes keys");
}

#[test]
fn between_produces_ordered_keys() {
    let lo = initial(origin(1));
    let hi = between(None, Some(&lo), origin(1)).unwrap();
    let mid = between(Some(&hi), Some(&lo), origin(1)).unwrap();
    assert!(hi < lo, "before-first is less than the anchor");
    assert!(hi < mid && mid < lo, "mid falls strictly between");
}

#[test]
fn concurrent_inserts_share_position_but_tiebreak_by_origin() {
    let lo = initial(origin(1));
    let hi = between(None, Some(&lo), origin(1)).unwrap();
    let a = between(Some(&hi), Some(&lo), origin(1)).unwrap();
    let b = between(Some(&hi), Some(&lo), origin(2)).unwrap();
    assert_eq!(a.position, b.position, "same gap yields same position");
    assert_ne!(a, b, "origin disambiguates the collision");
    assert!(a < b, "origin 1 sorts before origin 2");
}

#[test]
fn rank_exhaustion_is_reported() {
    let a = RankKey {
        position: vec![0x00],
        origin: origin(1),
    };
    let b = RankKey {
        position: vec![0x01],
        origin: origin(1),
    };
    assert!(between(Some(&a), Some(&b), origin(1)).is_err());
}

#[test]
fn concurrent_same_name_tags_resolve_to_one_canonical() {
    let mut s = ReplicaProjection::default();
    let t1 = EntityId::new_v7_for_test(100);
    let t2 = EntityId::new_v7_for_test(200);
    // Same normalized name ("important"), t1 created earlier (lower HLC).
    apply_operation(&mut s, &set_tag_name(t1, "IMPORTANT", stamp(1, 1, 100))).unwrap();
    apply_operation(&mut s, &set_tag_name(t2, "important", stamp(1, 2, 200))).unwrap();
    assert_eq!(s.tag_canonical.get("important"), Some(&t1));
}

#[test]
fn tag_delete_clears_canonical() {
    let mut s = ReplicaProjection::default();
    let t1 = EntityId::new_v7_for_test(300);
    apply_operation(&mut s, &set_tag_name(t1, "work", stamp(1, 1, 100))).unwrap();
    assert_eq!(s.tag_canonical.get("work"), Some(&t1));
    apply_operation(
        &mut s,
        &VerifiedOperation {
            entity: t1,
            kind: EntityKind::Tag,
            parent: None,
            stamp: stamp(1, 2, 200),
            payload: OperationPayload::Delete,
        },
    )
    .unwrap();
    assert!(
        !s.tag_canonical.contains_key("work"),
        "deleted tag is no longer canonical"
    );
}
