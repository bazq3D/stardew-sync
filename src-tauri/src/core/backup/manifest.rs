use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackupType {
    OriginalImport,
    PreHostSwitch,
    PreSync,
    PostSession,
    Manual,
}

impl std::fmt::Display for BackupType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupType::OriginalImport => write!(f, "ORIGINAL_IMPORT"),
            BackupType::PreHostSwitch => write!(f, "PRE_HOST_SWITCH"),
            BackupType::PreSync => write!(f, "PRE_SYNC"),
            BackupType::PostSession => write!(f, "POST_SESSION"),
            BackupType::Manual => write!(f, "MANUAL"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupFileEntry {
    pub relative_path: String,
    pub sha256_hash: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupManifest {
    pub backup_id: String,
    pub created_at: DateTime<Utc>,
    pub backup_type: BackupType,
    pub farm_name: String,
    pub game_id: String,
    pub source_directory: String,
    pub is_protected: bool,
    pub files: Vec<BackupFileEntry>,
    pub package_hash: String,
}
