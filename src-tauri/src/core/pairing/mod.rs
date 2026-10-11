pub mod session;
pub mod store;

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use chrono::Utc;
use sha2::{Digest, Sha256};
use crate::core::errors::CoreError;
use crate::core::identity::{DeviceIdentity, PublicDeviceInfo};
pub use session::{
    compute_sas_code, compute_transcript, derive_session_key, generate_invitation_code,
    generate_nonce_hex, ActivePairingSessionView, PairingConfirm, PairingFinalAck, PairingInvitation, PairingReject, PairingRequest,
    PairingResponse, PairingStatus, INVITATION_EXPIRATION_SECS,
};
pub use store::{PairedDevice, PairedDeviceStore};

pub struct InternalPairingSession {
    pub session_id: String,
    pub is_inviter: bool,
    pub invitation_code: String,
    pub inviter_nonce_hex: String,
    pub joiner_nonce_hex: String,
    pub inviter_pk_hex: String,
    pub joiner_pk_hex: String,
    pub inviter_ecdh_hex: String,
    pub joiner_ecdh_hex: String,
    pub peer_info: PublicDeviceInfo,
    pub sas_code: String,
    pub session_key: [u8; 32],
    pub our_confirmed: bool,
    pub peer_confirmed: bool,
    pub started_at: Instant,
    pub expires_at: Instant,
    pub status: PairingStatus,
}

pub struct PairingManager {
    current_session: Mutex<Option<InternalPairingSession>>,
    active_invitation: Mutex<Option<PairingInvitation>>,
    invitation_created_at: Mutex<Option<Instant>>,
}

impl PairingManager {
    pub fn new() -> Self {
        Self {
            current_session: Mutex::new(None),
            active_invitation: Mutex::new(None),
            invitation_created_at: Mutex::new(None),
        }
    }

    /// Starts a short-lived pairing invitation on the local device
    pub fn start_invitation(
        &self,
        local_identity: &DeviceIdentity,
        tcp_port: u16,
    ) -> Result<PairingInvitation, CoreError> {
        let code = generate_invitation_code();
        let nonce = generate_nonce_hex();
        let vk = local_identity.verifying_key();
        let pk_hex = hex::encode(vk.as_bytes());

        let invitation = PairingInvitation {
            code: code.clone(),
            inviter_device_id: local_identity.device_id(),
            inviter_device_name: local_identity.device_name().to_string(),
            inviter_public_key_hex: pk_hex,
            inviter_nonce_hex: nonce,
            tcp_port,
            expires_in_secs: INVITATION_EXPIRATION_SECS,
        };

        let mut inv_lock = self.active_invitation.lock().map_err(|_| {
            CoreError::Pairing("Internal pairing mutex poison error".to_string())
        })?;
        let mut time_lock = self.invitation_created_at.lock().map_err(|_| {
            CoreError::Pairing("Internal pairing mutex poison error".to_string())
        })?;

        *inv_lock = Some(invitation.clone());
        *time_lock = Some(Instant::now());

        Ok(invitation)
    }

    /// Cancels any active invitation or ongoing pairing session
    pub fn cancel_invitation(&self) -> Result<(), CoreError> {
        let mut inv_lock = self.active_invitation.lock().map_err(|_| {
            CoreError::Pairing("Internal pairing mutex poison error".to_string())
        })?;
        let mut time_lock = self.invitation_created_at.lock().map_err(|_| {
            CoreError::Pairing("Internal pairing mutex poison error".to_string())
        })?;
        let mut session_lock = self.current_session.lock().map_err(|_| {
            CoreError::Pairing("Internal pairing mutex poison error".to_string())
        })?;

        *inv_lock = None;
        *time_lock = None;
        *session_lock = None;

        Ok(())
    }

    /// Checks if the active invitation has expired
    pub fn is_invitation_expired(&self) -> bool {
        let time_lock = self.invitation_created_at.lock();
        if let Ok(guard) = time_lock {
            if let Some(created) = *guard {
                return created.elapsed() > Duration::from_secs(INVITATION_EXPIRATION_SECS);
            }
        }
        true
    }

    pub fn get_active_invitation(&self) -> Option<PairingInvitation> {
        if self.is_invitation_expired() {
            return None;
        }
        let inv_lock = self.active_invitation.lock().ok()?;
        inv_lock.clone()
    }

    /// Checks if an invitation record is present regardless of expiration
    pub fn is_invitation_present(&self) -> bool {
        self.active_invitation.lock().ok().and_then(|g| g.clone()).is_some()
    }

    #[doc(hidden)]
    pub fn expire_invitation_for_test(&self) {
        if let Ok(mut guard) = self.invitation_created_at.lock() {
            if let Some(ref mut created) = *guard {
                *created = Instant::now() - Duration::from_secs(INVITATION_EXPIRATION_SECS + 1);
            }
        }
    }

    /// Inspects the current pairing session view for UI rendering
    pub fn get_session_view(&self) -> ActivePairingSessionView {
        let session_lock = self.current_session.lock().ok();
        if let Some(guard) = session_lock {
            if let Some(ref session) = *guard {
                let remaining = session
                    .expires_at
                    .checked_duration_since(Instant::now())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let status = if remaining == 0 && session.status == PairingStatus::AwaitingDualConfirmation {
                    PairingStatus::Expired
                } else {
                    session.status.clone()
                };

                return ActivePairingSessionView {
                    status,
                    session_id: Some(session.session_id.clone()),
                    invitation_code: Some(session.invitation_code.clone()),
                    peer_device: Some(session.peer_info.clone()),
                    sas_code: Some(session.sas_code.clone()),
                    our_confirmed: session.our_confirmed,
                    peer_confirmed: session.peer_confirmed,
                    seconds_remaining: remaining,
                };
            }
        }

        // Check if invitation is active without connected session
        if let Some(inv) = self.get_active_invitation() {
            let time_lock = self.invitation_created_at.lock().ok();
            let elapsed = time_lock
                .and_then(|t| *t)
                .map(|t| t.elapsed().as_secs())
                .unwrap_or(INVITATION_EXPIRATION_SECS);
            let remaining = INVITATION_EXPIRATION_SECS.saturating_sub(elapsed);

            return ActivePairingSessionView {
                status: PairingStatus::Inviting,
                session_id: None,
                invitation_code: Some(inv.code),
                peer_device: None,
                sas_code: None,
                our_confirmed: false,
                peer_confirmed: false,
                seconds_remaining: remaining,
            };
        } else if self.is_invitation_present() && self.is_invitation_expired() {
            let code = self
                .active_invitation
                .lock()
                .ok()
                .and_then(|g| g.as_ref().map(|i| i.code.clone()));
            return ActivePairingSessionView {
                status: PairingStatus::Expired,
                session_id: None,
                invitation_code: code,
                peer_device: None,
                sas_code: None,
                our_confirmed: false,
                peer_confirmed: false,
                seconds_remaining: 0,
            };
        }

        ActivePairingSessionView {
            status: PairingStatus::Idle,
            session_id: None,
            invitation_code: None,
            peer_device: None,
            sas_code: None,
            our_confirmed: false,
            peer_confirmed: false,
            seconds_remaining: 0,
        }
    }

    /// Sets up an awaiting dual-confirmation session directly (used by handshake drivers or synthetic tests)
    pub fn start_confirmation_session(
        &self,
        session_id: &str,
        invitation_code: &str,
        inviter_pk_hex: &str,
        joiner_pk_hex: &str,
        inviter_ecdh_hex: &str,
        joiner_ecdh_hex: &str,
        inviter_nonce_hex: &str,
        joiner_nonce_hex: &str,
        peer_info: PublicDeviceInfo,
        is_inviter: bool,
    ) -> Result<String, CoreError> {
        self.start_confirmation_session_with_secret(
            session_id,
            invitation_code,
            inviter_pk_hex,
            joiner_pk_hex,
            inviter_ecdh_hex,
            joiner_ecdh_hex,
            inviter_nonce_hex,
            joiner_nonce_hex,
            peer_info,
            is_inviter,
            None,
        )
    }

    /// Sets up an awaiting dual-confirmation session bound to a verified X25519 shared secret
    pub fn start_confirmation_session_with_secret(
        &self,
        session_id: &str,
        invitation_code: &str,
        inviter_pk_hex: &str,
        joiner_pk_hex: &str,
        inviter_ecdh_hex: &str,
        joiner_ecdh_hex: &str,
        inviter_nonce_hex: &str,
        joiner_nonce_hex: &str,
        peer_info: PublicDeviceInfo,
        is_inviter: bool,
        shared_secret: Option<[u8; 32]>,
    ) -> Result<String, CoreError> {
        let transcript = compute_transcript(
            invitation_code,
            inviter_pk_hex,
            joiner_pk_hex,
            inviter_ecdh_hex,
            joiner_ecdh_hex,
            inviter_nonce_hex,
            joiner_nonce_hex,
        );

        let sas_code = compute_sas_code(
            invitation_code,
            inviter_pk_hex,
            joiner_pk_hex,
            inviter_ecdh_hex,
            joiner_ecdh_hex,
            inviter_nonce_hex,
            joiner_nonce_hex,
        );

        let session_key = if let Some(secret) = shared_secret {
            derive_session_key(&secret, &transcript)
        } else {
            let mut hasher = Sha256::new();
            hasher.update(&transcript);
            let mut key = [0u8; 32];
            key.copy_from_slice(&hasher.finalize());
            key
        };

        let session = InternalPairingSession {
            session_id: session_id.to_string(),
            is_inviter,
            invitation_code: invitation_code.to_string(),
            inviter_nonce_hex: inviter_nonce_hex.to_string(),
            joiner_nonce_hex: joiner_nonce_hex.to_string(),
            inviter_pk_hex: inviter_pk_hex.to_string(),
            joiner_pk_hex: joiner_pk_hex.to_string(),
            inviter_ecdh_hex: inviter_ecdh_hex.to_string(),
            joiner_ecdh_hex: joiner_ecdh_hex.to_string(),
            peer_info,
            sas_code: sas_code.clone(),
            session_key,
            our_confirmed: false,
            peer_confirmed: false,
            started_at: Instant::now(),
            expires_at: Instant::now() + Duration::from_secs(INVITATION_EXPIRATION_SECS),
            status: PairingStatus::AwaitingDualConfirmation,
        };

        let mut session_lock = self.current_session.lock().map_err(|_| {
            CoreError::Pairing("Internal pairing mutex poison error".to_string())
        })?;
        *session_lock = Some(session);

        Ok(sas_code)
    }

    /// Checks if the local user has approved the SAS confirmation code
    pub fn is_our_confirmed(&self) -> bool {
        self.current_session
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|s| s.our_confirmed))
            .unwrap_or(false)
    }

    /// Checks if the remote peer has approved the SAS confirmation code
    pub fn is_peer_confirmed(&self) -> bool {
        self.current_session
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|s| s.peer_confirmed))
            .unwrap_or(false)
    }

    /// Gets active session identifiers and SAS code for sending confirmation over the wire
    pub fn get_session_pairing_details(&self) -> Option<(String, String, String)> {
        let lock = self.current_session.lock().ok()?;
        let s = lock.as_ref()?;
        Some((s.session_id.clone(), s.sas_code.clone(), s.peer_info.device_id.clone()))
    }

    /// Records local user confirmation of the 6-digit SAS code
    pub fn confirm_local(
        &self,
        sas_code_input: &str,
        store: &mut PairedDeviceStore,
    ) -> Result<Option<PairedDevice>, CoreError> {
        let mut session_lock = self.current_session.lock().map_err(|_| {
            CoreError::Pairing("Internal pairing mutex poison error".to_string())
        })?;

        let session = session_lock
            .as_mut()
            .ok_or_else(|| CoreError::Pairing("No active pairing session awaiting confirmation".to_string()))?;

        if session.status != PairingStatus::AwaitingDualConfirmation {
            return Err(CoreError::Pairing(format!(
                "Pairing session is not awaiting confirmation (status: {:?})",
                session.status
            )));
        }

        if session.expires_at < Instant::now() {
            session.status = PairingStatus::Expired;
            return Err(CoreError::Pairing("Pairing session expired. Please re-initiate pairing.".to_string()));
        }

        // Verify the entered / displayed SAS code matches exactly
        let normalized_input = sas_code_input.trim().replace('-', " ");
        let normalized_sas = session.sas_code.trim();

        if !normalized_input.eq_ignore_ascii_case(normalized_sas) {
            session.status = PairingStatus::Rejected;
            return Err(CoreError::Pairing(
                "Security Alert: SAS confirmation code mismatch. Pairing rejected to prevent MITM tampering.".to_string(),
            ));
        }

        session.our_confirmed = true;

        if session.our_confirmed && session.peer_confirmed {
            let paired_dev = PairedDevice {
                device_id: session.peer_info.device_id.clone(),
                device_name: session.peer_info.device_name.clone(),
                public_key_hex: session.peer_info.public_key_hex.clone(),
                fingerprint: session.peer_info.fingerprint.clone(),
                paired_at: Utc::now().to_rfc3339(),
                last_seen_at: Some(Utc::now().to_rfc3339()),
                last_ip: None,
            };
            if let Err(e) = store.add_or_update(paired_dev.clone()) {
                session.status = PairingStatus::Rejected;
                return Err(CoreError::Pairing(format!("Failed to persist cryptographic trust store: {}", e)));
            }
            session.status = PairingStatus::Completed;
            crate::core::network::get_connection_manager()
                .register_session(&paired_dev.device_id, session.session_key);
            return Ok(Some(paired_dev));
        }

        Ok(None)
    }

    /// Records remote peer confirmation received over authenticated transport
    pub fn confirm_remote(
        &self,
        peer_sas_code: &str,
        store: &mut PairedDeviceStore,
    ) -> Result<Option<PairedDevice>, CoreError> {
        let mut session_lock = self.current_session.lock().map_err(|_| {
            CoreError::Pairing("Internal pairing mutex poison error".to_string())
        })?;

        let session = session_lock
            .as_mut()
            .ok_or_else(|| CoreError::Pairing("No active pairing session awaiting confirmation".to_string()))?;

        if session.status != PairingStatus::AwaitingDualConfirmation {
            return Err(CoreError::Pairing(format!(
                "Pairing session is not awaiting confirmation (status: {:?})",
                session.status
            )));
        }

        if session.expires_at < Instant::now() {
            session.status = PairingStatus::Expired;
            return Err(CoreError::Pairing("Pairing session expired".to_string()));
        }

        if !peer_sas_code.trim().eq_ignore_ascii_case(session.sas_code.trim()) {
            session.status = PairingStatus::Rejected;
            return Err(CoreError::Pairing("Remote confirmation code did not match session SAS".to_string()));
        }

        session.peer_confirmed = true;

        if session.our_confirmed && session.peer_confirmed {
            let paired_dev = PairedDevice {
                device_id: session.peer_info.device_id.clone(),
                device_name: session.peer_info.device_name.clone(),
                public_key_hex: session.peer_info.public_key_hex.clone(),
                fingerprint: session.peer_info.fingerprint.clone(),
                paired_at: Utc::now().to_rfc3339(),
                last_seen_at: Some(Utc::now().to_rfc3339()),
                last_ip: None,
            };
            if let Err(e) = store.add_or_update(paired_dev.clone()) {
                session.status = PairingStatus::Rejected;
                return Err(CoreError::Pairing(format!("Failed to persist cryptographic trust store: {}", e)));
            }
            session.status = PairingStatus::Completed;
            crate::core::network::get_connection_manager()
                .register_session(&paired_dev.device_id, session.session_key);
            return Ok(Some(paired_dev));
        }

        Ok(None)
    }

    /// Rejects the active pairing session
    pub fn reject(&self) -> Result<(), CoreError> {
        // Clear active invitation so no further joiners can connect
        let _ = self.active_invitation.lock().map(|mut g| *g = None);
        let _ = self.invitation_created_at.lock().map(|mut g| *g = None);

        let mut session_lock = self.current_session.lock().map_err(|_| {
            CoreError::Pairing("Internal pairing mutex poison error".to_string())
        })?;

        if let Some(ref mut session) = *session_lock {
            session.status = PairingStatus::Rejected;
        }

        Ok(())
    }
}

static GLOBAL_PAIRING_MANAGER: OnceLock<PairingManager> = OnceLock::new();

pub fn get_pairing_manager() -> &'static PairingManager {
    GLOBAL_PAIRING_MANAGER.get_or_init(PairingManager::new)
}
