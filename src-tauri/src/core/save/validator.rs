use crate::core::errors::CoreError;
use crate::core::save::parser::{get_child_text, ParsedSave};
use std::io::Cursor;
use std::path::Path;
use xmltree::Element;

pub struct SaveValidator;

impl SaveValidator {
    /// Layer 1: BASIC VALIDATION
    /// Checks that required files exist, are readable, and contain well-formed XML.
    pub fn validate_basic(folder_path: &Path) -> Result<(String, String), CoreError> {
        if !folder_path.is_dir() {
            return Err(CoreError::Validation(format!(
                "Save path is not a directory: {:?}",
                folder_path
            )));
        }

        let folder_name = folder_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let save_game_info_path = folder_path.join("SaveGameInfo");
        if !save_game_info_path.is_file() {
            return Err(CoreError::Validation(format!(
                "Missing required SaveGameInfo file: {:?}",
                save_game_info_path
            )));
        }

        let primary_save_path = if !folder_name.is_empty()
            && folder_path.join(folder_name).is_file()
        {
            folder_path.join(folder_name)
        } else {
            let mut candidate = None;
            for entry in std::fs::read_dir(folder_path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if fname != "SaveGameInfo"
                        && !fname.ends_with("_old")
                        && !fname.ends_with(".json")
                    {
                        candidate = Some(path);
                        break;
                    }
                }
            }
            candidate.ok_or_else(|| {
                CoreError::Validation(format!("No primary save file found in {:?}", folder_path))
            })?
        };

        if !save_game_info_path.is_file() {
            return Err(CoreError::Validation(format!(
                "Missing required SaveGameInfo file: {:?}",
                save_game_info_path
            )));
        }

        let save_xml = std::fs::read_to_string(&primary_save_path)
            .map_err(|e| CoreError::Validation(format!("Cannot read primary save file: {}", e)))?;

        let info_xml = std::fs::read_to_string(&save_game_info_path)
            .map_err(|e| CoreError::Validation(format!("Cannot read SaveGameInfo file: {}", e)))?;

        // Verify well-formed XML
        Element::parse(Cursor::new(save_xml.as_bytes()))
            .map_err(|e| CoreError::XmlParse(format!("Primary save XML is malformed: {}", e)))?;

        Element::parse(Cursor::new(info_xml.as_bytes()))
            .map_err(|e| CoreError::XmlParse(format!("SaveGameInfo XML is malformed: {}", e)))?;

        Ok((save_xml, info_xml))
    }

    /// Layer 2: STRUCTURAL VALIDATION
    /// Checks root SaveGame elements and core world integrity.
    pub fn validate_structural(parsed: &ParsedSave) -> Result<(), CoreError> {
        if parsed.root.name != "SaveGame" {
            return Err(CoreError::Validation(format!(
                "Root element must be 'SaveGame', got '{}'",
                parsed.root.name
            )));
        }

        let host = &parsed.metadata.host_player;
        if host.name.trim().is_empty() {
            return Err(CoreError::Validation(
                "Host player has empty name".to_string(),
            ));
        }
        if host.unique_multiplayer_id == 0 {
            return Err(CoreError::Validation(
                "Host player UniqueMultiplayerID cannot be 0".to_string(),
            ));
        }

        // Verify Farm location exists
        let has_farm = parsed
            .root
            .get_child("locations")
            .map(|locs| {
                locs.children.iter().any(|node| {
                    if let xmltree::XMLNode::Element(e) = node {
                        let name = get_child_text(e, "name").unwrap_or_default();
                        let typ = e.attributes.get("type").map(|s| s.as_str()).unwrap_or("");
                        name == "Farm" || typ.ends_with("Farm")
                    } else {
                        false
                    }
                })
            })
            .unwrap_or(false);

        if !has_farm {
            return Err(CoreError::Validation(
                "Save file does not contain a Farm location".to_string(),
            ));
        }

        Ok(())
    }

    /// Layer 3: MULTIPLAYER PRE-MIGRATION VALIDATION
    /// Checks that the target player exists, is in a cabin, and IDs are unique.
    pub fn validate_multiplayer_for_migration(
        parsed: &ParsedSave,
        target_player_id: i64,
    ) -> Result<
        (
            &crate::core::save::parser::PlayerSummary,
            &crate::core::save::parser::CabinSummary,
        ),
        CoreError,
    > {
        let host = &parsed.metadata.host_player;

        if host.unique_multiplayer_id == target_player_id {
            return Err(CoreError::Migration(format!(
                "Player '{}' (ID: {}) is already the host of this save",
                host.name, target_player_id
            )));
        }

        // Check for duplicate player IDs
        let mut all_ids = vec![host.unique_multiplayer_id];
        for fh in &parsed.metadata.farmhands {
            if all_ids.contains(&fh.unique_multiplayer_id) {
                return Err(CoreError::Validation(format!(
                    "Duplicate UniqueMultiplayerID detected in save: {}",
                    fh.unique_multiplayer_id
                )));
            }
            all_ids.push(fh.unique_multiplayer_id);
        }

        // Find the target farmhand in cabins
        let mut target_cabin = None;
        let mut target_fh = None;

        for cabin in &parsed.metadata.cabins {
            if let Some(ref fh) = cabin.farmhand {
                if fh.unique_multiplayer_id == target_player_id {
                    target_cabin = Some(cabin);
                    target_fh = Some(fh);
                    break;
                }
            }
        }

        match (target_fh, target_cabin) {
            (Some(fh), Some(c)) => Ok((fh, c)),
            _ => Err(CoreError::Migration(format!(
                "Target player with UniqueMultiplayerID {} was not found in any cabin",
                target_player_id
            ))),
        }
    }

    /// Layer 4: POST-MIGRATION VALIDATION
    /// Verifies that the new state accurately reflects the intended host switch.
    pub fn validate_post_migration(
        transformed_save: &ParsedSave,
        transformed_info_elem: &Element,
        expected_new_host_id: i64,
        expected_prev_host_id: i64,
        target_cabin_name: &str,
    ) -> Result<(), CoreError> {
        let host = &transformed_save.metadata.host_player;

        // 1. Root player must be new host
        if host.unique_multiplayer_id != expected_new_host_id {
            return Err(CoreError::Validation(format!(
                "Post-migration check failed: host ID is {}, expected {}",
                host.unique_multiplayer_id, expected_new_host_id
            )));
        }
        if host.home_location != "FarmHouse" {
            return Err(CoreError::Validation(format!(
                "Post-migration check failed: host homeLocation must be 'FarmHouse', got '{}'",
                host.home_location
            )));
        }

        // 2. Previous host must be in the specified cabin
        let cabin = transformed_save
            .metadata
            .cabins
            .iter()
            .find(|c| c.indoors_name == target_cabin_name)
            .ok_or_else(|| {
                CoreError::Validation(format!(
                    "Post-migration check failed: cabin '{}' not found",
                    target_cabin_name
                ))
            })?;

        let farmhand = cabin.farmhand.as_ref().ok_or_else(|| {
            CoreError::Validation(format!(
                "Post-migration check failed: cabin '{}' has no farmhand",
                target_cabin_name
            ))
        })?;

        if farmhand.unique_multiplayer_id != expected_prev_host_id {
            return Err(CoreError::Validation(format!(
                "Post-migration check failed: cabin farmhand ID is {}, expected {}",
                farmhand.unique_multiplayer_id, expected_prev_host_id
            )));
        }

        if farmhand.home_location != target_cabin_name {
            return Err(CoreError::Validation(format!(
                "Post-migration check failed: cabin farmhand homeLocation must be '{}', got '{}'",
                target_cabin_name, farmhand.home_location
            )));
        }

        // 3. SaveGameInfo must match new host
        let info_name = get_child_text(transformed_info_elem, "name").unwrap_or_default();
        let info_id: i64 = get_child_text(transformed_info_elem, "UniqueMultiplayerID")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        if info_id != expected_new_host_id || info_name != host.name {
            return Err(CoreError::Validation(format!(
                "Post-migration check failed: SaveGameInfo player (ID: {}, Name: '{}') does not match new host (ID: {}, Name: '{}')",
                info_id, info_name, expected_new_host_id, host.name
            )));
        }

        Ok(())
    }
}
