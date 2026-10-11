use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use ed25519_dalek::VerifyingKey;
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305, Key, Nonce,
};
use rand::Rng;
use crate::core::errors::CoreError;
use crate::core::identity::DeviceIdentity;
use crate::core::pairing::{PairingConfirm, PairingFinalAck, PairingReject, PairingRequest, PairingResponse};
pub use crate::core::pairing::session::derive_session_key;

pub const MAX_FRAME_SIZE: usize = 64 * 1024; // 64 KiB strict bounding
pub const TRANSPORT_PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PeerConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum TransportFramePayload {
    Handshake {
        protocol_version: u32,
        sender_device_id: String,
        sender_device_name: String,
        sender_pk_hex: String,
        nonce_hex: String,
    },
    HandshakeAck {
        protocol_version: u32,
        session_id: String,
    },
    Ping {
        timestamp_ms: u64,
    },
    Pong {
        timestamp_ms: u64,
    },
    StatusUpdate {
        status: String,
    },
    Disconnect {
        reason: String,
    },
    UnpairRevocation {
        target_device_id: String,
        reason: String,
    },
    UnpairRevocationAck {
        success: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportFrame {
    pub sequence: u64,
    pub timestamp: String,
    pub sender_device_id: String,
    pub payload: TransportFramePayload,
    pub signature_hex: String,
}

impl TransportFrame {
    /// Creates and cryptographically signs a frame using the sender's identity
    pub fn new_signed(
        sequence: u64,
        identity: &DeviceIdentity,
        payload: TransportFramePayload,
    ) -> Result<Self, CoreError> {
        let timestamp = Utc::now().to_rfc3339();
        let sender_device_id = identity.device_id();

        // Serialize payload without signature to build signed envelope
        let unsigned_data = serde_json::to_vec(&(&sequence, &timestamp, &sender_device_id, &payload))?;
        let signature_bytes = identity.sign(&unsigned_data);
        let signature_hex = hex::encode(signature_bytes);

        Ok(Self {
            sequence,
            timestamp,
            sender_device_id,
            payload,
            signature_hex,
        })
    }

    /// Verifies the cryptographic signature against the sender's verifying key
    pub fn verify_signature(&self, verifying_key: &VerifyingKey) -> Result<(), CoreError> {
        let unsigned_data = serde_json::to_vec(&(
            &self.sequence,
            &self.timestamp,
            &self.sender_device_id,
            &self.payload,
        ))?;
        let signature_bytes = hex::decode(&self.signature_hex)
            .map_err(|e| CoreError::Network(format!("Invalid signature hex: {}", e)))?;

        DeviceIdentity::verify(verifying_key, &unsigned_data, &signature_bytes)
    }

    /// Encodes the frame to length-prefixed bytes (4 bytes big-endian length + JSON)
    pub fn encode(&self) -> Result<Vec<u8>, CoreError> {
        let json_bytes = serde_json::to_vec(self)?;
        if json_bytes.len() > MAX_FRAME_SIZE {
            return Err(CoreError::Network(format!(
                "Frame size {} exceeds maximum allowed bound of {} bytes",
                json_bytes.len(),
                MAX_FRAME_SIZE
            )));
        }

        let len = json_bytes.len() as u32;
        let mut out = Vec::with_capacity(4 + json_bytes.len());
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(&json_bytes);
        Ok(out)
    }

    /// Decodes a length-prefixed frame from raw bytes
    pub fn decode(bytes: &[u8]) -> Result<Self, CoreError> {
        if bytes.len() < 4 {
            return Err(CoreError::Network("Incomplete frame header".to_string()));
        }

        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        if len > MAX_FRAME_SIZE {
            return Err(CoreError::Network(format!(
                "Security Alert: Incoming frame declared length {} exceeds {} byte limit",
                len, MAX_FRAME_SIZE
            )));
        }

        if bytes.len() < 4 + len {
            return Err(CoreError::Network("Frame buffer truncated".to_string()));
        }

        let frame_slice = &bytes[4..4 + len];
        let frame: TransportFrame = serde_json::from_slice(frame_slice)
            .map_err(|e| CoreError::Network(format!("Failed to parse transport frame: {}", e)))?;

        Ok(frame)
    }
}

/// Authenticated Encrypted Transport Frame providing confidentiality (ChaCha20-Poly1305) and replay protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedTransportFrame {
    pub protocol_version: u32,
    pub sequence: u64,
    pub sender_device_id: String,
    pub nonce_hex: String,
    pub ciphertext_hex: String,
}

impl EncryptedTransportFrame {
    /// Encrypts a payload into an authenticated ciphertext frame using ChaCha20-Poly1305 AEAD
    pub fn encrypt(
        sequence: u64,
        sender_device_id: &str,
        payload: &TransportFramePayload,
        session_key: &[u8; 32],
    ) -> Result<Self, CoreError> {
        let plaintext = serde_json::to_vec(payload)?;

        // Unique 12-byte nonce: 4 random salt bytes + 8-byte big-endian sequence to guarantee nonce uniqueness
        let mut nonce_bytes = [0u8; 12];
        let mut rng = rand::thread_rng();
        rng.fill(&mut nonce_bytes[0..4]);
        nonce_bytes[4..12].copy_from_slice(&sequence.to_be_bytes());

        // Associated Authenticated Data (AAD) binds frame metadata
        let aad = format!("{}:{}:{}", TRANSPORT_PROTOCOL_VERSION, sequence, sender_device_id);

        let cipher = ChaCha20Poly1305::new(Key::from_slice(session_key));
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce_bytes),
                Payload {
                    msg: &plaintext,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|e| CoreError::Network(format!("AEAD encryption failed: {}", e)))?;

        Ok(Self {
            protocol_version: TRANSPORT_PROTOCOL_VERSION,
            sequence,
            sender_device_id: sender_device_id.to_string(),
            nonce_hex: hex::encode(nonce_bytes),
            ciphertext_hex: hex::encode(ciphertext),
        })
    }

    /// Decrypts and authenticates a frame, verifying the AEAD tag and rejecting sequence replay
    pub fn decrypt(
        &self,
        session_key: &[u8; 32],
        min_expected_sequence: u64,
    ) -> Result<TransportFramePayload, CoreError> {
        if self.sequence < min_expected_sequence {
            return Err(CoreError::Network(format!(
                "Security Alert: Replay attack detected: sequence {} is less than minimum expected sequence {}",
                self.sequence, min_expected_sequence
            )));
        }

        let nonce_bytes = hex::decode(&self.nonce_hex)
            .map_err(|e| CoreError::Network(format!("Invalid nonce hex: {}", e)))?;
        if nonce_bytes.len() != 12 {
            return Err(CoreError::Network("Invalid AEAD nonce length".to_string()));
        }

        let ciphertext_bytes = hex::decode(&self.ciphertext_hex)
            .map_err(|e| CoreError::Network(format!("Invalid ciphertext hex: {}", e)))?;

        let aad = format!("{}:{}:{}", self.protocol_version, self.sequence, self.sender_device_id);

        let cipher = ChaCha20Poly1305::new(Key::from_slice(session_key));
        let plaintext = cipher
            .decrypt(
                Nonce::from_slice(&nonce_bytes),
                Payload {
                    msg: &ciphertext_bytes,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| {
                CoreError::Network(
                    "Security Alert: AEAD authentication failed (message altered, ciphertext tampered, or invalid session key)".to_string(),
                )
            })?;

        let payload: TransportFramePayload = serde_json::from_slice(&plaintext)?;
        Ok(payload)
    }

    /// Encodes the encrypted frame to length-prefixed bytes
    pub fn encode(&self) -> Result<Vec<u8>, CoreError> {
        let json_bytes = serde_json::to_vec(self)?;
        if json_bytes.len() > MAX_FRAME_SIZE {
            return Err(CoreError::Network(format!(
                "Frame size {} exceeds maximum allowed bound of {} bytes",
                json_bytes.len(),
                MAX_FRAME_SIZE
            )));
        }

        let len = json_bytes.len() as u32;
        let mut out = Vec::with_capacity(4 + json_bytes.len());
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(&json_bytes);
        Ok(out)
    }

    /// Decodes a length-prefixed encrypted frame from raw bytes
    pub fn decode(bytes: &[u8]) -> Result<Self, CoreError> {
        if bytes.len() < 4 {
            return Err(CoreError::Network("Incomplete frame header".to_string()));
        }

        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        if len > MAX_FRAME_SIZE {
            return Err(CoreError::Network(format!(
                "Security Alert: Incoming frame declared length {} exceeds {} byte limit",
                len, MAX_FRAME_SIZE
            )));
        }

        if bytes.len() < 4 + len {
            return Err(CoreError::Network("Frame buffer truncated".to_string()));
        }

        let frame_slice = &bytes[4..4 + len];
        let frame: EncryptedTransportFrame = serde_json::from_slice(frame_slice)
            .map_err(|e| CoreError::Network(format!("Failed to parse encrypted transport frame: {}", e)))?;

        Ok(frame)
    }
}

/// Unified wire envelope for TCP transport handling pairing negotiation and encrypted frames
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NetworkWireMessage {
    Encrypted(EncryptedTransportFrame),
    PairingRequest(PairingRequest),
    PairingResponse(PairingResponse),
    PairingConfirm(PairingConfirm),
    PairingFinalAck(PairingFinalAck),
    PairingReject(PairingReject),
    PlaintextFrame(TransportFrame),
}

impl NetworkWireMessage {
    pub fn encode(&self) -> Result<Vec<u8>, CoreError> {
        let json_bytes = serde_json::to_vec(self)?;
        if json_bytes.len() > MAX_FRAME_SIZE {
            return Err(CoreError::Network(format!(
                "Frame size {} exceeds maximum allowed bound of {} bytes",
                json_bytes.len(),
                MAX_FRAME_SIZE
            )));
        }

        let len = json_bytes.len() as u32;
        let mut out = Vec::with_capacity(4 + json_bytes.len());
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(&json_bytes);
        Ok(out)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, CoreError> {
        if bytes.len() < 4 {
            return Err(CoreError::Network("Incomplete frame header".to_string()));
        }

        let len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        if len > MAX_FRAME_SIZE {
            return Err(CoreError::Network(format!(
                "Security Alert: Incoming frame declared length {} exceeds {} byte limit",
                len, MAX_FRAME_SIZE
            )));
        }

        if bytes.len() < 4 + len {
            return Err(CoreError::Network("Frame buffer truncated".to_string()));
        }

        let frame_slice = &bytes[4..4 + len];
        let msg: NetworkWireMessage = serde_json::from_slice(frame_slice)
            .map_err(|e| CoreError::Network(format!("Failed to parse network wire message: {}", e)))?;

        Ok(msg)
    }
}

#[derive(Debug, Clone)]
pub struct ActivePeerSession {
    pub session_key: [u8; 32],
    pub last_seen_sequence: u64,
    pub next_send_sequence: u64,
}

pub struct ConnectionManager {
    peer_statuses: Mutex<HashMap<String, PeerConnectionStatus>>,
    active_sessions: Mutex<HashMap<String, ActivePeerSession>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            peer_statuses: Mutex::new(HashMap::new()),
            active_sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_status(&self, device_id: &str, status: PeerConnectionStatus) {
        if let Ok(mut lock) = self.peer_statuses.lock() {
            lock.insert(device_id.to_string(), status);
        }
    }

    pub fn get_status(&self, device_id: &str) -> PeerConnectionStatus {
        if let Ok(lock) = self.peer_statuses.lock() {
            lock.get(device_id).cloned().unwrap_or(PeerConnectionStatus::Disconnected)
        } else {
            PeerConnectionStatus::Disconnected
        }
    }

    pub fn get_all_statuses(&self) -> HashMap<String, String> {
        let mut result = HashMap::new();
        if let Ok(lock) = self.peer_statuses.lock() {
            for (id, status) in lock.iter() {
                let status_str = match status {
                    PeerConnectionStatus::Disconnected => "disconnected",
                    PeerConnectionStatus::Connecting => "connecting",
                    PeerConnectionStatus::Connected => "connected",
                    PeerConnectionStatus::Error(_) => "error",
                };
                result.insert(id.clone(), status_str.to_string());
            }
        }
        result
    }

    pub fn register_session(&self, peer_device_id: &str, session_key: [u8; 32]) {
        if let Ok(mut lock) = self.active_sessions.lock() {
            lock.insert(
                peer_device_id.to_string(),
                ActivePeerSession {
                    session_key,
                    last_seen_sequence: 0,
                    next_send_sequence: 1,
                },
            );
        }
        self.set_status(peer_device_id, PeerConnectionStatus::Connected);
    }

    pub fn get_session_key(&self, peer_device_id: &str) -> Option<[u8; 32]> {
        self.active_sessions
            .lock()
            .ok()
            .and_then(|lock| lock.get(peer_device_id).map(|s| s.session_key))
    }

    pub fn advance_send_sequence(&self, peer_device_id: &str) -> Option<(u64, [u8; 32])> {
        let mut lock = self.active_sessions.lock().ok()?;
        let session = lock.get_mut(peer_device_id)?;
        let seq = session.next_send_sequence;
        session.next_send_sequence += 1;
        Some((seq, session.session_key))
    }

    pub fn verify_and_advance_recv_sequence(
        &self,
        peer_device_id: &str,
        sequence: u64,
    ) -> Result<[u8; 32], CoreError> {
        let mut lock = self.active_sessions.lock().map_err(|_| {
            CoreError::Network("Connection manager mutex poisoned".to_string())
        })?;

        let session = lock.get_mut(peer_device_id).ok_or_else(|| {
            CoreError::Network(format!("No active encrypted session for peer {}", peer_device_id))
        })?;

        if sequence <= session.last_seen_sequence {
            return Err(CoreError::Network(format!(
                "Security Alert: Replay attack detected: sequence {} <= last seen {}",
                sequence, session.last_seen_sequence
            )));
        }

        session.last_seen_sequence = sequence;
        Ok(session.session_key)
    }
}

static GLOBAL_CONNECTION_MANAGER: OnceLock<ConnectionManager> = OnceLock::new();

pub fn get_connection_manager() -> &'static ConnectionManager {
    GLOBAL_CONNECTION_MANAGER.get_or_init(ConnectionManager::new)
}
