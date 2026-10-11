use hmac::{Hmac, Mac};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use crate::core::identity::PublicDeviceInfo;

pub const INVITATION_EXPIRATION_SECS: u64 = 300; // 5 minutes
pub const SAS_MAC_KEY: &[u8] = b"StardewSync-SAS-v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingInvitation {
    pub code: String,
    pub inviter_device_id: String,
    pub inviter_device_name: String,
    pub inviter_public_key_hex: String,
    pub inviter_nonce_hex: String,
    pub tcp_port: u16,
    pub expires_in_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingRequest {
    pub protocol_version: u32,
    pub invitation_code: String,
    pub joiner_device_id: String,
    pub joiner_device_name: String,
    pub joiner_public_key_hex: String,
    pub joiner_ecdh_public_hex: String,
    pub joiner_nonce_hex: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingResponse {
    pub protocol_version: u32,
    pub session_id: String,
    pub inviter_device_id: String,
    pub inviter_device_name: String,
    pub inviter_public_key_hex: String,
    pub inviter_ecdh_public_hex: String,
    pub inviter_nonce_hex: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingConfirm {
    pub session_id: String,
    pub sender_device_id: String,
    pub sas_code: String,
    pub accepted: bool,
    pub signature_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingFinalAck {
    pub session_id: String,
    pub sender_device_id: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairingReject {
    pub protocol_version: u32,
    pub sender_device_id: String,
    pub rejection_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PairingStatus {
    Idle,
    Inviting,
    Connecting,
    AwaitingDualConfirmation,
    Completed,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivePairingSessionView {
    pub status: PairingStatus,
    pub session_id: Option<String>,
    pub invitation_code: Option<String>,
    pub peer_device: Option<PublicDeviceInfo>,
    pub sas_code: Option<String>,
    pub our_confirmed: bool,
    pub peer_confirmed: bool,
    pub seconds_remaining: u64,
}

/// Computes the mutual handshake transcript bytes
pub fn compute_transcript(
    invitation_code: &str,
    inviter_pk_hex: &str,
    joiner_pk_hex: &str,
    inviter_ecdh_hex: &str,
    joiner_ecdh_hex: &str,
    inviter_nonce_hex: &str,
    joiner_nonce_hex: &str,
) -> Vec<u8> {
    let mut transcript_hasher = Sha256::new();
    transcript_hasher.update(invitation_code.as_bytes());
    transcript_hasher.update(inviter_pk_hex.as_bytes());
    transcript_hasher.update(joiner_pk_hex.as_bytes());
    transcript_hasher.update(inviter_ecdh_hex.as_bytes());
    transcript_hasher.update(joiner_ecdh_hex.as_bytes());
    transcript_hasher.update(inviter_nonce_hex.as_bytes());
    transcript_hasher.update(joiner_nonce_hex.as_bytes());
    transcript_hasher.finalize().to_vec()
}

/// Derives a 32-byte symmetric AEAD session key from a shared secret and transcript via HKDF/HMAC-SHA256
pub fn derive_session_key(shared_secret: &[u8; 32], transcript: &[u8]) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(shared_secret)
        .expect("HMAC can take 32-byte key");
    mac.update(transcript);
    mac.update(b"stardew-sync-v0.2.0-session-key");
    let result = mac.finalize().into_bytes();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
}

/// Computes the 6-digit Short Authentication String (SAS) from the mutual handshake transcript
pub fn compute_sas_code(
    invitation_code: &str,
    inviter_pk_hex: &str,
    joiner_pk_hex: &str,
    inviter_ecdh_hex: &str,
    joiner_ecdh_hex: &str,
    inviter_nonce_hex: &str,
    joiner_nonce_hex: &str,
) -> String {
    let transcript = compute_transcript(
        invitation_code,
        inviter_pk_hex,
        joiner_pk_hex,
        inviter_ecdh_hex,
        joiner_ecdh_hex,
        inviter_nonce_hex,
        joiner_nonce_hex,
    );

    let mut mac = Hmac::<Sha256>::new_from_slice(SAS_MAC_KEY)
        .expect("HMAC can take any key length");
    mac.update(&transcript);
    let hmac_bytes = mac.finalize().into_bytes();

    let num = u32::from_be_bytes([
        hmac_bytes[0],
        hmac_bytes[1],
        hmac_bytes[2],
        hmac_bytes[3],
    ]) % 1_000_000;

    let digits = format!("{:06}", num);
    format!("{} {}", &digits[0..3], &digits[3..6])
}

/// Generates a human-readable 6-character invitation code (e.g. "SD-8492")
pub fn generate_invitation_code() -> String {
    let mut rng = rand::thread_rng();
    let num: u16 = rng.gen_range(1000..9999);
    format!("SD-{}", num)
}

/// Generates a random 32-byte nonce
pub fn generate_nonce_hex() -> String {
    let mut rng = rand::thread_rng();
    let mut nonce = [0u8; 32];
    rng.fill(&mut nonce);
    hex::encode(nonce)
}
