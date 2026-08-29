//! Pairing orchestration: the end-to-end pairing flow.
//!
//! [`PairingFlow`] strings together every stage of pairing a new device:
//! offering an invitation, discovering the peer, completing the Noise XX
//! handshake, mutually confirming the short-authentication string (SAS), and
//! finally committing the peer to the membership graph. Like the sync
//! orchestrator, it owns only the *decision* of what to do next and emits a
//! [`PairingAction`] for the platform to perform.

use uuid::Uuid;

use todo_crypto::{PAIRING_EXPIRY_SECS, PairingSession, PairingState, now_millis};
use todo_domain::ids::DeviceId;

/// High-level phases of a pairing flow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PairingPhase {
    #[default]
    Idle,
    Offering,
    Discovering,
    Connecting,
    ConfirmingSas,
    Committing,
    Paired,
    Failed,
    Cancelled,
}

/// What the platform must perform next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PairingAction {
    /// Show the pairing QR code / invitation.
    ShowQr { session_id: Uuid },
    /// Discover the peer on the LAN.
    Discover,
    /// Connect to the peer's endpoint.
    Connect { ip: String, port: u16 },
    /// Display the SAS for the user to compare.
    ShowSas { sas: String },
    /// Wait for the peer's user to confirm the SAS.
    AwaitPeerConfirmation,
    /// Commit the peer into the membership graph.
    CommitMember { device: DeviceId },
    /// Pairing is complete.
    Done,
}

/// Drives one pairing from offer to committed membership.
pub struct PairingFlow {
    phase: PairingPhase,
    session: PairingSession,
    remote_device: Option<DeviceId>,
}

impl PairingFlow {
    /// Begin a new pairing flow with a fresh offer.
    pub fn new(session_id: Uuid) -> Self {
        let session =
            PairingSession::offered(session_id, now_millis() + PAIRING_EXPIRY_SECS * 1000);
        Self {
            phase: PairingPhase::Offering,
            session,
            remote_device: None,
        }
    }

    pub fn phase(&self) -> PairingPhase {
        self.phase
    }

    pub fn session_id(&self) -> Uuid {
        self.session.session_id()
    }

    /// The peer's identity once the handshake has authenticated it.
    pub fn remote_device(&self) -> Option<DeviceId> {
        self.remote_device
    }

    /// The SAS, available after the handshake transcript is set.
    pub fn sas(&self) -> Option<String> {
        self.session.sas()
    }

    /// Whether this flow has expired while still unpaired.
    pub fn is_expired(&self, now: i64) -> bool {
        self.session.is_expired(now)
    }

    /// Present the offer; the platform shows the QR code and we discover.
    pub fn start_offering(&mut self) -> PairingAction {
        self.phase = PairingPhase::Offering;
        PairingAction::ShowQr {
            session_id: self.session.session_id(),
        }
    }

    /// The platform observed an inbound pairing request; discover the peer.
    pub fn on_peer_detected(&mut self) -> PairingAction {
        self.phase = PairingPhase::Discovering;
        PairingAction::Discover
    }

    /// A candidate endpoint was found; connect to it.
    pub fn on_discovered(&mut self, ip: &str, port: u16) -> PairingAction {
        self.phase = PairingPhase::Connecting;
        // Advance the underlying session from Offered to Connecting.
        if self.session.begin_connecting().is_err() {
            self.phase = PairingPhase::Failed;
        }
        PairingAction::Connect {
            ip: ip.to_string(),
            port,
        }
    }

    /// The Noise XX handshake completed; record its transcript and show the SAS.
    pub fn on_connected(&mut self, transcript: [u8; 32], remote_device: DeviceId) -> PairingAction {
        self.remote_device = Some(remote_device);
        self.phase = PairingPhase::ConfirmingSas;
        match self.session.set_transcript(transcript) {
            Ok(()) => PairingAction::ShowSas {
                sas: self.session.sas().unwrap_or_default(),
            },
            Err(_) => {
                self.phase = PairingPhase::Failed;
                PairingAction::Done
            }
        }
    }

    /// The local user confirmed the SAS matches.
    pub fn confirm_local(&mut self) -> PairingAction {
        match self.session.confirm_local() {
            Ok(()) => self.after_confirmation(),
            Err(_) => {
                self.phase = PairingPhase::Failed;
                PairingAction::Done
            }
        }
    }

    /// The peer reported its user confirmed the SAS.
    pub fn confirm_remote(&mut self) -> PairingAction {
        match self.session.confirm_remote() {
            Ok(()) => self.after_confirmation(),
            Err(_) => {
                self.phase = PairingPhase::Failed;
                PairingAction::Done
            }
        }
    }

    /// Once both sides have confirmed, move to committing the member.
    fn after_confirmation(&mut self) -> PairingAction {
        if self.session.state() == PairingState::Paired {
            self.phase = PairingPhase::Committing;
            PairingAction::CommitMember {
                device: self
                    .remote_device
                    .unwrap_or_else(|| DeviceId::from_bytes([0u8; 32])),
            }
        } else {
            self.phase = PairingPhase::ConfirmingSas;
            PairingAction::AwaitPeerConfirmation
        }
    }

    /// The membership commit completed; pairing is done.
    pub fn on_member_committed(&mut self) -> PairingAction {
        self.phase = PairingPhase::Paired;
        PairingAction::Done
    }

    /// Cancel an in-flight pairing.
    pub fn cancel(&mut self) {
        self.session.cancel();
        self.phase = PairingPhase::Cancelled;
    }

    /// Mark an expired flow.
    pub fn mark_expired(&mut self) {
        self.session.mark_expired();
        self.phase = PairingPhase::Failed;
    }
}
