use crate::core::errors::CoreError;
use crate::core::process::monitor::{ProcessChecker, ProcessMonitor};
use crate::core::save::parser::get_child_text;
use std::path::Path;

pub const PRODUCTION_FARM_FOLDER: &str = "TXrk_450560341";
pub const PRODUCTION_GAME_ID: i64 = 450560341;

pub struct ProductionGuard;

impl ProductionGuard {
    /// Strictly validates that the given target path does NOT resolve to or masquerade
    /// as the live production farm.
    pub fn validate_target_path(target_path: &Path) -> Result<(), CoreError> {
        let folder_name = target_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let folder_lower = folder_name.to_lowercase();
        let prod_lower = PRODUCTION_FARM_FOLDER.to_lowercase();

        // 1. Exact or case-insensitive match on production folder name
        if folder_lower == prod_lower {
            return Err(CoreError::Validation(format!(
                "[CRITICAL GUARD TRIGGERED] Target folder '{}' is the LIVE PRODUCTION FARM! Overwrite strictly forbidden.",
                folder_name
            )));
        }

        // 2. Folder name must not contain the production Game ID
        if folder_lower.contains(&PRODUCTION_GAME_ID.to_string()) {
            return Err(CoreError::Validation(format!(
                "[CRITICAL GUARD TRIGGERED] Target folder '{}' contains the production Game ID ({}). Disposable tests must use a distinct Game ID.",
                folder_name, PRODUCTION_GAME_ID
            )));
        }

        // 3. Normalized path check
        let path_str = target_path.to_string_lossy().to_lowercase();
        if path_str.contains(&prod_lower) {
            return Err(CoreError::Validation(format!(
                "[CRITICAL GUARD TRIGGERED] Target path '{}' resolves inside the production farm directory. Action blocked.",
                target_path.display()
            )));
        }

        Ok(())
    }

    /// Strictly validates that the save XML content does NOT contain the production Game ID,
    /// preventing a disposable save from masquerading as the production farm.
    pub fn validate_save_content(xml_content: &str) -> Result<(), CoreError> {
        let root = xmltree::Element::parse(std::io::Cursor::new(xml_content.as_bytes()))
            .map_err(|e| CoreError::XmlParse(format!("Failed to parse save XML: {}", e)))?;

        let game_id_str = get_child_text(&root, "uniqueIDForThisGame").unwrap_or_default();
        if let Ok(id) = game_id_str.parse::<i64>() {
            if id == PRODUCTION_GAME_ID {
                return Err(CoreError::Validation(format!(
                    "[CRITICAL GUARD TRIGGERED] Save content contains production uniqueIDForThisGame ({}). Disposable runtime test must use a distinct test ID.",
                    PRODUCTION_GAME_ID
                )));
            }
        }

        Ok(())
    }

    /// Complete pre-install safety verification:
    /// 1. Verifies Stardew Valley is NOT running.
    /// 2. Verifies target folder is NOT the production farm.
    /// 3. Verifies save content is NOT masquerading as the production farm.
    pub fn validate_safe_for_install<C: ProcessChecker>(
        monitor: &ProcessMonitor<C>,
        target_path: &Path,
        save_xml: &str,
    ) -> Result<(), CoreError> {
        if monitor.is_stardew_running() {
            return Err(CoreError::GameRunning {
                process_name: "Stardew Valley".to_string(),
            });
        }

        Self::validate_target_path(target_path)?;
        Self::validate_save_content(save_xml)?;

        Ok(())
    }
}
