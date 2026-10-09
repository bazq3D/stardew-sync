# Phase 4.5 — Controlled Real-Save Dry Run Report

**Author:** bazq  
**Project:** `stardew-sync`  
**Execution Type:** Offline Dry Run on Manually Provided Copy  
**Date:** 2026-10-07  

---

## 1. SOURCE SAFETY

- **Explicit Supplied Path:** `C:\Users\bazq3\Desktop\stardew-sync-test\production-snapshots\TXrk_450560341_2026-10-09_180041`
- **Confirmed Outside Live Stardew Save Directory:** YES (Strictly validated outside `%APPDATA%\StardewValley\Saves`)
- **Source Initial File Hashes:**
  - `SaveGameInfo`: `ecbda32ebaa515a1ff5be17690099c401cb70995f6583af1ac3b32a908852299`
  - `SaveGameInfo_old`: `a7e5069e22e021e89fbe0fd229a989b6dea0ca1d89304b86dc75adb4e1eb50a2`
  - `TXrk_450560341`: `c835f8572aee9adae36f3bddab512936c6605339d59096954d052d0d0557dec1`
  - `TXrk_450560341_old`: `81b7a159b14765c3747789728a3b9fe69dfe5b14ad34ee07b3e1faf0c8693937`
- **Source Final File Hashes:**
  - `SaveGameInfo_old`: `a7e5069e22e021e89fbe0fd229a989b6dea0ca1d89304b86dc75adb4e1eb50a2`
  - `TXrk_450560341`: `c835f8572aee9adae36f3bddab512936c6605339d59096954d052d0d0557dec1`
  - `SaveGameInfo`: `ecbda32ebaa515a1ff5be17690099c401cb70995f6583af1ac3b32a908852299`
  - `TXrk_450560341_old`: `81b7a159b14765c3747789728a3b9fe69dfe5b14ad34ee07b3e1faf0c8693937`
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
