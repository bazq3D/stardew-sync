pub mod storage;

use std::fs;
use std::path::Path;
use chrono::Utc;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use crate::core::errors::CoreError;

pub const IDENTITY_FILE_JSON: &str = "device_identity.json";
pub const IDENTITY_FILE_ENC: &str = "device_identity.enc";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicDeviceInfo {
    pub device_id: String,
    pub fingerprint: String,
    pub device_name: String,
    pub public_key_hex: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredPublicMetadata {
    pub device_id: String,
    pub device_name: String,
    pub public_key_hex: String,
    pub created_at: String,
}

#[derive(Clone)]
pub struct DeviceIdentity {
    signing_key: SigningKey,
    device_name: String,
    created_at: String,
}

impl std::fmt::Debug for DeviceIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceIdentity")
            .field("device_id", &self.device_id())
            .field("fingerprint", &self.fingerprint())
            .field("device_name", &self.device_name)
            .field("created_at", &self.created_at)
            .finish_non_exhaustive()
    }
}

impl DeviceIdentity {
    /// Formats a SHA-256 hash or hex string into a human-verifiable grouped fingerprint
    pub fn format_fingerprint(public_key: &VerifyingKey) -> String {
        let mut hasher = Sha256::new();
        hasher.update(public_key.as_bytes());
        let digest = hasher.finalize();
        let hex_str = hex::encode(digest);
        // Take first 16 characters in 4 groups: e.g. "a1b2-c3d4-e5f6-7890"
        format!(
            "{}-{}-{}-{}",
            &hex_str[0..4],
            &hex_str[4..8],
            &hex_str[8..12],
            &hex_str[12..16]
        )
    }

    /// Computes full 64-char hex device_id from public key
    pub fn compute_device_id(public_key: &VerifyingKey) -> String {
        let mut hasher = Sha256::new();
        hasher.update(public_key.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Generates a new identity using OS cryptographically secure randomness
    pub fn generate(device_name: &str) -> Self {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        Self {
            signing_key,
            device_name: device_name.to_string(),
            created_at: Utc::now().to_rfc3339(),
        }
    }

    /// Creates a deterministic synthetic identity for unit tests only
    pub fn from_seed(seed: &[u8; 32], device_name: &str) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        Self {
            signing_key,
            device_name: device_name.to_string(),
            created_at: "2026-10-10T00:00:00Z".to_string(),
        }
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    pub fn device_id(&self) -> String {
        Self::compute_device_id(&self.verifying_key())
    }

    pub fn fingerprint(&self) -> String {
        Self::format_fingerprint(&self.verifying_key())
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn created_at(&self) -> &str {
        &self.created_at
    }

    pub fn public_info(&self) -> PublicDeviceInfo {
        let vk = self.verifying_key();
        PublicDeviceInfo {
            device_id: self.device_id(),
            fingerprint: self.fingerprint(),
            device_name: self.device_name.clone(),
            public_key_hex: hex::encode(vk.as_bytes()),
            created_at: self.created_at.clone(),
        }
    }

    /// Cryptographically signs a message payload
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let signature = self.signing_key.sign(message);
        signature.to_bytes().to_vec()
    }

    /// Verifies a signature against a public key
    pub fn verify(
        verifying_key: &VerifyingKey,
        message: &[u8],
        signature_bytes: &[u8],
    ) -> Result<(), CoreError> {
        if signature_bytes.len() != 64 {
            return Err(CoreError::Identity("Invalid signature length".to_string()));
        }
        let signature = Signature::from_slice(signature_bytes)
            .map_err(|e| CoreError::Identity(format!("Malformed signature: {}", e)))?;
        verifying_key
            .verify(message, &signature)
            .map_err(|e| CoreError::Identity(format!("Cryptographic signature verification failed: {}", e)))
    }

    /// Loads the persistent device identity, or generates one if it does not exist yet.
    /// Fails closed on corruption without silently trusting a fresh key.
    pub fn load_or_create() -> Result<Self, CoreError> {
        let dir = storage::get_identity_dir();
        fs::create_dir_all(&dir)?;

        let json_path = dir.join(IDENTITY_FILE_JSON);
        let enc_path = dir.join(IDENTITY_FILE_ENC);

        let json_exists = json_path.exists();
        let enc_exists = enc_path.exists();

        match (json_exists, enc_exists) {
            (true, true) => {
                // Load existing identity
                let json_data = fs::read_to_string(&json_path)?;
                let metadata: StoredPublicMetadata = serde_json::from_str(&json_data)?;

                let enc_data = fs::read(&enc_path)?;
                let decrypted_seed = storage::unprotect_secret(&enc_data)?;

                if decrypted_seed.len() != 32 {
                    return Err(CoreError::Identity(
                        "Device identity corrupted: decrypted key seed length is invalid. Refusing to overwrite.".to_string(),
                    ));
                }

                let mut seed_bytes = [0u8; 32];
                seed_bytes.copy_from_slice(&decrypted_seed);
                let signing_key = SigningKey::from_bytes(&seed_bytes);
                let derived_vk = signing_key.verifying_key();
                let derived_hex = hex::encode(derived_vk.as_bytes());

                if derived_hex != metadata.public_key_hex {
                    return Err(CoreError::Identity(
                        "Device identity corrupted: private key does not match stored public metadata. Refusing to overwrite.".to_string(),
                    ));
                }

                Ok(Self {
                    signing_key,
                    device_name: metadata.device_name,
                    created_at: metadata.created_at,
                })
            }
            (true, false) => Err(CoreError::Identity(
                "Device identity corrupted: public metadata exists but encrypted private key is missing. Refusing to silently generate new key.".to_string(),
            )),
            (false, true) => Err(CoreError::Identity(
                "Device identity corrupted: encrypted private key exists without public metadata. Refusing to silently generate new key.".to_string(),
            )),
            (false, false) => {
                // Fresh initialization
                let default_name = std::env::var("COMPUTERNAME")
                    .or_else(|_| std::env::var("HOSTNAME"))
                    .unwrap_or_else(|_| "Stardew-Sync-PC".to_string());

                let identity = Self::generate(&default_name);
                identity.save_to_dir(&dir)?;
                Ok(identity)
            }
        }
    }

    /// Saves the identity files into the target directory
    pub fn save_to_dir(&self, dir: &Path) -> Result<(), CoreError> {
        fs::create_dir_all(dir)?;
        let json_path = dir.join(IDENTITY_FILE_JSON);
        let enc_path = dir.join(IDENTITY_FILE_ENC);

        let metadata = StoredPublicMetadata {
            device_id: self.device_id(),
            device_name: self.device_name.clone(),
            public_key_hex: hex::encode(self.verifying_key().as_bytes()),
            created_at: self.created_at.clone(),
        };

        // DPAPI encrypt private key seed
        let seed = self.signing_key.to_bytes();
        let encrypted = storage::protect_secret(&seed)?;

        // Write encrypted key first, then metadata
        fs::write(&enc_path, &encrypted)?;
        let json_str = serde_json::to_string_pretty(&metadata)?;
        fs::write(&json_path, json_str)?;

        Ok(())
    }

    /// Updates the local device display name
    pub fn set_device_name(&mut self, new_name: &str) -> Result<(), CoreError> {
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            return Err(CoreError::Identity("Device name cannot be empty".to_string()));
        }
        if trimmed.len() > 64 {
            return Err(CoreError::Identity("Device name cannot exceed 64 characters".to_string()));
        }

        self.device_name = trimmed.to_string();
        let dir = storage::get_identity_dir();
        self.save_to_dir(&dir)?;
        Ok(())
    }
}
