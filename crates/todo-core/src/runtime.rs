//! Runtime status: replica, peer reachability, and trust state.

use std::collections::BTreeMap;
use std::sync::Mutex;

use todo_domain::ids::DeviceId;

use crate::recovery::ReplicaState;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerStatus {
    pub device_id: DeviceId,
    pub reachable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeStatus {
    pub replica: ReplicaState,
    pub peers: Vec<PeerStatus>,
}

pub(crate) struct RuntimeState {
    peers: Mutex<BTreeMap<DeviceId, bool>>,
}

impl RuntimeState {
    pub(crate) fn new() -> Self {
        Self {
            peers: Mutex::new(BTreeMap::new()),
        }
    }

    pub(crate) fn update_peer(&self, device: DeviceId, reachable: bool) {
        self.peers.lock().unwrap().insert(device, reachable);
    }

    pub(crate) fn status(&self, replica: ReplicaState) -> RuntimeStatus {
        let peers = self
            .peers
            .lock()
            .unwrap()
            .iter()
            .map(|(d, r)| PeerStatus {
                device_id: *d,
                reachable: *r,
            })
            .collect();
        RuntimeStatus { replica, peers }
    }
}
