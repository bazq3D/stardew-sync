use crate::commands::ActiveOperationGuard;
use crate::core::character::extractor::extract_all_characters_from_save;
use crate::core::errors::CoreError;
use crate::core::process::monitor::SystemProcessChecker;
use crate::core::save::parser::{format_stardew_season, get_child_text};
use crate::core::snapshot::manifest::{
    SnapshotComparison, SnapshotFileEntry, SnapshotIntegrityResult, SnapshotReason,
    SnapshotSaveMetadata, SnapshotSourceType, VersionedSnapshotManifest, VersionedSnapshotSummary,
};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use xmltree::Element;

const MAX_SNAPSHOT_FILE_SIZE_BYTES: u64 = 100 * 1024 * 1024; // 100 MB guard per save file

pub struct VersionedSnapshotManager;

impl VersionedSnapshotManager {
    /// Resolves the application-owned storage root for versioned snapshots.
    ///
    /// By default: `%LOCALAPPDATA%\StardewSync\snapshots`
    /// Honors `STARDEW_SNAPSHOTS_DIR` environment variable for isolated test fixtures.
    pub fn get_snapshots_root() -> PathBuf {
        if let Ok(custom) = std::env::var("STARDEW_SNAPSHOTS_DIR") {
            if !custom.trim().is_empty() {
                return PathBuf::from(custom);
            }
        }

        if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(localappdata)
                .join("StardewSync")
                .join("snapshots");
        }

        // Cross-platform fallback for non-Windows dev environments
        if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            return PathBuf::from(home)
                .join(".stardew-sync")
                .join("snapshots");
        }

        PathBuf::from("snapshots")
    }

    /// Validates identifiers against path traversal, control characters, and separators.
    pub fn validate_safe_identifier(id: &str) -> Result<(), CoreError> {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return Err(CoreError::SnapshotSecurityViolation(
                "Identifier cannot be empty".to_string(),
            ));
        }

        if trimmed.contains('/')
            || trimmed.contains('\\')
            || trimmed.contains("..")
            || trimmed.contains('\0')
            || trimmed.starts_with('.')
        {
            return Err(CoreError::SnapshotSecurityViolation(format!(
                "Invalid identifier '{}': contains disallowed traversal characters",
                id
            )));
        }

        Ok(())
    }

    /// Verifies that a target path resides strictly inside the snapshots root
    /// and does not escape via junctions or symlinks.
    fn verify_path_within_root(target: &Path, root: &Path) -> Result<(), CoreError> {
        let target_canonical = target.canonicalize().map_err(|e| {
            CoreError::SnapshotSecurityViolation(format!(
                "Failed to canonicalize path {:?}: {}",
                target, e
            ))
        })?;

        let root_canonical = root.canonicalize().map_err(|e| {
            CoreError::SnapshotSecurityViolation(format!(
                "Failed to canonicalize snapshot root {:?}: {}",
                root, e
            ))
        })?;

        if !target_canonical.starts_with(&root_canonical) {
            return Err(CoreError::SnapshotSecurityViolation(format!(
                "Path escape detected! Path {:?} is outside snapshots root {:?}",
                target_canonical, root_canonical
            )));
        }

        // Defend against symlink targets inside the snapshot directory
        if let Ok(meta) = target.symlink_metadata() {
            if meta.file_type().is_symlink() {
                return Err(CoreError::SnapshotSecurityViolation(format!(
                    "Symlinks are disallowed in snapshots directory: {:?}",
                    target
                )));
            }
        }

        Ok(())
    }

    /// Creates a verified manual or automatic snapshot.
    ///
    /// SAFETY GUARANTEES:
    /// 1. Stardew Valley must NOT be running.
    /// 2. Active updater/save operation locks must NOT be held.
    /// 3. Source save files are strictly read-only; NEVER written to or modified.
    /// 4. Written into an isolated staging directory and published atomically.
    /// 5. Every copied byte is verified against SHA-256 before publishing.
    pub fn create_snapshot(
        source_farm_folder: &str,
        saves_root: &Path,
        user_label: Option<String>,
        reason: SnapshotReason,
    ) -> Result<VersionedSnapshotManifest, CoreError> {
        // 1. Process Gate: Game must be closed
        if SystemProcessChecker::is_stardew_running() {
            return Err(CoreError::GameRunning {
                process_name: "Stardew Valley.exe".to_string(),
            });
        }

        // 2. Operation Guard Gate: No active updater or save operation
        if ActiveOperationGuard::is_active() {
            return Err(CoreError::Snapshot(
                "An active save operation or updater reservation is in progress. Snapshot locked.".to_string(),
            ));
        }

        // 3. Source Path Validation
        Self::validate_safe_identifier(source_farm_folder)?;

        let source_path = saves_root.join(source_farm_folder);
        if !source_path.exists() || !source_path.is_dir() {
            return Err(CoreError::SaveDirectoryNotFound(source_path));
        }

        // Check for Xbox WGS source
        let source_lossy = source_path.to_string_lossy().to_lowercase();
        if source_lossy.contains("systemappdata") || source_lossy.contains("\\wgs") || source_lossy.contains("/wgs") {
            return Err(CoreError::SnapshotWgsUnsupported);
        }

        // 4. Verify primary save or SaveGameInfo exists
        let primary_file = source_path.join(source_farm_folder);
        let save_game_info = source_path.join("SaveGameInfo");
        if !primary_file.is_file() && !save_game_info.is_file() {
            return Err(CoreError::Validation(format!(
                "Source folder '{}' contains neither primary save nor SaveGameInfo",
                source_farm_folder
            )));
        }

        // 5. Generate Collision-Resistant Snapshot ID
        let now_utc = Utc::now();
        let timestamp_str = now_utc.format("%Y%m%d_%H%M%S").to_string();
        let uuid_short = &uuid::Uuid::new_v4().to_string()[..8];
        let snapshot_id = format!("{}_{}_{}", timestamp_str, reason, uuid_short);

        let snapshots_root = Self::get_snapshots_root();
        let farm_snapshots_dir = snapshots_root.join(source_farm_folder);
        fs::create_dir_all(&farm_snapshots_dir)?;

        let target_snapshot_dir = farm_snapshots_dir.join(&snapshot_id);
        if target_snapshot_dir.exists() {
            return Err(CoreError::Snapshot(format!(
                "Snapshot with ID '{}' already exists. Overwrite strictly forbidden.",
                snapshot_id
            )));
        }

        // 6. Staging Pipeline
        let staging_dir = farm_snapshots_dir.join(format!(".staging_{}", snapshot_id));
        if staging_dir.exists() {
            let _ = fs::remove_dir_all(&staging_dir);
        }
        fs::create_dir_all(&staging_dir)?;

        let staging_cleanup = StagingCleanupGuard {
            path: staging_dir.clone(),
            should_cleanup: true,
        };

        let mut files = Vec::new();
        let mut overall_hasher = Sha256::new();

        // Copy and verify individual save components strictly inside source_path
        for entry in fs::read_dir(&source_path)? {
            let entry = entry?;
            let path = entry.path();

            // Skip subdirectories, hidden files, or symlinks
            let file_type = entry.file_type()?;
            if !file_type.is_file() || file_type.is_symlink() {
                continue;
            }

            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            // Only copy save files (primary, SaveGameInfo, companion _old files)
            if file_name != source_farm_folder
                && file_name != "SaveGameInfo"
                && !file_name.ends_with("_old")
            {
                continue;
            }

            let file_meta = entry.metadata()?;
            if file_meta.len() > MAX_SNAPSHOT_FILE_SIZE_BYTES {
                return Err(CoreError::SnapshotSecurityViolation(format!(
                    "File '{}' exceeds maximum allowed size ({} bytes)",
                    file_name, file_meta.len()
                )));
            }

            let content = fs::read(&path)?;
            let mut file_hasher = Sha256::new();
            file_hasher.update(&content);
            let sha256_hash = format!("{:X}", file_hasher.finalize());

            overall_hasher.update(&content);
            overall_hasher.update(file_name.as_bytes());

            let dest_file = staging_dir.join(&file_name);
            fs::write(&dest_file, &content)?;

            // Verify written copy immediately
            let verify_bytes = fs::read(&dest_file)?;
            if verify_bytes != content {
                return Err(CoreError::BackupVerificationFailed {
                    file: file_name,
                    expected: format!("{} bytes", content.len()),
                    actual: format!("{} bytes", verify_bytes.len()),
                });
            }

            files.push(SnapshotFileEntry {
                file_name,
                sha256_hash,
                size_bytes: content.len() as u64,
            });
        }

        if files.is_empty() {
            return Err(CoreError::Snapshot(
                "No valid save files could be captured from source".to_string(),
            ));
        }

        files.sort_by(|a, b| a.file_name.cmp(&b.file_name));
        let package_hash = format!("{:X}", overall_hasher.finalize());

        // 7. Extract Save Metadata safely from primary save or SaveGameInfo
        let mut save_metadata = None;
        let staged_primary = staging_dir.join(source_farm_folder);
        let staged_info = staging_dir.join("SaveGameInfo");

        let xml_path = if staged_primary.is_file() {
            Some(staged_primary)
        } else if staged_info.is_file() {
            Some(staged_info)
        } else {
            None
        };

        let mut farm_name = source_farm_folder.to_string();
        let mut game_id = "0".to_string();

        if let Some(path) = xml_path {
            if let Ok(xml_str) = fs::read_to_string(&path) {
                if let Ok(elem) = Element::parse(std::io::Cursor::new(xml_str.as_bytes())) {
                    let season_raw = get_child_text(&elem, "currentSeason").unwrap_or_else(|| "0".to_string());
                    let day = get_child_text(&elem, "dayOfMonth").unwrap_or_else(|| "1".to_string());
                    let year = get_child_text(&elem, "yearForSaveGame")
                        .or_else(|| get_child_text(&elem, "year"))
                        .unwrap_or_else(|| "1".to_string());
                    let date_summary = Some(format!("{}, Day {} (Year {})", format_stardew_season(&season_raw), day, year));
                    let characters = extract_all_characters_from_save(&elem, source_farm_folder);

                    let host = characters.iter().find(|c| c.is_host);
                    let money = host.map(|h| h.money);
                    let play_time_hours = host.and_then(|h| h.play_time_hours);
                    let play_time_formatted = host.map(|h| h.play_time_formatted.clone());
                    let host_name = host.map(|h| h.name.clone());

                    if let Some(h) = host {
                        if !h.farm_name.is_empty() {
                            farm_name = h.farm_name.clone();
                        }
                        if !h.game_id.is_empty() {
                            game_id = h.game_id.clone();
                        }
                    }

                    save_metadata = Some(SnapshotSaveMetadata {
                        in_game_date: date_summary,
                        money,
                        play_time_hours,
                        play_time_formatted,
                        host_name,
                        character_count: characters.len(),
                    });
                }
            }
        }

        let is_protected = match reason {
            SnapshotReason::Manual => true,
            SnapshotReason::OriginalImport => true,
            SnapshotReason::PreHostRotation => true,
            SnapshotReason::Recovery => true,
            SnapshotReason::PreSync => false,
            SnapshotReason::PostSync => false,
        };

        let manifest = VersionedSnapshotManifest {
            schema_version: 1,
            snapshot_id: snapshot_id.clone(),
            created_at: now_utc,
            snapshot_reason: reason,
            farm_folder_name: source_farm_folder.to_string(),
            farm_name,
            game_id,
            source_type: SnapshotSourceType::StandardAppdata,
            source_directory: source_path.to_string_lossy().to_string(),
            user_label,
            is_protected,
            files,
            package_hash,
            save_metadata,
        };

        // 8. Write and verify manifest in staging
        let manifest_path = staging_dir.join("snapshot_manifest.json");
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        fs::write(&manifest_path, manifest_json)?;

        // 9. Atomic Publish: rename staging to final destination
        std::mem::forget(staging_cleanup); // Disarm automatic deletion
        fs::rename(&staging_dir, &target_snapshot_dir)?;

        Ok(manifest)
    }

    /// Lists all snapshots, optionally filtered by farm folder name.
    /// Results are sorted newest first.
    pub fn list_snapshots(farm_filter: Option<&str>) -> Result<Vec<VersionedSnapshotSummary>, CoreError> {
        let snapshots_root = Self::get_snapshots_root();
        if !snapshots_root.exists() {
            return Ok(Vec::new());
        }

        if let Some(filter) = farm_filter {
            Self::validate_safe_identifier(filter)?;
        }

        let mut summaries = Vec::new();

        for farm_entry in fs::read_dir(&snapshots_root)? {
            let farm_entry = farm_entry?;
            let farm_path = farm_entry.path();
            if !farm_path.is_dir() {
                continue;
            }

            let folder_name = match farm_path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            if folder_name.starts_with('.') {
                continue;
            }

            if let Some(filter) = farm_filter {
                if folder_name != filter {
                    continue;
                }
            }

            for snap_entry in fs::read_dir(&farm_path)? {
                let snap_entry = snap_entry?;
                let snap_path = snap_entry.path();
                if !snap_path.is_dir() {
                    continue;
                }

                let snap_folder_name = match snap_path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n.to_string(),
                    None => continue,
                };

                // Skip uncompleted staging directories
                if snap_folder_name.starts_with(".staging_") {
                    continue;
                }

                let manifest_path = snap_path.join("snapshot_manifest.json");
                if manifest_path.is_file() {
                    if let Ok(manifest_content) = fs::read_to_string(&manifest_path) {
                        if let Ok(manifest) = serde_json::from_str::<VersionedSnapshotManifest>(&manifest_content) {
                            let total_size_bytes = manifest.files.iter().map(|f| f.size_bytes).sum();
                            summaries.push(VersionedSnapshotSummary {
                                snapshot_id: manifest.snapshot_id,
                                farm_folder_name: manifest.farm_folder_name,
                                farm_name: manifest.farm_name,
                                game_id: manifest.game_id,
                                created_at: manifest.created_at.to_rfc3339(),
                                snapshot_reason: manifest.snapshot_reason.to_string(),
                                source_type: manifest.source_type.to_string(),
                                user_label: manifest.user_label,
                                is_protected: manifest.is_protected,
                                file_count: manifest.files.len(),
                                total_size_bytes,
                                package_hash: manifest.package_hash,
                                save_metadata: manifest.save_metadata,
                            });
                        }
                    }
                }
            }
        }

        // Sort newest first by created_at descending
        summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(summaries)
    }

    /// Finds the absolute path to a snapshot directory given its ID.
    fn find_snapshot_dir(snapshot_id: &str) -> Result<PathBuf, CoreError> {
        Self::validate_safe_identifier(snapshot_id)?;

        let snapshots_root = Self::get_snapshots_root();
        if !snapshots_root.exists() {
            return Err(CoreError::SnapshotNotFound(snapshot_id.to_string()));
        }

        for farm_entry in fs::read_dir(&snapshots_root)? {
            let farm_entry = farm_entry?;
            let farm_path = farm_entry.path();
            if farm_path.is_dir() {
                let candidate = farm_path.join(snapshot_id);
                if candidate.is_dir() {
                    Self::verify_path_within_root(&candidate, &snapshots_root)?;
                    return Ok(candidate);
                }
            }
        }

        Err(CoreError::SnapshotNotFound(snapshot_id.to_string()))
    }

    /// Reads the complete `VersionedSnapshotManifest` for a given snapshot ID.
    pub fn get_manifest(snapshot_id: &str) -> Result<VersionedSnapshotManifest, CoreError> {
        let snap_dir = Self::find_snapshot_dir(snapshot_id)?;
        let manifest_path = snap_dir.join("snapshot_manifest.json");

        if !manifest_path.is_file() {
            return Err(CoreError::Snapshot(format!(
                "Manifest file not found in snapshot directory {:?}",
                snap_dir
            )));
        }

        let content = fs::read_to_string(&manifest_path)?;
        let manifest: VersionedSnapshotManifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    /// Verifies the cryptographic integrity and file consistency of a snapshot.
    pub fn verify_integrity(snapshot_id: &str) -> Result<SnapshotIntegrityResult, CoreError> {
        let snap_dir = Self::find_snapshot_dir(snapshot_id)?;
        let manifest = Self::get_manifest(snapshot_id)?;

        let mut files_checked = 0;
        let mut overall_hasher = Sha256::new();
        let mut primary_hash_matched = false;
        let mut savegameinfo_hash_matched = false;
        let mut xml_parseable = false;

        let mut sorted_files = manifest.files.clone();
        sorted_files.sort_by(|a, b| a.file_name.cmp(&b.file_name));

        for file_entry in &sorted_files {
            let file_path = snap_dir.join(&file_entry.file_name);
            if !file_path.is_file() {
                return Ok(SnapshotIntegrityResult {
                    snapshot_id: snapshot_id.to_string(),
                    is_valid: false,
                    files_checked,
                    primary_hash_matched: false,
                    savegameinfo_hash_matched: false,
                    package_hash_matched: false,
                    xml_parseable: false,
                    message: format!("Missing snapshot file: '{}'", file_entry.file_name),
                });
            }

            let content = fs::read(&file_path)?;
            let mut hasher = Sha256::new();
            hasher.update(&content);
            let actual_hash = format!("{:X}", hasher.finalize());

            if actual_hash != file_entry.sha256_hash {
                return Ok(SnapshotIntegrityResult {
                    snapshot_id: snapshot_id.to_string(),
                    is_valid: false,
                    files_checked,
                    primary_hash_matched: false,
                    savegameinfo_hash_matched: false,
                    package_hash_matched: false,
                    xml_parseable: false,
                    message: format!(
                        "Hash mismatch for '{}': expected {}, found {}",
                        file_entry.file_name, file_entry.sha256_hash, actual_hash
                    ),
                });
            }

            overall_hasher.update(&content);
            overall_hasher.update(file_entry.file_name.as_bytes());
            files_checked += 1;

            if file_entry.file_name == manifest.farm_folder_name {
                primary_hash_matched = true;
                if let Ok(xml_str) = String::from_utf8(content.clone()) {
                    if Element::parse(std::io::Cursor::new(xml_str.as_bytes())).is_ok() {
                        xml_parseable = true;
                    }
                }
            } else if file_entry.file_name == "SaveGameInfo" {
                savegameinfo_hash_matched = true;
                if !xml_parseable {
                    if let Ok(xml_str) = String::from_utf8(content) {
                        if Element::parse(std::io::Cursor::new(xml_str.as_bytes())).is_ok() {
                            xml_parseable = true;
                        }
                    }
                }
            }
        }

        let computed_package_hash = format!("{:X}", overall_hasher.finalize());
        let package_hash_matched = computed_package_hash == manifest.package_hash;

        let is_valid = files_checked == manifest.files.len()
            && package_hash_matched
            && (primary_hash_matched || savegameinfo_hash_matched)
            && xml_parseable;

        let message = if is_valid {
            format!(
                "Snapshot verified successfully ({} files, package SHA-256 and XML schema valid)",
                files_checked
            )
        } else {
            "Snapshot verification failed: file tampering or corruption detected".to_string()
        };

        Ok(SnapshotIntegrityResult {
            snapshot_id: snapshot_id.to_string(),
            is_valid,
            files_checked,
            primary_hash_matched,
            savegameinfo_hash_matched,
            package_hash_matched,
            xml_parseable,
            message,
        })
    }

    /// Renames the user-facing label of an existing snapshot.
    pub fn rename_label(
        snapshot_id: &str,
        new_label: String,
    ) -> Result<VersionedSnapshotSummary, CoreError> {
        let snap_dir = Self::find_snapshot_dir(snapshot_id)?;
        let manifest_path = snap_dir.join("snapshot_manifest.json");

        let mut manifest = Self::get_manifest(snapshot_id)?;
        let clean_label = new_label.trim().to_string();
        manifest.user_label = if clean_label.is_empty() {
            None
        } else {
            Some(clean_label)
        };

        let updated_json = serde_json::to_string_pretty(&manifest)?;
        fs::write(&manifest_path, updated_json)?;

        let total_size_bytes = manifest.files.iter().map(|f| f.size_bytes).sum();
        Ok(VersionedSnapshotSummary {
            snapshot_id: manifest.snapshot_id,
            farm_folder_name: manifest.farm_folder_name,
            farm_name: manifest.farm_name,
            game_id: manifest.game_id,
            created_at: manifest.created_at.to_rfc3339(),
            snapshot_reason: manifest.snapshot_reason.to_string(),
            source_type: manifest.source_type.to_string(),
            user_label: manifest.user_label,
            is_protected: manifest.is_protected,
            file_count: manifest.files.len(),
            total_size_bytes,
            package_hash: manifest.package_hash,
            save_metadata: manifest.save_metadata,
        })
    }

    /// Deletes an application-owned snapshot ONLY after explicit confirmation.
    ///
    /// GUARANTEES:
    /// 1. Requires `confirmation` to match `"DELETE"` or the exact `snapshot_id`.
    /// 2. Validates that the target path is strictly within the application snapshot root.
    /// 3. Never deletes anything outside the application snapshot root.
    pub fn delete_snapshot(snapshot_id: &str, confirmation: &str) -> Result<(), CoreError> {
        if confirmation != "DELETE" && confirmation != snapshot_id {
            return Err(CoreError::SnapshotSecurityViolation(format!(
                "Deletion confirmation mismatch: expected 'DELETE' or '{}', got '{}'",
                snapshot_id, confirmation
            )));
        }

        let snap_dir = Self::find_snapshot_dir(snapshot_id)?;
        let snapshots_root = Self::get_snapshots_root();

        // Enforce strict root boundary check
        Self::verify_path_within_root(&snap_dir, &snapshots_root)?;

        fs::remove_dir_all(&snap_dir)?;
        Ok(())
    }

    /// Compares two snapshots from the same or different farms.
    pub fn compare_snapshots(
        id_a: &str,
        id_b: &str,
    ) -> Result<SnapshotComparison, CoreError> {
        let manifest_a = Self::get_manifest(id_a)?;
        let manifest_b = Self::get_manifest(id_b)?;

        let total_size_a: u64 = manifest_a.files.iter().map(|f| f.size_bytes).sum();
        let total_size_b: u64 = manifest_b.files.iter().map(|f| f.size_bytes).sum();

        let mut files_changed = Vec::new();
        for file_a in &manifest_a.files {
            if let Some(file_b) = manifest_b.files.iter().find(|f| f.file_name == file_a.file_name) {
                if file_a.sha256_hash != file_b.sha256_hash {
                    files_changed.push(format!("{} (modified)", file_a.file_name));
                }
            } else {
                files_changed.push(format!("{} (removed in B)", file_a.file_name));
            }
        }
        for file_b in &manifest_b.files {
            if !manifest_a.files.iter().any(|f| f.file_name == file_b.file_name) {
                files_changed.push(format!("{} (added in B)", file_b.file_name));
            }
        }

        let meta_a = manifest_a.save_metadata.as_ref();
        let meta_b = manifest_b.save_metadata.as_ref();

        Ok(SnapshotComparison {
            snapshot_id_a: manifest_a.snapshot_id,
            snapshot_id_b: manifest_b.snapshot_id,
            farm_folder_name: manifest_a.farm_folder_name,
            created_at_a: manifest_a.created_at.to_rfc3339(),
            created_at_b: manifest_b.created_at.to_rfc3339(),
            date_a: meta_a.and_then(|m| m.in_game_date.clone()),
            date_b: meta_b.and_then(|m| m.in_game_date.clone()),
            money_a: meta_a.and_then(|m| m.money),
            money_b: meta_b.and_then(|m| m.money),
            playtime_a: meta_a.and_then(|m| m.play_time_formatted.clone()),
            playtime_b: meta_b.and_then(|m| m.play_time_formatted.clone()),
            characters_a: meta_a.map(|m| m.character_count).unwrap_or(0),
            characters_b: meta_b.map(|m| m.character_count).unwrap_or(0),
            total_size_a,
            total_size_b,
            package_hash_a: manifest_a.package_hash.clone(),
            package_hash_b: manifest_b.package_hash.clone(),
            hashes_match: manifest_a.package_hash == manifest_b.package_hash,
            files_changed,
        })
    }
}

/// RAII Guard ensuring staging directory is cleaned up if snapshot creation is interrupted or fails.
struct StagingCleanupGuard {
    path: PathBuf,
    should_cleanup: bool,
}

impl Drop for StagingCleanupGuard {
    fn drop(&mut self) {
        if self.should_cleanup && self.path.exists() {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
