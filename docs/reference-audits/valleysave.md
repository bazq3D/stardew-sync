# Reference Architecture Audit: ValleySave (`hirieo/valleysave-app`)

**Auditor:** bazq  
**Date:** October 2026  
**License Compliance Notice:**  
*This document contains high-level architectural analysis and observations only. No source code has been copied, transplanted, or reused from ValleySave (PolyForm Noncommercial License 1.0.0).*

---

## 1. Executive Summary

ValleySave is an open-source cross-platform Flutter application designed to synchronize Stardew Valley save files across Windows, macOS, Linux, and Android via Google Drive.

Our audit examined whether ValleySave has existing solutions for Microsoft Store / Xbox Game Pass PC save synchronization, Xbox Live Connected Storage (WGS), cloud-save race conditions, and host migration.

### Key Finding:
**ValleySave operates strictly on standard single-layer filesystem paths (`%APPDATA%\StardewValley\Saves` on Windows) and has zero awareness or handling of Xbox Game Pass Connected Storage (`SystemAppData\wgs`).**

If a user were to run ValleySave against an Xbox Game Pass Stardew Valley installation, ValleySave would encounter the identical startup rollback issue observed during our Phase 4.6 State B runtime test: Xbox WGS restoring its cached container over the modified `%APPDATA%` slot.

---

## 2. Comparative Matrix

| Architectural Dimension | ValleySave Behavior | `stardew-sync` (Current Baseline) | Recommended `stardew-sync` Architecture |
| :--- | :--- | :--- | :--- |
| **Windows Target Directory** | Hardcoded `%APPDATA%\StardewValley\Saves` (`SaveService.dart`). | Configurable discovery resolving `%APPDATA%\StardewValley\Saves`. | Two-layer platform adapter resolving both `%APPDATA%` and package detection. |
| **Xbox Game Pass Awareness** | Treats `C:\XboxGames\...\Stardew Valley.exe` only as a launch executable target (`GameLaunchService.dart`). | Discovered live WGS container `85AD22AC...` mapping to `TXrkTest_999450560`. | Explicit `SavePlatform::MicrosoftStoreXbox` classification with two-layer persistence semantics. |
| **WGS / Connected Storage** | **None.** No awareness of `SystemAppData\wgs` or `containers.index`. | Read-only inspection of `containers.index` and blob GUIDs. | **Rejects direct WGS manipulation.** Coordinates around platform lifecycle. |
| **Cloud-Save Race Strategy** | None. Relies entirely on manual sync buttons or simple folder polling. | `ProcessMonitor` + `SaveSettleDetector` + `CloudObserver`. | Pre-launch baseline verification + generation mismatch assertion before menu render. |
| **Game Shutdown Coordination** | No process monitoring or exit synchronization. | `ProcessMonitor` actively tracks game PID and detects shutdown. | Triggers sync only after game exit and save settlement. |
| **External Overwrite Detection** | None. Ignores external rollback events. | `CloudObserver` snapshots pre-launch and post-exit states. | `CloudObserver::verify_generation_integrity` detects and blocks silent WGS rollbacks. |
| **Transactional Replacement** | Sibling staging (`.vs_tmp_<uuid>`), zip backup, double rename (`.vs_rollback_<uuid>`). | Sibling staging (`.tmp_*`), backup manifest, transactional directory swap, rollback on panic. | Retain sibling transactional staging + post-commit hash verification. |
| **Host Migration Logic** | Swaps `<player>` and `<farmhands>`, updates `homeLocation`, `slotCanHost`, `farmhandReference`, interior swap, building tile relocation. | Structured XML transformation, semantic fingerprinting, world-state preservation checks, namespace/BOM preservation. | Fully decoupled, platform-agnostic migration engine (`HostMigrator`). |
| **Save Slot Identity on Migration** | In-place reuse: preserves existing `folderName` and `uniqueIDForThisGame` (to maintain stable Google Drive folder IDs). | Tested in-place reuse in State B (which exposed WGS rollback). | **Strategy F (Fresh Disposable Identity):** Generate fresh game ID and folder name for test migrations to bypass stale WGS containers. |

---

## 3. Detailed Component Audit

### 3.1 Platform Path Resolution & MS Store Support
In ValleySave's `SaveService.dart`, Windows path discovery is implemented as:
- Read environment variable `%APPDATA%`.
- Append `StardewValley\Saves`.

In `GameLaunchService.dart`, ValleySave maintains a list of known executable paths:
- Steam (Program Files, x86)
- GOG Galaxy
- `C:\XboxGames\Stardew Valley\Content\Stardew Valley.exe`

While it detects the Xbox installation folder to launch the `.exe`, it directs all save reads and writes to `%APPDATA%\StardewValley\Saves`. ValleySave has no logic for reading or updating `Packages\ConcernedApe.StardewValleyPC_...\SystemAppData\wgs`.

### 3.2 Cloud Overwrite & Race Condition Handling
ValleySave includes a `LocalSaveWatcher.dart` class that wraps `Directory.watch(recursive: true)` with an 800ms debounce timer.
- **Purpose:** Used strictly to trigger a UI re-render when files on disk change.
- **Race Protection:** None. If an external service (such as Xbox Gaming Services or OneDrive) replaces save files on launch or while the app is running, ValleySave silently reloads whatever is on disk.
- **Delayed Writes / Process Awareness:** ValleySave does not wait for the game process to exit. It allows the user to download or restore a save at any time, even if the game is running.

### 3.3 Transactional Replacement Architecture
ValleySave's `SaveReplaceService.dart` establishes good filesystem hygiene:
1. **Sibling Temp Directory:** Stages files in `<savesDir>/.vs_tmp_<uuid>/<folderName>` rather than `%TEMP%`, avoiding cross-filesystem / cross-volume `rename()` failures (specifically noted as fixing an Android bug).
2. **Pre-Validation:** Checks that minimum files exist and XML is parsable.
3. **Automatic Backup:** Creates a ZIP backup in a designated backups folder before modifying the target.
4. **Double Rename Swap:**
   - Destination -> `.vs_rollback_<uuid>`
   - Staging -> Destination
   - If the second rename fails, immediately renames rollback back to destination.
5. **Cleanup:** Removes temporary directories.

*Lesson for stardew-sync:* Our `SafeReplacer` and `StagingArea` already implement this exact sibling-directory transactional pattern. However, the ValleySave audit confirms that **transactional filesystem replacement is only effective on platforms where the filesystem is authoritative.** On Microsoft Store / Xbox PC, the filesystem is not the sole authority.

### 3.4 Host Migration Architecture
ValleySave includes a local host swap service (`HostSwapService.dart`).
- **Core Transformation:** It moves the host `<player>` into a `<Farmer>` farmhand slot and promotes the target `<Farmer>` into `<player>`.
- **Relocation Planner:** If the farmhouse or cabin footprints collide with existing buildings or obstacles, it computes free tiles and moves objects.
- **Preservation of Game Attributes:** It explicitly preserves `seasonForSaveGame`, `dayOfMonthForSaveGame`, `yearForSaveGame`, and `gameVersion` in `SaveGameInfo`.
- **Identity Decision:** ValleySave chose to keep `uniqueIDForThisGame` unchanged during a swap so that the Google Drive remote folder ID would not change.
  - *Direct Relevance to our State B Problem:* Keeping `uniqueIDForThisGame` and the folder name identical is the very reason why Xbox WGS restored State A over State B in our experiment! Because the folder name `TXrkTest_999450560` was already registered in `containers.index`, WGS recognized the slot identity and enforced its cached generation.

---

## 4. Key Architectural Takeaways for `stardew-sync`

1. **ValleySave Does Not Solve the Xbox Problem:**
   ValleySave is designed primarily for Steam/GOG/Mobile and ignores Connected Storage. We cannot look to ValleySave for WGS synchronization patterns.

2. **In-Place Swap is Dangerous on Xbox PC:**
   On Xbox Game Pass PC, reusing the same slot name for an externally modified save triggers WGS container restoration. `stardew-sync` must avoid in-place replacement of existing WGS-registered slots unless coordinated after process exit or staged under a fresh identity.

3. **Detection of External Rollbacks is Essential:**
   Neither ValleySave nor Stardew Valley itself warns the user if Xbox WGS silently rolls back a save. `stardew-sync`'s `CloudObserver` fills this crucial safety gap by detecting generation rollbacks and halting with clear error diagnostics.
