# Stardew Valley Co-op Save Synchronization & Host Migration Research

## 1. Executive Summary

This document presents technical research into the Stardew Valley save format (up to v1.6+), multiplayer architecture, host ownership mechanics, community implementations, and synchronization dynamics.

The fundamental findings:
1. **Host Definition in Stardew Valley:** The game engine hardcodes host ownership to the `<player>` element at the root of the `<SaveGame>` XML tree. Whoever is inside `<player>` is loaded as the host farmer residing in the main `FarmHouse`. Farmhands are nested inside their respective cabins under `<locations> -> <GameLocation xsi:type="Farm"> -> <buildings> -> <Building> -> <indoors xsi:type="Cabin"> -> <farmhand>`.
2. **True Host Switching is Achievable and Reversible:** A farmhand can become the host without losing identity, inventory, skills, relationships, or quests. This requires moving the entire `<Farmer>` object from the cabin's `<farmhand>` into `<player>`, moving the previous host into the cabin's `<farmhand>`, adjusting residence bindings (`homeLocation` and `houseUpgradeLevel`), and updating `SaveGameInfo`.
3. **Save Safety is Non-Negotiable:** Because the users have an active, high-value farm (Kubilay and Elbi), direct in-place mutation of live saves must be forbidden. A strict transactional pipeline (*Backup -> Verify -> Stage -> Transform -> Validate -> Atomic Replace*) is required.
4. **Cloudflare Architecture Fits the Requirements:** A Cloudflare Workers API coordinating D1 (metadata/versions/presence) and R2 (save blobs) eliminates peer-to-peer availability bottlenecks, while zero-knowledge client-side encryption ensures privacy without trusting cloud infrastructure.

---

## 2. Stardew Valley Save File Architecture

### 2.1 File System Layout

On Windows, Stardew Valley stores saves in the user's roaming application data directory:
```
%APPDATA%\StardewValley\Saves\
  └── <FarmName>_<GameID>\
        ├── <FarmName>_<GameID>        (Primary save file: complete world state XML)
        ├── SaveGameInfo               (Summary XML: loaded by title screen Load menu)
        ├── <FarmName>_<GameID>_old    (Previous in-game day backup)
        └── SaveGameInfo_old           (Previous in-game day summary backup)
```

- **Folder Name:** Composed of the farm name plus a random numeric ID generated on creation (e.g., `OakWood_391820491`).
- **Primary Save File:** An extensionless XML document (often 5 MB – 15 MB in size) serialized via .NET's `XmlSerializer`.
- **`SaveGameInfo`:** A small XML file containing a single `<Farmer>` element that matches the host player. Stardew Valley reads ONLY this file when rendering the Load Game menu. If this file does not match the primary save file, the load menu displays stale farmer graphics/stats, though loading the save will load the primary file.
- **`_old` Files:** Created by Stardew Valley at the start of each in-game day as a single-day rollback point.

### 2.2 XML Hierarchy & Serialization

The primary save file root element is `<SaveGame xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema">`.

Key sections within `<SaveGame>`:
```xml
<SaveGame>
  <player>
    <!-- Host Farmer Object -->
    <name>Kubilay</name>
    <UniqueMultiplayerID>819283019283019</UniqueMultiplayerID>
    <homeLocation>FarmHouse</homeLocation>
    <houseUpgradeLevel>2</houseUpgradeLevel>
    <items>...</items>
    <experiencePoints>...</experiencePoints>
    <friendshipData>...</friendshipData>
    ...
  </player>
  
  <locations>
    <GameLocation xsi:type="Farm">
      <buildings>
        <!-- Cabins for Farmhands -->
        <Building>
          <buildingType>Stone Cabin</buildingType>
          <tileX>48</tileX>
          <tileY>14</tileY>
          <indoors xsi:type="Cabin">
            <uniqueName>Cabin_abc123</uniqueName>
            <upgradeLevel>1</upgradeLevel>
            <farmhand>
              <!-- Farmhand Farmer Object -->
              <name>Elbi</name>
              <UniqueMultiplayerID>102938475610293</UniqueMultiplayerID>
              <homeLocation>Cabin_abc123</homeLocation>
              <houseUpgradeLevel>1</houseUpgradeLevel>
              <items>...</items>
              <experiencePoints>...</experiencePoints>
              <friendshipData>...</friendshipData>
              ...
            </farmhand>
          </indoors>
        </Building>
      </buildings>
    </GameLocation>
    
    <GameLocation xsi:type="FarmHouse">
      <!-- Main Farmhouse Interior -->
      <upgradeLevel>2</upgradeLevel>
      ...
    </GameLocation>
  </locations>
  
  <!-- Global World State -->
  <currentSeason>summer</currentSeason>
  <dayOfMonth>18</dayOfMonth>
  <year>2</year>
  <dailyLuck>0.04</dailyLuck>
  <farmerTeam>...</farmerTeam>
  <useSeparateWallets>false</useSeparateWallets>
</SaveGame>
```

---

## 3. Multiplayer Ownership & Host Migration

### 3.1 Why Farmhands Cannot Simply Open the Save

If Elbi receives Kubilay's raw save folder and clicks "Load":
1. Stardew Valley assigns whichever human is playing locally to the root `<player>` tag.
2. Elbi would immediately be playing as **Kubilay** (his name, his inventory, his appearance, his spouse, his skill levels).
3. Elbi's own character would remain trapped inside the cabin.
4. If Kubilay joins via multiplayer co-op, he would not be able to join because his character is already the host!

To allow Elbi to host using her character, the save file must be converted so that Elbi occupies the `<player>` slot, and Kubilay occupies Elbi's cabin slot.

### 3.2 State Categorization: What Travels, What Stays

To prevent data corruption, state must be categorized into three distinct buckets:

| State Bucket | Examples | Migration Handling |
| :--- | :--- | :--- |
| **Personal Player State** | Name, UniqueMultiplayerID, inventory/items, clothing, appearance, stats, skills, experience points, professions, friendship data, active quests, completed quests, mail received, personal mailbox, cooking/crafting recipes | **MUST travel with the player.** The entire `<Farmer>` XML subtree moves intact. |
| **Residence / Building State** | `homeLocation`, `houseUpgradeLevel` | **Stays with the physical building.** The host must have `homeLocation = "FarmHouse"` and `houseUpgradeLevel` equal to the Farmhouse upgrade tier. The farmhand must have `homeLocation = <CabinLocationName>` and `houseUpgradeLevel` equal to the Cabin's upgrade tier. |
| **Global World State** | Calendar date, weather, community center / Joja bundles, museum collection, mine progression, golden walnuts, perfection tracker, shipping bin logs, `farmerTeam` shared flags | **Shared world data.** Remains untouched in `<SaveGame>` root. |

### 3.3 The Crucial Role of `homeLocation` and `houseUpgradeLevel`

- **`homeLocation`:** In Stardew Valley C#, `Game1.player` assumes its spawn point and bed location is `"FarmHouse"`. A farmhand in a cabin must have their `homeLocation` set to that specific cabin's location name (`Cabin` or unique indoors string). If this is not updated when swapping, the new host may spawn out of bounds or letters may fail to deliver.
- **`houseUpgradeLevel`:** The physical size and floor layout of the main Farmhouse (crib, kitchen, cellar) is dictated by the host's `houseUpgradeLevel`. If Elbi's character has upgrade level 0, moving her to host without setting her `houseUpgradeLevel` to match the Farmhouse (e.g., 2) will cause the game to render the starter house layout, hiding furniture placed in the kitchen/nursery. Therefore, `houseUpgradeLevel` is swapped between the players to match their residences.

### 3.4 Spouses, Children, and Pets

- **NPC Spouses:** The spouse NPC is linked to the player via `<spouse>` inside the player's `<Farmer>` node. When the player moves to the Farmhouse, the spouse sleeps in the Farmhouse; when in a cabin, they sleep in the cabin. Preserving the `<Farmer>` node preserves romance and marriage state.
- **Pets:** Pets belong to the Farm and the host. In Stardew Valley 1.6, multiple pets and pet bowls are supported across the farm.
- **Horses:** Stables track `<owner>` by `UniqueMultiplayerID`. Because each farmer retains their `UniqueMultiplayerID`, horse ownership remains stable.

### 3.5 Potential Host-Swap Side Effects & Mitigations

From studying community reports and tools:
1. **Demetrius Cave Event (Mushroom vs Bat):** Stardew Valley checks event ID `65` on the active host. If the new host was never present when Demetrius offered the cave choice, the cutscene may re-trigger upon morning exit. *Mitigation:* Ensure key world event IDs (such as event 65) are mirrored in the new host's `eventsSeen` if already completed.
2. **Community Center Cutscene:** If a player has never stepped into the Community Center while hosting, a Junimo cutscene may play once. World bundle progress remains intact.

---

## 4. Analysis of Existing Open-Source Implementations

### 4.1 ValleySave (`hirieo/valleysave-app`)
- **Technology:** Flutter / Dart desktop and mobile app.
- **Sync Transport:** Google Drive API (user's personal Google account).
- **Core Strengths:**
  - Strict transactional replace pipeline: *prepare -> validate -> backup -> swap -> verify* with automatic rollback.
  - Incomplete save detection (refuses to sync broken directory states).
  - Explicit platform save detection (Windows, Linux, macOS, Android).
- **Limitations:** Focuses on single-user cloud sync across devices; does not implement multiplayer host migration or role switching.
- **License:** Polyform Non-Commercial (cannot be directly copied for commercial derivatives; concepts and architectural patterns are public).

### 4.2 Stardew Valley Host Swapper (`u4ik/StardewValleyHostSwapper`)
- **Technology:** C# standalone executable.
- **Mechanism:** Swaps the root `<player>` with the first farmhand found in cabins.
- **Key Insight:** Specifically handles swapping `homeLocation` and `UniqueMultiplayerID` to maintain mailbox and house integrity.

### 4.3 Stardew Host Swap (`fsih/stardew-host-swap`)
- **Technology:** Browser-based JavaScript utility.
- **Mechanism:** String/chunk-based token swapping between `<player><name>` and `<farmhand><name>`.
- **Key Insight:** Demonstrates that swapping the character subtrees directly transfers inventory, skills, and progress.
- **Limitations:** Fragile string indexing prone to breaking if XML formatting changes or identical names exist. A real implementation must use structured XML parsing (DOM/ElementTree or Rust `quick-xml` / `roxmltree`).

### 4.4 Stardew-Coop-Host-Swapper (`JRitmeester/Stardew-Coop-Host-Swapper`)
- **Technology:** Python CLI using `xml.etree.ElementTree`.
- **Mechanism:** Locates player by name in `<farmhands>` and `<player>`, swaps character nodes.

---

## 5. Platform Distribution & Cloud Interaction Analysis (Microsoft Store / Xbox PC & Steam)

### 5.1 Platform Identification: Microsoft Store / Xbox PC
- **Primary Runtime Platform:** The user owns and runs Stardew Valley 1.6 through the **Microsoft Store / Xbox PC** distribution (NOT Steam).
- **Save Location:** Confirmed to use the standard Windows save path:
  `%APPDATA%\StardewValley\Saves`
- **Process Executable:** Can appear as `Stardew Valley.exe`, `StardewValley.exe`, or packaged Xbox app names. `ProcessMonitor` monitors all naming variants and provides a substring diagnostic scanner.
- **Xbox Cloud Synchronization:** Microsoft Store / Xbox PC games may have Xbox Live Cloud Save integration. We do NOT disable, modify, or interfere with Microsoft/Xbox cloud settings.
- **Observation Strategy:** Rather than making assumptions, `CloudObserver` snapshots file sizes, modification times (`mtime`), and SHA-256 hashes immediately before launch and immediately after exit to detect any background or cloud alterations.

### 5.2 Steam Cloud Mechanics (Reference / Alternative Platform)
- On Steam installations, Steam Cloud syncs `%APPDATA%\StardewValley\Saves`.
- Steam checks file modification timestamps (`mtime`) on game start and game exit.
- While Steam-specific considerations remain relevant for future multi-platform support, they do not apply to the current user's runtime environment.

### 5.3 Unified Safety Protocol
1. **Never mutate while Stardew is running:** Check for `Stardew Valley.exe`, `StardewValley.exe`, and `StardewModdingAPI.exe` before any write.
2. **Explicit Timestamps:** When writing synchronized or transformed saves to disk, ensure filesystem `mtime` reflects the actual write time.
3. **Local Safety Backup First:** Always snapshot to `%LOCALAPPDATA%\stardew-sync\backups\` before replacing anything, guaranteeing one-click rollback regardless of platform.
4. **Hard Production Guard:** Prevent any operation on the production farm `TXrk_450560341` during testing phases.

---

## 6. Process & Save Settle Detection

When Stardew Valley saves (overnight when all players sleep):
1. The game writes `<FarmName>_<GameID>` and `SaveGameInfo` to disk.
2. It renames the previous day's save to `_old`.
3. If the user exits to desktop directly after saving:
   - The game process exits.
   - Operating system write buffers and antivirus scans may hold temporary locks for 500 ms – 2000 ms.

### 6.1 Process Detection Protocol
- Monitor running processes on Windows using native APIs (`CreateToolhelp32Snapshot` / `EnumProcesses` or Rust `sysinfo`).
- Track:
  - `Stardew Valley.exe`
  - `StardewModdingAPI.exe` (SMAPI)
- States: `STOPPED` -> `STARTING` -> `RUNNING` -> `EXITED`.

### 6.2 Settle Timer
Upon process exit:
1. Wait a configurable settle duration (e.g., 3000 ms).
2. Verify file locks by attempting a non-exclusive read open.
3. Calculate SHA-256 hash of the save files.
4. Compare against the session's pre-launch hash:
   - If unchanged: User exited without saving (do not create redundant version).
   - If changed: Session generated new farm progress -> trigger snapshot and sync pipeline.
