# Stardew Sync v0.2.0 — Character Engine & Transfer Feasibility Study

**Document Version:** 1.0.0  
**Author:** bazq  
**Target Milestone:** v0.2.0  
**Status:** TECHNICAL ARCHITECTURE SPECIFICATION & RESEARCH ASSESSMENT

---

## 1. "My Characters" Read-Only Inspection Engine

Stardew Valley save files store characters in two XML locations:
1. **The Host Character:** Directly under root `/SaveGame/player`.
2. **Farmhand Characters:**
   - In Stardew 1.6: In the root `<farmhands>` collection AND inside cabin interiors `/SaveGame/locations/GameLocation[Farm]/buildings/Building/indoors[Cabin]/farmhand`.
   - In Stardew 1.5 and earlier: Exclusively inside cabin interiors.

### Character Data Model (`CharacterProfile`)

```rust
pub struct CharacterProfile {
    pub name: String,
    pub unique_multiplayer_id: i64,
    pub role: CharacterRole, // Host | Farmhand
    pub farm_name: String,
    pub game_id: String,
    pub cabin_name: Option<String>,
    
    // Progression & Stats
    pub skills: SkillLevels, // Farming, Mining, Foraging, Fishing, Combat (0..10)
    pub experience_points: Vec<u32>,
    pub max_health: i32,
    pub max_stamina: i32,
    pub money: u32,
    pub total_money_earned: u64,
    pub play_time_hours: Option<f64>,
    pub play_time_formatted: String,
    
    // Inventory & Equipment Summary
    pub backpack_capacity: u32, // 12, 24, 36
    pub inventory_item_count: usize,
    pub equipped_items: EquippedGear, // Weapon, Boots, Rings, Hat
    
    // Social & Relationships
    pub spouse: Option<String>,
    pub friendships: Vec<FriendshipSummary>, // NPC Name, Heart Points, Status
    
    // Visual Appearance
    pub gender: String,
    pub hair_style: i32,
    pub skin_color: i32,
}
```

### Version & Mod Compatibility Rules:
- **Missing Tags Fallback:** If `<combatLevel>` or `<experiencePoints>` are missing or malformed, default to 0 rather than failing save parsing.
- **Modded Items (`Item xsi:type="..."`):** Custom modded items from SMAPI / Content Patcher are safely indexed by their display name or type attribute; the parser preserves their raw XML node structures bit-for-bit.

---

## 2. Character Transfer Between Farms — Deep Feasibility Study

### The Core Problem:
Players frequently ask: *"Can I take my established character from Farm A and move them into my friend's Farm B?"*

A Stardew Valley character is **not an isolated entity**. The `<Farmer>` XML object contains hundreds of deep linkages to the world state.

### Architectural Dissection of Character Subsystems

| Subsystem | Stored Under `<Farmer>` | World State Dependency | Transfer Hazard / Failure Mode |
|---|---|---|---|
| **Identity & Appearance** | `<name>`, `<UniqueMultiplayerID>`, `<hair>`, `<skin>` | None (self-contained) | Safe. ID collision check required. |
| **Skills & Stats** | `<farmingLevel>`, `<experiencePoints>`, `<maxHealth>` | None (self-contained) | Safe to transfer. |
| **Inventory & Backpack** | `<items>`, `<equippedItems>` | High for farm-specific items | Generally safe, but tools tied to farm buildings (e.g. Auto-Petter, quest items) or modded items not installed on Farm B will vanish or cause red warnings in SMAPI. |
| **Cabin Binding** | `<homeLocation>` | **Strict (Requires physical cabin)** | **CRITICAL BLOCKER:** Stardew engine crashes if a farmhand lacks a valid assigned cabin in Farm B's `<buildings>`. Farm B **must** have an unoccupied cabin. |
| **World Flags (`mailReceived`)** | `<mailReceived>` (thousands of string flags) | **Absolute World Dependency** | **CRITICAL CORRUPTION HAZARD:** Flags like `ccBoilerRoom`, `JojaMember`, `abandonedJojaMart`, `volcanoBridgeUnlocked`, `willyBoatFixed` dictate world logic. Injecting Farm A's flags into Farm B breaks Farm B's progression state. |
| **Events & Cutscenes** | `<eventsSeen>` | **World Dependency** | Causes cutscenes to skip or desynchronize on Farm B. |
| **Friendships & Marriage** | `<friendshipData>`, `<spouse>` | **Multiplayer Dependency** | **CRITICAL HAZARD:** If Character is married to an NPC (e.g., Abigail) on Farm A, but Farm B's host is married to Abigail, or Abigail has 0 hearts with everyone on Farm B, the relationship dictionary enters an invalid state resulting in divorce/dialogue crashes. |
| **Horse & Pets** | `<horseName>` | **Strict Location Entity** | Character references horse by ID. If horse does not exist in Farm B's `<locations>`, `Game1.loadForNewGame()` throws `NullReferenceException`. |

---

## 3. Feasibility Verdict & Implementation Strategy

### Strategy Evaluation

1. **Direct Raw XML Node Copy:** **REJECTED AS UNVIABLE.**
   - Copying raw `<Farmer>` XML verbatim between saves introduces catastrophic world corruption, relationship desync, and potential title-screen crashes.
2. **Sanitized Profile Transfer ("Stat & Gear Export"):** **ARCHITECTURALLY FEASIBLE.**
   - Instead of copying the entire XML node, the engine extracts a **Sanitized Character Template**:
     - Keeps: Appearance, Skills, Experience, Basic Inventory (weapons, clothing, standard tools, materials), Personal Stats.
     - Strips / Resets: `<mailReceived>`, `<eventsSeen>`, `<activeDialogueEvents>`, `<questLog>`, `<horseName>`, `<spouse>`.
     - Re-initializes: `<friendshipData>` to neutral baselines matching Farm B.
     - Assigns into an existing **unoccupied cabin** on Farm B.
     - Generates a **new `UniqueMultiplayerID`** guaranteed not to collide with Farm B.

### Recommendation for v0.2.0:
- **Scope for v0.2.0:** **Research, Technical Documentation & Read-Only Character Inspector ONLY.**
- **Deferred to Future Milestone:** Writing/importing transferred characters between saves is deferred until the sanitized profile transformation can be exhaustively validated on isolated test harnesses.
