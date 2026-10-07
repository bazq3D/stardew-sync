use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io::Cursor;
use std::path::Path;
use xmltree::{Element, XMLNode};

use crate::core::errors::CoreError;
use crate::core::runtime::disposable::RuntimeBaselineManifest;
use crate::core::save::fingerprint::compute_migration_stable_fingerprint;
use crate::core::save::parser::{get_child_text, ParsedSave};
use crate::core::save::validator::SaveValidator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FarmerAnalysis {
    pub name: String,
    pub id: i64,
    pub is_host: bool,
    pub home_location: String,
    pub baseline_fingerprint: Option<String>,
    pub current_fingerprint: String,
    pub fingerprint_changed: bool,
    pub money: Option<u64>,
    pub gameplay_changes_detected: Vec<String>,
    pub identity_preserved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldAnalysis {
    pub day_of_month: u32,
    pub season: String,
    pub year: u32,
    pub time_of_day: Option<u32>,
    pub location_count: usize,
    pub farm_building_count: usize,
    pub gameplay_progression_observed: bool,
    pub observed_progression_details: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralAnalysis {
    pub xml_parses_successfully: bool,
    pub exactly_one_root_host: bool,
    pub expected_host_matches: bool,
    pub unique_multiplayer_ids: bool,
    pub all_expected_farmers_present: bool,
    pub missing_farmers: Vec<String>,
    pub duplicate_farmers: Vec<String>,
    pub cabin_references_valid: bool,
    pub home_locations_valid: bool,
    pub save_game_info_valid: bool,
    pub mod_data_preserved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostRuntimeReport {
    pub state_label: String,
    pub folder_name: String,
    pub game_id: i64,
    pub structural: StructuralAnalysis,
    pub farmers: Vec<FarmerAnalysis>,
    pub world: WorldAnalysis,
    pub byte_identical_to_baseline: bool,
    pub serialization_rewrite_detected: bool,
    pub unexpected_destructive_changes: Vec<String>,
    pub is_healthy: bool,
}

pub struct PostRuntimeAnalyzer;

impl PostRuntimeAnalyzer {
    /// Analyzes a save directory after Stardew Valley runtime execution.
    pub fn analyze_folder(
        manifest: &RuntimeBaselineManifest,
        folder_path: &Path,
    ) -> Result<PostRuntimeReport, CoreError> {
        let (save_xml, info_xml) = SaveValidator::validate_basic(folder_path)?;
        Self::analyze_xml(manifest, &save_xml, &info_xml)
    }

    /// Analyzes raw save and SaveGameInfo XML content against the baseline manifest.
    pub fn analyze_xml(
        manifest: &RuntimeBaselineManifest,
        save_xml: &str,
        info_xml: &str,
    ) -> Result<PostRuntimeReport, CoreError> {
        let mut destructive_changes = Vec::new();

        // 1. XML Parse verification
        let parsed_save_res = ParsedSave::parse(save_xml);
        let parsed_info_elem_res = Element::parse(Cursor::new(info_xml.as_bytes()));

        let xml_parses_successfully = parsed_save_res.is_ok() && parsed_info_elem_res.is_ok();

        if !xml_parses_successfully {
            destructive_changes.push("Failed to parse post-runtime XML".to_string());
            return Ok(PostRuntimeReport {
                state_label: manifest.state_label.clone(),
                folder_name: manifest.folder_name.clone(),
                game_id: manifest.game_id,
                structural: StructuralAnalysis {
                    xml_parses_successfully: false,
                    exactly_one_root_host: false,
                    expected_host_matches: false,
                    unique_multiplayer_ids: false,
                    all_expected_farmers_present: false,
                    missing_farmers: vec![],
                    duplicate_farmers: vec![],
                    cabin_references_valid: false,
                    home_locations_valid: false,
                    save_game_info_valid: false,
                    mod_data_preserved: false,
                },
                farmers: vec![],
                world: WorldAnalysis {
                    day_of_month: 0,
                    season: "unknown".to_string(),
                    year: 0,
                    time_of_day: None,
                    location_count: 0,
                    farm_building_count: 0,
                    gameplay_progression_observed: false,
                    observed_progression_details: vec![],
                },
                byte_identical_to_baseline: false,
                serialization_rewrite_detected: false,
                unexpected_destructive_changes: destructive_changes,
                is_healthy: false,
            });
        }

        let parsed = parsed_save_res.unwrap();
        let info_elem = parsed_info_elem_res.unwrap();

        // Structural validation
        let structural_res = SaveValidator::validate_structural(&parsed);
        if let Err(e) = structural_res {
            destructive_changes.push(format!("Structural validation error: {}", e));
        }

        // Host validation
        let host = &parsed.metadata.host_player;
        let exactly_one_root_host = host.is_host && host.home_location == "FarmHouse";
        let expected_host_matches = host.unique_multiplayer_id == manifest.host_id;

        if !expected_host_matches {
            destructive_changes.push(format!(
                "Host mismatch: expected {} ({}), got {} ({})",
                manifest.host_id, manifest.host_name, host.unique_multiplayer_id, host.name
            ));
        }

        // Unique Multiplayer IDs check across root player and farmhands
        let mut seen_ids = HashSet::new();
        let mut duplicate_farmers = Vec::new();
        seen_ids.insert(host.unique_multiplayer_id);

        for fh in &parsed.metadata.farmhands {
            if !seen_ids.insert(fh.unique_multiplayer_id) {
                duplicate_farmers.push(format!("{} ({})", fh.name, fh.unique_multiplayer_id));
                destructive_changes.push(format!(
                    "Duplicate UniqueMultiplayerID: {}",
                    fh.unique_multiplayer_id
                ));
            }
        }
        let unique_multiplayer_ids = duplicate_farmers.is_empty();

        // Check for expected farmers: Kubilay & elbi
        let mut missing_farmers = Vec::new();
        let all_farmers: Vec<_> = std::iter::once(host)
            .chain(parsed.metadata.farmhands.iter())
            .collect();

        let has_host_farmer = all_farmers
            .iter()
            .any(|f| f.unique_multiplayer_id == manifest.host_id);
        if !has_host_farmer {
            missing_farmers.push(format!("Expected Host: {}", manifest.host_name));
            destructive_changes.push(format!("Missing expected host: {}", manifest.host_name));
        }

        let all_expected_farmers_present = missing_farmers.is_empty();

        // Cabin references check
        let mut cabin_references_valid = true;
        for cabin in &parsed.metadata.cabins {
            if let Some(ref fh) = cabin.farmhand {
                if fh.home_location != cabin.indoors_name
                    && cabin.farmhand_ref != Some(fh.unique_multiplayer_id)
                {
                    cabin_references_valid = false;
                    destructive_changes.push(format!(
                        "Cabin '{}' binding mismatch with farmhand '{}' (ID: {})",
                        cabin.indoors_name, fh.name, fh.unique_multiplayer_id
                    ));
                }
            }
        }

        // Home locations check
        let home_locations_valid = host.home_location == "FarmHouse"
            && parsed
                .metadata
                .farmhands
                .iter()
                .all(|fh| fh.home_location != "FarmHouse" || fh.cabin_indoors_name.is_some());

        // SaveGameInfo validity
        let info_id: i64 = get_child_text(&info_elem, "UniqueMultiplayerID")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let info_name = get_child_text(&info_elem, "name").unwrap_or_default();
        let info_farm_name = get_child_text(&info_elem, "farmName").unwrap_or_default();

        let save_game_info_valid = info_id == host.unique_multiplayer_id
            && info_name == host.name
            && info_farm_name == parsed.metadata.farm_name;

        if !save_game_info_valid {
            destructive_changes.push(format!(
                "SaveGameInfo mismatch: expected ID {} / Name '{}' / Farm '{}', got ID {} / Name '{}' / Farm '{}'",
                host.unique_multiplayer_id, host.name, parsed.metadata.farm_name, info_id, info_name, info_farm_name
            ));
        }

        // ModData / unknown data check
        let mod_data_preserved = parsed
            .root
            .get_child("player")
            .and_then(|p| p.get_child("modData"))
            .is_some();

        // 2. Farmer Analysis & Gameplay Changes
        let mut farmers = Vec::new();
        for f_summary in &all_farmers {
            if let Some(f_elem) = find_farmer_element(&parsed.root, f_summary.unique_multiplayer_id)
            {
                let current_fp = compute_migration_stable_fingerprint(&f_elem)?;

                let baseline_fp = if f_summary.name.contains("Kubilay")
                    || f_summary.unique_multiplayer_id == manifest.host_id
                {
                    Some(manifest.kubilay_fingerprint.clone())
                } else if f_summary.name.contains("elbi") {
                    Some(manifest.elbi_fingerprint.clone())
                } else {
                    None
                };

                let fp_changed = baseline_fp
                    .as_ref()
                    .map(|b| b != &current_fp)
                    .unwrap_or(false);

                let money: Option<u64> =
                    get_child_text(&f_elem, "money").and_then(|s| s.parse().ok());

                let mut gameplay_changes = Vec::new();
                if fp_changed {
                    // Inspect what changed
                    if let Some(m) = money {
                        gameplay_changes.push(format!("Current money: {}", m));
                    }
                    if let Some(tp) = get_child_text(&f_elem, "millisecondsPlayed") {
                        gameplay_changes.push(format!("Playtime milliseconds: {}", tp));
                    }
                    gameplay_changes
                        .push("Attributes modified by runtime gameplay session".to_string());
                }

                let identity_preserved =
                    f_summary.unique_multiplayer_id != 0 && !f_summary.name.is_empty();

                farmers.push(FarmerAnalysis {
                    name: f_summary.name.clone(),
                    id: f_summary.unique_multiplayer_id,
                    is_host: f_summary.is_host,
                    home_location: f_summary.home_location.clone(),
                    baseline_fingerprint: baseline_fp,
                    current_fingerprint: current_fp,
                    fingerprint_changed: fp_changed,
                    money,
                    gameplay_changes_detected: gameplay_changes,
                    identity_preserved,
                });
            }
        }

        // 3. World Analysis
        let time_of_day: Option<u32> =
            get_child_text(&parsed.root, "timeOfDay").and_then(|s| s.parse().ok());
        let location_count = parsed
            .root
            .get_child("locations")
            .map(|l| {
                l.children
                    .iter()
                    .filter(|n| matches!(n, XMLNode::Element(_)))
                    .count()
            })
            .unwrap_or(0);

        let farm_building_count = parsed.metadata.cabins.len();

        let mut observed_progression = Vec::new();
        let mut gameplay_progression_observed = false;

        if let Some(t) = time_of_day {
            if t != 600 {
                // 600 is 6:00 AM start of day
                observed_progression.push(format!("Time advanced to {}", t));
                gameplay_progression_observed = true;
            }
        }

        // 4. Byte & Serialization Comparison
        let current_save_sha = sha256_hex(save_xml);
        let byte_identical = current_save_sha == manifest.primary_save_sha256;
        let serialization_rewrite_detected = !byte_identical;

        let structural_analysis = StructuralAnalysis {
            xml_parses_successfully,
            exactly_one_root_host,
            expected_host_matches,
            unique_multiplayer_ids,
            all_expected_farmers_present,
            missing_farmers,
            duplicate_farmers,
            cabin_references_valid,
            home_locations_valid,
            save_game_info_valid,
            mod_data_preserved,
        };

        let world_analysis = WorldAnalysis {
            day_of_month: parsed.metadata.day_of_month,
            season: parsed.metadata.current_season,
            year: parsed.metadata.year,
            time_of_day,
            location_count,
            farm_building_count,
            gameplay_progression_observed,
            observed_progression_details: observed_progression,
        };

        let is_healthy = destructive_changes.is_empty()
            && xml_parses_successfully
            && exactly_one_root_host
            && expected_host_matches
            && unique_multiplayer_ids
            && all_expected_farmers_present
            && cabin_references_valid
            && save_game_info_valid;

        Ok(PostRuntimeReport {
            state_label: manifest.state_label.clone(),
            folder_name: manifest.folder_name.clone(),
            game_id: manifest.game_id,
            structural: structural_analysis,
            farmers,
            world: world_analysis,
            byte_identical_to_baseline: byte_identical,
            serialization_rewrite_detected,
            unexpected_destructive_changes: destructive_changes,
            is_healthy,
        })
    }
}

fn find_farmer_element(root: &Element, id: i64) -> Option<Element> {
    if let Some(player) = root.get_child("player") {
        if get_child_text(player, "UniqueMultiplayerID").as_deref() == Some(&id.to_string()) {
            return Some(player.clone());
        }
    }

    if let Some(farmhands) = root.get_child("farmhands") {
        for child in &farmhands.children {
            if let XMLNode::Element(fh) = child {
                if get_child_text(fh, "UniqueMultiplayerID").as_deref() == Some(&id.to_string()) {
                    return Some(fh.clone());
                }
            }
        }
    }

    if let Some(locations) = root.get_child("locations") {
        for loc in &locations.children {
            if let XMLNode::Element(loc_elem) = loc {
                if let Some(buildings) = loc_elem.get_child("buildings") {
                    for b in &buildings.children {
                        if let XMLNode::Element(b_elem) = b {
                            if let Some(indoors) = b_elem.get_child("indoors") {
                                if let Some(fh) = indoors.get_child("farmhand") {
                                    if get_child_text(fh, "UniqueMultiplayerID").as_deref()
                                        == Some(&id.to_string())
                                    {
                                        return Some(fh.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn sha256_hex(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    format!("{:x}", hasher.finalize())
}
