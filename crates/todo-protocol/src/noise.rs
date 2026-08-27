//! Authenticated Noise sessions (XX for pairing, IK for reconnect).

use snow::{Builder, HandshakeState, TransportState};

use crate::error::ProtocolError;

pub const NOISE_XX: &str = "Noise_XX_25519_ChaChaPoly_SHA256";
pub const NOISE_IK: &str = "Noise_IK_25519_ChaChaPoly_SHA256";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoiseRole {
    Initiator,
    Responder,
}

/// A Noise handshake (XX or IK) with a bound prologue.
pub struct NoiseSession {
    handshake: HandshakeState,
    role: NoiseRole,
}

impl NoiseSession {
    pub fn xx_initiator(static_key: &[u8; 32], prologue: &[u8]) -> Result<Self, ProtocolError> {
        Self::build(NOISE_XX, static_key, None, true, prologue)
    }

    pub fn xx_responder(static_key: &[u8; 32], prologue: &[u8]) -> Result<Self, ProtocolError> {
        Self::build(NOISE_XX, static_key, None, false, prologue)
    }

    pub fn ik_initiator(
        static_key: &[u8; 32],
        remote_static: &[u8; 32],
        prologue: &[u8],
    ) -> Result<Self, ProtocolError> {
        Self::build(NOISE_IK, static_key, Some(remote_static), true, prologue)
    }

    pub fn ik_responder(static_key: &[u8; 32], prologue: &[u8]) -> Result<Self, ProtocolError> {
        Self::build(NOISE_IK, static_key, None, false, prologue)
    }

    fn build(
        pattern: &str,
        static_key: &[u8; 32],
        remote_static: Option<&[u8; 32]>,
        initiator: bool,
        prologue: &[u8],
    ) -> Result<Self, ProtocolError> {
        let mut builder = Builder::new(pattern.parse().map_err(|_| ProtocolError::MalformedFrame)?)
            .local_private_key(static_key)
            .map_err(|_| ProtocolError::MalformedFrame)?;
        if let Some(remote) = remote_static {
            builder = builder
                .remote_public_key(remote)
                .map_err(|_| ProtocolError::MalformedFrame)?;
        }
        builder = builder
            .prologue(prologue)
            .map_err(|_| ProtocolError::MalformedFrame)?;
        let handshake = if initiator {
            builder
                .build_initiator()
                .map_err(|_| ProtocolError::MalformedFrame)?
        } else {
            builder
                .build_responder()
                .map_err(|_| ProtocolError::MalformedFrame)?
        };
        Ok(Self {
            handshake,
            role: if initiator {
                NoiseRole::Initiator
            } else {
                NoiseRole::Responder
            },
        })
    }

    pub fn role(&self) -> NoiseRole {
        self.role
    }

    pub fn write_message(
        &mut self,
        payload: &[u8],
        out: &mut [u8],
    ) -> Result<usize, ProtocolError> {
        self.handshake
            .write_message(payload, out)
            .map_err(|_| ProtocolError::MalformedFrame)
    }

    pub fn read_message(&mut self, input: &[u8], out: &mut [u8]) -> Result<usize, ProtocolError> {
        self.handshake
            .read_message(input, out)
            .map_err(|_| ProtocolError::MalformedFrame)
    }

    pub fn is_finished(&self) -> bool {
        self.handshake.is_handshake_finished()
    }

    pub fn remote_static(&self) -> Option<[u8; 32]> {
        self.handshake
            .get_remote_static()
            .and_then(|k| k.try_into().ok())
    }

    pub fn handshake_hash(&self) -> Option<[u8; 32]> {
        let h = self.handshake.get_handshake_hash();
        h.try_into().ok()
    }

    pub fn into_transport(self) -> Result<Transport, ProtocolError> {
        let state = self
            .handshake
            .into_transport_mode()
            .map_err(|_| ProtocolError::MalformedFrame)?;
        Ok(Transport { state })
    }
}

/// An established Noise transport for authenticated encryption.
pub struct Transport {
    state: TransportState,
}

impl Transport {
    pub fn write(&mut self, payload: &[u8], out: &mut [u8]) -> Result<usize, ProtocolError> {
        self.state
            .write_message(payload, out)
            .map_err(|_| ProtocolError::MalformedFrame)
    }

    pub fn read(&mut self, input: &[u8], out: &mut [u8]) -> Result<usize, ProtocolError> {
        self.state
            .read_message(input, out)
            .map_err(|_| ProtocolError::MalformedFrame)
    }
}
