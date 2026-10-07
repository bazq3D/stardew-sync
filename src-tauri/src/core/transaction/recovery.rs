use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::core::errors::CoreError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionState {
    Prepared,
    Staged,
    Swapping,
    Committed,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryManifest {
    pub tx_id: String,
    pub created_at: DateTime<Utc>,
    pub state: TransactionState,
    pub live_path: PathBuf,
    pub staging_path: PathBuf,
    pub rollback_path: Option<PathBuf>,
}

pub struct RecoveryManager;

impl RecoveryManager {
    /// Writes an active transaction recovery manifest to disk.
    pub fn write_manifest(manifest_path: &Path, manifest: &RecoveryManifest) -> Result<(), CoreError> {
        if let Some(parent) = manifest_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(manifest)?;
        std::fs::write(manifest_path, json)?;
        Ok(())
    }

    /// Reads an active transaction recovery manifest from disk if one exists.
    pub fn read_manifest(manifest_path: &Path) -> Result<Option<RecoveryManifest>, CoreError> {
        if !manifest_path.is_file() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(manifest_path)?;
        let manifest: RecoveryManifest = serde_json::from_str(&content)?;
        Ok(Some(manifest))
    }

    /// Attempts automatic recovery if a previous process was interrupted mid-transaction.
    pub fn recover_interrupted_transaction(manifest_path: &Path) -> Result<bool, CoreError> {
        let manifest = match Self::read_manifest(manifest_path)? {
            Some(m) => m,
            None => return Ok(false),
        };

        let mut recovered = false;

        // If transaction was in Swapping state and a rollback path exists
        if manifest.state == TransactionState::Swapping {
            if let Some(ref rollback_path) = manifest.rollback_path {
                if rollback_path.exists() && !manifest.live_path.exists() {
                    // Live path was missing, restore from rollback backup
                    std::fs::rename(rollback_path, &manifest.live_path)?;
                    recovered = true;
                } else if rollback_path.exists() && manifest.live_path.exists() {
                    // Both exist: live path is intact, remove lingering rollback backup
                    std::fs::remove_dir_all(rollback_path)?;
                    recovered = true;
                }
            }
        }

        // Clean up temporary staging directory if it exists
        if manifest.staging_path.exists() {
            let _ = std::fs::remove_dir_all(&manifest.staging_path);
        }

        // Remove manifest
        let _ = std::fs::remove_file(manifest_path);

        Ok(recovered)
    }
}
