use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use crate::core::errors::CoreError;
use crate::core::monitoring::types::{
    SaveFileEntry, SaveFingerprint, ValidationStatus,
};
use crate::commands::{
    extract_date_from_xml_element, extract_milliseconds_played, format_playtime_summary, ms_to_hours,
};
use crate::core::save::parser::get_child_text;
use crate::core::snapshot::SnapshotSaveMetadata;

pub const MAX_MONITORED_SAVE_FILE_SIZE_BYTES: u64 = 150 * 1024 * 1024; // 150 MB

pub struct FingerprintGenerator;

impl FingerprintGenerator {
    /// Validates safety of a farm folder identifier against path traversal and reparse tricks.
    pub fn validate_farm_identifier(farm_folder: &str) -> Result<(), CoreError> {
        let trimmed = farm_folder.trim();
        if trimmed.is_empty()
            || trimmed.contains('/')
            || trimmed.contains('\\')
            || trimmed.contains("..")
            || trimmed.starts_with('.')
        {
            return Err(CoreError::SnapshotSecurityViolation(format!(
                "Invalid farm folder identifier: '{}'",
                farm_folder
            )));
        }
        Ok(())
    }

    /// Computes a deterministic, cryptographic save fingerprint for a specific farm slot directory.
    pub fn compute_fingerprint(
        farm_folder: &str,
        farm_dir: &Path,
        source_type: &str,
    ) -> Result<SaveFingerprint, CoreError> {
        Self::validate_farm_identifier(farm_folder)?;

        let dir_str = farm_dir.to_string_lossy().to_lowercase();
        if dir_str.contains("systemappdata") || dir_str.contains("\\wgs") || dir_str.contains("/wgs") {
            return Ok(SaveFingerprint {
                farm_id: farm_folder.to_string(),
                source_type: "XboxWgs".to_string(),
                files: Vec::new(),
                aggregate_hash: String::new(),
                observed_at: Utc::now(),
                in_game_metadata: None,
                validation_status: ValidationStatus::UnsupportedSource(
                    "Xbox WGS container storage cannot be safely monitored without live synchronization engine integration"
                        .to_string(),
                ),
                validation_message: Some("Xbox WGS source unsupported for monitoring".to_string()),
            });
        }

        if !farm_dir.exists() || !farm_dir.is_dir() {
            return Err(CoreError::SaveDirectoryNotFound(farm_dir.to_path_buf()));
        }

        let primary_path = farm_dir.join(farm_folder);
        let save_game_info_path = farm_dir.join("SaveGameInfo");

        // Check for presence of primary components
        let primary_exists = primary_path.is_file();
        let info_exists = save_game_info_path.is_file();

        if !primary_exists && !info_exists {
            return Ok(SaveFingerprint {
                farm_id: farm_folder.to_string(),
                source_type: source_type.to_string(),
                files: Vec::new(),
                aggregate_hash: String::new(),
                observed_at: Utc::now(),
                in_game_metadata: None,
                validation_status: ValidationStatus::MissingFiles(
                    "Neither primary save file nor SaveGameInfo exists in farm directory".to_string(),
                ),
                validation_message: Some("Missing all essential save files".to_string()),
            });
        }

        let mut files = Vec::new();
        let target_files = vec![
            (farm_folder.to_string(), primary_path.clone()),
            ("SaveGameInfo".to_string(), save_game_info_path.clone()),
            (
                format!("{}_old", farm_folder),
                farm_dir.join(format!("{}_old", farm_folder)),
            ),
            (
                "SaveGameInfo_old".to_string(),
                farm_dir.join("SaveGameInfo_old"),
            ),
        ];

        for (name, path) in target_files {
            if path.is_file() {
                // Ignore symlinks
                if let Ok(sym_meta) = fs::symlink_metadata(&path) {
                    if sym_meta.file_type().is_symlink() {
                        continue;
                    }
                }

                let meta = match fs::metadata(&path) {
                    Ok(m) => m,
                    Err(e) => {
                        return Ok(SaveFingerprint {
                            farm_id: farm_folder.to_string(),
                            source_type: source_type.to_string(),
                            files: Vec::new(),
                            aggregate_hash: String::new(),
                            observed_at: Utc::now(),
                            in_game_metadata: None,
                            validation_status: ValidationStatus::ReadError(format!(
                                "Failed to inspect metadata for '{}': {}",
                                name, e
                            )),
                            validation_message: Some(format!("Metadata read error on {}", name)),
                        });
                    }
                };

                let size_bytes = meta.len();
                if size_bytes > MAX_MONITORED_SAVE_FILE_SIZE_BYTES {
                    return Ok(SaveFingerprint {
                        farm_id: farm_folder.to_string(),
                        source_type: source_type.to_string(),
                        files: Vec::new(),
                        aggregate_hash: String::new(),
                        observed_at: Utc::now(),
                        in_game_metadata: None,
                        validation_status: ValidationStatus::SizeMismatch(format!(
                            "File '{}' exceeds maximum allowed size ({} bytes)",
                            name, size_bytes
                        )),
                        validation_message: Some(format!("Size limit exceeded on {}", name)),
                    });
                }

                let modified_timestamp_utc = meta.modified().ok().map(|m| DateTime::<Utc>::from(m));

                let content = match fs::read(&path) {
                    Ok(b) => b,
                    Err(e) => {
                        return Ok(SaveFingerprint {
                            farm_id: farm_folder.to_string(),
                            source_type: source_type.to_string(),
                            files: Vec::new(),
                            aggregate_hash: String::new(),
                            observed_at: Utc::now(),
                            in_game_metadata: None,
                            validation_status: ValidationStatus::ReadError(format!(
                                "Failed to read content of '{}': {}",
                                name, e
                            )),
                            validation_message: Some(format!("File read error on {}", name)),
                        });
                    }
                };

                let mut hasher = Sha256::new();
                hasher.update(&content);
                let sha256_hash = format!("{:X}", hasher.finalize());

                files.push(SaveFileEntry {
                    file_name: name,
                    sha256_hash,
                    size_bytes,
                    modified_timestamp_utc,
                });
            }
        }

        if files.is_empty() {
            return Ok(SaveFingerprint {
                farm_id: farm_folder.to_string(),
                source_type: source_type.to_string(),
                files: Vec::new(),
                aggregate_hash: String::new(),
                observed_at: Utc::now(),
                in_game_metadata: None,
                validation_status: ValidationStatus::MissingFiles(
                    "No valid save files could be loaded from disk".to_string(),
                ),
                validation_message: Some("Empty save inventory".to_string()),
            });
        }

        // Canonical deterministic ordering
        files.sort_by(|a, b| a.file_name.cmp(&b.file_name));

        // Aggregate hash computation over canonical inventory
        let mut agg_hasher = Sha256::new();
        for f in &files {
            agg_hasher.update(format!("{}:{}:{};", f.file_name, f.sha256_hash, f.size_bytes).as_bytes());
        }
        let aggregate_hash = format!("{:X}", agg_hasher.finalize());

        // Validate XML well-formedness and extract in-game metadata safely
        let mut save_metadata = None;
        let mut xml_status = ValidationStatus::Valid;

        let xml_path = if primary_exists {
            Some(primary_path)
        } else if info_exists {
            Some(save_game_info_path)
        } else {
            None
        };

        if let Some(target_xml) = xml_path {
            match fs::read_to_string(&target_xml) {
                Ok(xml_str) => {
                    match xmltree::Element::parse(std::io::Cursor::new(xml_str.as_bytes())) {
                        Ok(elem) => {
                            let (_, _, _, in_game_date_str) = extract_date_from_xml_element(&elem);
                            let in_game_date = Some(in_game_date_str);
                            let ms_played = extract_milliseconds_played(&elem);
                            let play_time_hours = ms_played.map(ms_to_hours);
                            let play_time_formatted = Some(format_playtime_summary(ms_played));

                            let money = elem
                                .get_child("player")
                                .and_then(|p| get_child_text(p, "money"))
                                .or_else(|| get_child_text(&elem, "money"))
                                .and_then(|m| m.trim().parse::<u32>().ok());

                            let fallback_host_name = elem
                                .get_child("player")
                                .and_then(|p| get_child_text(p, "name"))
                                .or_else(|| get_child_text(&elem, "name"));

                            let characters = crate::core::character::extract_all_characters_from_save(&elem, farm_folder);
                            let character_count = if characters.is_empty() { 1 } else { characters.len() };
                            let host_name = characters
                                .iter()
                                .find(|c| c.is_host)
                                .map(|h| h.name.clone())
                                .or(fallback_host_name);

                            save_metadata = Some(SnapshotSaveMetadata {
                                in_game_date,
                                money,
                                play_time_hours,
                                play_time_formatted,
                                host_name,
                                character_count,
                            });
                        }
                        Err(e) => {
                            xml_status = ValidationStatus::MalformedXml(format!(
                                "Failed to parse XML: {}",
                                e
                            ));
                        }
                    }
                }
                Err(e) => {
                    xml_status = ValidationStatus::ReadError(format!("XML string decode error: {}", e));
                }
            }
        }

        // Check if essential companion SaveGameInfo is missing
        let validation_status = if !info_exists {
            ValidationStatus::MissingFiles("Companion SaveGameInfo file is missing".to_string())
        } else if !primary_exists {
            ValidationStatus::MissingFiles("Primary farm save file is missing".to_string())
        } else {
            xml_status
        };

        let validation_message = match &validation_status {
            ValidationStatus::Valid => None,
            other => Some(other.to_string()),
        };

        Ok(SaveFingerprint {
            farm_id: farm_folder.to_string(),
            source_type: source_type.to_string(),
            files,
            aggregate_hash,
            observed_at: Utc::now(),
            in_game_metadata: save_metadata,
            validation_status,
            validation_message,
        })
    }
}
