use crate::core::errors::CoreError;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub struct StagingArea {
    pub path: PathBuf,
    pub tx_id: String,
    auto_cleanup: bool,
}

impl StagingArea {
    /// Creates a fresh staging area in `work_dir` tagged with a unique transaction ID.
    pub fn new(work_dir: &Path) -> Result<Self, CoreError> {
        let tx_id = Uuid::new_v4().to_string();
        let path = work_dir.join(format!("staging_{}", tx_id));

        if path.exists() {
            std::fs::remove_dir_all(&path)?;
        }
        std::fs::create_dir_all(&path)?;

        Ok(Self {
            path,
            tx_id,
            auto_cleanup: true,
        })
    }

    /// Populates the staging area by copying an existing save folder.
    pub fn populate_from(&self, source_dir: &Path) -> Result<(), CoreError> {
        if !source_dir.is_dir() {
            return Err(CoreError::Transaction(format!(
                "Source save directory does not exist: {:?}",
                source_dir
            )));
        }

        for entry in std::fs::read_dir(source_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name().unwrap();
                let dest = self.path.join(file_name);
                std::fs::copy(&path, dest)?;
            }
        }

        Ok(())
    }

    /// Disables auto-cleanup and releases the staging path for atomic replacement.
    pub fn release_for_commit(mut self) -> PathBuf {
        self.auto_cleanup = false;
        self.path.clone()
    }

    /// Explicitly aborts and cleans up the staging directory.
    pub fn abort(mut self) -> Result<(), CoreError> {
        self.auto_cleanup = false;
        if self.path.exists() {
            std::fs::remove_dir_all(&self.path)?;
        }
        Ok(())
    }
}

impl Drop for StagingArea {
    fn drop(&mut self) {
        if self.auto_cleanup && self.path.exists() {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}
