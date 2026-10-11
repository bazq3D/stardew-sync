use std::fs;
use std::path::PathBuf;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use crate::core::errors::CoreError;

pub const PAIRED_DEVICES_FILE: &str = "paired_devices.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairedDevice {
    pub device_id: String,
    pub device_name: String,
    pub public_key_hex: String,
    pub fingerprint: String,
    pub paired_at: String,
    pub last_seen_at: Option<String>,
    pub last_ip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PairedDevicesManifest {
    pub schema_version: u32,
    pub updated_at: String,
    pub devices: Vec<PairedDevice>,
}

pub struct PairedDeviceStore {
    manifest: PairedDevicesManifest,
    storage_dir: PathBuf,
}

impl PairedDeviceStore {
    pub fn get_devices_dir() -> PathBuf {
        if let Ok(custom) = std::env::var("STARDEW_SYNC_DEVICES_DIR") {
            return PathBuf::from(custom);
        }

        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            PathBuf::from(local_app_data).join("StardewSync").join("devices")
        } else if let Ok(app_data) = std::env::var("APPDATA") {
            PathBuf::from(app_data).join("StardewSync").join("devices")
        } else {
            PathBuf::from(".").join("StardewSync").join("devices")
        }
    }

    pub fn load() -> Result<Self, CoreError> {
        let dir = Self::get_devices_dir();
        fs::create_dir_all(&dir)?;
        let file_path = dir.join(PAIRED_DEVICES_FILE);

        if !file_path.exists() {
            let manifest = PairedDevicesManifest {
                schema_version: 1,
                updated_at: Utc::now().to_rfc3339(),
                devices: Vec::new(),
            };
            return Ok(Self {
                manifest,
                storage_dir: dir,
            });
        }

        let content = fs::read_to_string(&file_path)?;
        let manifest: PairedDevicesManifest = serde_json::from_str(&content)
            .map_err(|e| CoreError::Pairing(format!("Failed to parse paired devices manifest: {}", e)))?;

        Ok(Self {
            manifest,
            storage_dir: dir,
        })
    }

    pub fn save(&self) -> Result<(), CoreError> {
        fs::create_dir_all(&self.storage_dir)?;
        let file_path = self.storage_dir.join(PAIRED_DEVICES_FILE);
        let content = serde_json::to_string_pretty(&self.manifest)?;
        fs::write(&file_path, content)?;
        Ok(())
    }

    pub fn list(&self) -> Vec<PairedDevice> {
        self.manifest.devices.clone()
    }

    pub fn get(&self, device_id: &str) -> Option<&PairedDevice> {
        self.manifest.devices.iter().find(|d| d.device_id == device_id)
    }

    /// Checks if a device is trusted and validates cryptographic key pinning
    pub fn is_trusted(&self, device_id: &str, public_key_hex: &str) -> bool {
        if let Some(dev) = self.get(device_id) {
            dev.public_key_hex.eq_ignore_ascii_case(public_key_hex)
        } else {
            false
        }
    }

    pub fn add_or_update(&mut self, device: PairedDevice) -> Result<(), CoreError> {
        // Enforce key pinning: if device_id exists with a DIFFERENT public key, reject modification!
        if let Some(existing) = self.manifest.devices.iter().find(|d| d.device_id == device.device_id) {
            if !existing.public_key_hex.eq_ignore_ascii_case(&device.public_key_hex) {
                return Err(CoreError::Pairing(format!(
                    "Security Violation: Key conflict for device {}. Public key mismatch with pinned record.",
                    device.device_id
                )));
            }
        }

        self.manifest.devices.retain(|d| d.device_id != device.device_id);
        self.manifest.devices.push(device);
        self.manifest.updated_at = Utc::now().to_rfc3339();
        self.save()
    }

    pub fn update_last_seen(&mut self, device_id: &str, ip: Option<String>) -> Result<(), CoreError> {
        if let Some(dev) = self.manifest.devices.iter_mut().find(|d| d.device_id == device_id) {
            dev.last_seen_at = Some(Utc::now().to_rfc3339());
            if ip.is_some() {
                dev.last_ip = ip;
            }
            self.manifest.updated_at = Utc::now().to_rfc3339();
            self.save()?;
        }
        Ok(())
    }

    pub fn remove(&mut self, device_id: &str) -> Result<bool, CoreError> {
        let initial_len = self.manifest.devices.len();
        self.manifest.devices.retain(|d| d.device_id != device_id);
        let removed = self.manifest.devices.len() < initial_len;
        if removed {
            self.manifest.updated_at = Utc::now().to_rfc3339();
            self.save()?;
        }
        Ok(removed)
    }
}
