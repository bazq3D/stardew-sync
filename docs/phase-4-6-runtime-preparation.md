# Phase 4.6 — Controlled Runtime Validation Preparation (Stage 1 Report)

**Author:** bazq  
**Date:** 2026-10-08  
**Project:** `stardew-sync`  
**Baseline Commit:** `dd960e3`  
**Current Stage:** Stage 1 — PREPARE ONLY (Gated before Installation and Launch)

---

## 1. Executive Summary

Phase 4.5 established that our host migration engine parses and transforms the real Stardew Valley 1.6 multiplayer save losslessly across 10 roundtrip cycles with zero semantic drift.

Phase 4.6 Stage 1 prepares the environment for **controlled runtime validation**: proving that the game executable itself (Stardew Valley 1.6 on Microsoft Store / Xbox PC) loads our transformed save, renders the characters and farm intact, and executes a full overnight save/load cycle without corruption.

In accordance with strict safety mandates:
- **No live Stardew save has been modified.**
- **The live production farm `TXrk_450560341` is hard-guarded against overwrite, deletion, or renaming.**
- **The disposable test saves have NOT been installed into `%APPDATA%\StardewValley\Saves`.**
- **Stardew Valley has NOT been launched.**
- **Two isolated test states (State A: Kubilay host, State B: elbi host) have been staged offline and verified.**

---

## 2. Platform Realignment: Microsoft Store / Xbox PC

The user runs Stardew Valley through the **Microsoft Store / Xbox PC** distribution (not Steam).

### Implications & Architecture Adjustments:
1. **Save Location Unchanged:** The save directory remains `%APPDATA%\StardewValley\Saves`.
2. **Process Monitoring:** The game executable can appear as `Stardew Valley.exe`, `StardewValley.exe`, or without the `.exe` extension depending on Xbox application packaging. `ProcessMonitor` was updated with:
   - Full case-insensitive target coverage (`Stardew Valley.exe`, `Stardew Valley`, `StardewValley.exe`, `StardewValley`, `StardewModdingAPI.exe`, `StardewModdingAPI`).
   - A diagnostic helper (`ProcessMonitor::get_stardew_diagnostics`) that scans all running system processes for any process containing the substring `"stardew"`.
3. **Xbox Cloud Safety:** We do NOT alter, disable, or tamper with Microsoft Store or Xbox cloud configuration.
4. **Cloud Observation vs Assumptions:** Rather than assuming Xbox Cloud sync timing, `CloudObserver` takes cryptographic before-and-after snapshots (SHA-256, file size, modification timestamps) of the save directory to observe and isolate any external sync activity.

---

## 3. Production Save Guard (`ProductionGuard`)

A dedicated safety guard (`ProductionGuard`) was engineered to prevent accidental mutation of the user's real farm:

- **Target Folder Rejection:** Any operation attempting to write to `TXrk_450560341` (case-insensitive) or any path containing the production Game ID (`450560341`) is immediately aborted with a fatal error.
- **Save Content Rejection:** Any XML payload containing `<uniqueIDForThisGame>450560341</uniqueIDForThisGame>` is rejected, preventing disposable tests from masquerading as the production farm.
- **Process Running Lock:** If Stardew Valley is running in any capacity, all save installations and modifications are hard-blocked (`GameRunning`).

---

## 4. Disposable Save Strategy & Identity

To ensure Stardew Valley treats the test save as a completely independent farm slot without colliding with `TXrk_450560341`:

| Parameter | Live Production Farm | Disposable Test Farm |
|---|---|---|
| **Farm Name** | `Türk` | `TürkTest` |
| **Sanitized Prefix** | `TXrk` | `TXrkTest` |
| **uniqueIDForThisGame** | `450560341` | `999450560` |
| **Folder Name** | `TXrk_450560341` | `TXrkTest_999450560` |
| **Primary Save File** | `TXrk_450560341` | `TXrkTest_999450560` |
| **Load Menu Slot** | `Kubilay - Türk Farm` | `Kubilay - TürkTest Farm` |

The transformation was derived **strictly from the offline manually provided copy** in `C:\Users\bazq3\Desktop\stardew-sync-test\FARM_KLASORU\TXrk_450560341`. The live `%APPDATA%` directory was never accessed.

---

## 5. Offline Staging: Two Validated States

Both validation states were generated and staged offline in `C:\Users\bazq3\Desktop\stardew-sync-test\disposable-runtime-states`:

### State A: Kubilay Host
- **Location:** `disposable-runtime-states\state-a-kubilay-host\TXrkTest_999450560\`
- **Host:** Kubilay (ID: `1506041005934522956`, Residence: `FarmHouse`)
- **Farmhand:** elbi (ID: `-3807748606349857203`, Residence: `FarmHouse644329ab-e942-4dad-8f28-9b089b9a30e8`)
- **Primary Save SHA-256:** `52d3fff4ddbe814743949532da202e85a089ed311cb8c0187e3efc6e6d704c60`
- **SaveGameInfo SHA-256:** `89f43354b6f75fd4d9c219ec58023064ea41c7c5843a3ad165081f4ddbc70765`
- **Kubilay Baseline Fingerprint:** `15dafd7b047cf9bccf352b7d4c3c8c39e3018e3bad9b6dc0bb6e32068f9b82b2`
- **elbi Baseline Fingerprint:** `99c963caf5b7a46cbf2a975696b2fe34f76f109b113b67c2158b47d735c648d8`
- **Status:** Structural & Semantic Validation PASS

### State B: elbi Host
- **Location:** `disposable-runtime-states\state-b-elbi-host\TXrkTest_999450560\`
- **Host:** elbi (ID: `-3807748606349857203`, Residence: `FarmHouse`)
- **Farmhand:** Kubilay (ID: `1506041005934522956`, Residence: `FarmHouse644329ab-e942-4dad-8f28-9b089b9a30e8`)
- **Primary Save SHA-256:** `5d8bb437fff5c6c1a668f92b4d5b2e5a8355965cf808c5a95332800b657739bc`
- **SaveGameInfo SHA-256:** `296b242c48becae3c0df15572921924217a768eba17c8584ff1c989325449917`
- **Kubilay Baseline Fingerprint:** `15dafd7b047cf9bccf352b7d4c3c8c39e3018e3bad9b6dc0bb6e32068f9b82b2`
- **elbi Baseline Fingerprint:** `99c963caf5b7a46cbf2a975696b2fe34f76f109b113b67c2158b47d735c648d8`
- **Status:** Structural & Semantic Validation PASS

### Source Immutability
After generating both states, all 4 files in `C:\Users\bazq3\Desktop\stardew-sync-test\FARM_KLASORU\TXrk_450560341` were re-hashed. Hashes were verified to be **100% identical** (zero bytes modified).

---

## 6. Post-Runtime Analyzer (`PostRuntimeAnalyzer`)

When Stardew Valley executes an overnight save, byte-level identity is not expected because the game engine legitimately re-serializes XML, advances timestamps, and records gameplay progression.

`PostRuntimeAnalyzer` (`src-tauri/src/core/runtime/analyzer.rs` and CLI `analyze_runtime_test`) handles this by categorizing differences into:
1. **Serialization Differences:** Element ordering or whitespace adjustments made by .NET XmlSerializer.
2. **Normal Gameplay Progression:** In-game time advancing, gold earned/spent, daily luck changes, sleep/save cycle.
3. **Expected Host Migration Changes:** Swapping host and cabin positions.
4. **Unexpected Destructive Changes:** Dropped farmhands, corrupted IDs, broken cabin indoor bindings, missing farm locations.

---

## 7. Automated Test Suite Results

A comprehensive automated test suite covers the new runtime safety components:
- `test_production_farm_folder_identity_strictly_rejected`: PASS
- `test_production_save_content_strictly_rejected`: PASS
- `test_stardew_running_state_blocks_installation`: PASS
- `test_disposable_save_preparation_and_baseline_manifest_creation`: PASS
- `test_post_runtime_analyzer_handles_gameplay_progression_and_serialization_changes`: PASS
- `test_post_runtime_analyzer_detects_destructive_anomalies`: PASS
- `test_validator_rejects_duplicate_multiplayer_ids`: PASS

**Total Test Suite Result:** 22/22 tests passing across core safety, host migration, and runtime safety.

---

## 8. Remaining Risks & Mitigation

| Risk | Mitigation |
|---|---|
| **Accidental Overwrite of Live Farm** | `ProductionGuard` hard-rejects `TXrk_450560341` and `450560341`. Test uses `TXrkTest_999450560`. |
| **Xbox Cloud Conflict** | `CloudObserver` snapshots directory before launch and after exit to observe any background cloud sync. |
| **Process Collision** | Installation requires Stardew to be closed; running processes block write transactions. |
| **Multiplayer State Drift** | Both State A and State B have immutable `RUNTIME_TEST_BASELINE.json` manifests for verification. |

---

## 9. Stage 1 Verdict

**READY FOR USER-APPROVED RUNTIME INSTALL**

*(All Stage 1 preparation tasks are complete. Awaiting user authorization before installing State A into the live saves folder.)*
