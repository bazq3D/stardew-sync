use crate::core::errors::CoreError;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredSave {
    pub farm_name: String,
    pub game_id: String,
    pub folder_name: String,
    pub folder_path: PathBuf,
    pub primary_save_path: PathBuf,
    pub save_game_info_path: PathBuf,
    pub has_old_save: bool,
    pub has_old_save_info: bool,
}

/// Discovers Stardew Valley save folders in an explicitly supplied directory.
///
/// SAFETY: This function strictly operates ONLY within the provided `root` directory.
/// It never reads, scans, or touches `%APPDATA%\StardewValley\Saves` unless that exact
/// path is explicitly passed as `root`.
pub fn discover_saves(root: &Path) -> Result<Vec<DiscoveredSave>, CoreError> {
    if !root.exists() {
        return Err(CoreError::SaveDirectoryNotFound(root.to_path_buf()));
    }

    let mut saves = Vec::new();

    let entries = std::fs::read_dir(root)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let folder_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        // Standard Stardew save folder format: <FarmName>_<GameID>
        let parts: Vec<&str> = folder_name.rsplitn(2, '_').collect();
        if parts.len() != 2 {
            continue;
        }
        let game_id = parts[0].to_string();
        let farm_name = parts[1].to_string();

        // 1. Required Live Files
        let primary_save_path = path.join(&folder_name);
        let save_game_info_path = path.join("SaveGameInfo");

        if !primary_save_path.is_file() || !save_game_info_path.is_file() {
            // Missing required files; skip invalid directory
            continue;
        }

        // 2. Optional Rollback Files
        let old_save_name = format!("{}_old", folder_name);
        let old_save_path = path.join(&old_save_name);
        let old_save_info_path = path.join("SaveGameInfo_old");

        let has_old_save = old_save_path.is_file();
        let has_old_save_info = old_save_info_path.is_file();

        saves.push(DiscoveredSave {
            farm_name,
            game_id,
            folder_name,
            folder_path: path,
            primary_save_path,
            save_game_info_path,
            has_old_save,
            has_old_save_info,
        });
    }

    // Sort deterministically by folder name
    saves.sort_by(|a, b| a.folder_name.cmp(&b.folder_name));

    Ok(saves)
}
