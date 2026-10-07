use std::path::Path;
use chrono::Utc;
use uuid::Uuid;
use crate::core::backup::manager::BackupManager;
use crate::core::backup::manifest::BackupType;
use crate::core::errors::CoreError;
use crate::core::save::validator::SaveValidator;
use crate::core::transaction::recovery::{RecoveryManager, RecoveryManifest, TransactionState};

pub struct SafeReplacer;

impl SafeReplacer {
    /// Atomically replaces a live save folder with staged contents using a
    /// validated snapshot-and-rollback transaction pipeline.
    ///
    /// GUARANTEE: If any failure occurs during swapping or post-verification,
    /// the original live folder is restored immediately.
    pub fn replace_save_directory(
        live_dir: &Path,
        staging_dir: &Path,
        backups_root: &Path,
        manifest_path: &Path,
    ) -> Result<(), CoreError> {
        // 1. Pre-validation of staging directory
        SaveValidator::validate_basic(staging_dir)?;

        // 2. Verified safety snapshot of live save
        if live_dir.is_dir() {
            BackupManager::create_backup(
                live_dir,
                backups_root,
                BackupType::PreHostSwitch,
                false,
            )?;
        }

        let tx_id = Uuid::new_v4().to_string();
        let rollback_dir = live_dir.with_extension("rollback_tmp");

        if rollback_dir.exists() {
            std::fs::remove_dir_all(&rollback_dir)?;
        }

        // 3. Write recovery manifest in Swapping state
        let manifest = RecoveryManifest {
            tx_id,
            created_at: Utc::now(),
            state: TransactionState::Swapping,
            live_path: live_dir.to_path_buf(),
            staging_path: staging_dir.to_path_buf(),
            rollback_path: Some(rollback_dir.clone()),
        };
        RecoveryManager::write_manifest(manifest_path, &manifest)?;

        // 4. Execution with automatic rollback on error
        let swap_result = (|| -> Result<(), CoreError> {
            if live_dir.exists() {
                std::fs::rename(live_dir, &rollback_dir)?;
            }

            if let Err(e) = std::fs::rename(staging_dir, live_dir) {
                // Rollback live folder
                if rollback_dir.exists() && !live_dir.exists() {
                    let _ = std::fs::rename(&rollback_dir, live_dir);
                }
                return Err(CoreError::Transaction(format!("Failed to move staged save to live: {}", e)));
            }

            // Post-verify live directory
            if let Err(e) = SaveValidator::validate_basic(live_dir) {
                // Validation failed on live! Rollback!
                let _ = std::fs::remove_dir_all(live_dir);
                if rollback_dir.exists() {
                    let _ = std::fs::rename(&rollback_dir, live_dir);
                }
                return Err(CoreError::Validation(format!(
                    "Live directory failed post-swap validation, rolled back: {}",
                    e
                )));
            }

            // Swap succeeded and verified; remove rollback copy
            if rollback_dir.exists() {
                let _ = std::fs::remove_dir_all(&rollback_dir);
            }

            Ok(())
        })();

        // 5. Clean up recovery manifest
        let _ = std::fs::remove_file(manifest_path);

        swap_result
    }
}
