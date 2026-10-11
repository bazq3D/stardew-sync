use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;
use crate::core::errors::CoreError;
use crate::core::monitoring::fingerprint::FingerprintGenerator;
use crate::core::monitoring::journal::ObservationJournalManager;
use crate::core::monitoring::settle::{SettleConfig, SettleAndVerifyPipeline};
use crate::core::monitoring::types::{
    DetectionReason, FarmMonitoringStatus, ObservationJournalEntry,
    SaveChangeClassification,
};
use crate::core::monitoring::watcher::{SaveFilesystemWatcher, WatcherNotification};
use crate::core::process::monitor::SystemProcessChecker;
use crate::core::save::discovery::discover_saves;

struct ServiceInner {
    saves_root: PathBuf,
    is_enabled: bool,
    statuses: HashMap<String, FarmMonitoringStatus>,
    last_reconciled: Option<DateTime<Utc>>,
    settle_config: SettleConfig,
}

#[derive(Clone)]
pub struct SaveMonitoringService {
    inner: Arc<RwLock<ServiceInner>>,
    _stop_tx: Arc<Mutex<Option<std::sync::mpsc::Sender<()>>>>,
}

impl SaveMonitoringService {
    /// Creates a new SaveMonitoringService targeting the specified saves directory.
    pub fn new(saves_root: PathBuf) -> Self {
        let default_config = SettleConfig::default();
        let inner = Arc::new(RwLock::new(ServiceInner {
            saves_root,
            is_enabled: true,
            statuses: HashMap::new(),
            last_reconciled: None,
            settle_config: default_config,
        }));

        Self {
            inner,
            _stop_tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Sets custom settle configuration (e.g. for fast isolated unit tests).
    pub fn set_settle_config(&self, config: SettleConfig) {
        if let Ok(mut inner) = self.inner.write() {
            inner.settle_config = config;
        }
    }

    /// Checks whether the service is actively monitoring.
    pub fn is_enabled(&self) -> bool {
        self.inner.read().map(|i| i.is_enabled).unwrap_or(false)
    }

    /// Toggles the monitoring service enabled/disabled state.
    pub fn set_enabled(&self, enabled: bool) {
        if let Ok(mut inner) = self.inner.write() {
            inner.is_enabled = enabled;
        }
    }

    /// Gets the monitored status of a specific farm.
    pub fn get_status(&self, farm_id: &str) -> Option<FarmMonitoringStatus> {
        let inner = self.inner.read().ok()?;
        inner.statuses.get(farm_id).cloned()
    }

    /// Gets all monitored farm statuses.
    pub fn get_all_statuses(&self) -> Vec<FarmMonitoringStatus> {
        let inner = match self.inner.read() {
            Ok(i) => i,
            Err(_) => return Vec::new(),
        };
        inner.statuses.values().cloned().collect()
    }

    /// Loads the observation journal history for a farm.
    pub fn get_journal(&self, farm_id: &str) -> Result<Vec<ObservationJournalEntry>, CoreError> {
        ObservationJournalManager::load_journal(farm_id)
    }

    /// Triggers an immediate settle-and-verify check on a specific farm.
    pub fn check_farm_now(
        &self,
        farm_id: &str,
        reason: DetectionReason,
    ) -> Result<FarmMonitoringStatus, CoreError> {
        FingerprintGenerator::validate_farm_identifier(farm_id)?;

        let (saves_root, is_enabled, settle_config) = {
            let inner = self.inner.read().map_err(|_| {
                CoreError::Monitoring("Failed to acquire service lock".to_string())
            })?;
            (inner.saves_root.clone(), inner.is_enabled, inner.settle_config.clone())
        };

        if !is_enabled {
            return Err(CoreError::Monitoring("Save monitoring service is disabled".to_string()));
        }

        let farm_dir = saves_root.join(farm_id);
        let is_running = SystemProcessChecker::is_stardew_running();

        // Get previous known hash from journal
        let previous_hash = ObservationJournalManager::get_latest_entry(farm_id)
            .map(|e| e.current_fingerprint);

        let (fingerprint, classification) = SettleAndVerifyPipeline::execute(
            farm_id,
            &farm_dir,
            "StandardAppdata",
            is_running,
            previous_hash.as_deref(),
            &settle_config,
        )?;

        // Record observation in application-owned journal
        let journal_entry = ObservationJournalManager::record_observation(
            farm_id,
            &fingerprint,
            classification,
            reason,
            is_running,
        )?;

        let short_fingerprint = if !fingerprint.aggregate_hash.is_empty() {
            Some(fingerprint.aggregate_hash[..8.min(fingerprint.aggregate_hash.len())].to_string())
        } else {
            None
        };

        // Determine last verified change timestamp
        let last_change = {
            let inner = self.inner.read().ok();
            let prev_status = inner.as_ref().and_then(|i| i.statuses.get(farm_id));
            if classification == SaveChangeClassification::LocalChangeDetected {
                Some(journal_entry.last_seen)
            } else {
                prev_status.and_then(|s| s.last_verified_change_at).or(Some(journal_entry.last_seen))
            }
        };

        let status = FarmMonitoringStatus {
            farm_id: farm_id.to_string(),
            is_enabled: true,
            classification,
            classification_label: classification.to_string(),
            current_fingerprint: if fingerprint.aggregate_hash.is_empty() {
                None
            } else {
                Some(fingerprint.aggregate_hash)
            },
            short_fingerprint,
            last_observed_at: Some(journal_entry.last_seen),
            last_verified_change_at: last_change,
            observation_sequence: journal_entry.sequence_number,
            is_game_running: is_running,
            is_settling: classification == SaveChangeClassification::WaitingForFileStability,
            validation_status: fingerprint.validation_status,
            error_message: fingerprint.validation_message,
            sync_indicator: "Sync Status Unknown".to_string(),
        };

        // Update in-memory cache
        if let Ok(mut inner) = self.inner.write() {
            inner.statuses.insert(farm_id.to_string(), status.clone());
        }

        Ok(status)
    }

    /// Reconciles all discovered farms in the saves directory.
    pub fn reconcile_all(&self, reason: DetectionReason) -> Vec<FarmMonitoringStatus> {
        let (saves_root, is_enabled) = match self.inner.read() {
            Ok(inner) => (inner.saves_root.clone(), inner.is_enabled),
            Err(_) => return Vec::new(),
        };

        if !is_enabled || !saves_root.exists() {
            return Vec::new();
        }

        let discovered = match discover_saves(&saves_root) {
            Ok(saves) => saves,
            Err(_) => return Vec::new(),
        };

        let mut results = Vec::new();
        for save in discovered {
            if let Ok(status) = self.check_farm_now(&save.folder_name, reason) {
                results.push(status);
            }
        }

        if let Ok(mut inner) = self.inner.write() {
            inner.last_reconciled = Some(Utc::now());
        }

        results
    }

    /// Spawns background worker thread for filesystem events and periodic reconciliation.
    pub fn start_background_loop(&self) {
        let saves_root = match self.inner.read() {
            Ok(i) => i.saves_root.clone(),
            Err(_) => return,
        };

        let (stop_tx, stop_rx) = std::sync::mpsc::channel();
        if let Ok(mut lock) = self._stop_tx.lock() {
            *lock = Some(stop_tx);
        }

        let service = self.clone();

        thread::spawn(move || {
            // Initial scan of all farms on startup
            service.reconcile_all(DetectionReason::InitialScan);

            // Attempt to bind native watcher
            let watcher_res = SaveFilesystemWatcher::new(&saves_root);
            let mut event_rx_opt = watcher_res.ok().map(|w| w.event_rx);

            let mut last_periodic = std::time::Instant::now();
            let periodic_interval = Duration::from_secs(30);

            loop {
                // Check for stop signal
                if stop_rx.try_recv().is_ok() {
                    break;
                }

                // Process filesystem events if watcher is active
                if let Some(ref rx) = event_rx_opt {
                    while let Ok(notification) = rx.try_recv() {
                        match notification {
                            WatcherNotification::FarmModified(farm_folder) => {
                                let _ = service.check_farm_now(&farm_folder, DetectionReason::FilesystemEvent);
                            }
                            WatcherNotification::DirectoryRecreated => {
                                service.reconcile_all(DetectionReason::PeriodicReconcile);
                            }
                            WatcherNotification::WatcherError(_) => {
                                // Watcher buffer overflow / error -> reconcile everything
                                service.reconcile_all(DetectionReason::PeriodicReconcile);
                            }
                        }
                    }
                } else if saves_root.exists() {
                    // Try to re-bind watcher if directory was created
                    if let Ok(w) = SaveFilesystemWatcher::new(&saves_root) {
                        event_rx_opt = Some(w.event_rx);
                    }
                }

                // Periodic reconciliation to catch missed events (e.g. after sleep or drop)
                if last_periodic.elapsed() >= periodic_interval {
                    service.reconcile_all(DetectionReason::PeriodicReconcile);
                    last_periodic = std::time::Instant::now();
                }

                thread::sleep(Duration::from_millis(500));
            }
        });
    }

    /// Stops the background monitoring worker loop.
    pub fn stop(&self) {
        if let Ok(mut lock) = self._stop_tx.lock() {
            if let Some(tx) = lock.take() {
                let _ = tx.send(());
            }
        }
    }
}
