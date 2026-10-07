# Runtime Test Checklist — Stardew-Sync Phase 4.6

**Author:** bazq  
**Date:** 2026-10-08  
**Scope:** Controlled Single-Machine Validation of Disposable Save (`TXrkTest_999450560`)  
**Platform:** Microsoft Store / Xbox PC Stardew Valley 1.6  

> [!IMPORTANT]
> **SAFETY REITERATION:**
> The live production farm `TXrk_450560341` remains 100% untouched and protected.
> The upcoming runtime test uses the isolated disposable test save `TXrkTest_999450560` (`TürkTest Farm`).
> Stage 1 prepares the test. DO NOT install or launch until explicit approval is given.

---

## Pre-Installation Sanity Checks

- [ ] Stardew Valley is completely closed (not running in Task Manager).
- [ ] No background sync or migration tool is currently running.
- [ ] Target directory to install is `TXrkTest_999450560` (NOT `TXrk_450560341`).
- [ ] Pre-launch file snapshot recorded by `CloudObserver`.

---

## Test Execution 1: State A (Kubilay = Host)

After user approves test installation of State A:

### 1. Main Menu & Save Loading
- [ ] Launch Stardew Valley via Xbox App / Start Menu.
- [ ] Open **Load Game** menu:
  - [ ] Does `Kubilay - TürkTest Farm` appear as a separate save slot alongside the original farm?
  - [ ] Is the date displayed accurately (Year, Season, Day)?
  - [ ] Click the save to load. Does the game load cleanly without crashing or XML errors?

### 2. Host Character State (Kubilay)
- [ ] Character appearance matches Kubilay exactly (hair, clothes, skin, accessories).
- [ ] Inventory and tools are intact (correct tool upgrade tiers, hotbar order).
- [ ] Gold / Money balance is correct.
- [ ] Skills, professions, and masteries are correct.
- [ ] Social / Friendship tab relationships are intact.
- [ ] Mailbox / Quest log status is correct.

### 3. Farm & World State
- [ ] Farmhouse interior is intact (furniture, wallpaper, chests).
- [ ] Farm exterior is intact (crops watered/grown, paths, debris, trees).
- [ ] Farm buildings are present (barns, coops, sheds).
- [ ] Animals are present and healthy.
- [ ] Cabins are present in their original locations.
- [ ] elbi's cabin interior contains her furnishings.
- [ ] Date, season, year, and weather are correct.

### 4. Gameplay Save Cycle
- [ ] Walk around the farm, perform minor actions (e.g. check a chest, move an item).
- [ ] Walk into the Farmhouse and go to bed to trigger an end-of-day save.
- [ ] Confirm game reaches overnight summary screen and advances to the next morning.
- [ ] Open game menu and select **Exit to Title**.
- [ ] Reload `TürkTest Farm` to confirm the new day loads cleanly.
- [ ] Exit Stardew Valley completely.

---

## Test Execution 2: State B (elbi = Host) — Platform Persistence Rule

> [!CAUTION]
> **CONFIRMED PLATFORM BEHAVIOR (Phase 4.6B):**
> Microsoft Store / Xbox PC Stardew Valley restores a WGS-managed generation over `%APPDATA%\StardewValley\Saves` during startup if the save slot identity is already indexed in `containers.index`.
> Because State A established container `TXrkTest_999450560` in WGS, in-place replacement of that exact slot is rejected by the Xbox runtime.
> **Therefore, State B runtime validation MUST use Strategy F (Fresh Disposable Identity: `TXrkTestB_999450561`) to ensure clean load without WGS rollback.**

*(To be executed under Strategy F after user approval.)*

### 1. Main Menu & Save Loading
- [ ] Open **Load Game** menu.
- [ ] Does `elbi - TürkTest Farm` appear with elbi as the host portrait?
- [ ] Load the save cleanly without errors.

### 2. Host Character State (elbi)
- [ ] Character loaded is elbi (her appearance, clothes, gender, accessories).
- [ ] Inventory and tools belong to elbi.
- [ ] Skills, professions, and friendship tab belong to elbi.
- [ ] Residence is now the primary Farmhouse.

### 3. Farmhand Character State (Kubilay)
- [ ] Kubilay is now assigned to the Cabin indoors.
- [ ] Cabin upgrade tier reflects Kubilay's tier.
- [ ] Kubilay's inventory and data remain intact in the save structure.

### 4. World & Save Cycle
- [ ] Farm world state is intact.
- [ ] Co-op / Multiplayer options menu functions normally.
- [ ] Sleeping saves successfully.
- [ ] Reloading succeeds.
- [ ] Exit Stardew Valley completely.

---

## Post-Run Automated Analysis

Immediately after exiting Stardew Valley:
- [ ] Run `CloudObserver::take_snapshot` to compare pre/post filesystem states.
- [ ] Run `cargo run --bin analyze_runtime_test -- <MANIFEST_PATH> <TEST_SAVE_DIR>`.
- [ ] Verify `PostRuntimeAnalyzer` output:
  - [ ] XML parsed cleanly.
  - [ ] Root host matches expected host.
  - [ ] No missing or duplicate players.
  - [ ] Cabin bindings preserved.
  - [ ] Normal gameplay progression recognized.
  - [ ] Destructive changes count is `0`.
  - [ ] Overall status: `HEALTHY`.
