use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SnapshotReason {
    Manual,
    PreSync,
    PostSync,
    PreHostRotation,
    OriginalImport,
    Recovery,
}

impl std::fmt::Display for SnapshotReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotReason::Manual => write!(f, "MANUAL"),
            SnapshotReason::PreSync => write!(f, "PRE_SYNC"),
            SnapshotReason::PostSync => write!(f, "POST_SYNC"),
            SnapshotReason::PreHostRotation => write!(f, "PRE_HOST_ROTATION"),
            SnapshotReason::OriginalImport => write!(f, "ORIGINAL_IMPORT"),
            SnapshotReason::Recovery => write!(f, "RECOVERY"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SnapshotSourceType {
    StandardAppdata,
    Custom,
    XboxWgs,
}

impl std::fmt::Display for SnapshotSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SnapshotSourceType::StandardAppdata => write!(f, "StandardAppdata"),
            SnapshotSourceType::Custom => write!(f, "Custom"),
            SnapshotSourceType::XboxWgs => write!(f, "XboxWgs"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotFileEntry {
    pub file_name: String,
    pub sha256_hash: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotSaveMetadata {
    pub in_game_date: Option<String>,
    pub money: Option<u32>,
    pub play_time_hours: Option<f64>,
    pub play_time_formatted: Option<String>,
    pub host_name: Option<String>,
    pub character_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionedSnapshotManifest {
    pub schema_version: u32,
    pub snapshot_id: String,
    pub created_at: DateTime<Utc>,
    pub snapshot_reason: SnapshotReason,
    pub farm_folder_name: String,
    pub farm_name: String,
    pub game_id: String,
    pub source_type: SnapshotSourceType,
    pub source_directory: String,
    pub user_label: Option<String>,
    pub is_protected: bool,
    pub files: Vec<SnapshotFileEntry>,
    pub package_hash: String,
    pub save_metadata: Option<SnapshotSaveMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionedSnapshotSummary {
    pub snapshot_id: String,
    pub farm_folder_name: String,
    pub farm_name: String,
    pub game_id: String,
    pub created_at: String,
    pub snapshot_reason: String,
    pub source_type: String,
    pub user_label: Option<String>,
    pub is_protected: bool,
    pub file_count: usize,
    pub total_size_bytes: u64,
    pub package_hash: String,
    pub save_metadata: Option<SnapshotSaveMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotIntegrityResult {
    pub snapshot_id: String,
    pub is_valid: bool,
    pub files_checked: usize,
    pub primary_hash_matched: bool,
    pub savegameinfo_hash_matched: bool,
    pub package_hash_matched: bool,
    pub xml_parseable: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotComparison {
    pub snapshot_id_a: String,
    pub snapshot_id_b: String,
    pub farm_folder_name: String,
    pub created_at_a: String,
    pub created_at_b: String,
    pub date_a: Option<String>,
    pub date_b: Option<String>,
    pub money_a: Option<u32>,
    pub money_b: Option<u32>,
    pub playtime_a: Option<String>,
    pub playtime_b: Option<String>,
    pub characters_a: usize,
    pub characters_b: usize,
    pub total_size_a: u64,
    pub total_size_b: u64,
    pub package_hash_a: String,
    pub package_hash_b: String,
    pub hashes_match: bool,
    pub files_changed: Vec<String>,
}
