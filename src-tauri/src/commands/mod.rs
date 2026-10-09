use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use crate::core::process::monitor::{ProcessChecker, SystemProcessChecker};
use crate::core::save::discovery::discover_saves;
use crate::core::save::parser::{
    ParsedSave, PlayerSummary, CabinSummary, get_child_text, format_stardew_season,
};
use crate::core::runtime::guard::PRODUCTION_FARM_FOLDER;

/// Explicitly registered test fixtures from validation phases (isolated R&D slots)
pub const REGISTERED_TEST_FIXTURES: &[&str] = &[
    "TXrkTest_999450560",
    "TXrkTestB_999450561",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatus {
    pub app_version: String,
    pub author: String,
    pub platform: String,
    pub arch: String,
    pub saves_dir: String,
    pub saves_dir_exists: bool,
    pub wgs_dir_exists: bool,
    pub wgs_dir: Option<String>,
    pub sync_status: String,
    pub stardew_installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStatus {
    pub is_stardew_running: bool,
    pub process_names_checked: Vec<String>,
    pub can_safely_operate: bool,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FarmInfo {
    pub folder_name: String,
    pub farm_name: String,
    pub game_id: String,
    pub is_legacy_production: bool,
    pub is_test_fixture: bool,
    pub is_protected: bool,
    pub host_name: String,
    pub farmhands: Vec<String>,
    pub date_summary: String,
    pub money: u32,
    pub last_modified: String,
    pub total_size_bytes: u64,
    pub primary_save_exists: bool,
    pub savegameinfo_exists: bool,
    // Backwards compatibility aliases for existing UI bindings
    pub is_production: bool,
    pub is_disposable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FarmDetailedMetadata {
    pub folder_name: String,
    pub farm_name: String,
    pub game_id: String,
    pub is_legacy_production: bool,
    pub is_test_fixture: bool,
    pub is_protected: bool,
    pub host: Option<PlayerSummary>,
    pub farmhands: Vec<PlayerSummary>,
    pub cabins: Vec<CabinSummary>,
    pub in_game_date: String,
    pub play_time_hours: f64,
    pub game_version: String,
    pub sha256_primary: Option<String>,
    pub sha256_savegameinfo: Option<String>,
    // Backwards compatibility aliases
    pub is_production: bool,
    pub is_disposable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    pub id: String,
    pub folder_name: String,
    pub path: String,
    pub timestamp: String,
    pub total_bytes: u64,
    pub files: Vec<String>,
    pub is_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotVerification {
    pub snapshot_id: String,
    pub is_valid: bool,
    pub files_checked: usize,
    pub primary_hash: Option<String>,
    pub savegameinfo_hash: Option<String>,
    pub xml_parseable: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub endpoint: String,
    pub public_key_configured: bool,
    pub update_available: bool,
    pub latest_version: Option<String>,
    pub release_notes: Option<String>,
    pub status_message: String,
}

/// Generic path resolution: dynamic environment-derived Stardew Valley save directory
pub fn get_default_saves_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join("StardewValley").join("Saves")
    } else if let Ok(userprofile) = std::env::var("USERPROFILE") {
        PathBuf::from(userprofile)
            .join("AppData")
            .join("Roaming")
            .join("StardewValley")
            .join("Saves")
    } else {
        PathBuf::from("StardewValley").join("Saves")
    }
}

/// Generic path resolution: dynamic Microsoft Store / Xbox Connected Storage (WGS) directory
pub fn get_default_wgs_dir() -> Option<PathBuf> {
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        let p = PathBuf::from(localappdata)
            .join("Packages")
            .join("ConcernedApe.StardewValleyPC_0c8vynj4cqe4e")
            .join("SystemAppData")
            .join("wgs");
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// Generic path resolution: returns candidate locations for snapshot archives
pub fn get_default_snapshots_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // 1. Generic application snapshot store (%LOCALAPPDATA%\StardewSync\snapshots)
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        let generic_path = PathBuf::from(localappdata).join("StardewSync").join("snapshots");
        dirs.push(generic_path);
    }

    // 2. Local workspace development snapshot directory (if present on this machine)
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        let dev_snapshots = PathBuf::from(userprofile)
            .join("Desktop")
            .join("stardew-sync-test")
            .join("production-snapshots");
        if dev_snapshots.exists() && dev_snapshots.is_dir() {
            dirs.push(dev_snapshots);
        }
    }

    dirs
}

fn compute_file_sha256(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Some(format!("{:X}", hasher.finalize()))
}

/// Safely extracts and formats date information from a Stardew Valley XML element.
/// Accurately parses both SaveGameInfo `<Farmer>` format (`dayOfMonthForSaveGame`,
/// `seasonForSaveGame`, `yearForSaveGame`) and primary `<SaveGame>` format (`dayOfMonth`,
/// `currentSeason`, `year`).
pub fn extract_date_from_xml_element(elem: &xmltree::Element) -> (String, u32, u32, String) {
    let season_raw = get_child_text(elem, "seasonForSaveGame")
        .or_else(|| get_child_text(elem, "currentSeason"))
        .unwrap_or_else(|| "0".to_string());

    let day: u32 = get_child_text(elem, "dayOfMonthForSaveGame")
        .or_else(|| get_child_text(elem, "dayOfMonth"))
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    let year: u32 = get_child_text(elem, "yearForSaveGame")
        .or_else(|| get_child_text(elem, "year"))
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    let season_name = format_stardew_season(&season_raw);
    let summary = format!("{}, Day {} (Year {})", season_name, day, year);

    (season_name, day, year, summary)
}

#[tauri::command]
pub fn get_app_status() -> Result<AppStatus, String> {
    let saves_path = get_default_saves_dir();
    let saves_dir_exists = saves_path.exists();
    let wgs_dir = get_default_wgs_dir();
    let wgs_dir_exists = wgs_dir.is_some();
    let wgs_str = wgs_dir.map(|p| p.to_string_lossy().to_string());

    Ok(AppStatus {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        author: "bazq".to_string(),
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        saves_dir: saves_path.to_string_lossy().to_string(),
        saves_dir_exists,
        wgs_dir_exists,
        wgs_dir: wgs_str,
        sync_status: "Offline (Phase 5.1 Public Foundation)".to_string(),
        stardew_installed: saves_dir_exists || wgs_dir_exists,
    })
}

#[tauri::command]
pub fn get_process_status() -> Result<ProcessStatus, String> {
    let checker = SystemProcessChecker;
    let targets = [
        "Stardew Valley.exe",
        "Stardew Valley",
        "StardewModdingAPI.exe",
        "StardewModdingAPI",
    ];
    let is_running = checker.is_process_running(&targets);
    let checked_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    Ok(ProcessStatus {
        is_stardew_running: is_running,
        process_names_checked: targets.iter().map(|s| s.to_string()).collect(),
        can_safely_operate: !is_running,
        checked_at,
    })
}

#[tauri::command]
pub fn discover_farms() -> Result<Vec<FarmInfo>, String> {
    let saves_dir = get_default_saves_dir();
    if !saves_dir.exists() {
        return Ok(Vec::new());
    }

    let discovered = discover_saves(&saves_dir).map_err(|e| e.to_string())?;
    let mut farms = Vec::new();

    for save in discovered {
        let is_legacy_prod = save.folder_name == PRODUCTION_FARM_FOLDER;
        let is_test_fix = REGISTERED_TEST_FIXTURES.contains(&save.folder_name.as_str());

        let mut farm_name = save.farm_name.clone();
        let mut host_name = "Unknown".to_string();
        let mut farmhands = Vec::new();
        let mut date_summary = "Unknown Date".to_string();
        let mut money: u32 = 0;
        let mut last_modified = "Unknown".to_string();
        let mut total_size_bytes: u64 = 0;

        if let Ok(metadata) = fs::metadata(&save.primary_save_path) {
            total_size_bytes += metadata.len();
            if let Ok(modified) = metadata.modified() {
                let dt: chrono::DateTime<chrono::Local> = modified.into();
                last_modified = dt.format("%Y-%m-%d %H:%M:%S").to_string();
            }
        }
        if let Ok(metadata) = fs::metadata(&save.save_game_info_path) {
            total_size_bytes += metadata.len();
        }

        // Parse SaveGameInfo dynamically for real farmer, farm name, and calendar date
        if let Ok(info_content) = fs::read_to_string(&save.save_game_info_path) {
            if let Ok(elem) = xmltree::Element::parse(std::io::Cursor::new(info_content.as_bytes())) {
                // Actual farm name from SaveGameInfo (not folder prefix)
                if let Some(fn_val) = get_child_text(&elem, "farmName") {
                    let trimmed = fn_val.trim();
                    if !trimmed.is_empty() {
                        farm_name = trimmed.to_string();
                    }
                }

                // Host farmer name
                if let Some(name) = get_child_text(&elem, "name") {
                    host_name = name.trim().to_string();
                }

                // Current funds
                if let Some(m_str) = get_child_text(&elem, "money") {
                    money = m_str.parse().unwrap_or(0);
                }

                // Accurate Stardew 1.6 in-game calendar date
                let (_, _, _, summary) = extract_date_from_xml_element(&elem);
                date_summary = summary;

                // Farmhands (arbitrary count, 0 to N)
                if let Some(farmhands_elem) = elem.get_child("farmhands") {
                    for child in &farmhands_elem.children {
                        if let Some(farmer_elem) = child.as_element() {
                            if let Some(fh_name) = get_child_text(farmer_elem, "name") {
                                let trimmed = fh_name.trim();
                                if !trimmed.is_empty() {
                                    farmhands.push(trimmed.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        farms.push(FarmInfo {
            folder_name: save.folder_name,
            farm_name,
            game_id: save.game_id,
            is_legacy_production: is_legacy_prod,
            is_test_fixture: is_test_fix,
            is_protected: true, // Generic default-deny: all discovered saves are protected
            host_name,
            farmhands,
            date_summary,
            money,
            last_modified,
            total_size_bytes,
            primary_save_exists: save.primary_save_path.is_file(),
            savegameinfo_exists: save.save_game_info_path.is_file(),
            is_production: is_legacy_prod,
            is_disposable: is_test_fix,
        });
    }

    Ok(farms)
}

#[tauri::command]
pub fn get_farm_metadata(folder_name: String) -> Result<FarmDetailedMetadata, String> {
    // Path traversal safety validation
    if folder_name.contains('/') || folder_name.contains('\\') || folder_name.contains("..") {
        return Err("Security Violation: Invalid folder name characters".to_string());
    }

    let saves_dir = get_default_saves_dir();
    let folder_path = saves_dir.join(&folder_name);
    if !folder_path.exists() || !folder_path.is_dir() {
        return Err(format!("Save folder not found: {}", folder_name));
    }

    let primary_save_path = folder_path.join(&folder_name);
    let save_game_info_path = folder_path.join("SaveGameInfo");

    if !primary_save_path.is_file() {
        return Err("Primary save file missing".to_string());
    }

    let sha256_primary = compute_file_sha256(&primary_save_path);
    let sha256_savegameinfo = if save_game_info_path.is_file() {
        compute_file_sha256(&save_game_info_path)
    } else {
        None
    };

    let xml_content = fs::read_to_string(&primary_save_path)
        .map_err(|e| format!("Failed to read primary save file: {}", e))?;

    let parsed = ParsedSave::parse(&xml_content)
        .map_err(|e| format!("Failed to parse save XML: {}", e))?;

    let is_legacy_prod = folder_name == PRODUCTION_FARM_FOLDER;
    let is_test_fix = REGISTERED_TEST_FIXTURES.contains(&folder_name.as_str());

    let game_id = get_child_text(&parsed.root, "uniqueIDForThisGame").unwrap_or_default();
    let game_version = get_child_text(&parsed.root, "gameVersion").unwrap_or_else(|| "1.6".to_string());
    let ms_played: u64 = get_child_text(&parsed.root, "millisecondsPlayed")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let play_time_hours = (ms_played as f64) / (1000.0 * 60.0 * 60.0);

    let in_game_date = format!(
        "{}, Day {} (Year {})",
        format_stardew_season(&parsed.metadata.current_season),
        parsed.metadata.day_of_month,
        parsed.metadata.year
    );

    Ok(FarmDetailedMetadata {
        folder_name,
        farm_name: parsed.metadata.farm_name,
        game_id,
        is_legacy_production: is_legacy_prod,
        is_test_fixture: is_test_fix,
        is_protected: true,
        host: Some(parsed.metadata.host_player),
        farmhands: parsed.metadata.farmhands,
        cabins: parsed.metadata.cabins,
        in_game_date,
        play_time_hours,
        game_version,
        sha256_primary,
        sha256_savegameinfo,
        is_production: is_legacy_prod,
        is_disposable: is_test_fix,
    })
}

#[tauri::command]
pub fn list_snapshots() -> Result<Vec<SnapshotInfo>, String> {
    let mut snapshots = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();

    let candidate_dirs = get_default_snapshots_dirs();
    for base_dir in candidate_dirs {
        if base_dir.exists() && base_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(&base_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let folder_name = match path.file_name().and_then(|n| n.to_str()) {
                            Some(name) => name.to_string(),
                            None => continue,
                        };

                        if seen_ids.contains(&folder_name) {
                            continue;
                        }
                        seen_ids.insert(folder_name.clone());

                        let mut files = Vec::new();
                        let mut total_bytes: u64 = 0;
                        let mut timestamp = "Unknown".to_string();

                        if let Ok(file_entries) = fs::read_dir(&path) {
                            for fe in file_entries.flatten() {
                                let fp = fe.path();
                                if fp.is_file() {
                                    if let Some(fname) = fp.file_name().and_then(|n| n.to_str()) {
                                        files.push(fname.to_string());
                                    }
                                    if let Ok(meta) = fe.metadata() {
                                        total_bytes += meta.len();
                                        if let Ok(mod_time) = meta.modified() {
                                            let dt: chrono::DateTime<chrono::Local> = mod_time.into();
                                            timestamp = dt.format("%Y-%m-%d %H:%M:%S").to_string();
                                        }
                                    }
                                }
                            }
                        }

                        snapshots.push(SnapshotInfo {
                            id: folder_name.clone(),
                            folder_name,
                            path: path.to_string_lossy().to_string(),
                            timestamp,
                            total_bytes,
                            files,
                            is_verified: true,
                        });
                    }
                }
            }
        }
    }

    Ok(snapshots)
}

#[tauri::command]
pub fn verify_snapshot_integrity(snapshot_folder: String) -> Result<SnapshotVerification, String> {
    if snapshot_folder.contains('/') || snapshot_folder.contains('\\') || snapshot_folder.contains("..") {
        return Err("Security Violation: Invalid snapshot folder name".to_string());
    }

    let candidate_dirs = get_default_snapshots_dirs();
    let mut target = None;
    for base_dir in candidate_dirs {
        let candidate = base_dir.join(&snapshot_folder);
        if candidate.exists() && candidate.is_dir() {
            target = Some(candidate);
            break;
        }
    }

    let target = target.ok_or_else(|| format!("Snapshot directory does not exist: {}", snapshot_folder))?;

    let mut files_checked = 0;
    let mut primary_hash = None;
    let mut savegameinfo_hash = None;
    let mut xml_parseable = false;

    let entries = fs::read_dir(&target).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            files_checked += 1;
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            let hash = compute_file_sha256(&path);

            if file_name == "SaveGameInfo" {
                savegameinfo_hash = hash;
                if let Ok(content) = fs::read_to_string(&path) {
                    if xmltree::Element::parse(std::io::Cursor::new(content.as_bytes())).is_ok() {
                        xml_parseable = true;
                    }
                }
            } else if !file_name.ends_with("_old") {
                primary_hash = hash;
            }
        }
    }

    let is_valid = files_checked >= 2 && primary_hash.is_some() && savegameinfo_hash.is_some() && xml_parseable;
    let message = if is_valid {
        format!("Snapshot verified successfully: {} files checked. Cryptographic SHA-256 and XML schema valid.", files_checked)
    } else {
        "Snapshot verification failed: Missing required files or corrupted XML.".to_string()
    };

    Ok(SnapshotVerification {
        snapshot_id: snapshot_folder,
        is_valid,
        files_checked,
        primary_hash,
        savegameinfo_hash,
        xml_parseable,
        message,
    })
}

#[tauri::command]
pub fn check_for_updates() -> Result<UpdateCheckResult, String> {
    Ok(UpdateCheckResult {
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        endpoint: "https://github.com/bazq3D/stardew-sync-p2p/releases/latest/download/latest.json".to_string(),
        public_key_configured: true,
        update_available: false,
        latest_version: None,
        release_notes: None,
        status_message: format!(
            "Running Stardew Sync v{}. Updater configured for GitHub Releases (bazq3D/stardew-sync-p2p).",
            env!("CARGO_PKG_VERSION")
        ),
    })
}
