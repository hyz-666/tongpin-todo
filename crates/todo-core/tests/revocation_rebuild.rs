//! Revocation rebuild: post-cutoff effects disappear, pre-cutoff remain.

use std::collections::BTreeMap;

use serde_json::json;
use todo_core::rebuild_with_cutoff;
use todo_domain::clock::Hlc;
use todo_domain::ids::{DeviceId, EntityId, LifecycleGeneration, OperationId};
use todo_domain::operation::{EntityKind, OperationPayload, VerifiedOperation};
use todo_domain::register::VersionStamp;

fn dev(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 32])
}

fn op(seq: u64, title: &str) -> VerifiedOperation {
    VerifiedOperation {
        entity: EntityId::from_uuid(uuid::Uuid::nil()),
        kind: EntityKind::Task,
        parent: None,
        stamp: VersionStamp {
            generation: LifecycleGeneration(1),
            hlc: Hlc {
                physical_millis: 1_700_000_000_000 + seq as i64,
                logical: 0,
            },
            device: dev(7),
            operation: OperationId::new(dev(7), seq),
        },
        payload: OperationPayload::SetField {
            field: "title".to_string(),
            value: json!(title),
        },
    }
}

#[test]
fn post_cutoff_effects_disappear() {
    let ops = vec![op(1, "前"), op(2, "中"), op(3, "后")];
    let mut cutoff = BTreeMap::new();
    cutoff.insert(dev(7), 1u64); // keep only seq 1

    let rebuilt = rebuild_with_cutoff(&ops, &cutoff);
    let entity = rebuilt
        .entities
        .get(&EntityId::from_uuid(uuid::Uuid::nil()))
        .unwrap();
    // The last surviving write is seq 1.
    assert_eq!(entity.fields["title"].value.as_str().unwrap(), "前");
    assert_eq!(entity.fields["title"].stamp.operation.sequence, 1);
}

#[test]
fn pre_cutoff_effects_remain() {
    let ops = vec![op(1, "一"), op(2, "二")];
    let mut cutoff = BTreeMap::new();
    cutoff.insert(dev(7), 2u64);

    let rebuilt = rebuild_with_cutoff(&ops, &cutoff);
    let entity = rebuilt
        .entities
        .get(&EntityId::from_uuid(uuid::Uuid::nil()))
        .unwrap();
    assert_eq!(entity.fields["title"].value.as_str().unwrap(), "二");
}

#[test]
fn concurrent_valid_peer_ops_remain_after_rebuild() {
    // Two origins; only one is revoked with a cutoff.
    let a1 = op(1, "A-1");
    let mut a2 = op(2, "A-2");
    a2.stamp.device = dev(8);
    a2.stamp.operation = OperationId::new(dev(8), 1);
    a2.stamp.hlc.physical_millis += 10_000;

    let ops = vec![a1, a2];
    let mut cutoff = BTreeMap::new();
    cutoff.insert(dev(7), 1u64); // keep A up to seq 1

    let rebuilt = rebuild_with_cutoff(&ops, &cutoff);
    let entity = rebuilt
        .entities
        .get(&EntityId::from_uuid(uuid::Uuid::nil()))
        .unwrap();
    // Peer 8's concurrent op wins by HLC.
    assert_eq!(entity.fields["title"].value.as_str().unwrap(), "A-2");
    assert_eq!(entity.fields["title"].stamp.device, dev(8));
}
