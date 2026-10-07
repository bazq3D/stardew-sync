use crate::core::errors::CoreError;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};
use sysinfo::System;

pub trait ProcessChecker: Send + Sync {
    fn is_process_running(&self, target_names: &[&str]) -> bool;
}

pub struct SystemProcessChecker;

impl ProcessChecker for SystemProcessChecker {
    fn is_process_running(&self, target_names: &[&str]) -> bool {
        let mut sys = System::new();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        for process in sys.processes().values() {
            let proc_name = process.name().to_string_lossy().to_lowercase();
            for target in target_names {
                let target_lower = target.to_lowercase();
                if proc_name == target_lower || proc_name.starts_with(&target_lower) {
                    return true;
                }
            }
        }
        false
    }
}

pub struct MockProcessChecker {
    pub running_processes: std::sync::Mutex<Vec<String>>,
}

impl Default for MockProcessChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl MockProcessChecker {
    pub fn new() -> Self {
        Self {
            running_processes: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn set_running(&self, processes: Vec<&str>) {
        let mut lock = self.running_processes.lock().unwrap();
        *lock = processes.into_iter().map(|s| s.to_string()).collect();
    }
}

impl ProcessChecker for MockProcessChecker {
    fn is_process_running(&self, target_names: &[&str]) -> bool {
        let lock = self.running_processes.lock().unwrap();
        for proc in lock.iter() {
            let proc_lower = proc.to_lowercase();
            for target in target_names {
                let target_lower = target.to_lowercase();
                if proc_lower == target_lower || proc_lower.starts_with(&target_lower) {
                    return true;
                }
            }
        }
        false
    }
}

pub struct ProcessMonitor<C: ProcessChecker = SystemProcessChecker> {
    pub checker: C,
}

impl ProcessMonitor<SystemProcessChecker> {
    pub fn new_system() -> Self {
        Self {
            checker: SystemProcessChecker,
        }
    }
}

impl<C: ProcessChecker> ProcessMonitor<C> {
    pub fn with_checker(checker: C) -> Self {
        Self { checker }
    }

    pub fn is_stardew_running(&self) -> bool {
        self.checker.is_process_running(&[
            "Stardew Valley.exe",
            "Stardew Valley",
            "StardewModdingAPI.exe",
            "StardewModdingAPI",
        ])
    }
}

#[derive(Debug, Clone)]
pub struct SaveSettleConfig {
    pub required_stable_observations: usize,
    pub check_interval: Duration,
    pub timeout: Duration,
}

impl Default for SaveSettleConfig {
    fn default() -> Self {
        Self {
            required_stable_observations: 3,
            check_interval: Duration::from_millis(300),
            timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileObservation {
    size: u64,
    mtime: Option<SystemTime>,
    hash: String,
}

pub struct SaveSettleDetector;

impl SaveSettleDetector {
    /// Monitors a save directory until its files exhibit consecutive, stable observations
    /// indicating that all operating system write buffers and game flushes have settled.
    pub fn wait_for_settled(save_dir: &Path, config: &SaveSettleConfig) -> Result<(), CoreError> {
        let start = Instant::now();
        let mut stable_count = 0;
        let mut last_observation: Option<BTreeMap<String, FileObservation>> = None;

        while start.elapsed() < config.timeout {
            let current = observe_directory(save_dir)?;

            if let Some(ref prev) = last_observation {
                if prev == &current {
                    stable_count += 1;
                    if stable_count >= config.required_stable_observations {
                        return Ok(());
                    }
                } else {
                    stable_count = 0;
                }
            }

            last_observation = Some(current);
            std::thread::sleep(config.check_interval);
        }

        Err(CoreError::SaveSettleTimeout {
            timeout_secs: config.timeout.as_secs(),
        })
    }
}

fn observe_directory(dir: &Path) -> Result<BTreeMap<String, FileObservation>, CoreError> {
    let mut map = BTreeMap::new();

    if !dir.exists() {
        return Ok(map);
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let metadata = std::fs::metadata(&path)?;
            let size = metadata.len();
            let mtime = metadata.modified().ok();

            // Try to open and compute hash (verifies file is readable and not exclusively locked)
            let content = std::fs::read(&path)?;
            let mut hasher = Sha256::new();
            hasher.update(&content);
            let hash = format!("{:x}", hasher.finalize());

            map.insert(name, FileObservation { size, mtime, hash });
        }
    }

    Ok(map)
}
