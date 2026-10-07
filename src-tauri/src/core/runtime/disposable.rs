use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use xmltree::{Element, XMLNode};

use crate::core::errors::CoreError;
use crate::core::runtime::guard::ProductionGuard;
use crate::core::save::fingerprint::compute_migration_stable_fingerprint;
use crate::core::save::host_migrator::HostMigrator;
use crate::core::save::parser::{get_child_text, set_child_text, ParsedSave};
use crate::core::save::validator::SaveValidator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisposableIdentity {
    pub farm_name: String,
    pub sanitized_folder_prefix: String,
    pub game_id: i64,
}

impl Default for DisposableIdentity {
    fn default() -> Self {
        Self {
            farm_name: "TürkTest".to_string(),
            sanitized_folder_prefix: "TXrkTest".to_string(),
            game_id: 999450560,
        }
    }
}

impl DisposableIdentity {
    pub fn folder_name(&self) -> String {
        format!("{}_{}", self.sanitized_folder_prefix, self.game_id)
    }

    pub fn primary_save_file_name(&self) -> String {
        self.folder_name()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeBaselineManifest {
    pub state_label: String,
    pub folder_name: String,
    pub primary_save_file_name: String,
    pub game_id: i64,
    pub farm_name: String,
    pub host_name: String,
    pub host_id: i64,
    pub primary_save_sha256: String,
    pub save_game_info_sha256: String,
    pub kubilay_fingerprint: String,
    pub elbi_fingerprint: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DisposableState {
    pub state_label: String,
    pub identity: DisposableIdentity,
    pub save_xml: String,
    pub save_game_info_xml: String,
    pub manifest: RuntimeBaselineManifest,
}

pub struct DisposableSaveManager;

impl DisposableSaveManager {
    /// Transforms a source save into a completely isolated, disposable test identity
    /// with distinct uniqueIDForThisGame and farmName.
    pub fn create_disposable_save(
        source_save_xml: &str,
        source_info_xml: &str,
        identity: &DisposableIdentity,
    ) -> Result<(String, String), CoreError> {
        let mut save_elem = Element::parse(Cursor::new(source_save_xml.as_bytes()))
            .map_err(|e| CoreError::XmlParse(format!("Failed to parse save XML: {}", e)))?;

        let mut info_elem = Element::parse(Cursor::new(source_info_xml.as_bytes()))
            .map_err(|e| CoreError::XmlParse(format!("Failed to parse info XML: {}", e)))?;

        // 1. Update uniqueIDForThisGame
        set_child_text(
            &mut save_elem,
            "uniqueIDForThisGame",
            &identity.game_id.to_string(),
        );

        // 2. Update farmName across root player and farmhands
        if let Some(player) = save_elem.get_mut_child("player") {
            set_child_text(player, "farmName", &identity.farm_name);
        }

        if let Some(farmhands) = save_elem.get_mut_child("farmhands") {
            for child in &mut farmhands.children {
                if let XMLNode::Element(fh) = child {
                    set_child_text(fh, "farmName", &identity.farm_name);
                }
            }
        }

        // 3. Update farmName in SaveGameInfo
        set_child_text(&mut info_elem, "farmName", &identity.farm_name);

        let out_save_xml = serialize_element(&save_elem)?;
        let out_info_xml = serialize_element(&info_elem)?;

        // Ensure newly created disposable save does NOT trigger the production guard
        ProductionGuard::validate_save_content(&out_save_xml)?;

        Ok((out_save_xml, out_info_xml))
    }

    /// Prepares both State A (Kubilay host) and State B (elbi host) as isolated,
    /// fully validated runtime validation states.
    pub fn prepare_both_states(
        source_save_xml: &str,
        source_info_xml: &str,
        identity: &DisposableIdentity,
        elbi_player_id: i64,
    ) -> Result<(DisposableState, DisposableState), CoreError> {
        // Create base disposable save (State A: Kubilay host)
        let (state_a_save_xml, state_a_info_xml) =
            Self::create_disposable_save(source_save_xml, source_info_xml, identity)?;

        let parsed_a = ParsedSave::parse(&state_a_save_xml)?;
        SaveValidator::validate_structural(&parsed_a)?;

        let kubilay_id = parsed_a.metadata.host_player.unique_multiplayer_id;
        let kubilay_fp_a =
            compute_migration_stable_fingerprint(parsed_a.root.get_child("player").unwrap())?;

        let elbi_elem_a = find_farmer_by_id(&parsed_a.root, elbi_player_id)?
            .ok_or_else(|| CoreError::Migration("Elbi not found in State A".to_string()))?;
        let elbi_fp_a = compute_migration_stable_fingerprint(&elbi_elem_a)?;

        let manifest_a = RuntimeBaselineManifest {
            state_label: "A_KubilayHost".to_string(),
            folder_name: identity.folder_name(),
            primary_save_file_name: identity.primary_save_file_name(),
            game_id: identity.game_id,
            farm_name: identity.farm_name.clone(),
            host_name: parsed_a.metadata.host_player.name.clone(),
            host_id: kubilay_id,
            primary_save_sha256: sha256_hex(&state_a_save_xml),
            save_game_info_sha256: sha256_hex(&state_a_info_xml),
            kubilay_fingerprint: kubilay_fp_a.clone(),
            elbi_fingerprint: elbi_fp_a.clone(),
            created_at: Utc::now(),
        };

        let state_a = DisposableState {
            state_label: "A_KubilayHost".to_string(),
            identity: identity.clone(),
            save_xml: state_a_save_xml.clone(),
            save_game_info_xml: state_a_info_xml.clone(),
            manifest: manifest_a,
        };

        // Migrate to State B (elbi host)
        let mig_result =
            HostMigrator::migrate(&state_a_save_xml, &state_a_info_xml, elbi_player_id)?;

        let parsed_b = ParsedSave::parse(&mig_result.transformed_save_xml)?;
        SaveValidator::validate_structural(&parsed_b)?;

        let elbi_fp_b =
            compute_migration_stable_fingerprint(parsed_b.root.get_child("player").unwrap())?;
        let kubilay_elem_b = find_farmer_by_id(&parsed_b.root, kubilay_id)?
            .ok_or_else(|| CoreError::Migration("Kubilay not found in State B".to_string()))?;
        let kubilay_fp_b = compute_migration_stable_fingerprint(&kubilay_elem_b)?;

        // Assert fingerprints match
        if elbi_fp_b != elbi_fp_a || kubilay_fp_b != kubilay_fp_a {
            return Err(CoreError::Migration(
                "Fingerprint mismatch during State B preparation".to_string(),
            ));
        }

        let manifest_b = RuntimeBaselineManifest {
            state_label: "B_ElbiHost".to_string(),
            folder_name: identity.folder_name(),
            primary_save_file_name: identity.primary_save_file_name(),
            game_id: identity.game_id,
            farm_name: identity.farm_name.clone(),
            host_name: parsed_b.metadata.host_player.name.clone(),
            host_id: elbi_player_id,
            primary_save_sha256: sha256_hex(&mig_result.transformed_save_xml),
            save_game_info_sha256: sha256_hex(&mig_result.transformed_save_game_info_xml),
            kubilay_fingerprint: kubilay_fp_b,
            elbi_fingerprint: elbi_fp_b,
            created_at: Utc::now(),
        };

        let state_b = DisposableState {
            state_label: "B_ElbiHost".to_string(),
            identity: identity.clone(),
            save_xml: mig_result.transformed_save_xml,
            save_game_info_xml: mig_result.transformed_save_game_info_xml,
            manifest: manifest_b,
        };

        Ok((state_a, state_b))
    }
}

fn find_farmer_by_id(root: &Element, id: i64) -> Result<Option<Element>, CoreError> {
    if let Some(player) = root.get_child("player") {
        if get_child_text(player, "UniqueMultiplayerID").as_deref() == Some(&id.to_string()) {
            return Ok(Some(player.clone()));
        }
    }

    if let Some(farmhands) = root.get_child("farmhands") {
        for child in &farmhands.children {
            if let XMLNode::Element(fh) = child {
                if get_child_text(fh, "UniqueMultiplayerID").as_deref() == Some(&id.to_string()) {
                    return Ok(Some(fh.clone()));
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

fn sha256_hex(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    format!("{:x}", hasher.finalize())
}
