use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    pub autostart_enabled: bool,
    pub start_minimized: bool,
    pub minimize_to_tray_on_close: bool,
    pub auto_check_updates: bool,
    pub last_update_check: Option<String>,
    pub last_notified_version: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            autostart_enabled: false,
            start_minimized: false,
            minimize_to_tray_on_close: false,
            auto_check_updates: true,
            last_update_check: None,
            last_notified_version: None,
        }
    }
}

static SETTINGS_CACHE: OnceLock<Mutex<AppSettings>> = OnceLock::new();

pub struct SettingsManager;

impl SettingsManager {
    /// Resolves the default settings file path under %APPDATA%\stardew-sync\settings.json
    pub fn get_default_settings_path() -> PathBuf {
        if let Ok(custom) = std::env::var("STARDEW_SYNC_SETTINGS_PATH") {
            return PathBuf::from(custom);
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            PathBuf::from(appdata).join("stardew-sync").join("settings.json")
        } else {
            PathBuf::from("stardew-sync-settings.json")
        }
    }

    /// Loads settings from the specified path, falling back to defaults if missing or invalid.
    pub fn load_from_path(path: &Path) -> AppSettings {
        if path.is_file() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
                    return settings;
                }
            }
        }
        AppSettings::default()
    }

    /// Saves settings to the specified path, ensuring parent directory exists.
    pub fn save_to_path(path: &Path, settings: &AppSettings) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| format!("Failed to create settings directory: {}", e))?;
            }
        }

        let json = serde_json::to_string_pretty(settings)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        fs::write(path, json).map_err(|e| format!("Failed to write settings file: {}", e))?;
        Ok(())
    }

    /// Retrieves current settings from the global memory cache (or loads from disk on first call).
    pub fn get_cached() -> AppSettings {
        let mutex = SETTINGS_CACHE.get_or_init(|| {
            let path = Self::get_default_settings_path();
            let initial = Self::load_from_path(&path);
            Mutex::new(initial)
        });

        mutex.lock().map(|guard| guard.clone()).unwrap_or_default()
    }

    /// Updates the global memory cache and persists settings to disk.
    pub fn update(settings: AppSettings) -> Result<AppSettings, String> {
        let path = Self::get_default_settings_path();
        Self::save_to_path(&path, &settings)?;

        let mutex = SETTINGS_CACHE.get_or_init(|| Mutex::new(settings.clone()));
        if let Ok(mut guard) = mutex.lock() {
            *guard = settings.clone();
        }

        Ok(settings)
    }

    /// Updates the last update check timestamp in the settings cache and on disk.
    pub fn record_update_check(timestamp_iso: String, release_version: Option<String>) {
        let mut settings = Self::get_cached();
        settings.last_update_check = Some(timestamp_iso);
        if let Some(ver) = release_version {
            settings.last_notified_version = Some(ver);
        }
        let _ = Self::update(settings);
    }

    /// For testing: resets the in-memory cache and returns a fresh Mutex.
    pub fn reset_cache_for_test(settings: AppSettings) {
        if let Some(mutex) = SETTINGS_CACHE.get() {
            if let Ok(mut guard) = mutex.lock() {
                *guard = settings;
            }
        }
    }
}
