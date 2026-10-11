use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use crate::core::errors::CoreError;

pub struct SaveFilesystemWatcher {
    _watcher: RecommendedWatcher,
    pub event_rx: Receiver<WatcherNotification>,
}

#[derive(Debug, Clone)]
pub enum WatcherNotification {
    FarmModified(String),
    DirectoryRecreated,
    WatcherError(String),
}

impl SaveFilesystemWatcher {
    /// Starts watching the specified save root directory recursively.
    pub fn new(saves_dir: &Path) -> Result<Self, CoreError> {
        let (tx, event_rx) = channel();
        let watched_root = saves_dir.to_path_buf();

        let event_handler = move |res: notify::Result<Event>| {
            match res {
                Ok(event) => {
                    // Check if event paths target a known farm folder
                    for path in event.paths {
                        if let Some(farm_folder) = Self::extract_farm_folder(&path, &watched_root) {
                            let _ = tx.send(WatcherNotification::FarmModified(farm_folder));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(WatcherNotification::WatcherError(e.to_string()));
                }
            }
        };

        let mut watcher = RecommendedWatcher::new(event_handler, Config::default())
            .map_err(|e| CoreError::Watcher(format!("Failed to initialize filesystem watcher: {}", e)))?;

        if saves_dir.exists() {
            watcher
                .watch(saves_dir, RecursiveMode::Recursive)
                .map_err(|e| CoreError::Watcher(format!("Failed to watch saves path '{:?}': {}", saves_dir, e)))?;
        }

        Ok(Self {
            _watcher: watcher,
            event_rx,
        })
    }

    /// Extracts the immediate farm folder name from a modified file path under root.
    fn extract_farm_folder(path: &Path, root: &Path) -> Option<String> {
        let relative = path.strip_prefix(root).ok()?;
        let first_component = relative.components().next()?;
        let folder_name = first_component.as_os_str().to_str()?.to_string();

        if folder_name.is_empty() || folder_name.starts_with('.') {
            None
        } else {
            Some(folder_name)
        }
    }
}
