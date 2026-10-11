use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("XML parsing error: {0}")]
    XmlParse(String),

    #[error("XML write error: {0}")]
    XmlWrite(String),

    #[error("Save validation failed: {0}")]
    Validation(String),

    #[error("Host migration failed: {0}")]
    Migration(String),

    #[error("Farmer fingerprint mismatch for player '{name}' (ID: {id}): expected {expected}, got {actual}")]
    FingerprintMismatch {
        name: String,
        id: i64,
        expected: String,
        actual: String,
    },

    #[error("World state diff violation at XML path '{path}': {detail}")]
    WorldDiffViolation { path: String, detail: String },

    #[error("Backup error: {0}")]
    Backup(String),

    #[error("Backup verification failed: hash mismatch for file '{file}': expected {expected}, actual {actual}")]
    BackupVerificationFailed {
        file: String,
        expected: String,
        actual: String,
    },

    #[error("Cannot delete protected backup '{id}' of type '{backup_type}'")]
    ProtectedBackupDeletion { id: String, backup_type: String },

    #[error("Snapshot error: {0}")]
    Snapshot(String),

    #[error("Snapshot not found: '{0}'")]
    SnapshotNotFound(String),

    #[error("Snapshot security violation: {0}")]
    SnapshotSecurityViolation(String),

    #[error("Xbox WGS snapshot unsupported: Xbox container and cloud state cannot be safely backed up without live sync engine integration")]
    SnapshotWgsUnsupported,

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Game process is currently running: '{process_name}'. Operations on save files are strictly forbidden.")]
    GameRunning { process_name: String },

    #[error("Save settle timeout: save files did not stabilize within {timeout_secs} seconds")]
    SaveSettleTimeout { timeout_secs: u64 },

    #[error("Save directory not found at path: {0}")]
    SaveDirectoryNotFound(PathBuf),

    #[error("External save replacement detected: expected generation '{expected}', but found '{actual}' ({detail})")]
    ExternalSaveReplacement {
        expected: String,
        actual: String,
        detail: String,
    },

    #[error("Monitoring error: {0}")]
    Monitoring(String),

    #[error("Watcher error: {0}")]
    Watcher(String),

    #[error("Device identity error: {0}")]
    Identity(String),

    #[error("Pairing error: {0}")]
    Pairing(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
}
