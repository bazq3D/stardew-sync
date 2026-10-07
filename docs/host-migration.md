# Stardew Valley Host Migration Engine Specification

**Author:** bazq  
**Project:** `stardew-sync`  
**Phase:** 4 — Host Switching Engine & Invariant Verification  
**Compatibility:** Stardew Valley 1.6+ (Vanilla & SMAPI Modded)

---

## 1. Executive Summary

In multiplayer Stardew Valley, the host player and farmhand players are structured fundamentally differently in the save XML:
- The **Host** is serialized directly under the document root: `/SaveGame/player`.
- Each **Farmhand** is serialized inside their assigned cabin indoors:  
  `/SaveGame/locations/GameLocation[Farm]/buildings/Building/indoors[Cabin]/farmhand`.
- An auxiliary companion file, `SaveGameInfo`, contains a shallow/root `<Farmer>` structure representing the host character displayed on the title screen load menu.

When players alternate hosting duties (e.g. Kubilay and Elbi), the world state must remain identical, while the two players cleanly swap places between the primary Farmhouse (`/SaveGame/player`) and the cabin (`/SaveGame/.../farmhand`).

This document details the exact, lossless host migration algorithm, the deterministic fingerprinting model, the allowed XML diff surface, and the mathematical proof of reversibility ($A \to B \to A$).

---

## 2. Player Identity & Cabin Matching

### 2.1 UniqueMultiplayerID vs Player Name
Player names in Stardew Valley are mutable and not guaranteed to be unique. A farmhand may be renamed, or two players could hypothetically share a name.
- **Identity Key:** `UniqueMultiplayerID` (a 64-bit signed integer / `long`).
- All identity resolutions, cabin matching, and validation lookups **must strictly use** `UniqueMultiplayerID`.

### 2.2 Cabin Location & Indirection
In Stardew Valley 1.6+, cabins reside within the `Farm` location's `<buildings>` list. Each cabin has:
1. `buildingType`: e.g. `"Stone Cabin"`, `"Plank Cabin"`, `"Log Cabin"`.
2. `<indoors xsi:type="Cabin">`: Contains `<uniqueName>` (or `<name>`), e.g., `"Cabin_stone_1"`.
3. `<farmhand>`: An entire `<Farmer>` object containing the farmhand's state, inventory, and metadata.

A valid target for migration must satisfy:
- Target player exists in an existing cabin indoors.
- Target player has a valid `UniqueMultiplayerID`.
- Target player is NOT already the current host (`source_save.metadata.host_player.unique_multiplayer_id != target_player_id`).

---

## 3. The Migration Algorithm

The host migration pipeline operates as a functional, non-destructive transformation over the XML DOM:

```
[Save XML + SaveGameInfo XML]
             │
             ▼
   [Structural Validation]
             │
             ▼
[Multiplayer Prerequisite Check]
             │
             ▼
[Baseline Fingerprint Extraction]
  - Prev Host Stable Fingerprint
  - Target Farmhand Stable Fingerprint
             │
             ▼
     [DOM Node Transformation]
  - Clone target farmhand -> set tag to <player>, set homeLocation="FarmHouse", upgradeLevel=farmhouse_level
  - Clone original host   -> set tag to <farmhand>, set homeLocation=cabin_name, upgradeLevel=cabin_level
  - Swap <player> in root
  - Swap <farmhand> inside target cabin
  - Clone new host children into SaveGameInfo <Farmer>
             │
             ▼
   [Post-Migration Validation]
  - Host identity == target_player_id
  - Cabin farmhand identity == previous_host_id
  - SaveGameInfo synchronized
             │
             ▼
  [Fingerprint Equality Check]
  - Prev host fingerprint preserved bit-for-bit
  - Target farmhand fingerprint preserved bit-for-bit
             │
             ▼
    [Allowed Diff Verification]
  - Strict assertion: ONLY swapped nodes differ; world state delta == 0
             │
             ▼
   [Verified MigrationResult]
```

---

## 4. Fingerprint Model & Tag Normalization

### 4.1 Full Fingerprint vs Migration-Stable Fingerprint
To guarantee that zero inventory, stats, skills, quests, mail, or mod data are lost or corrupted, we compute SHA-256 fingerprints:

1. **Full Farmer Fingerprint (`compute_full_farmer_fingerprint`):**
   - Canonicalizes and hashes 100% of the player's XML subtree.
   - Used to verify complete $A \to B \to A$ roundtrip restoration.
2. **Migration-Stable Fingerprint (`compute_migration_stable_fingerprint`):**
   - Strips **only** residence-bound fields:
     - `homeLocation`: Changes between `"FarmHouse"` and the cabin indoors name (e.g. `"Cabin_stone_1"`).
     - `houseUpgradeLevel`: Swaps between the Farmhouse upgrade level (0, 1, 2, or 3) and the Cabin upgrade level (0, 1, or 2).
   - All other 100+ elements (including `items`, `experiencePoints`, `professions`, `friendshipData`, `questLog`, `mailReceived`, `stats`, `modData`) **must match exactly**.

### 4.2 Tag Normalization to `<Farmer>`
In the save file:
- The host element is named `<player>`.
- The cabin farmhand element is named `<farmhand>`.
- In `SaveGameInfo`, the root element is named `<Farmer>`.

When a farmhand (`<farmhand>`) becomes host (`<player>`), serializing their XML would produce different outer tags. To prevent spurious hash mismatches:
- In `compute_migration_stable_fingerprint` and `compute_full_farmer_fingerprint`, the root element name is normalized to `"Farmer"` prior to canonical emission:
```rust
let mut cloned = elem.clone();
cloned.name = "Farmer".to_string();
```
This ensures that the semantic player data produces an identical hash regardless of whether the character currently resides in `<player>`, `<farmhand>`, or `SaveGameInfo`.

---

## 5. Allowed XML Diff Surface Model

Any tool that writes to a save file risks introducing unintended collateral modifications (e.g., date changes, weather resets, crop despawns, chest deletion).

`stardew-sync` enforces the **Allowed Diff Surface Invariant**:
During host migration, **no XML node outside the explicit swap path is permitted to differ by even a single byte**.

| XML Path | Permitted Change |
|---|---|
| `/SaveGame/player` | Must contain the target farmhand, re-tagged as `<player>` with Farmhouse residence |
| `/SaveGame/locations/GameLocation[Farm]/buildings/Building/indoors[Cabin]/farmhand` | Must contain the former host, re-tagged as `<farmhand>` with Cabin residence |
| `/SaveGame/locations/GameLocation[Farm]/buildings/Building[Other]` | **STRICTLY FORBIDDEN** (0 bytes delta) |
| `/SaveGame/locations/GameLocation[NonFarm]` | **STRICTLY FORBIDDEN** (0 bytes delta) |
| `/SaveGame/currentSeason`, `/year`, `/dayOfMonth` | **STRICTLY FORBIDDEN** (0 bytes delta) |
| `/SaveGame/farmerTeam`, `/dailyLuck`, `/weather*` | **STRICTLY FORBIDDEN** (0 bytes delta) |
| All other root elements | **STRICTLY FORBIDDEN** (0 bytes delta) |

This is verified algorithmically in `verify_allowed_diff`:
1. Every top-level node except `player` and `locations` is canonicalized and compared for exact string equality.
2. Every non-Farm location is canonicalized and compared for exact string equality.
3. Every non-target building inside Farm is canonicalized and compared for exact string equality.
4. All non-farmhand fields inside the target cabin are canonicalized and compared for exact string equality.

If any other node in the save was modified, `CoreError::WorldDiffViolation` is raised and the transaction aborts before anything is written to disk.

---

## 6. SaveGameInfo Synchronization

`SaveGameInfo` is read by the Stardew Valley title screen to render the load slot preview (character portrait, name, farm name, money, date, played time).

If `SaveGameInfo` is not synchronized with the new host:
1. The load menu displays the wrong player name and appearance.
2. In multiplayer lobby lists, the game may misidentify the hosting character.

### Synchronization Technique:
1. Parse `SaveGameInfo` XML into a DOM element (`<Farmer>`).
2. Replace all child elements of `<Farmer>` with a clone of the new host's `<player>` children.
3. Retain the original XML declaration and root tag attributes (namespaces).
4. Re-serialize deterministically.

---

## 7. Mod Compatibility & SMAPI `modData`

Many Stardew Valley mods (e.g. Expanded, Ridgeside Village, Automate, UI Info Suite) store persistent data in:
- `<modData>` dictionaries under `<player>`, `<farmhand>`, `<Building>`, and `<GameLocation>`.
- Custom XML nodes with custom schemas or attributes.

Because `stardew-sync` utilizes a lossless DOM parser (`xmltree` preserving all nodes, attributes, comments, and structure) rather than a rigid deserializer schema:
- **Unknown tags are never discarded.**
- Custom namespaces and `xsi:type` annotations remain untouched.
- Mod-specific keys inside `<modData>` (e.g. `mod_test_flag`, quest progress, custom warp points) stay attached to their respective player or cabin through any number of migrations.

---

## 8. Reversibility & Stress Testing Proof

The core requirement of safe host migration is **algebraic reversibility**:
$$\text{Migrate}(\text{Migrate}(S, B), A) \equiv S$$

In our automated test suite:
1. **Single Roundtrip (`test_host_migration_roundtrip_a_to_b_to_a`):**
   - Save migrated from PlayerA to PlayerB.
   - Result migrated back from PlayerB to PlayerA.
   - SHA-256 full fingerprint of PlayerA after roundtrip matches original PlayerA bit-for-bit.
2. **Repeated Migration Stress Test (`test_repeated_roundtrip_stress_test`):**
   - Save subjected to **30 consecutive host switches** ($A \to B \to A \to B \dots$).
   - Fingerprint checked after each iteration.
   - Zero drift or degradation observed over all 30 cycles.
3. **Mod Data Retention (`test_mod_data_and_unknown_xml_nodes_preserved`):**
   - Mod flags (`alpha_42`, `beta_99`) verified preserved through migration and roundtrip.
