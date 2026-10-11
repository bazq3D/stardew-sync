use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use crate::core::snapshot::SnapshotSaveMetadata;

/// Change classification of an observed save state relative to the prior observation.
/// Strictly conservative: Never automatically assumes a save is safe to overwrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaveChangeClassification {
    /// Save hashes and metadata exactly match the previous observation.
    Unchanged,
    /// Verified stable local save with different hashes/progression, game closed.
    LocalChangeDetected,
    /// Stardew Valley process is currently running; save may be mid-flush.
    WaitingForGameClosure,
    /// Filesystem modifications detected; quiet-period debounce in progress.
    WaitingForFileStability,
    /// Save files are missing required components, 0 bytes, or malformed XML.
    InvalidOrIncompleteSave,
    /// Monitored farm directory does not exist or was deleted.
    SourceMissing,
    /// Watcher I/O, OS buffer overflow, or permission failure.
    MonitoringError,
    /// Ambiguous state requiring manual user review.
    UnknownNeedsReview,
}

impl fmt::Display for SaveChangeClassification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unchanged => write!(f, "Unchanged"),
            Self::LocalChangeDetected => write!(f, "Local Change Detected"),
            Self::WaitingForGameClosure => write!(f, "Waiting for Game Closure"),
            Self::WaitingForFileStability => write!(f, "Waiting for File Stability"),
            Self::InvalidOrIncompleteSave => write!(f, "Invalid or Incomplete Save"),
            Self::SourceMissing => write!(f, "Source Missing"),
            Self::MonitoringError => write!(f, "Monitoring Error"),
            Self::UnknownNeedsReview => write!(f, "Unknown / Needs Review"),
        }
    }
}

/// Reason triggering the observation/detection pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectionReason {
    InitialScan,
    FilesystemEvent,
    PeriodicReconcile,
    ManualCheck,
    PostGameExit,
}

impl fmt::Display for DetectionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitialScan => write!(f, "Initial Scan"),
            Self::FilesystemEvent => write!(f, "Filesystem Event"),
            Self::PeriodicReconcile => write!(f, "Periodic Reconcile"),
            Self::ManualCheck => write!(f, "Manual Check"),
            Self::PostGameExit => write!(f, "Post-Game Exit Settle"),
        }
    }
}

/// Structural and cryptographic validation status of an observed save.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Valid,
    MalformedXml(String),
    MissingFiles(String),
    SizeMismatch(String),
    ReadError(String),
    UnsupportedSource(String),
}

impl fmt::Display for ValidationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Valid => write!(f, "Valid"),
            Self::MalformedXml(msg) => write!(f, "Malformed XML: {}", msg),
            Self::MissingFiles(msg) => write!(f, "Missing Files: {}", msg),
            Self::SizeMismatch(msg) => write!(f, "Size Mismatch: {}", msg),
            Self::ReadError(msg) => write!(f, "Read Error: {}", msg),
            Self::UnsupportedSource(msg) => write!(f, "Unsupported Source: {}", msg),
        }
    }
}

/// Canonical entry representing an individual monitored save file within a farm slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveFileEntry {
    pub file_name: String,
    pub sha256_hash: String,
    pub size_bytes: u64,
    pub modified_timestamp_utc: Option<DateTime<Utc>>,
}

/// Deterministic, cryptographically verified fingerprint of a farm save at an observation instant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveFingerprint {
    pub farm_id: String,
    pub source_type: String,
    pub files: Vec<SaveFileEntry>,
    pub aggregate_hash: String,
    pub observed_at: DateTime<Utc>,
    pub in_game_metadata: Option<SnapshotSaveMetadata>,
    pub validation_status: ValidationStatus,
    pub validation_message: Option<String>,
}

/// A recorded entry in the application-owned local observation journal.
///
/// NOTE: The `sequence_number` represents a strictly LOCAL observation counter.
/// It is NOT a globally authoritative sync generation and does not imply peer agreement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservationJournalEntry {
    pub entry_id: String,
    pub farm_id: String,
    pub sequence_number: u64,
    pub previous_fingerprint: Option<String>,
    pub current_fingerprint: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub detection_reason: DetectionReason,
    pub classification: SaveChangeClassification,
    pub game_process_running: bool,
    pub validation_status: ValidationStatus,
    pub in_game_metadata: Option<SnapshotSaveMetadata>,
}

/// Frontend-facing monitoring status for a specific farm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FarmMonitoringStatus {
    pub farm_id: String,
    pub is_enabled: bool,
    pub classification: SaveChangeClassification,
    pub classification_label: String,
    pub current_fingerprint: Option<String>,
    pub short_fingerprint: Option<String>,
    pub last_observed_at: Option<DateTime<Utc>>,
    pub last_verified_change_at: Option<DateTime<Utc>>,
    pub observation_sequence: u64,
    pub is_game_running: bool,
    pub is_settling: bool,
    pub validation_status: ValidationStatus,
    pub error_message: Option<String>,
    /// Always "Sync Status Unknown" until peer pairing and baseline verification exist.
    pub sync_indicator: String,
}
