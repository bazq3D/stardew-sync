# Phase 4.7 — Current Production Farm Snapshot & Safety Audit Report

**Author:** bazq  
**Date:** 2026-10-09  
**Platform:** Microsoft Store / Xbox PC Stardew Valley 1.6  
**Scope:** Strictly READ-ONLY Snapshot, Verification & WGS Assessment  
**Production Target:** `%APPDATA%\StardewValley\Saves\TXrk_450560341`  

---

## 1. Production Safety Verification

Prior to performing any read or copy operations:
1. **Process Safety:** Confirmed 0 active processes matching `stardew` or `smapi`.
2. **Path & Structure Verification:** Confirmed `%APPDATA%\StardewValley\Saves\TXrk_450560341` exists with standard 4-file structure (`TXrk_450560341`, `SaveGameInfo`, `TXrk_450560341_old`, `SaveGameInfo_old`).
3. **Internal Game ID:** Verified `<uniqueIDForThisGame>` is `450560341`.
4. **Player Identities:** Verified root host is `Kubilay` (ID: `-1971852983897051810`), residence `FarmHouse`; farmhand is `elbi` (ID: `-3807748606349857203`), cabin bound.
5. **Successive Read Stability:** Verified across two independent reads with 1-second delay: file sizes, modification ticks, and SHA-256 hashes are **100% stable (no uncommitted disk writes)**.
6. **Production Protection:** Live production files were read strictly with non-locking, non-mutating copy operations. Zero edits, zero renames, zero writes were performed against the production directory.

---

## 2. New Snapshot Location & Full SHA-256 Manifest

- **New Snapshot Absolute Path:**  
  `C:\Users\bazq3\Desktop\stardew-sync-test\production-snapshots\TXrk_450560341_2026-10-09_180041`
- **Historical Offline Copy:**  
  `C:\Users\bazq3\Desktop\stardew-sync-test\FARM_KLASORU\TXrk_450560341` (100% Untouched, retained as historical baseline)

### Source-to-Copy Byte-for-Byte Equality Matrix:

| File Name | Size (Bytes) | Source Production SHA-256 | Copied Snapshot SHA-256 | Byte-for-Byte Match |
| :--- | :---: | :--- | :--- | :---: |
| **`TXrk_450560341`** | 4,211,366 | `C835F8572AEE9ADAE36F3BDDAB512936C6605339D59096954D052D0D0557DEC1` | `C835F8572AEE9ADAE36F3BDDAB512936C6605339D59096954D052D0D0557DEC1` | **YES (Identical)** |
| **`SaveGameInfo`** | 106,118 | `ECBDA32EBAA515A1FF5BE17690099C401CB70995F6583AF1AC3B32A908852299` | `ECBDA32EBAA515A1FF5BE17690099C401CB70995F6583AF1AC3B32A908852299` | **YES (Identical)** |
| **`TXrk_450560341_old`** | 4,237,111 | `81B7A159B14765C3747789728A3B9FE69DFE5B14AD34EE07B3E1FAF0C8693937` | `81B7A159B14765C3747789728A3B9FE69DFE5B14AD34EE07B3E1FAF0C8693937` | **YES (Identical)** |
| **`SaveGameInfo_old`** | 96,180 | `A7E5069E22E021E89FBE0FD229A989B6DEA0CA1D89304B86DC75ADB4E1EB50A2` | `A7E5069E22E021E89FBE0FD229A989B6DEA0CA1D89304B86DC75ADB4E1EB50A2` | **YES (Identical)** |

---

## 3. Character Identity, Cabin & World Checks

### Character Profiles:
- **Root Host:** `Kubilay`
  - Multiplayer ID: `-1971852983897051810`
  - Residence: `FarmHouse` (House upgrade level: 1)
  - Money: 39,891g
  - Items in inventory: 24
  - Skills: Farming 2, Mining 9, Combat 8, Fishing 5, Foraging 7
  - Social: 30 relationships preserved; 110 mail received; 5 active quests
- **Farmhand:** `elbi`
  - Multiplayer ID: `-3807748606349857203`
  - Residence: `FarmHouse644329ab-e942-4dad-8f28-9b089b9a30e8` (Cabin)
  - Money: 39,891g
  - Items in inventory: 24
  - Skills: Farming 10 (Mastery level!), Fishing 4, Foraging 3, Mining 0, Combat 0
  - Social: 30 relationships preserved; 113 mail received; 4 active quests

### Cabin Binding:
- Cabin unique indoors name: `FarmHouse644329ab-e942-4dad-8f28-9b089b9a30e8`
- Cabin `farmhandReference`: `-3807748606349857203` (**Strict match with elbi's UniqueMultiplayerID**)

### XML Serialization & Schema Integrity:
- UTF-8 BOM present on all files (`0xEF, 0xBB, 0xBF`).
- Root namespaces: `xmlns:xsi` and `xmlns:xsd` present.
- 519,770 XML nodes parsed cleanly without error.

---

## 4. Gameplay Progression Comparison (New vs Historical Snapshot)

| Metric | Historical Snapshot (`FARM_KLASORU`) | New Production Snapshot (`2026-10-09`) | Delta / Progress Description |
| :--- | :---: | :---: | :--- |
| **In-Game Date** | Summer 20, Year 1 | **Fall 23, Year 1** | **+31 in-game days** (Over 1 full season played) |
| **Playtime** | 36,803,712 ms (~10.2 hrs) | **62,800,368 ms (~17.4 hrs)** | **+7.2 hours** real gameplay recorded |
| **Farm Money** | 16,061g | **39,891g** | **+23,830g** net farm profit |
| **Farm Buildings** | 6 buildings | **8 buildings** | **Added Barn and Silo** |
| **Building Types** | Farmhouse, Greenhouse, Shipping Bin, Pet Bowl, Cabin, Coop | Farmhouse, Greenhouse, Shipping Bin, Pet Bowl, Cabin, Coop, **Barn, Silo** | Expanded animal farming infrastructure |
| **Kubilay Skills** | Farm 1 / Mine 7 / Comb 6 / Fish 3 / For 5 | **Farm 2 / Mine 9 / Comb 8 / Fish 5 / For 7** | Substantial skill advancement |
| **elbi Skills** | Farm 7 / Fish 3 / For 3 | **Farm 10 / Fish 4 / For 3** | **Farming reached maximum Level 10** |
| **Mail Received** | Kubilay: 85 / elbi: 72 | **Kubilay: 110 / elbi: 113** | +25 mail (Kubilay), +41 mail (elbi) |

---

## 5. Xbox WGS Read-Only Consistency Findings

Read-only inspection of the production WGS container directory:
`%LOCALAPPDATA%\Packages\ConcernedApe.StardewValleyPC_0c8vynj4cqe4e\SystemAppData\wgs\000901FE00CB68AF_0000000000000000000000007BFD81C5\68DA91E3A52C4108BF6B07C540281EFD`

- **Blob `F3FC7921C96E4E3B8BCBE34580A77647`:**
  - Size: 4,211,366 bytes
  - SHA-256: `C835F8572AEE9ADAE36F3BDDAB512936C6605339D59096954D052D0D0557DEC1`
  - **Matches live `TXrk_450560341` with 100% byte-for-byte fidelity.**
- **Blob `42EF1687CB874CB0B1D4A7297514DD5E`:**
  - Size: 106,118 bytes
  - SHA-256: `ECBDA32EBAA515A1FF5BE17690099C401CB70995F6583AF1AC3B32A908852299`
  - **Matches live `SaveGameInfo` with 100% byte-for-byte fidelity.**
- **Container Index File:** `container.110` timestamp matches last save time.

### Local Consistency Status:
**LOCAL WGS CONSISTENCY VERIFIED:** The working save in `%APPDATA%` and the local Connected Storage container in `SystemAppData\wgs` are in complete, zero-skew parity.

> [!NOTE]
> *Boundary Confirmation:* This confirms local persistence layer consistency on this PC. It does not certify remote Xbox cloud sync status, which is controlled by Microsoft Gaming Services background network operations.

---

## 6. Migration Feasibility & Automated Test Results

1. **Dry Run Migration Against New Snapshot:**
   - Executed `cargo run --bin dry_run -- <NEW_SNAPSHOT_PATH>`.
   - Test 1 (Kubilay -> elbi migration): **PASS**.
   - Test 2 (elbi -> Kubilay roundtrip): **PASS**.
   - Test 3 (10-cycle repeated roundtrip stress test): **PASS (0 semantic drift across 10 cycles)**.
   - Snapshot immutability: **PASS (Hashes identical before and after dry run)**.
2. **Automated Regression Suite (`cargo test`):**
   - `core_safety_tests`: 7 passed
   - `host_migration_tests`: 9 passed
   - `runtime_safety_tests`: 12 passed
   - **Total: 28 passed; 0 failed; 0 warnings.**

---

## 7. Risks & Unresolved Questions

1. **Xbox WGS In-Place Overwrite Limitation:**
   As confirmed in Phase 4.6B, Microsoft Store Stardew Valley restores cached WGS containers over existing slot names at launch. Therefore, any test migration derived from this new snapshot **must use a fresh disposable identity** (e.g. `TXrkTestC_<new_id>`) to avoid startup rollbacks.
2. **Multiplayer Host Switch Deployment:**
   Deploying an elbi-hosted save onto elbi's PC for co-op will require transferring the transformed disposable save folder to elbi's `%APPDATA%\StardewValley\Saves` and ensuring elbi's Xbox Live account launches the game as the hosting player.

---

## 8. Recommendation for Next Step

The newly captured snapshot is structurally pristine, 100% stable, and proven compatible with the host migration engine.

**Recommended Action:**
Derive a new, fresh disposable test state (State C, e.g. `TXrkTestC_<new_id>` with host `elbi`) from this snapshot for the upcoming two-PC multiplayer validation.
