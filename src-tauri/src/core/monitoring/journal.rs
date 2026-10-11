use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use crate::core::errors::CoreError;
use crate::core::monitoring::types::{
    DetectionReason, ObservationJournalEntry, SaveChangeClassification, SaveFingerprint,
};

pub const DEFAULT_MAX_JOURNAL_ENTRIES: usize = 200;

pub struct ObservationJournalManager;

impl ObservationJournalManager {
    /// Resolves the application-owned monitoring directory root.
    /// Uses %LOCALAPPDATA%\StardewSync\monitoring by default, or STARDEW_MONITORING_DIR for test isolation.
    pub fn get_monitoring_root() -> PathBuf {
        if let Ok(env_override) = std::env::var("STARDEW_MONITORING_DIR") {
            let path = PathBuf::from(env_override);
            if !path.as_os_str().is_empty() {
                return path;
            }
        }

        if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local_appdata).join("StardewSync").join("monitoring");
        }

        if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            return PathBuf::from(home).join(".stardew-sync").join("monitoring");
        }

        PathBuf::from("monitoring")
    }

    /// Gets the journal file path for a specific farm identifier.
    pub fn get_journal_file_path(farm_id: &str) -> Result<PathBuf, CoreError> {
        let trimmed = farm_id.trim();
        if trimmed.is_empty()
            || trimmed.contains('/')
            || trimmed.contains('\\')
            || trimmed.contains("..")
            || trimmed.starts_with('.')
        {
            return Err(CoreError::SnapshotSecurityViolation(format!(
                "Invalid farm id for observation journal: '{}'",
                farm_id
            )));
        }

        let root = Self::get_monitoring_root();
        Ok(root.join(trimmed).join("observation_journal.json"))
    }

    /// Loads the observation history journal for a farm.
    pub fn load_journal(farm_id: &str) -> Result<Vec<ObservationJournalEntry>, CoreError> {
        let path = Self::get_journal_file_path(farm_id)?;
        if !path.exists() {
            return Ok(Vec::new());
        }

        let json_str = fs::read_to_string(&path)?;
        let entries: Vec<ObservationJournalEntry> = serde_json::from_str(&json_str)?;
        Ok(entries)
    }

    /// Records an observation in the journal.
    ///
    /// Monotonically increments the local sequence counter on newly detected states.
    /// If the save fingerprint and classification have not changed since the last check,
    /// it updates `last_seen` in-place on the latest entry rather than bloating the journal.
    pub fn record_observation(
        farm_id: &str,
        fingerprint: &SaveFingerprint,
        classification: SaveChangeClassification,
        reason: DetectionReason,
        game_running: bool,
    ) -> Result<ObservationJournalEntry, CoreError> {
        let path = Self::get_journal_file_path(farm_id)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut entries = Self::load_journal(farm_id).unwrap_or_default();
        let now = Utc::now();

        // Check if the latest entry represents the exact same observation state
        if let Some(last) = entries.last_mut() {
            if last.current_fingerprint == fingerprint.aggregate_hash
                && last.classification == classification
                && last.game_process_running == game_running
            {
                last.last_seen = now;
                last.validation_status = fingerprint.validation_status.clone();
                let updated_entry = last.clone();
                Self::save_journal_atomic(&path, &entries)?;
                return Ok(updated_entry);
            }
        }

        // New observation state detected
        let prev_fingerprint = entries.last().map(|e| e.current_fingerprint.clone());
        let next_seq = entries.last().map(|e| e.sequence_number + 1).unwrap_or(1);

        let new_entry = ObservationJournalEntry {
            entry_id: Uuid::new_v4().to_string(),
            farm_id: farm_id.to_string(),
            sequence_number: next_seq,
            previous_fingerprint: prev_fingerprint,
            current_fingerprint: fingerprint.aggregate_hash.clone(),
            first_seen: now,
            last_seen: now,
            detection_reason: reason,
            classification,
            game_process_running: game_running,
            validation_status: fingerprint.validation_status.clone(),
            in_game_metadata: fingerprint.in_game_metadata.clone(),
        };

        entries.push(new_entry.clone());

        // Bound journal size to prevent unbounded memory/disk usage
        if entries.len() > DEFAULT_MAX_JOURNAL_ENTRIES {
            let drain_count = entries.len() - DEFAULT_MAX_JOURNAL_ENTRIES;
            entries.drain(0..drain_count);
        }

        Self::save_journal_atomic(&path, &entries)?;
        Ok(new_entry)
    }

    /// Gets the most recent observation entry for a farm, if available.
    pub fn get_latest_entry(farm_id: &str) -> Option<ObservationJournalEntry> {
        Self::load_journal(farm_id).ok()?.into_iter().last()
    }

    /// Atomically persists the journal to disk using a temporary sibling file.
    fn save_journal_atomic(target_path: &Path, entries: &[ObservationJournalEntry]) -> Result<(), CoreError> {
        let parent = target_path.parent().ok_or_else(|| {
            CoreError::Monitoring("Journal target path has no parent directory".to_string())
        })?;
        fs::create_dir_all(parent)?;

        let temp_path = parent.join(format!(".tmp_journal_{}.json", Uuid::new_v4()));
        let json_content = serde_json::to_string_pretty(entries)?;
        fs::write(&temp_path, json_content)?;

        // Atomic replace on Windows
        if target_path.exists() {
            let _ = fs::remove_file(target_path);
        }
        fs::rename(&temp_path, target_path)?;

        Ok(())
    }
}
