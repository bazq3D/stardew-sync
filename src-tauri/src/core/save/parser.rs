use crate::core::errors::CoreError;
use std::io::Cursor;
use xmltree::{Element, XMLNode};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PlayerSummary {
    pub name: String,
    pub unique_multiplayer_id: i64,
    pub home_location: String,
    pub house_upgrade_level: u32,
    pub is_host: bool,
    pub cabin_indoors_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CabinSummary {
    pub building_type: String,
    pub tile_x: i32,
    pub tile_y: i32,
    pub indoors_name: String,
    pub upgrade_level: u32,
    pub farmhand: Option<PlayerSummary>,
    pub farmhand_ref: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SaveMetadata {
    pub farm_name: String,
    pub current_season: String,
    pub day_of_month: u32,
    pub year: u32,
    pub host_player: PlayerSummary,
    pub farmhands: Vec<PlayerSummary>,
    pub cabins: Vec<CabinSummary>,
}

#[derive(Debug, Clone)]
pub struct ParsedSave {
    pub root: Element,
    pub metadata: SaveMetadata,
}

impl ParsedSave {
    /// Parses a Stardew Valley primary save XML document into a lossless DOM tree
    /// and extracts player/cabin metadata.
    pub fn parse(xml_content: &str) -> Result<Self, CoreError> {
        let cursor = Cursor::new(xml_content.as_bytes());
        let root = Element::parse(cursor)
            .map_err(|e| CoreError::XmlParse(format!("Failed to parse SaveGame XML: {}", e)))?;

        if root.name != "SaveGame" {
            return Err(CoreError::Validation(format!(
                "Root XML element must be 'SaveGame', found '{}'",
                root.name
            )));
        }

        let metadata = extract_metadata(&root)?;

        Ok(ParsedSave { root, metadata })
    }

    /// Serializes the DOM back to an XML string with canonical schema attributes and UTF-8 BOM.
    pub fn to_xml_string(&self) -> Result<String, CoreError> {
        serialize_element(&self.root)
    }
}

/// Recursively restores XML Schema-Instance prefixes (`xsi:type` and `xsi:nil`)
/// that may have had their prefix stripped by `xmltree` during parsing.
pub fn restore_xml_schema_instance_attributes(elem: &mut Element) {
    if let Some(val) = elem.attributes.remove("type") {
        elem.attributes.insert("xsi:type".to_string(), val);
    }
    if let Some(val) = elem.attributes.remove("nil") {
        elem.attributes.insert("xsi:nil".to_string(), val);
    }
    for child in &mut elem.children {
        if let XMLNode::Element(child_elem) = child {
            restore_xml_schema_instance_attributes(child_elem);
        }
    }
}

/// Canonical XML serializer for Stardew Valley save files and SaveGameInfo.
/// Guarantees:
/// 1. `xsi:type` and `xsi:nil` attributes are preserved for polymorphic .NET deserialization.
/// 2. Standard `xmlns:xsi` and `xmlns:xsd` declarations are present on the root element.
/// 3. Document declaration `<?xml version="1.0" encoding="utf-8"?>` is written.
/// 4. No artificial whitespace indentation is introduced.
/// 5. Output starts with UTF-8 BOM (`\u{feff}`) matching .NET/Stardew native saves.
pub fn serialize_element(elem: &Element) -> Result<String, CoreError> {
    let mut elem_clone = elem.clone();
    restore_xml_schema_instance_attributes(&mut elem_clone);

    // Ensure root has standard XML schema instance and schema definitions
    if let Some(ref mut ns) = elem_clone.namespaces {
        ns.put("xsi", "http://www.w3.org/2001/XMLSchema-instance");
        ns.put("xsd", "http://www.w3.org/2001/XMLSchema");
    } else {
        let mut ns = xmltree::Namespace::empty();
        ns.put("xsi", "http://www.w3.org/2001/XMLSchema-instance");
        ns.put("xsd", "http://www.w3.org/2001/XMLSchema");
        elem_clone.namespaces = Some(ns);
    }

    let mut buffer = Vec::new();
    let config = xmltree::EmitterConfig::new()
        .perform_indent(false)
        .write_document_declaration(true);

    elem_clone
        .write_with_config(&mut buffer, config)
        .map_err(|e| CoreError::XmlWrite(format!("Failed to serialize XML: {}", e)))?;

    let raw_xml = String::from_utf8(buffer)
        .map_err(|e| CoreError::XmlWrite(format!("Serialized XML is invalid UTF-8: {}", e)))?;

    // Prepend UTF-8 BOM (\u{feff}) to exactly match .NET StreamWriter / Stardew save format
    if !raw_xml.starts_with('\u{feff}') {
        Ok(format!("\u{feff}{}", raw_xml))
    } else {
        Ok(raw_xml)
    }
}


pub fn get_child_text(elem: &Element, child_name: &str) -> Option<String> {
    elem.get_child(child_name)
        .and_then(|c| c.get_text())
        .map(|t| t.into_owned())
}

pub fn set_child_text(elem: &mut Element, child_name: &str, text: &str) {
    if let Some(child) = elem.get_mut_child(child_name) {
        child.children.clear();
        child.children.push(XMLNode::Text(text.to_string()));
    } else {
        let mut new_child = Element::new(child_name);
        new_child.children.push(XMLNode::Text(text.to_string()));
        elem.children.push(XMLNode::Element(new_child));
    }
}

fn extract_metadata(root: &Element) -> Result<SaveMetadata, CoreError> {
    let player_elem = root.get_child("player").ok_or_else(|| {
        CoreError::Validation("Missing '<player>' element in SaveGame".to_string())
    })?;

    let host_name = get_child_text(player_elem, "name").unwrap_or_else(|| "Unknown".to_string());
    let host_id: i64 = get_child_text(player_elem, "UniqueMultiplayerID")
        .as_deref()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| {
            CoreError::Validation(
                "Missing or invalid 'UniqueMultiplayerID' in host <player>".to_string(),
            )
        })?;
    let host_home =
        get_child_text(player_elem, "homeLocation").unwrap_or_else(|| "FarmHouse".to_string());
    let host_upgrade: u32 = get_child_text(player_elem, "houseUpgradeLevel")
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let farm_name = get_child_text(player_elem, "farmName").unwrap_or_else(|| "Farm".to_string());

    let host_player = PlayerSummary {
        name: host_name,
        unique_multiplayer_id: host_id,
        home_location: host_home,
        house_upgrade_level: host_upgrade,
        is_host: true,
        cabin_indoors_name: None,
    };

    let current_season =
        get_child_text(root, "currentSeason").unwrap_or_else(|| "spring".to_string());
    let day_of_month: u32 = get_child_text(root, "dayOfMonth")
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let year: u32 = get_child_text(root, "year")
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);

    // Locate Farm location buildings & cabins
    let mut cabins = Vec::new();
    let mut legacy_farmhands = Vec::new();

    if let Some(locations) = root.get_child("locations") {
        for loc in &locations.children {
            if let XMLNode::Element(loc_elem) = loc {
                let loc_name = get_child_text(loc_elem, "name").unwrap_or_default();
                let loc_type = loc_elem
                    .attributes
                    .get("type")
                    .map(|s| s.as_str())
                    .unwrap_or("");

                if loc_name == "Farm" || loc_type.ends_with("Farm") {
                    if let Some(buildings) = loc_elem.get_child("buildings") {
                        for b in &buildings.children {
                            if let XMLNode::Element(b_elem) = b {
                                if let Some((cabin, legacy_fh)) = extract_cabin_summary(b_elem)? {
                                    if let Some(fh) = legacy_fh {
                                        legacy_farmhands.push(fh);
                                    }
                                    cabins.push(cabin);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Locate farmhands: either from root <farmhands> (Stardew 1.6+) or from cabins (legacy)
    let mut farmhands = Vec::new();
    if let Some(root_farmhands) = root.get_child("farmhands") {
        for child in &root_farmhands.children {
            if let XMLNode::Element(fh_elem) = child {
                let name = get_child_text(fh_elem, "name").unwrap_or_default();
                let id_res = get_child_text(fh_elem, "UniqueMultiplayerID")
                    .as_deref()
                    .and_then(|s| s.parse::<i64>().ok());

                if let Some(id) = id_res {
                    let home = get_child_text(fh_elem, "homeLocation").unwrap_or_default();
                    let upgrade = get_child_text(fh_elem, "houseUpgradeLevel")
                        .as_deref()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);

                    // Find matching cabin by indoors_name == home or farmhandReference == id
                    let mut matched = false;
                    for cabin in &mut cabins {
                        if cabin.indoors_name == home || cabin.farmhand_ref == Some(id) {
                            let player = PlayerSummary {
                                name: name.clone(),
                                unique_multiplayer_id: id,
                                home_location: if home.is_empty() {
                                    cabin.indoors_name.clone()
                                } else {
                                    home.clone()
                                },
                                house_upgrade_level: upgrade,
                                is_host: false,
                                cabin_indoors_name: Some(cabin.indoors_name.clone()),
                            };
                            cabin.farmhand = Some(player.clone());
                            farmhands.push(player);
                            matched = true;
                            break;
                        }
                    }

                    if !matched {
                        farmhands.push(PlayerSummary {
                            name,
                            unique_multiplayer_id: id,
                            home_location: home,
                            house_upgrade_level: upgrade,
                            is_host: false,
                            cabin_indoors_name: None,
                        });
                    }
                }
            }
        }
    } else {
        farmhands = legacy_farmhands;
    }

    Ok(SaveMetadata {
        farm_name,
        current_season,
        day_of_month,
        year,
        host_player,
        farmhands,
        cabins,
    })
}

fn extract_cabin_summary(
    building: &Element,
) -> Result<Option<(CabinSummary, Option<PlayerSummary>)>, CoreError> {
    let building_type = get_child_text(building, "buildingType").unwrap_or_default();

    // Check if building has an indoors of type Cabin
    let indoors = match building.get_child("indoors") {
        Some(ind) => ind,
        None => return Ok(None),
    };

    let indoors_type = indoors
        .attributes
        .get("type")
        .map(|s| s.as_str())
        .unwrap_or("");
    if !indoors_type.ends_with("Cabin") && !building_type.contains("Cabin") {
        return Ok(None);
    }

    let tile_x: i32 = get_child_text(building, "tileX")
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let tile_y: i32 = get_child_text(building, "tileY")
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let indoors_name = get_child_text(indoors, "uniqueName")
        .or_else(|| get_child_text(indoors, "name"))
        .unwrap_or_else(|| "Cabin".to_string());
    let upgrade_level: u32 = get_child_text(indoors, "upgradeLevel")
        .as_deref()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let farmhand_ref = get_child_text(indoors, "farmhandReference")
        .as_deref()
        .and_then(|s| s.parse::<i64>().ok());

    let (cabin_fh, legacy_fh) = if let Some(fh_elem) = indoors.get_child("farmhand") {
        let name = get_child_text(fh_elem, "name").unwrap_or_default();
        let id_res = get_child_text(fh_elem, "UniqueMultiplayerID")
            .as_deref()
            .and_then(|s| s.parse().ok());

        if let Some(id) = id_res {
            let home =
                get_child_text(fh_elem, "homeLocation").unwrap_or_else(|| indoors_name.clone());
            let upgrade = get_child_text(fh_elem, "houseUpgradeLevel")
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            let summary = PlayerSummary {
                name,
                unique_multiplayer_id: id,
                home_location: home,
                house_upgrade_level: upgrade,
                is_host: false,
                cabin_indoors_name: Some(indoors_name.clone()),
            };
            (Some(summary.clone()), Some(summary))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    Ok(Some((
        CabinSummary {
            building_type,
            tile_x,
            tile_y,
            indoors_name,
            upgrade_level,
            farmhand: cabin_fh,
            farmhand_ref,
        },
        legacy_fh,
    )))
}
