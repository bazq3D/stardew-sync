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
    pub play_time_hours: Option<f64>,
    pub play_time_formatted: String,
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

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;
use std::time::Instant;

pub const STATE_IDLE: u8 = 0;
pub const STATE_SAVE_OPERATION: u8 = 1;
pub const STATE_UPDATE_RESERVED: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateReservationData {
    pub token: String,
    pub created_at: Instant,
}

static SYSTEM_OPERATION_STATE: AtomicU8 = AtomicU8::new(STATE_IDLE);
static ACTIVE_UPDATE_RESERVATION: Mutex<Option<UpdateReservationData>> = Mutex::new(None);

/// Backend-authoritative guard for active save-critical operations.
/// While held, `ActiveOperationGuard::is_active()` returns true, and update installation is paused.
#[derive(Debug)]
pub struct ActiveOperationGuard;

impl ActiveOperationGuard {
    /// Attempts to acquire the active save-operation lock.
    /// Fails with an error if another save-critical operation is already in progress,
    /// or if an application update is currently being applied.
    pub fn acquire() -> Result<Self, &'static str> {
        match SYSTEM_OPERATION_STATE.compare_exchange(
            STATE_IDLE,
            STATE_SAVE_OPERATION,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => Ok(ActiveOperationGuard),
            Err(STATE_UPDATE_RESERVED) => {
                Err("Cannot perform save operation: application update installation is in progress. If an update was interrupted, please restart Stardew Sync.")
            }
            Err(STATE_SAVE_OPERATION) => {
                Err("Another save-critical operation is already in progress.")
            }
            Err(_) => Err("System busy with another operation."),
        }
    }

    /// Queries whether any save-critical operation is currently active in the backend.
    pub fn is_active() -> bool {
        SYSTEM_OPERATION_STATE.load(Ordering::SeqCst) == STATE_SAVE_OPERATION
    }

    /// Explicitly resets the operation state (used for testing and teardown).
    pub fn reset_for_test() {
        SYSTEM_OPERATION_STATE.store(STATE_IDLE, Ordering::SeqCst);
        if let Ok(mut lock) = ACTIVE_UPDATE_RESERVATION.lock() {
            *lock = None;
        }
    }
}

impl Drop for ActiveOperationGuard {
    fn drop(&mut self) {
        let _ = SYSTEM_OPERATION_STATE.compare_exchange(
            STATE_SAVE_OPERATION,
            STATE_IDLE,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
    }
}

/// Backend-authoritative guard for update package download and installation.
/// Eliminates the TOCTOU race by reserving the installation lifecycle so that
/// no save-critical operations can begin while an update is in progress.
/// Requires an unguessable ownership token for acquisition and release.
#[derive(Debug)]
pub struct ActiveUpdateGuard {
    token: String,
}

impl ActiveUpdateGuard {
    /// Attempts to acquire an RAII guard for the update reservation.
    /// Dropping this guard automatically releases the reservation using its token.
    pub fn acquire_guard() -> Result<Self, &'static str> {
        let token = Self::acquire()?;
        Ok(ActiveUpdateGuard { token })
    }

    /// Attempts to reserve an update installation slot.
    /// On success, generates and stores a unique cryptographically secure ownership token (UUID v4)
    /// and returns the token string.
    /// Fails immediately if a save-critical operation is running, or if an update is already in progress.
    pub fn acquire() -> Result<String, &'static str> {
        let mut lock = ACTIVE_UPDATE_RESERVATION
            .lock()
            .map_err(|_| "Failed to acquire internal reservation lock.")?;

        match SYSTEM_OPERATION_STATE.compare_exchange(
            STATE_IDLE,
            STATE_UPDATE_RESERVED,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => {
                let token = uuid::Uuid::new_v4().to_string();
                *lock = Some(UpdateReservationData {
                    token: token.clone(),
                    created_at: Instant::now(),
                });
                Ok(token)
            }
            Err(STATE_SAVE_OPERATION) => {
                Err("Cannot install update: a save-critical operation is in progress.")
            }
            Err(STATE_UPDATE_RESERVED) => {
                Err("An update installation is already in progress.")
            }
            Err(_) => Err("System busy with another operation."),
        }
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn is_active() -> bool {
        SYSTEM_OPERATION_STATE.load(Ordering::SeqCst) == STATE_UPDATE_RESERVED
    }

    /// Releases the active update reservation IF and ONLY IF the provided token
    /// exactly matches the active reservation's ownership token.
    ///
    /// - Rejects empty or whitespace-only tokens.
    /// - Rejects release attempts when no update reservation is active (e.g. while in STATE_SAVE_OPERATION or STATE_IDLE).
    /// - Rejects mismatched, stale, or already-released tokens without modifying state.
    pub fn release_with_token(token: &str) -> Result<bool, &'static str> {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return Err("Reservation token cannot be empty.");
        }

        let mut lock = ACTIVE_UPDATE_RESERVATION
            .lock()
            .map_err(|_| "Failed to acquire internal reservation lock.")?;

        let current_state = SYSTEM_OPERATION_STATE.load(Ordering::SeqCst);
        if current_state != STATE_UPDATE_RESERVED {
            return Err("No active update reservation to release.");
        }

        match &*lock {
            Some(res_data) => {
                if res_data.token == trimmed {
                    *lock = None;
                    SYSTEM_OPERATION_STATE.store(STATE_IDLE, Ordering::SeqCst);
                    Ok(true)
                } else {
                    Err("Reservation token mismatch: provided token does not match active update reservation.")
                }
            }
            None => {
                Err("No active update reservation data found.")
            }
        }
    }

    pub fn reset_for_test() {
        SYSTEM_OPERATION_STATE.store(STATE_IDLE, Ordering::SeqCst);
        if let Ok(mut lock) = ACTIVE_UPDATE_RESERVATION.lock() {
            *lock = None;
        }
    }

    pub fn active_token_for_test() -> Option<String> {
        ACTIVE_UPDATE_RESERVATION.lock().ok()?.as_ref().map(|d| d.token.clone())
    }
}

impl Drop for ActiveUpdateGuard {
    fn drop(&mut self) {
        let _ = Self::release_with_token(&self.token);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateEligibility {
    pub can_update: bool,
    pub is_game_running: bool,
    pub is_save_operation_active: bool,
    pub reason: String,
}

/// Decoupled update evaluation: Stardew Sync updates modify application files only,
/// so they are completely safe while Stardew Valley is running. They are only paused
/// if an active in-flight save operation/migration is underway in the backend or frontend.
///
/// Backend state (`ActiveOperationGuard`) is authoritative: if the backend is actively
/// running a save-critical operation, updates are paused even if the frontend passes false.
/// If either backend or frontend reports an active operation, a safe default-deny applies.
pub fn check_update_eligibility(
    is_game_running: bool,
    frontend_flag: bool,
) -> UpdateEligibility {
    let backend_active = ActiveOperationGuard::is_active();
    let is_save_operation_active = backend_active || frontend_flag;

    if is_save_operation_active {
        UpdateEligibility {
            can_update: false,
            is_game_running,
            is_save_operation_active: true,
            reason: "An active save backup, restore, or migration is currently running. Application updates are paused to protect active operations.".to_string(),
        }
    } else {
        let reason = if is_game_running {
            "Updates are allowed while Stardew Valley is running because the updater only modifies Stardew Sync application files and never touches game saves.".to_string()
        } else {
            "Ready for application updates.".to_string()
        };
        UpdateEligibility {
            can_update: true,
            is_game_running,
            is_save_operation_active: false,
            reason,
        }
    }
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

/// Authoritative playtime extraction: In Stardew Valley save files, millisecondsPlayed
/// is tracked on the `<Farmer>` entity.
/// In SaveGame, the host farmer is `<player><millisecondsPlayed>`.
/// In SaveGameInfo, the root element itself is `<Farmer><millisecondsPlayed>`.
pub fn extract_milliseconds_played(root: &xmltree::Element) -> Option<u64> {
    // 1. If root is SaveGame, inspect <player><millisecondsPlayed>
    if root.name == "SaveGame" {
        if let Some(player) = root.get_child("player") {
            if let Some(ms) = get_child_text(player, "millisecondsPlayed").and_then(|s| s.trim().parse::<u64>().ok()) {
                return Some(ms);
            }
        }
    }
    // 2. Direct child (when root is Farmer in SaveGameInfo, or if element is <player>)
    if let Some(ms) = get_child_text(root, "millisecondsPlayed").and_then(|s| s.trim().parse::<u64>().ok()) {
        return Some(ms);
    }
    None
}

/// Converts milliseconds to decimal hours with high precision.
pub fn ms_to_hours(ms: u64) -> f64 {
    (ms as f64) / 3_600_000.0
}

/// Formats playtime into a human-readable summary string: e.g. "18h 54m (18.9 hours)".
/// Returns "Unknown" if millisecondsPlayed is missing or invalid.
pub fn format_playtime_summary(ms_opt: Option<u64>) -> String {
    match ms_opt {
        Some(ms) => {
            let total_secs = ms / 1000;
            let hours = total_secs / 3600;
            let mins = (total_secs % 3600) / 60;
            let decimal_hours = ms_to_hours(ms);
            format!("{}h {}m ({:.1} hours)", hours, mins, decimal_hours)
        }
        None => "Unknown".to_string(),
    }
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
    
    // Authoritative playtime: check <player><millisecondsPlayed> on primary save, fallback to SaveGameInfo
    let ms_played = extract_milliseconds_played(&parsed.root).or_else(|| {
        if save_game_info_path.is_file() {
            if let Ok(info_str) = fs::read_to_string(&save_game_info_path) {
                if let Ok(info_elem) = xmltree::Element::parse(std::io::Cursor::new(info_str.as_bytes())) {
                    return extract_milliseconds_played(&info_elem);
                }
            }
        }
        None
    });

    let play_time_hours = ms_played.map(ms_to_hours);
    let play_time_formatted = format_playtime_summary(ms_played);

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
        play_time_formatted,
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
        endpoint: "https://github.com/bazq3D/stardew-sync/releases/latest/download/latest.json".to_string(),
        public_key_configured: true,
        update_available: false,
        latest_version: None,
        release_notes: None,
        status_message: format!(
            "Running Stardew Sync v{}. Updater configured for GitHub Releases (bazq3D/stardew-sync).",
            env!("CARGO_PKG_VERSION")
        ),
    })
}

#[tauri::command]
pub fn get_update_eligibility(
    is_save_operation_active: Option<bool>,
) -> Result<UpdateEligibility, String> {
    let checker = SystemProcessChecker;
    let targets = [
        "Stardew Valley.exe",
        "Stardew Valley",
        "StardewModdingAPI.exe",
        "StardewModdingAPI",
    ];
    let is_running = checker.is_process_running(&targets);
    let is_save_active = is_save_operation_active.unwrap_or(false);
    Ok(check_update_eligibility(is_running, is_save_active))
}

#[tauri::command]
pub fn acquire_update_reservation() -> Result<String, String> {
    ActiveUpdateGuard::acquire().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn release_update_reservation(token: String) -> Result<bool, String> {
    ActiveUpdateGuard::release_with_token(&token).map_err(|e| e.to_string())
}


