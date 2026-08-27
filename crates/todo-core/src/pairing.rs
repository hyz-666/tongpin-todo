//! Core-side pairing orchestration.

use std::collections::HashMap;
use std::sync::Mutex;

use todo_crypto::{PairingSession, PairingState, now_millis};
use uuid::Uuid;

use crate::error::CoreError;

/// Manages in-flight pairing sessions keyed by session id.
#[derive(Default)]
pub struct PairingManager {
    sessions: Mutex<HashMap<Uuid, PairingSession>>,
}

impl PairingManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new pairing session, returning its id.
    pub fn start(&self) -> Uuid {
        let id = Uuid::new_v4();
        let session = PairingSession::offered(id, now_millis() + 120_000);
        self.sessions.lock().unwrap().insert(id, session);
        id
    }

    pub fn state(&self, id: &Uuid) -> Option<PairingState> {
        self.sessions.lock().unwrap().get(id).map(|s| s.state())
    }

    pub fn sas(&self, id: &Uuid) -> Option<String> {
        self.sessions.lock().unwrap().get(id).and_then(|s| s.sas())
    }

    pub fn begin_connecting(&self, id: &Uuid) -> Result<(), CoreError> {
        let mut sessions = self.sessions.lock().unwrap();
        let s = sessions
            .get_mut(id)
            .ok_or_else(|| CoreError::InvalidCommand("unknown session".into()))?;
        s.begin_connecting()
            .map_err(|e| CoreError::InvalidCommand(e.to_string()))
    }

    pub fn set_transcript(&self, id: &Uuid, transcript: [u8; 32]) -> Result<(), CoreError> {
        let mut sessions = self.sessions.lock().unwrap();
        let s = sessions
            .get_mut(id)
            .ok_or_else(|| CoreError::InvalidCommand("unknown session".into()))?;
        s.set_transcript(transcript)
            .map_err(|e| CoreError::InvalidCommand(e.to_string()))
    }

    pub fn confirm_local(&self, id: &Uuid) -> Result<(), CoreError> {
        let mut sessions = self.sessions.lock().unwrap();
        let s = sessions
            .get_mut(id)
            .ok_or_else(|| CoreError::InvalidCommand("unknown session".into()))?;
        s.confirm_local()
            .map_err(|e| CoreError::InvalidCommand(e.to_string()))
    }

    pub fn confirm_remote(&self, id: &Uuid) -> Result<(), CoreError> {
        let mut sessions = self.sessions.lock().unwrap();
        let s = sessions
            .get_mut(id)
            .ok_or_else(|| CoreError::InvalidCommand("unknown session".into()))?;
        s.confirm_remote()
            .map_err(|e| CoreError::InvalidCommand(e.to_string()))
    }

    pub fn cancel(&self, id: &Uuid) {
        if let Some(s) = self.sessions.lock().unwrap().get_mut(id) {
            s.cancel();
        }
    }
}
