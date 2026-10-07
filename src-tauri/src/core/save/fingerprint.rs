use crate::core::errors::CoreError;
use sha2::{Digest, Sha256};
use xmltree::{Element, XMLNode};

/// Fields excluded from the migration-stable fingerprint:
/// 1. `homeLocation`: Expected to swap between "FarmHouse" and the Cabin indoors identifier.
/// 2. `houseUpgradeLevel`: Expected to swap to match the residence building tier (Farmhouse vs Cabin).
const MIGRATION_EXCLUDED_FIELDS: &[&str] = &["homeLocation", "houseUpgradeLevel"];

/// Computes a SHA-256 fingerprint of the complete, unmodified Farmer XML element.
pub fn compute_full_farmer_fingerprint(elem: &Element) -> Result<String, CoreError> {
    let mut cloned = elem.clone();
    cloned.name = "Farmer".to_string();
    let canonical = canonicalize_element(&cloned)?;
    Ok(sha256_hex(&canonical))
}

/// Computes a migration-stable SHA-256 fingerprint for a Farmer XML element.
///
/// This excludes ONLY `homeLocation` and `houseUpgradeLevel`.
/// ALL other attributes, inventory items, skills, professions, friendships,
/// quests, mail, stats, and appearance MUST match bit-for-bit.
pub fn compute_migration_stable_fingerprint(elem: &Element) -> Result<String, CoreError> {
    let mut cloned = elem.clone();
    cloned.name = "Farmer".to_string();

    // Strip excluded residence fields
    cloned.children.retain(|node| {
        if let XMLNode::Element(child) = node {
            !MIGRATION_EXCLUDED_FIELDS.contains(&child.name.as_str())
        } else {
            true
        }
    });

    let canonical = canonicalize_element(&cloned)?;
    Ok(sha256_hex(&canonical))
}

/// Canonicalizes an XML element into deterministic string representation.
pub fn canonicalize_element(elem: &Element) -> Result<String, CoreError> {
    let mut buffer = Vec::new();
    let config = xmltree::EmitterConfig::new()
        .perform_indent(false)
        .write_document_declaration(false);

    elem.write_with_config(&mut buffer, config)
        .map_err(|e| CoreError::XmlWrite(format!("Failed to canonicalize element: {}", e)))?;

    String::from_utf8(buffer)
        .map_err(|e| CoreError::XmlWrite(format!("Canonical XML is not valid UTF-8: {}", e)))
}

fn sha256_hex(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Validates that the difference between the original save and the transformed save
/// strictly conforms to the allowed change surface.
///
/// Allowed changes:
/// 1. `/SaveGame/player` (swapped farmer)
/// 2. `/SaveGame/locations/GameLocation[Farm]/buildings/Building/indoors[Cabin]/farmhand` (swapped farmer)
///
/// Any change to other world state (date, weather, crops, bundles, chests, animals)
/// causes an immediate validation failure.
pub fn verify_allowed_diff(
    original: &Element,
    transformed: &Element,
    target_cabin_name: &str,
) -> Result<(), CoreError> {
    // 1. Root tag check
    if original.name != transformed.name {
        return Err(CoreError::WorldDiffViolation {
            path: "/SaveGame".to_string(),
            detail: format!(
                "Root name changed from '{}' to '{}'",
                original.name, transformed.name
            ),
        });
    }

    // 2. Compare direct children of SaveGame
    for orig_node in &original.children {
        if let XMLNode::Element(orig_child) = orig_node {
            let tag = &orig_child.name;

            if tag == "player" {
                // Allowed change: player node swapped during host migration
                continue;
            }

            if tag == "locations" {
                // Inspect locations with care
                let trans_locations = transformed.get_child("locations").ok_or_else(|| {
                    CoreError::WorldDiffViolation {
                        path: "/SaveGame/locations".to_string(),
                        detail: "Transformed XML is missing 'locations' element".to_string(),
                    }
                })?;
                verify_locations_diff(orig_child, trans_locations, target_cabin_name)?;
                continue;
            }

            // All other root tags (e.g., currentSeason, dayOfMonth, year, dailyLuck, farmerTeam, etc.)
            // MUST be identical.
            let trans_child = transformed.get_child(tag.as_str()).ok_or_else(|| {
                CoreError::WorldDiffViolation {
                    path: format!("/SaveGame/{}", tag),
                    detail: format!("Element '{}' was removed from transformed XML", tag),
                }
            })?;

            let orig_canon = canonicalize_element(orig_child)?;
            let trans_canon = canonicalize_element(trans_child)?;

            if orig_canon != trans_canon {
                return Err(CoreError::WorldDiffViolation {
                    path: format!("/SaveGame/{}", tag),
                    detail: format!("Unexpected world state change in tag '{}'", tag),
                });
            }
        }
    }

    Ok(())
}

fn verify_locations_diff(
    orig_locations: &Element,
    trans_locations: &Element,
    target_cabin_name: &str,
) -> Result<(), CoreError> {
    let orig_loc_children: Vec<&Element> = orig_locations
        .children
        .iter()
        .filter_map(|n| {
            if let XMLNode::Element(e) = n {
                Some(e)
            } else {
                None
            }
        })
        .collect();

    let trans_loc_children: Vec<&Element> = trans_locations
        .children
        .iter()
        .filter_map(|n| {
            if let XMLNode::Element(e) = n {
                Some(e)
            } else {
                None
            }
        })
        .collect();

    if orig_loc_children.len() != trans_loc_children.len() {
        return Err(CoreError::WorldDiffViolation {
            path: "/SaveGame/locations".to_string(),
            detail: format!(
                "Number of locations changed from {} to {}",
                orig_loc_children.len(),
                trans_loc_children.len()
            ),
        });
    }

    for (i, orig_loc) in orig_loc_children.iter().enumerate() {
        let trans_loc = trans_loc_children[i];
        let name = orig_loc
            .get_child("name")
            .and_then(|c| c.get_text())
            .unwrap_or_default();

        if name != "Farm" {
            // Non-farm location must be 100% identical
            let orig_c = canonicalize_element(orig_loc)?;
            let trans_c = canonicalize_element(trans_loc)?;
            if orig_c != trans_c {
                return Err(CoreError::WorldDiffViolation {
                    path: format!("/SaveGame/locations/GameLocation[{}]", name),
                    detail: format!("Unexpected modification to non-farm location '{}'", name),
                });
            }
        } else {
            // For Farm location, verify everything except the target cabin
            verify_farm_diff(orig_loc, trans_loc, target_cabin_name)?;
        }
    }

    Ok(())
}

fn verify_farm_diff(
    orig_farm: &Element,
    trans_farm: &Element,
    target_cabin_name: &str,
) -> Result<(), CoreError> {
    for orig_node in &orig_farm.children {
        if let XMLNode::Element(orig_child) = orig_node {
            let tag = &orig_child.name;

            if tag == "buildings" {
                let trans_buildings = trans_farm.get_child("buildings").ok_or_else(|| {
                    CoreError::WorldDiffViolation {
                        path: "/SaveGame/locations/GameLocation[Farm]/buildings".to_string(),
                        detail: "Missing buildings element in transformed farm".to_string(),
                    }
                })?;
                verify_buildings_diff(orig_child, trans_buildings, target_cabin_name)?;
                continue;
            }

            // Other farm elements (crops, debris, terrainFeatures, etc.) must be unchanged
            let trans_child = trans_farm.get_child(tag.as_str()).ok_or_else(|| {
                CoreError::WorldDiffViolation {
                    path: format!("/SaveGame/locations/GameLocation[Farm]/{}", tag),
                    detail: format!("Element '{}' missing in transformed farm", tag),
                }
            })?;

            let orig_c = canonicalize_element(orig_child)?;
            let trans_c = canonicalize_element(trans_child)?;
            if orig_c != trans_c {
                return Err(CoreError::WorldDiffViolation {
                    path: format!("/SaveGame/locations/GameLocation[Farm]/{}", tag),
                    detail: format!("Unexpected modification to Farm child '{}'", tag),
                });
            }
        }
    }

    Ok(())
}

fn verify_buildings_diff(
    orig_buildings: &Element,
    trans_buildings: &Element,
    target_cabin_name: &str,
) -> Result<(), CoreError> {
    let orig_list: Vec<&Element> = orig_buildings
        .children
        .iter()
        .filter_map(|n| {
            if let XMLNode::Element(e) = n {
                Some(e)
            } else {
                None
            }
        })
        .collect();

    let trans_list: Vec<&Element> = trans_buildings
        .children
        .iter()
        .filter_map(|n| {
            if let XMLNode::Element(e) = n {
                Some(e)
            } else {
                None
            }
        })
        .collect();

    if orig_list.len() != trans_list.len() {
        return Err(CoreError::WorldDiffViolation {
            path: "/SaveGame/locations/GameLocation[Farm]/buildings".to_string(),
            detail: "Building count mismatch".to_string(),
        });
    }

    for (i, orig_b) in orig_list.iter().enumerate() {
        let trans_b = trans_list[i];
        let indoors_name = orig_b
            .get_child("indoors")
            .and_then(|ind| {
                ind.get_child("uniqueName")
                    .or_else(|| ind.get_child("name"))
            })
            .and_then(|c| c.get_text())
            .unwrap_or_default();

        if indoors_name != target_cabin_name {
            // Non-target building must be identical
            let orig_c = canonicalize_element(orig_b)?;
            let trans_c = canonicalize_element(trans_b)?;
            if orig_c != trans_c {
                return Err(CoreError::WorldDiffViolation {
                    path: format!(
                        "/SaveGame/locations/GameLocation[Farm]/buildings/Building[{}]",
                        indoors_name
                    ),
                    detail: format!(
                        "Unexpected modification to non-target building '{}'",
                        indoors_name
                    ),
                });
            }
        } else {
            // For the target cabin, only the <farmhand> element may change
            verify_target_cabin_diff(orig_b, trans_b, target_cabin_name)?;
        }
    }

    Ok(())
}

fn verify_target_cabin_diff(
    orig_b: &Element,
    trans_b: &Element,
    cabin_name: &str,
) -> Result<(), CoreError> {
    // Check non-indoors building fields
    for orig_node in &orig_b.children {
        if let XMLNode::Element(orig_child) = orig_node {
            if orig_child.name != "indoors" {
                let trans_child = trans_b.get_child(orig_child.name.as_str()).ok_or_else(|| {
                    CoreError::WorldDiffViolation {
                        path: format!("/Building[{}]/{}", cabin_name, orig_child.name),
                        detail: "Building property missing".to_string(),
                    }
                })?;
                let orig_c = canonicalize_element(orig_child)?;
                let trans_c = canonicalize_element(trans_child)?;
                if orig_c != trans_c {
                    return Err(CoreError::WorldDiffViolation {
                        path: format!("/Building[{}]/{}", cabin_name, orig_child.name),
                        detail: format!("Building property '{}' modified", orig_child.name),
                    });
                }
            }
        }
    }

    // Inside target cabin indoors, everything EXCEPT farmhand must be identical
    let orig_ind = orig_b.get_child("indoors").unwrap();
    let trans_ind = trans_b.get_child("indoors").unwrap();

    for orig_node in &orig_ind.children {
        if let XMLNode::Element(orig_child) = orig_node {
            if orig_child.name != "farmhand" {
                let trans_child =
                    trans_ind
                        .get_child(orig_child.name.as_str())
                        .ok_or_else(|| CoreError::WorldDiffViolation {
                            path: format!("/Building[{}]/indoors/{}", cabin_name, orig_child.name),
                            detail: "Cabin interior property missing".to_string(),
                        })?;
                let orig_c = canonicalize_element(orig_child)?;
                let trans_c = canonicalize_element(trans_child)?;
                if orig_c != trans_c {
                    return Err(CoreError::WorldDiffViolation {
                        path: format!("/Building[{}]/indoors/{}", cabin_name, orig_child.name),
                        detail: format!("Cabin interior property '{}' modified", orig_child.name),
                    });
                }
            }
        }
    }

    Ok(())
}
