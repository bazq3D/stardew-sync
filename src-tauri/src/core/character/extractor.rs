use serde::{Deserialize, Serialize};
use xmltree::{Element, XMLNode};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillLevels {
    pub farming: u32,
    pub mining: u32,
    pub foraging: u32,
    pub fishing: u32,
    pub combat: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquippedGear {
    pub hat: Option<String>,
    pub boots: Option<String>,
    pub left_ring: Option<String>,
    pub right_ring: Option<String>,
    pub weapon: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendshipSummary {
    pub npc_name: String,
    pub points: i32,
    pub hearts: u32,
    pub status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryItemSummary {
    pub name: String,
    pub stack: u32,
    pub quality: u32,
    pub item_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterProfile {
    pub name: String,
    pub unique_multiplayer_id: i64,
    pub is_host: bool,
    pub farm_name: String,
    pub folder_name: String,
    pub game_id: String,
    pub cabin_name: Option<String>,
    pub raw_cabin_id: Option<String>,

    // Progression & Stats
    pub skills: SkillLevels,
    pub experience_points: Vec<u32>,
    pub max_health: i32,
    pub max_stamina: i32,
    pub money: u32,
    pub total_money_earned: u64,
    pub is_wallet_shared: bool,
    pub play_time_hours: Option<f64>,
    pub play_time_formatted: String,

    // Inventory & Equipment Summary
    pub backpack_capacity: u32,
    pub inventory_item_count: usize,
    pub items: Vec<InventoryItemSummary>,
    pub equipped_gear: EquippedGear,

    // Social & Relationships
    pub spouse: Option<String>,
    pub friendships: Vec<FriendshipSummary>,

    // Visual Appearance
    pub gender: String,
    pub hair_style: i32,
    pub skin_color: i32,
}

fn get_child_text(elem: &Element, child_name: &str) -> Option<String> {
    elem.get_child(child_name)
        .and_then(|c| c.get_text())
        .map(|t| t.into_owned())
}

/// Robust extraction of gender supporting Stardew Valley 1.6+ `<Gender>` / `<gender>`
/// and historical `<isMale>` (including `xsi:nil="true"` and non-boolean variations).
/// Never assumes missing or unparseable gender means male.
pub fn extract_gender(farmer_elem: &Element) -> String {
    // 1. Check Stardew 1.6+ explicit <Gender> or <gender> element
    if let Some(g) = get_child_text(farmer_elem, "Gender").or_else(|| get_child_text(farmer_elem, "gender")) {
        let trimmed = g.trim();
        if !trimmed.is_empty() {
            let lower = trimmed.to_lowercase();
            if lower == "female" || lower == "f" {
                return "Female".to_string();
            } else if lower == "male" || lower == "m" {
                return "Male".to_string();
            } else if lower == "undefined" || lower == "unknown" {
                return "Unknown".to_string();
            } else {
                return trimmed.to_string();
            }
        }
    }

    // 2. Check <isMale> element
    if let Some(is_male_elem) = farmer_elem.get_child("isMale") {
        let is_nil = is_male_elem
            .attributes
            .get("nil")
            .or_else(|| is_male_elem.attributes.get("xsi:nil"))
            .map(|v| v == "true")
            .unwrap_or(false);

        if !is_nil {
            if let Some(text) = is_male_elem.get_text() {
                let trimmed = text.trim().to_lowercase();
                if trimmed == "false" || trimmed == "0" || trimmed == "f" {
                    return "Female".to_string();
                } else if trimmed == "true" || trimmed == "1" || trimmed == "t" {
                    return "Male".to_string();
                }
            }
        }
    }

    // Never assume missing or unparseable gender means male
    "Unknown".to_string()
}

/// Convert Stardew Valley experience points to skill level (0..10).
pub fn exp_to_level(exp: u32) -> u32 {
    let thresholds = [100, 380, 770, 1300, 2150, 3300, 4800, 6900, 10000, 15000];
    let mut level = 0;
    for (idx, &threshold) in thresholds.iter().enumerate() {
        if exp >= threshold {
            level = (idx + 1) as u32;
        } else {
            break;
        }
    }
    level
}

fn ms_to_hours(ms: u64) -> f64 {
    (ms as f64) / (1000.0 * 60.0 * 60.0)
}

fn format_playtime(ms_opt: Option<u64>) -> String {
    match ms_opt {
        Some(ms) => {
            let total_seconds = ms / 1000;
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let decimal_hours = ms_to_hours(ms);
            format!("{}h {}m ({:.1} hours)", hours, minutes, decimal_hours)
        }
        None => "Unknown".to_string(),
    }
}

/// Extracts a single `CharacterProfile` from a `<player>`, `<farmhand>`, or `<Farmer>` XML element.
pub fn extract_character_from_farmer_element(
    farmer_elem: &Element,
    is_host: bool,
    farm_name_fallback: &str,
    folder_name: &str,
    game_id_fallback: &str,
    cabin_name: Option<String>,
    raw_cabin_id: Option<String>,
    is_wallet_shared: bool,
) -> CharacterProfile {
    let name = get_child_text(farmer_elem, "name").unwrap_or_else(|| "Unknown".to_string());
    let unique_multiplayer_id = get_child_text(farmer_elem, "UniqueMultiplayerID")
        .as_deref()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    let farm_name = get_child_text(farmer_elem, "farmName")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| farm_name_fallback.to_string());

    let game_id = get_child_text(farmer_elem, "uniqueIDForThisGame")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| game_id_fallback.to_string());

    // Extract experience points: Stardew order: [0: Farming, 1: Fishing, 2: Foraging, 3: Mining, 4: Combat, 5: Luck]
    let mut experience_points = Vec::new();
    if let Some(exp_node) = farmer_elem.get_child("experiencePoints") {
        for child in &exp_node.children {
            if let XMLNode::Element(int_elem) = child {
                let pts: u32 = int_elem
                    .get_text()
                    .and_then(|t| t.parse().ok())
                    .unwrap_or(0);
                experience_points.push(pts);
            }
        }
    }
    while experience_points.len() < 6 {
        experience_points.push(0);
    }

    // Extract skills with fallback to experience points
    let farming = get_child_text(farmer_elem, "farmingLevel")
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| exp_to_level(experience_points[0]));

    let fishing = get_child_text(farmer_elem, "fishingLevel")
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| exp_to_level(experience_points[1]));

    let foraging = get_child_text(farmer_elem, "foragingLevel")
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| exp_to_level(experience_points[2]));

    let mining = get_child_text(farmer_elem, "miningLevel")
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| exp_to_level(experience_points[3]));

    let combat = get_child_text(farmer_elem, "combatLevel")
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| exp_to_level(experience_points[4]));

    let skills = SkillLevels {
        farming,
        mining,
        foraging,
        fishing,
        combat,
    };

    // Vitals
    let max_health = get_child_text(farmer_elem, "maxHealth")
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    let max_stamina = get_child_text(farmer_elem, "maxStamina")
        .and_then(|s| s.parse().ok())
        .unwrap_or(270);

    let money = get_child_text(farmer_elem, "money")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let total_money_earned = get_child_text(farmer_elem, "totalMoneyEarned")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let ms_played = get_child_text(farmer_elem, "millisecondsPlayed")
        .and_then(|s| s.parse().ok());
    let play_time_hours = ms_played.map(ms_to_hours);
    let play_time_formatted = format_playtime(ms_played);

    let backpack_capacity = get_child_text(farmer_elem, "maxItems")
        .and_then(|s| s.parse().ok())
        .unwrap_or(12);

    // Inventory items
    let mut items = Vec::new();
    let mut detected_weapon: Option<String> = None;

    if let Some(items_node) = farmer_elem.get_child("items") {
        for child in &items_node.children {
            if let XMLNode::Element(item_elem) = child {
                let is_nil = item_elem
                    .attributes
                    .get("nil")
                    .or_else(|| item_elem.attributes.get("xsi:nil"))
                    .map(|v| v == "true")
                    .unwrap_or(false);

                if is_nil {
                    continue;
                }

                let item_name = get_child_text(item_elem, "DisplayName")
                    .or_else(|| get_child_text(item_elem, "name"));

                if let Some(name) = item_name {
                    if name.is_empty() {
                        continue;
                    }

                    let stack = get_child_text(item_elem, "Stack")
                        .or_else(|| get_child_text(item_elem, "stack"))
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1);

                    let quality = get_child_text(item_elem, "Quality")
                        .or_else(|| get_child_text(item_elem, "quality"))
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);

                    let item_type = item_elem
                        .attributes
                        .get("type")
                        .or_else(|| item_elem.attributes.get("xsi:type"))
                        .map(|s| s.as_str())
                        .unwrap_or("Item")
                        .to_string();

                    // Detect equipped or primary weapon
                    if detected_weapon.is_none()
                        && (item_type.contains("Weapon")
                            || name.contains("Sword")
                            || name.contains("Dagger")
                            || name.contains("Club")
                            || name.contains("Hammer")
                            || name.contains("Slingshot"))
                    {
                        detected_weapon = Some(name.clone());
                    }

                    items.push(InventoryItemSummary {
                        name,
                        stack,
                        quality,
                        item_type,
                    });
                }
            }
        }
    }

    let inventory_item_count = items.len();

    // Equipped Gear
    let hat = farmer_elem
        .get_child("hat")
        .and_then(|e| get_child_text(e, "DisplayName").or_else(|| get_child_text(e, "name")))
        .filter(|s| !s.is_empty());

    let boots = farmer_elem
        .get_child("boots")
        .and_then(|e| get_child_text(e, "DisplayName").or_else(|| get_child_text(e, "name")))
        .filter(|s| !s.is_empty());

    let left_ring = farmer_elem
        .get_child("leftRing")
        .and_then(|e| get_child_text(e, "DisplayName").or_else(|| get_child_text(e, "name")))
        .filter(|s| !s.is_empty());

    let right_ring = farmer_elem
        .get_child("rightRing")
        .and_then(|e| get_child_text(e, "DisplayName").or_else(|| get_child_text(e, "name")))
        .filter(|s| !s.is_empty());

    let equipped_gear = EquippedGear {
        hat,
        boots,
        left_ring,
        right_ring,
        weapon: detected_weapon,
    };

    // Social Relationships
    let spouse = get_child_text(farmer_elem, "spouse").filter(|s| !s.is_empty());

    let mut friendships = Vec::new();
    if let Some(friendship_node) = farmer_elem.get_child("friendshipData") {
        for child in &friendship_node.children {
            if let XMLNode::Element(item_elem) = child {
                let npc_name = item_elem
                    .get_child("key")
                    .and_then(|k| get_child_text(k, "string"));

                let value_elem = item_elem.get_child("value");
                let friendship_elem = value_elem.and_then(|v| v.get_child("Friendship"));

                if let (Some(npc), Some(fr)) = (npc_name, friendship_elem) {
                    let points = get_child_text(fr, "Points")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    let hearts = (points.max(0) / 250) as u32;
                    let status = get_child_text(fr, "Status").filter(|s| !s.is_empty());

                    friendships.push(FriendshipSummary {
                        npc_name: npc,
                        points,
                        hearts,
                        status,
                    });
                }
            }
        }
    }
    // Sort friendships by highest hearts first
    friendships.sort_by(|a, b| b.points.cmp(&a.points));

    // Appearance: Robust extraction, never assumes missing gender means male
    let gender = extract_gender(farmer_elem);

    let hair_style = get_child_text(farmer_elem, "hair")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let skin_color = get_child_text(farmer_elem, "skin")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    CharacterProfile {
        name,
        unique_multiplayer_id,
        is_host,
        farm_name,
        folder_name: folder_name.to_string(),
        game_id,
        cabin_name,
        raw_cabin_id,
        skills,
        experience_points,
        max_health,
        max_stamina,
        money,
        total_money_earned,
        is_wallet_shared,
        play_time_hours,
        play_time_formatted,
        backpack_capacity,
        inventory_item_count,
        items,
        equipped_gear,
        spouse,
        friendships,
        gender,
        hair_style,
        skin_color,
    }
}

/// Discovers and extracts all characters (Host + Farmhands) across a parsed `SaveGame` DOM tree.
pub fn extract_all_characters_from_save(
    root: &Element,
    folder_name: &str,
) -> Vec<CharacterProfile> {
    let mut characters = Vec::new();
    let farm_name = get_child_text(root, "farmName").unwrap_or_else(|| "Farm".to_string());
    let game_id = get_child_text(root, "uniqueIDForThisGame").unwrap_or_default();

    // Determine shared-wallet mode from <farmerTeam><useSeparateWallets>
    let is_wallet_shared = root
        .get_child("farmerTeam")
        .and_then(|ft| get_child_text(ft, "useSeparateWallets"))
        .map(|s| {
            let lower = s.trim().to_lowercase();
            lower == "false" || lower == "0"
        })
        .unwrap_or(true); // Default in Stardew Valley co-op is shared wallet

    // 1. Host character under /SaveGame/player
    if let Some(player_elem) = root.get_child("player") {
        let host_profile = extract_character_from_farmer_element(
            player_elem,
            true,
            &farm_name,
            folder_name,
            &game_id,
            None,
            None,
            is_wallet_shared,
        );
        characters.push(host_profile);
    }

    // Map of cabin unique names / indoors names to (friendly_label, raw_id)
    let mut cabin_map: std::collections::HashMap<i64, (String, String)> = std::collections::HashMap::new();

    // 2. Discover cabins under locations
    if let Some(locations) = root.get_child("locations") {
        for loc in &locations.children {
            if let XMLNode::Element(loc_elem) = loc {
                if let Some(buildings) = loc_elem.get_child("buildings") {
                    let mut cabin_idx = 0;
                    for b in &buildings.children {
                        if let XMLNode::Element(b_elem) = b {
                            if let Some(indoors) = b_elem.get_child("indoors") {
                                let building_type = get_child_text(b_elem, "buildingType")
                                    .unwrap_or_else(|| "Cabin".to_string());
                                let raw_unique_name = get_child_text(indoors, "uniqueName")
                                    .or_else(|| get_child_text(indoors, "name"))
                                    .unwrap_or_else(|| "Cabin".to_string());

                                cabin_idx += 1;
                                let friendly_label = if !building_type.is_empty() && building_type != "FarmHouse" {
                                    building_type.clone()
                                } else {
                                    format!("Cabin {}", cabin_idx)
                                };

                                if let Some(fh_ref) = get_child_text(indoors, "farmhandReference")
                                    .and_then(|s| s.parse::<i64>().ok())
                                {
                                    cabin_map.insert(fh_ref, (friendly_label.clone(), raw_unique_name.clone()));
                                }

                                // Also check legacy interior farmhand node
                                if let Some(fh_elem) = indoors.get_child("farmhand") {
                                    if let Some(id) = get_child_text(fh_elem, "UniqueMultiplayerID")
                                        .and_then(|s| s.parse::<i64>().ok())
                                    {
                                        cabin_map.insert(id, (friendly_label.clone(), raw_unique_name.clone()));
                                        let name = get_child_text(fh_elem, "name").unwrap_or_default();
                                        if !name.is_empty()
                                            && !characters.iter().any(|c| c.unique_multiplayer_id == id)
                                        {
                                            let profile = extract_character_from_farmer_element(
                                                fh_elem,
                                                false,
                                                &farm_name,
                                                folder_name,
                                                &game_id,
                                                Some(friendly_label.clone()),
                                                Some(raw_unique_name.clone()),
                                                is_wallet_shared,
                                            );
                                            characters.push(profile);
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

    // 3. Stardew 1.6+ Root <farmhands> collection
    if let Some(root_farmhands) = root.get_child("farmhands") {
        for child in &root_farmhands.children {
            if let XMLNode::Element(fh_elem) = child {
                let name = get_child_text(fh_elem, "name").unwrap_or_default();
                let id_opt = get_child_text(fh_elem, "UniqueMultiplayerID")
                    .and_then(|s| s.parse::<i64>().ok());

                if let Some(id) = id_opt {
                    // Only add if not an empty uncreated farmhand slot and not already extracted
                    if !name.is_empty() && !characters.iter().any(|c| c.unique_multiplayer_id == id) {
                        let (friendly_cabin, raw_cabin) = match cabin_map.get(&id) {
                            Some((f, r)) => (Some(f.clone()), Some(r.clone())),
                            None => (None, None),
                        };
                        let profile = extract_character_from_farmer_element(
                            fh_elem,
                            false,
                            &farm_name,
                            folder_name,
                            &game_id,
                            friendly_cabin,
                            raw_cabin,
                            is_wallet_shared,
                        );
                        characters.push(profile);
                    }
                }
            }
        }
    }

    characters
}
