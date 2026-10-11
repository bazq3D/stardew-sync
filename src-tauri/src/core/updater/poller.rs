use crate::commands::ActiveOperationGuard;
use crate::core::process::monitor::{ProcessChecker, SystemProcessChecker};
use crate::core::settings::SettingsManager;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundUpdateStatus {
    pub is_checking: bool,
    pub update_available: bool,
    pub latest_version: Option<String>,
    pub current_version: String,
    pub last_check_timestamp: Option<String>,
    pub release_notes: Option<String>,
    pub error: Option<String>,
}

static IS_CHECKING: AtomicBool = AtomicBool::new(false);
static LATEST_STATUS: Mutex<Option<BackgroundUpdateStatus>> = Mutex::new(None);

pub struct BackgroundUpdatePoller;

impl BackgroundUpdatePoller {
    /// Retrieves the most recent update check status from cache.
    pub fn get_latest_status() -> BackgroundUpdateStatus {
        if let Ok(guard) = LATEST_STATUS.lock() {
            if let Some(status) = &*guard {
                return status.clone();
            }
        }

        let settings = SettingsManager::get_cached();
        BackgroundUpdateStatus {
            is_checking: IS_CHECKING.load(Ordering::SeqCst),
            update_available: false,
            latest_version: None,
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            last_check_timestamp: settings.last_update_check,
            release_notes: None,
            error: None,
        }
    }

    /// Executes an update check against GitHub Releases.
    ///
    /// Safety Invariants:
    /// 1. Prevents overlapping requests using atomic compare-and-swap.
    /// 2. If `is_manual` is false and Stardew Valley is active, check is deferred so gameplay is never interrupted.
    /// 3. If a save-critical operation is active, check is deferred.
    /// 4. Does NOT acquire an updater reservation token (reservations are held strictly for installation).
    /// 5. Deduplicates notification events by comparing against `last_notified_version`.
    pub async fn check_for_updates(app: &AppHandle, is_manual: bool) -> Result<BackgroundUpdateStatus, String> {
        // 1. Concurrency control: prevent overlapping checks
        if IS_CHECKING.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_err() {
            return Err("An update check is already in progress.".to_string());
        }

        let result = Self::execute_check_internal(app, is_manual).await;
        IS_CHECKING.store(false, Ordering::SeqCst);

        match result {
            Ok(status) => {
                if let Ok(mut guard) = LATEST_STATUS.lock() {
                    *guard = Some(status.clone());
                }
                let _ = app.emit("background-update-status", &status);
                Ok(status)
            }
            Err(err_msg) => {
                let current_ver = env!("CARGO_PKG_VERSION").to_string();
                let status = BackgroundUpdateStatus {
                    is_checking: false,
                    update_available: false,
                    latest_version: None,
                    current_version: current_ver,
                    last_check_timestamp: SettingsManager::get_cached().last_update_check,
                    release_notes: None,
                    error: Some(err_msg.clone()),
                };
                if let Ok(mut guard) = LATEST_STATUS.lock() {
                    *guard = Some(status.clone());
                }
                let _ = app.emit("background-update-status", &status);
                Err(err_msg)
            }
        }
    }

    async fn execute_check_internal(app: &AppHandle, is_manual: bool) -> Result<BackgroundUpdateStatus, String> {
        let current_ver = env!("CARGO_PKG_VERSION").to_string();

        // 2. Check save-operation safety
        if ActiveOperationGuard::is_active() {
            return Err("Save operation is currently in progress; update check deferred.".to_string());
        }

        // 3. Process detection: If automated periodic check and game is active, defer without disturbing user
        let checker = SystemProcessChecker;
        let targets = [
            "Stardew Valley.exe",
            "Stardew Valley",
            "StardewModdingAPI.exe",
            "StardewModdingAPI",
        ];
        if !is_manual && checker.is_process_running(&targets) {
            return Err("Stardew Valley is currently running; background update notification deferred.".to_string());
        }

        // 4. Query Tauri 2 updater plugin
        let updater = app.updater().map_err(|e| format!("Updater initialization failed: {}", e))?;
        let check_res = updater.check().await.map_err(|e| format!("Failed to fetch release manifest: {}", e))?;

        let now_iso = Utc::now().to_rfc3339();

        match check_res {
            Some(update) => {
                let latest_ver = update.version.clone();
                let body = update.body.clone();
                let settings = SettingsManager::get_cached();

                // Check if we should notify: only if this version hasn't been notified yet
                let should_notify = settings.last_notified_version.as_deref() != Some(&latest_ver);

                // Record check timestamp and new version
                SettingsManager::record_update_check(now_iso.clone(), Some(latest_ver.clone()));

                let status = BackgroundUpdateStatus {
                    is_checking: false,
                    update_available: true,
                    latest_version: Some(latest_ver.clone()),
                    current_version: current_ver,
                    last_check_timestamp: Some(now_iso),
                    release_notes: body,
                    error: None,
                };

                if should_notify {
                    let _ = app.emit("update-available-notice", &status);
                }

                Ok(status)
            }
            None => {
                // Up to date
                SettingsManager::record_update_check(now_iso.clone(), None);

                Ok(BackgroundUpdateStatus {
                    is_checking: false,
                    update_available: false,
                    latest_version: None,
                    current_version: current_ver,
                    last_check_timestamp: Some(now_iso),
                    release_notes: None,
                    error: None,
                })
            }
        }
    }

    /// Starts the background periodic update polling loop.
    ///
    /// Runs an initial check 20 seconds after launch, then repeats every 4 hours
    /// if `settings.auto_check_updates` is enabled.
    pub fn spawn_periodic_poller(app: AppHandle) {
        tauri::async_runtime::spawn(async move {
            // Initial delay to avoid slowing down cold application launch
            tokio::time::sleep(Duration::from_secs(20)).await;

            loop {
                let settings = SettingsManager::get_cached();
                if settings.auto_check_updates {
                    let _ = Self::check_for_updates(&app, false).await;
                }

                // Poll interval: 4 hours
                tokio::time::sleep(Duration::from_secs(4 * 3600)).await;
            }
        });
    }
}
