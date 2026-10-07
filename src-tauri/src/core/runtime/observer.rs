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
}
