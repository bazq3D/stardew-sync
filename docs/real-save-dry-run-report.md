# Phase 4.5 — Controlled Real-Save Dry Run Report

**Author:** bazq  
**Project:** `stardew-sync`  
**Execution Type:** Offline Dry Run on Manually Provided Copy  
**Date:** 2026-10-07  

---

## 1. SOURCE SAFETY

- **Explicit Supplied Path:** `C:\Users\bazq3\Desktop\stardew-sync-test\FARM_KLASORU\TXrk_450560341`
- **Confirmed Outside Live Stardew Save Directory:** YES (Strictly validated outside `%APPDATA%\StardewValley\Saves`)
- **Source Initial File Hashes:**
  - `TXrk_450560341_old`: `90c51fb82a76bf168cd83b14a8ecb85eba3b1edc27b796ff579006934023204e`
  - `TXrk_450560341`: `e89b4f18c7e587f416e870de0295327e87d7a5f32a8c7f37bb31195faba7f057`
  - `SaveGameInfo_old`: `9d088905d0bc19ec47558014dc2a436086debdf648d6105fb1c94c74445e1ab3`
  - `SaveGameInfo`: `b05e2f6fb5b2c485c12ca7b9bbdd81d9ca7fdad424dc2bfb622caaa095de4d2f`
- **Source Final File Hashes:**
  - `SaveGameInfo_old`: `9d088905d0bc19ec47558014dc2a436086debdf648d6105fb1c94c74445e1ab3`
  - `TXrk_450560341`: `e89b4f18c7e587f416e870de0295327e87d7a5f32a8c7f37bb31195faba7f057`
  - `TXrk_450560341_old`: `90c51fb82a76bf168cd83b14a8ecb85eba3b1edc27b796ff579006934023204e`
  - `SaveGameInfo`: `b05e2f6fb5b2c485c12ca7b9bbdd81d9ca7fdad424dc2bfb622caaa095de4d2f`
- **MANUALLY PROVIDED SOURCE COPY MODIFIED: NO**

---

## 2. SAVE STRUCTURE

- **Current Host Detected:** `Kubilay` (Multiplayer ID: [REDACTED])
- **Target Farmhand Detected:** `elbi` (Multiplayer ID: [REDACTED])
- **Cabin Mappings:** Target farmhand bound to `FarmHouse644329ab-e942-4dad-8f28-9b089b9a30e8`
- **Uninvolved Farmhands:** 0
- **Relevant Mod/Unknown Structures:** Losslessly preserved in DOM
- **`modData` Dictionaries:** Detected & preserved across players and locations

---

## 3. MIGRATION TEST RESULTS

### A → B (Kubilay → Elbi)
**Result:** PASS  
**Unexpected changes:** NONE  
- Root `<player>` updated to Elbi with residence `FarmHouse`.
- Cabin `FarmHouse644329ab-e942-4dad-8f28-9b089b9a30e8` updated to Kubilay with residence `FarmHouse644329ab-e942-4dad-8f28-9b089b9a30e8`.
- House upgrade levels accurately exchanged.

### B → A (Elbi → Kubilay)
**Result:** PASS  
**Unexpected changes:** NONE  

### Roundtrip (A → B → A)
**Result:** PASS  
- Kubilay full farmer state bit-for-bit identical to baseline.

### Repeated Roundtrip (10 Cycles Stress Test)
**Result:** PASS  
- 10 complete consecutive migrations executed with 0 semantic drift.

---

## 4. INVARIANT VERIFICATION SUMMARY

| Target | Status | Detail |
|---|---|---|
| **Kubilay State** | **PASS** | 100% preservation of inventory, tools, skills, quests, mail, appearance, modData |
| **Elbi State** | **PASS** | 100% preservation of inventory, tools, skills, quests, mail, appearance, modData |
| **Other Farmhands** | **PASS** | Uninvolved farmhands untouched (0 bytes modified) |
| **Protected World State** | **PASS** | Zero modification to crops, chests, buildings, season, day, year, weather |
| **SaveGameInfo Sync** | **PASS** | Fully synchronized with active host portrait & metadata |
| **Unknown / Mod Data** | **PASS** | Complete retention of modData dictionaries and unknown XML tags |

---

## 5. STRUCTURED DIFF SUMMARY

During host migration, the ONLY permitted XML paths modified are:
1. `/SaveGame/player` (Swapped farmer entity; `homeLocation` -> `FarmHouse`, upgrade level bound to Farmhouse)
2. `/SaveGame/locations/GameLocation[Farm]/buildings/Building/indoors[FarmHouse644329ab-e942-4dad-8f28-9b089b9a30e8]/farmhand` (Swapped farmer entity; `homeLocation` -> `FarmHouse644329ab-e942-4dad-8f28-9b089b9a30e8`, upgrade level bound to Cabin)
3. `/Farmer` in `SaveGameInfo` (Children updated to mirror new root `<player>`)

All other nodes in the entire save document have **zero delta**.

---

## 6. RISKS & UNCERTAINTIES

1. **Host-Bound Steam Achievements:** Steam achievements triggered exclusively by the host (e.g. shipping goals, museum completion) will now trigger for whoever is hosting at that time.
2. **First-Party Cutscene Flags:** Certain single-player cutscenes bound to Farmhouse entry may re-evaluate if not previously flagged in the player's event seen list.
3. **Steam Cloud Collision:** When transitioning to live testing in a future phase, Steam Cloud must be monitored to ensure it does not overwrite the locally swapped save before gameplay starts.

---

## 7. RUNTIME READINESS CONCLUSION

**READY FOR CONTROLLED RUNTIME TEST USING A DISPOSABLE COPY**

*(Note: Production activation remains gated until approved by the user.)*
