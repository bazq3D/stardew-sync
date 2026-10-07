use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use crate::core::errors::CoreError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileObservationSnapshot {
    pub file_name: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub mtime_epoch_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirectoryObservationSnapshot {
    pub dir_path: String,
    pub timestamp: DateTime<Utc>,
    pub files: BTreeMap<String, FileObservationSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationDelta {
    pub unchanged_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub new_files: Vec<String>,
    pub deleted_files: Vec<String>,
    pub external_cloud_activity_suspected: bool,
}

pub struct CloudObserver;

impl CloudObserver {
    /// Takes an immutable observation snapshot of the given directory.
    pub fn take_snapshot(dir: &Path) -> Result<DirectoryObservationSnapshot, CoreError> {
        let mut files = BTreeMap::new();

        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let metadata = entry.metadata()?;
                    let size_bytes = metadata.len();

                    let mtime_epoch_secs = metadata
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);

                    let content = fs::read(entry.path())?;
                    let mut hasher = Sha256::new();
                    hasher.update(&content);
                    let sha256 = format!("{:x}", hasher.finalize());

                    files.insert(
                        file_name.clone(),
                        FileObservationSnapshot {
                            file_name,
                            size_bytes,
                            sha256,
                            mtime_epoch_secs,
                        },
                    );
                }
            }
        }

        Ok(DirectoryObservationSnapshot {
            dir_path: dir.to_string_lossy().to_string(),
            timestamp: Utc::now(),
            files,
        })
    }

    /// Compares pre-launch and post-exit snapshots to characterize all filesystem alterations.
    pub fn compare_snapshots(
        before: &DirectoryObservationSnapshot,
        after: &DirectoryObservationSnapshot,
    ) -> ObservationDelta {
        let mut unchanged_files = Vec::new();
        let mut modified_files = Vec::new();
        let mut new_files = Vec::new();
        let mut deleted_files = Vec::new();

        for (name, b_file) in &before.files {
            match after.files.get(name) {
                Some(a_file) => {
                    if b_file.sha256 == a_file.sha256 {
                        unchanged_files.push(name.clone());
                    } else {
                        modified_files.push(name.clone());
                    }
                }
                None => {
                    deleted_files.push(name.clone());
                }
            }
        }

        for name in after.files.keys() {
            if !before.files.contains_key(name) {
                new_files.push(name.clone());
            }
        }

        // Suspicious activity heuristic: if non-save files were injected or changed
        let external_cloud_activity_suspected = new_files
            .iter()
            .any(|f| f.ends_with(".tmp") || f.contains("cloud") || f.contains("sync"));

        ObservationDelta {
            unchanged_files,
            modified_files,
            new_files,
            deleted_files,
            external_cloud_activity_suspected,
        }
    }

    /// Verifies that the files in `actual_dir` match the `expected_snapshot`.
    /// If an optional `stale_reference` is supplied (e.g. State A when expecting State B),
    /// checks whether live files were replaced by that specific older generation (e.g. Xbox WGS restore).
    pub fn verify_generation_integrity(
        actual_dir: &Path,
        expected_snapshot: &DirectoryObservationSnapshot,
        expected_label: &str,
        stale_reference: Option<(&DirectoryObservationSnapshot, &str)>,
    ) -> Result<GenerationIntegrityStatus, CoreError> {
        let actual_snapshot = Self::take_snapshot(actual_dir)?;

        // First check: does actual match expected?
        let mut differing_files = Vec::new();
        let mut missing_files = Vec::new();

        for (name, exp_file) in &expected_snapshot.files {
            match actual_snapshot.files.get(name) {
                Some(act_file) => {
                    if exp_file.sha256 != act_file.sha256 {
                        differing_files.push(name.clone());
                    }
                }
                None => {
                    missing_files.push(name.clone());
                }
            }
        }

        if differing_files.is_empty() && missing_files.is_empty() {
            return Ok(GenerationIntegrityStatus::Verified);
        }

        // Second check: if differing, does it match the known stale reference?
        if let Some((stale_snapshot, stale_label)) = stale_reference {
            let mut matches_stale = true;
            let mut matched_files = Vec::new();

            for (name, stale_file) in &stale_snapshot.files {
                match actual_snapshot.files.get(name) {
                    Some(act_file) if act_file.sha256 == stale_file.sha256 => {
                        matched_files.push(name.clone());
                    }
                    _ => {
                        matches_stale = false;
                        break;
                    }
                }
            }

            if matches_stale && !matched_files.is_empty() {
                return Ok(GenerationIntegrityStatus::RollbackDetected {
                    expected_generation: expected_label.to_string(),
                    restored_stale_generation: stale_label.to_string(),
                    matched_files,
                });
            }
        }

        Ok(GenerationIntegrityStatus::ExternalReplacementDetected {
            expected_generation: expected_label.to_string(),
            differing_files,
            missing_files,
        })
    }

    /// Asserts that no external save replacement or rollback has occurred.
    /// Returns Err(CoreError::ExternalSaveReplacement) if a mismatch is detected.
    pub fn assert_no_external_replacement(
        actual_dir: &Path,
        expected_snapshot: &DirectoryObservationSnapshot,
        expected_label: &str,
        stale_reference: Option<(&DirectoryObservationSnapshot, &str)>,
    ) -> Result<(), CoreError> {
        let status = Self::verify_generation_integrity(
            actual_dir,
            expected_snapshot,
            expected_label,
            stale_reference,
        )?;

        match status {
            GenerationIntegrityStatus::Verified => Ok(()),
            GenerationIntegrityStatus::RollbackDetected {
                expected_generation,
                restored_stale_generation,
                matched_files,
            } => Err(CoreError::ExternalSaveReplacement {
                expected: expected_generation,
                actual: restored_stale_generation,
                detail: format!(
                    "Platform/WGS rollback detected: files restored to older generation. Matched files: {:?}",
                    matched_files
                ),
            }),
            GenerationIntegrityStatus::ExternalReplacementDetected {
                expected_generation,
                differing_files,
                missing_files,
            } => Err(CoreError::ExternalSaveReplacement {
                expected: expected_generation,
                actual: "Unknown / External".to_string(),
                detail: format!(
                    "Files differed or missing from expected state. Differing: {:?}, Missing: {:?}",
                    differing_files, missing_files
                ),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GenerationIntegrityStatus {
    /// Live files match the expected generation exactly
    Verified,
    /// Live files match a known older/stale generation (e.g. Xbox WGS restored State A over State B)
    RollbackDetected {
        expected_generation: String,
        restored_stale_generation: String,
        matched_files: Vec<String>,
    },
    /// Live files differ from expected and do not match any known generation
    ExternalReplacementDetected {
        expected_generation: String,
        differing_files: Vec<String>,
        missing_files: Vec<String>,
    },
}
