use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, SystemTime};
use crate::core::errors::CoreError;
use crate::core::monitoring::fingerprint::FingerprintGenerator;
use crate::core::monitoring::types::{
    SaveChangeClassification, SaveFingerprint, ValidationStatus,
};

#[derive(Debug, Clone)]
pub struct SettleConfig {
    pub quiet_period: Duration,
    pub max_retries: usize,
    pub retry_backoff: Duration,
}

impl Default for SettleConfig {
    fn default() -> Self {
        Self {
            quiet_period: Duration::from_millis(2000), // 2.0s settle quiet period
            max_retries: 3,
            retry_backoff: Duration::from_millis(400),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileMetaSnapshot {
    len: u64,
    modified: Option<SystemTime>,
}

pub struct SettleAndVerifyPipeline;

impl SettleAndVerifyPipeline {
    /// Executes the full settle-and-verify pipeline on a farm directory.
    ///
    /// Evaluates game process presence, waits for filesystem write quiet periods,
    /// validates pre- and post-read metadata consistency, checks XML well-formedness,
    /// and classifies the resulting state against any known prior fingerprint.
    pub fn execute(
        farm_folder: &str,
        farm_dir: &Path,
        source_type: &str,
        is_game_running: bool,
        previous_aggregate_hash: Option<&str>,
        config: &SettleConfig,
    ) -> Result<(SaveFingerprint, SaveChangeClassification), CoreError> {
        FingerprintGenerator::validate_farm_identifier(farm_folder)?;

        // 1. Source existence check
        if !farm_dir.exists() || !farm_dir.is_dir() {
            let fp = SaveFingerprint {
                farm_id: farm_folder.to_string(),
                source_type: source_type.to_string(),
                files: Vec::new(),
                aggregate_hash: String::new(),
                observed_at: chrono::Utc::now(),
                in_game_metadata: None,
                validation_status: ValidationStatus::MissingFiles("Farm directory does not exist".to_string()),
                validation_message: Some("Farm folder missing on disk".to_string()),
            };
            return Ok((fp, SaveChangeClassification::SourceMissing));
        }

        // 2. Game Running Gate: Defer finalization if Stardew Valley is active
        if is_game_running {
            let fp = FingerprintGenerator::compute_fingerprint(farm_folder, farm_dir, source_type)?;
            return Ok((fp, SaveChangeClassification::WaitingForGameClosure));
        }

        // 3. Stabilization & Settle Loop with Bounded Retries
        let mut attempts = 0;
        let mut last_fingerprint = None;

        while attempts < config.max_retries {
            attempts += 1;

            // Sample snapshot of file attributes across the directory
            let snapshot_before = Self::take_meta_snapshot(farm_dir, farm_folder);

            // If required files don't exist yet, wait and retry
            let primary_exists = snapshot_before.contains_key(farm_folder);
            let info_exists = snapshot_before.contains_key("SaveGameInfo");

            if !primary_exists || !info_exists {
                if attempts < config.max_retries {
                    sleep(config.retry_backoff);
                    continue;
                } else {
                    let fp = FingerprintGenerator::compute_fingerprint(farm_folder, farm_dir, source_type)?;
                    return Ok((fp, SaveChangeClassification::InvalidOrIncompleteSave));
                }
            }

            // Quiet period wait
            if config.quiet_period > Duration::from_millis(0) {
                sleep(config.quiet_period);
            }

            let snapshot_after = Self::take_meta_snapshot(farm_dir, farm_folder);

            // If file attributes shifted during quiet period, files are actively being flushed
            if snapshot_before != snapshot_after {
                if attempts < config.max_retries {
                    sleep(config.retry_backoff);
                    continue;
                } else {
                    let fp = FingerprintGenerator::compute_fingerprint(farm_folder, farm_dir, source_type)?;
                    return Ok((fp, SaveChangeClassification::WaitingForFileStability));
                }
            }

            // Compute fingerprint and check metadata consistency during read
            let fp = FingerprintGenerator::compute_fingerprint(farm_folder, farm_dir, source_type)?;

            // Re-verify attributes after computation
            let snapshot_post_read = Self::take_meta_snapshot(farm_dir, farm_folder);
            if snapshot_after != snapshot_post_read {
                // File modified mid-read; retry
                if attempts < config.max_retries {
                    sleep(config.retry_backoff);
                    continue;
                }
            }

            last_fingerprint = Some(fp);
            break;
        }

        let fingerprint = match last_fingerprint {
            Some(fp) => fp,
            None => FingerprintGenerator::compute_fingerprint(farm_folder, farm_dir, source_type)?,
        };

        // 4. Validate Structural Correctness
        match &fingerprint.validation_status {
            ValidationStatus::Valid => (),
            ValidationStatus::MalformedXml(_)
            | ValidationStatus::MissingFiles(_)
            | ValidationStatus::SizeMismatch(_)
            | ValidationStatus::ReadError(_) => {
                return Ok((fingerprint, SaveChangeClassification::InvalidOrIncompleteSave));
            }
            ValidationStatus::UnsupportedSource(_) => {
                return Ok((fingerprint, SaveChangeClassification::MonitoringError));
            }
        }

        // 5. Change Classification relative to previous observation
        let classification = match previous_aggregate_hash {
            Some(prev_hash) if prev_hash == fingerprint.aggregate_hash => {
                SaveChangeClassification::Unchanged
            }
            Some(_) => SaveChangeClassification::LocalChangeDetected,
            None => SaveChangeClassification::LocalChangeDetected, // Initial verified baseline
        };

        Ok((fingerprint, classification))
    }

    /// Captures file metadata for relevant save components in the folder.
    fn take_meta_snapshot(farm_dir: &Path, farm_folder: &str) -> BTreeMap<String, FileMetaSnapshot> {
        let mut map = BTreeMap::new();
        let targets = [
            farm_folder.to_string(),
            "SaveGameInfo".to_string(),
            format!("{}_old", farm_folder),
            "SaveGameInfo_old".to_string(),
        ];

        for name in &targets {
            let path = farm_dir.join(name);
            if path.is_file() {
                if let Ok(meta) = fs::metadata(&path) {
                    map.insert(
                        name.clone(),
                        FileMetaSnapshot {
                            len: meta.len(),
                            modified: meta.modified().ok(),
                        },
                    );
                }
            }
        }

        map
    }
}
