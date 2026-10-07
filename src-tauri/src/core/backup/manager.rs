use crate::core::backup::manifest::{BackupFileEntry, BackupManifest, BackupType};
use crate::core::errors::CoreError;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct BackupManager;

impl BackupManager {
    /// Creates a verified local backup of a save directory.
    ///
    /// GUARANTEE: The backup is immediately verified against SHA-256 hashes before
    /// this function returns Ok.
    pub fn create_backup(
        save_dir: &Path,
        backups_root: &Path,
        backup_type: BackupType,
        is_protected: bool,
    ) -> Result<BackupManifest, CoreError> {
        if !save_dir.is_dir() {
            return Err(CoreError::Backup(format!(
                "Source save directory does not exist: {:?}",
                save_dir
            )));
        }

        let folder_name = save_dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| CoreError::Backup("Invalid save directory name".to_string()))?;

        let parts: Vec<&str> = folder_name.rsplitn(2, '_').collect();
        let (game_id, farm_name) = if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            ("0".to_string(), folder_name.to_string())
        };

        let backup_id = Uuid::new_v4().to_string();
        let target_backup_dir = backups_root.join(folder_name).join(&backup_id);
        std::fs::create_dir_all(&target_backup_dir)?;

        let mut files = Vec::new();
        let mut overall_hasher = Sha256::new();

        // Copy and hash each file in save_dir
        for entry in std::fs::read_dir(save_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
                let content = std::fs::read(&path)?;

                let mut file_hasher = Sha256::new();
                file_hasher.update(&content);
                let hash = format!("{:x}", file_hasher.finalize());

                overall_hasher.update(&content);
                overall_hasher.update(file_name.as_bytes());

                let dest_path = target_backup_dir.join(&file_name);
                std::fs::write(&dest_path, &content)?;

                files.push(BackupFileEntry {
                    relative_path: file_name,
                    sha256_hash: hash,
                    size_bytes: content.len() as u64,
                });
            }
        }

        files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

        let package_hash = format!("{:x}", overall_hasher.finalize());

        let manifest = BackupManifest {
            backup_id,
            created_at: Utc::now(),
            backup_type: backup_type.clone(),
            farm_name,
            game_id,
            source_directory: save_dir.to_string_lossy().to_string(),
            is_protected: is_protected || backup_type == BackupType::OriginalImport,
            files,
            package_hash,
        };

        // Write manifest
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        std::fs::write(target_backup_dir.join("manifest.json"), manifest_json)?;

        // Immediately verify the created backup
        Self::verify_backup(&target_backup_dir)?;

        Ok(manifest)
    }

    /// Verifies the integrity of an existing backup directory against its manifest.
    pub fn verify_backup(backup_dir: &Path) -> Result<BackupManifest, CoreError> {
        let manifest_path = backup_dir.join("manifest.json");
        if !manifest_path.is_file() {
            return Err(CoreError::Backup(format!(
                "Manifest not found in backup directory: {:?}",
                backup_dir
            )));
        }

        let manifest_content = std::fs::read_to_string(&manifest_path)?;
        let manifest: BackupManifest = serde_json::from_str(&manifest_content)?;

        for file_entry in &manifest.files {
            let file_path = backup_dir.join(&file_entry.relative_path);
            if !file_path.is_file() {
                return Err(CoreError::Backup(format!(
                    "Missing file in backup: {:?}",
                    file_path
                )));
            }

            let content = std::fs::read(&file_path)?;
            let mut hasher = Sha256::new();
            hasher.update(&content);
            let actual_hash = format!("{:x}", hasher.finalize());

            if actual_hash != file_entry.sha256_hash {
                return Err(CoreError::BackupVerificationFailed {
                    file: file_entry.relative_path.clone(),
                    expected: file_entry.sha256_hash.clone(),
                    actual: actual_hash,
                });
            }
        }

        Ok(manifest)
    }

    /// Restores a verified backup into a temporary/staging directory.
    pub fn restore_backup_to_staging(
        backup_dir: &Path,
        staging_dir: &Path,
    ) -> Result<(), CoreError> {
        let manifest = Self::verify_backup(backup_dir)?;

        if staging_dir.exists() {
            std::fs::remove_dir_all(staging_dir)?;
        }
        std::fs::create_dir_all(staging_dir)?;

        for file_entry in &manifest.files {
            let src = backup_dir.join(&file_entry.relative_path);
            let dst = staging_dir.join(&file_entry.relative_path);
            std::fs::copy(&src, &dst)?;
        }

        Ok(())
    }

    /// Enforces retention limits for unprotected backups.
    ///
    /// GUARANTEE: Never deletes backups marked `is_protected = true`.
    pub fn enforce_retention(
        farm_backups_dir: &Path,
        max_unprotected: usize,
    ) -> Result<usize, CoreError> {
        if !farm_backups_dir.exists() {
            return Ok(0);
        }

        let mut unprotected_backups: Vec<(DateTime<Utc>, PathBuf, String)> = Vec::new();

        for entry in std::fs::read_dir(farm_backups_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Ok(manifest) = Self::verify_backup(&path) {
                    if !manifest.is_protected {
                        unprotected_backups.push((manifest.created_at, path, manifest.backup_id));
                    }
                }
            }
        }

        // Sort oldest first
        unprotected_backups.sort_by_key(|(dt, _, _)| *dt);

        let mut deleted_count = 0;
        if unprotected_backups.len() > max_unprotected {
            let excess = unprotected_backups.len() - max_unprotected;
            for (_, path, _) in unprotected_backups.into_iter().take(excess) {
                std::fs::remove_dir_all(path)?;
                deleted_count += 1;
            }
        }

        Ok(deleted_count)
    }
}
