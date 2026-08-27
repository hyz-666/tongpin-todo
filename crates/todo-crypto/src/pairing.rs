//! Pairing state machine and QR/announce payload.

use std::time::{SystemTime, UNIX_EPOCH};

use snow::Builder;
use uuid::Uuid;

use crate::error::CryptoError;
use crate::sas::derive_sas;

/// Pairing expiry (seconds).
pub const PAIRING_EXPIRY_SECS: i64 = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairingState {
    Offered,
    Connecting,
    SasPendingBoth,
    LocalConfirmed,
    RemoteConfirmed,
    SnapshotCommitting,
    Paired,
    Cancelled,
    Expired,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateEndpoint {
    pub address: String,
    pub port: u16,
}

/// The QR/announce payload for a pairing invitation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairingPayloadV1 {
    pub session_id: Uuid,
    pub ephemeral_public: [u8; 32],
    pub candidates: Vec<CandidateEndpoint>,
    pub protocol_major: u16,
    pub expires_at: i64,
}

/// A pairing session: Noise XX transcript plus mutual SAS confirmation.
pub struct PairingSession {
    state: PairingState,
    session_id: Uuid,
    expires_at: i64,
    transcript: Option<[u8; 32]>,
    local_confirmed: bool,
    remote_confirmed: bool,
}

impl PairingSession {
    pub fn offered(session_id: Uuid, expires_at: i64) -> Self {
        Self {
            state: PairingState::Offered,
            session_id,
            expires_at,
            transcript: None,
            local_confirmed: false,
            remote_confirmed: false,
        }
    }

    pub fn state(&self) -> PairingState {
        self.state
    }

    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    pub fn expires_at(&self) -> i64 {
        self.expires_at
    }

    /// Advance from Offered to Connecting.
    pub fn begin_connecting(&mut self) -> Result<(), CryptoError> {
        if self.state != PairingState::Offered {
            return Err(CryptoError::Identity("not offered".into()));
        }
        self.state = PairingState::Connecting;
        Ok(())
    }

    /// Record the authenticated handshake transcript once Noise XX completes.
    pub fn set_transcript(&mut self, transcript: [u8; 32]) -> Result<(), CryptoError> {
        if self.state != PairingState::Connecting {
            return Err(CryptoError::Identity("not connecting".into()));
        }
        self.transcript = Some(transcript);
        self.state = PairingState::SasPendingBoth;
        Ok(())
    }

    /// The SAS, available only after the transcript is set.
    pub fn sas(&self) -> Option<String> {
        self.transcript.as_ref().map(derive_sas)
    }

    /// Local user confirms the SAS matches the peer's display.
    pub fn confirm_local(&mut self) -> Result<(), CryptoError> {
        if self.state != PairingState::SasPendingBoth && self.state != PairingState::RemoteConfirmed
        {
            return Err(CryptoError::Identity("not awaiting confirmation".into()));
        }
        self.local_confirmed = true;
        self.advance();
        Ok(())
    }

    /// The peer reports its user confirmed the SAS.
    pub fn confirm_remote(&mut self) -> Result<(), CryptoError> {
        if self.state != PairingState::SasPendingBoth && self.state != PairingState::LocalConfirmed
        {
            return Err(CryptoError::Identity("not awaiting confirmation".into()));
        }
        self.remote_confirmed = true;
        self.advance();
        Ok(())
    }

    fn advance(&mut self) {
        if self.local_confirmed && !self.remote_confirmed {
            self.state = PairingState::LocalConfirmed;
        } else if self.remote_confirmed && !self.local_confirmed {
            self.state = PairingState::RemoteConfirmed;
        } else if self.local_confirmed && self.remote_confirmed {
            self.state = PairingState::Paired;
        }
    }

    pub fn begin_snapshot(&mut self) -> Result<(), CryptoError> {
        if self.state != PairingState::Paired {
            return Err(CryptoError::Identity("not paired".into()));
        }
        self.state = PairingState::SnapshotCommitting;
        Ok(())
    }

    pub fn cancel(&mut self) {
        if matches!(
            self.state,
            PairingState::Offered
                | PairingState::Connecting
                | PairingState::SasPendingBoth
                | PairingState::LocalConfirmed
                | PairingState::RemoteConfirmed
        ) {
            self.state = PairingState::Cancelled;
        }
    }

    /// True once the session has passed its expiry while still unpaired.
    pub fn is_expired(&self, now: i64) -> bool {
        now > self.expires_at
            && !matches!(self.state, PairingState::Paired | PairingState::Cancelled)
    }

    pub fn mark_expired(&mut self) {
        if !matches!(self.state, PairingState::Paired | PairingState::Cancelled) {
            self.state = PairingState::Expired;
        }
    }
}

/// Run an XX handshake transcript as the initiator, returning the handshake
/// hash used to derive the SAS.
pub fn xx_initiator_transcript(
    static_secret: &[u8; 32],
    remote_ephemeral: &[u8; 32],
) -> Result<([u8; 32], Vec<u8>), CryptoError> {
    let mut hs = Builder::new("Noise_XX_25519_ChaChaPoly_SHA256".parse().unwrap())
        .local_private_key(static_secret)
        .map_err(|e| CryptoError::Identity(e.to_string()))?
        .build_initiator()
        .map_err(|e| CryptoError::Identity(e.to_string()))?;
    let mut first = [0u8; 1024];
    let n = hs
        .write_message(&[], &mut first)
        .map_err(|e| CryptoError::Identity(e.to_string()))?;
    let mut second_in = [0u8; 1024];
    let n2 = hs
        .read_message(remote_ephemeral, &mut second_in)
        .map_err(|e| CryptoError::Identity(e.to_string()))?;
    let transcript_slice = hs.get_handshake_hash();
    let mut transcript = [0u8; 32];
    transcript.copy_from_slice(&transcript_slice[..32]);
    let mut out = Vec::with_capacity(n + n2);
    out.extend_from_slice(&first[..n]);
    out.extend_from_slice(&second_in[..n2]);
    Ok((transcript, out))
}

/// Current wall-clock milliseconds since the Unix epoch.
pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
