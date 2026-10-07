use crate::core::errors::CoreError;
use crate::core::save::fingerprint::{compute_migration_stable_fingerprint, verify_allowed_diff};
use crate::core::save::parser::{get_child_text, set_child_text, ParsedSave};
use crate::core::save::validator::SaveValidator;
use std::io::Cursor;
use xmltree::{Element, XMLNode};

#[derive(Debug, Clone)]
pub struct MigrationResult {
    pub transformed_save_xml: String,
    pub transformed_save_game_info_xml: String,
    pub previous_host_id: i64,
    pub new_host_id: i64,
    pub target_cabin_name: String,
}

pub struct HostMigrator;

impl HostMigrator {
    /// Safely transforms a Stardew Valley save and SaveGameInfo to switch the host
    /// to the player with `target_player_id`.
    ///
    /// GUARANTEES:
    /// 1. The input XML is treated as strictly read-only.
    /// 2. Farmer fingerprints are verified to ensure zero loss of inventory, skills,
    ///    friendships, quests, mail, or appearance.
    /// 3. All non-player world state is strictly verified to be 100% identical.
    /// 4. Unknown/modded tags and attributes are fully preserved.
    pub fn migrate(
        save_xml: &str,
        save_game_info_xml: &str,
        target_player_id: i64,
    ) -> Result<MigrationResult, CoreError> {
        // 1. Parse and validate source save
        let source_save = ParsedSave::parse(save_xml)?;
        SaveValidator::validate_structural(&source_save)?;

        // 2. Validate multiplayer prerequisites
        let (target_fh, target_cabin) =
            SaveValidator::validate_multiplayer_for_migration(&source_save, target_player_id)?;

        let previous_host_id = source_save.metadata.host_player.unique_multiplayer_id;
        let new_host_id = target_fh.unique_multiplayer_id;
        let target_cabin_name = target_cabin.indoors_name.clone();

        // 3. Extract XML element references and compute baseline fingerprints
        let orig_player_elem = source_save
            .root
            .get_child("player")
            .ok_or_else(|| CoreError::Validation("Missing <player> element".to_string()))?;

        let orig_farmhand_elem = find_farmhand_element_by_id(&source_save.root, target_player_id)?
            .ok_or_else(|| {
                CoreError::Migration("Target farmhand element not found in DOM".to_string())
            })?;

        let prev_host_stable_fp = compute_migration_stable_fingerprint(orig_player_elem)?;
        let target_player_stable_fp = compute_migration_stable_fingerprint(&orig_farmhand_elem)?;

        // Capture residence binding values
        let farmhouse_upgrade = source_save.metadata.host_player.house_upgrade_level;
        let cabin_upgrade = target_fh.house_upgrade_level;

        // 4. Perform structured XML transformation on cloned DOM
        let mut transformed_root = source_save.root.clone();
        let has_root_farmhands = transformed_root.get_child("farmhands").is_some();

        // Prepare new host element (cloned from target farmhand)
        let mut new_host_elem = orig_farmhand_elem.clone();
        new_host_elem.name = "player".to_string();
        set_child_text(&mut new_host_elem, "homeLocation", "FarmHouse");
        set_child_text(
            &mut new_host_elem,
            "houseUpgradeLevel",
            &farmhouse_upgrade.to_string(),
        );

        // Prepare new farmhand element (cloned from original host)
        let mut new_farmhand_elem = orig_player_elem.clone();
        if has_root_farmhands {
            new_farmhand_elem.name = "Farmer".to_string();
        } else {
            new_farmhand_elem.name = "farmhand".to_string();
        }
        set_child_text(&mut new_farmhand_elem, "homeLocation", &target_cabin_name);
        set_child_text(
            &mut new_farmhand_elem,
            "houseUpgradeLevel",
            &cabin_upgrade.to_string(),
        );

        // Replace <player> with new host element
        replace_root_player(&mut transformed_root, new_host_elem)?;

        if has_root_farmhands {
            // Replace inside root <farmhands>
            replace_root_farmhand(
                &mut transformed_root,
                target_player_id,
                new_farmhand_elem.clone(),
            )?;
            // Update cabin indoors: update farmhandReference and/or legacy farmhand
            update_cabin_farmhand_reference(
                &mut transformed_root,
                &target_cabin_name,
                previous_host_id,
                Some(new_farmhand_elem),
            )?;
        } else {
            // Replace <farmhand> inside target cabin with new farmhand element
            replace_cabin_farmhand(&mut transformed_root, &target_cabin_name, new_farmhand_elem)?;
        }

        // 5. Build transformed SaveGameInfo
        let mut transformed_info_elem = Element::parse(Cursor::new(save_game_info_xml.as_bytes()))
            .map_err(|e| CoreError::XmlParse(format!("Failed to parse SaveGameInfo XML: {}", e)))?;

        // In SaveGameInfo, root is <Farmer>. We replace its children with the new host's children,
        // preserving root element name and namespaces.
        let new_host_for_info = transformed_root.get_child("player").unwrap();
        transformed_info_elem.children = new_host_for_info.children.clone();

        // 6. Serialize and Re-parse for Post-Validation
        let transformed_save_xml = serialize_element(&transformed_root)?;
        let transformed_save_game_info_xml = serialize_element(&transformed_info_elem)?;

        let re_parsed = ParsedSave::parse(&transformed_save_xml)?;

        // 7. Post-migration validations
        SaveValidator::validate_post_migration(
            &re_parsed,
            &transformed_info_elem,
            new_host_id,
            previous_host_id,
            &target_cabin_name,
        )?;

        // 8. Farmer Fingerprint Assertions
        let new_host_actual_fp =
            compute_migration_stable_fingerprint(re_parsed.root.get_child("player").unwrap())?;
        if new_host_actual_fp != target_player_stable_fp {
            return Err(CoreError::FingerprintMismatch {
                name: target_fh.name.clone(),
                id: new_host_id,
                expected: target_player_stable_fp,
                actual: new_host_actual_fp,
            });
        }

        let new_farmhand_actual_elem =
            find_farmhand_element_by_id(&re_parsed.root, previous_host_id)?.ok_or_else(|| {
                CoreError::Migration("New farmhand not found in re-parsed DOM".to_string())
            })?;
        let new_farmhand_actual_fp =
            compute_migration_stable_fingerprint(&new_farmhand_actual_elem)?;
        if new_farmhand_actual_fp != prev_host_stable_fp {
            return Err(CoreError::FingerprintMismatch {
                name: source_save.metadata.host_player.name.clone(),
                id: previous_host_id,
                expected: prev_host_stable_fp,
                actual: new_farmhand_actual_fp,
            });
        }

        // 9. World State Diff Check
        verify_allowed_diff(&source_save.root, &re_parsed.root, &target_cabin_name)?;

        Ok(MigrationResult {
            transformed_save_xml,
            transformed_save_game_info_xml,
            previous_host_id,
            new_host_id,
            target_cabin_name,
        })
    }
}

fn replace_root_player(root: &mut Element, new_player: Element) -> Result<(), CoreError> {
    for node in &mut root.children {
        if let XMLNode::Element(child) = node {
            if child.name == "player" {
                *child = new_player;
                return Ok(());
            }
        }
    }
    Err(CoreError::Validation(
        "Could not find <player> in root to replace".to_string(),
    ))
}

fn replace_root_farmhand(
    root: &mut Element,
    target_player_id: i64,
    new_farmhand: Element,
) -> Result<(), CoreError> {
    if let Some(farmhands) = root.get_mut_child("farmhands") {
        for node in &mut farmhands.children {
            if let XMLNode::Element(child) = node {
                if let Some(id_str) = get_child_text(child, "UniqueMultiplayerID") {
                    if let Ok(id) = id_str.parse::<i64>() {
                        if id == target_player_id {
                            *child = new_farmhand;
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
    Err(CoreError::Migration(format!(
        "Target farmhand ID {} not found in <farmhands>",
        target_player_id
    )))
}

fn update_cabin_farmhand_reference(
    root: &mut Element,
    target_cabin_name: &str,
    new_farmhand_id: i64,
    legacy_farmhand: Option<Element>,
) -> Result<(), CoreError> {
    let locations = root
        .get_mut_child("locations")
        .ok_or_else(|| CoreError::Validation("Missing <locations> in root".to_string()))?;

    for loc in &mut locations.children {
        if let XMLNode::Element(loc_elem) = loc {
            let loc_name = get_child_text(loc_elem, "name").unwrap_or_default();
            let loc_type = loc_elem
                .attributes
                .get("type")
                .map(|s| s.as_str())
                .unwrap_or("");

            if loc_name == "Farm" || loc_type.ends_with("Farm") {
                if let Some(buildings) = loc_elem.get_mut_child("buildings") {
                    for b in &mut buildings.children {
                        if let XMLNode::Element(b_elem) = b {
                            if let Some(indoors) = b_elem.get_mut_child("indoors") {
                                let name = get_child_text(indoors, "uniqueName")
                                    .or_else(|| get_child_text(indoors, "name"))
                                    .unwrap_or_default();

                                if name == target_cabin_name {
                                    if indoors.get_child("farmhandReference").is_some() {
                                        set_child_text(
                                            indoors,
                                            "farmhandReference",
                                            &new_farmhand_id.to_string(),
                                        );
                                    }
                                    if let Some(ref leg_fh) = legacy_farmhand {
                                        for ind_child in &mut indoors.children {
                                            if let XMLNode::Element(c_elem) = ind_child {
                                                if c_elem.name == "farmhand" {
                                                    *c_elem = leg_fh.clone();
                                                }
                                            }
                                        }
                                    }
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn replace_cabin_farmhand(
    root: &mut Element,
    target_cabin_name: &str,
    new_farmhand: Element,
) -> Result<(), CoreError> {
    let locations = root
        .get_mut_child("locations")
        .ok_or_else(|| CoreError::Validation("Missing <locations> in root".to_string()))?;

    for loc in &mut locations.children {
        if let XMLNode::Element(loc_elem) = loc {
            let loc_name = get_child_text(loc_elem, "name").unwrap_or_default();
            let loc_type = loc_elem
                .attributes
                .get("type")
                .map(|s| s.as_str())
                .unwrap_or("");

            if loc_name == "Farm" || loc_type.ends_with("Farm") {
                if let Some(buildings) = loc_elem.get_mut_child("buildings") {
                    for b in &mut buildings.children {
                        if let XMLNode::Element(b_elem) = b {
                            if let Some(indoors) = b_elem.get_mut_child("indoors") {
                                let name = get_child_text(indoors, "uniqueName")
                                    .or_else(|| get_child_text(indoors, "name"))
                                    .unwrap_or_default();

                                if name == target_cabin_name {
                                    for ind_child in &mut indoors.children {
                                        if let XMLNode::Element(c_elem) = ind_child {
                                            if c_elem.name == "farmhand" {
                                                *c_elem = new_farmhand;
                                                return Ok(());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Err(CoreError::Migration(format!(
        "Cabin '{}' not found to replace farmhand",
        target_cabin_name
    )))
}

fn find_farmhand_element_by_id(
    root: &Element,
    player_id: i64,
) -> Result<Option<Element>, CoreError> {
    // 1. Check root <farmhands> list (Stardew 1.6+)
    if let Some(farmhands) = root.get_child("farmhands") {
        for child in &farmhands.children {
            if let XMLNode::Element(fh) = child {
                if let Some(id_str) = get_child_text(fh, "UniqueMultiplayerID") {
                    if let Ok(id) = id_str.parse::<i64>() {
                        if id == player_id {
                            return Ok(Some(fh.clone()));
                        }
                    }
                }
            }
        }
    }

    // 2. Fallback to legacy <locations> cabins
    if let Some(locations) = root.get_child("locations") {
        for loc in &locations.children {
            if let XMLNode::Element(loc_elem) = loc {
                if let Some(buildings) = loc_elem.get_child("buildings") {
                    for b in &buildings.children {
                        if let XMLNode::Element(b_elem) = b {
                            if let Some(indoors) = b_elem.get_child("indoors") {
                                if let Some(fh) = indoors.get_child("farmhand") {
                                    if let Some(id_str) = get_child_text(fh, "UniqueMultiplayerID")
                                    {
                                        if let Ok(id) = id_str.parse::<i64>() {
                                            if id == player_id {
                                                return Ok(Some(fh.clone()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

fn serialize_element(elem: &Element) -> Result<String, CoreError> {
    let mut buffer = Vec::new();
    let config = xmltree::EmitterConfig::new()
        .perform_indent(false)
        .write_document_declaration(true);

    elem.write_with_config(&mut buffer, config)
        .map_err(|e| CoreError::XmlWrite(format!("Failed to serialize XML: {}", e)))?;

    String::from_utf8(buffer)
        .map_err(|e| CoreError::XmlWrite(format!("Serialized XML is invalid UTF-8: {}", e)))
}
