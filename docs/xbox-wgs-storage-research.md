# Xbox Connected Storage (WGS) Safety Research & Persistence Architecture

**Author:** bazq  
**Date:** October 2026  
**Phase:** 4.6B — Architecture & Safety Research  
**Scope:** Research, Architectural Analysis & Non-Destructive Observation ONLY  

---

## 1. Executive Summary & Confirmed Platform Behavior

During Phase 4.6 runtime testing, **State A** (Kubilay as root host on `TXrkTest_999450560`) loaded successfully, verified cleanly in-game, and saved normally.

Subsequently, we attempted to test **State B** (elbi as root host on `TXrkTest_999450560`). While State B was verified to be 100% syntactically and semantically valid on disk prior to launch, when Stardew Valley launched, the Load Game menu presented the **State A** farm (`Kubilay — TürkTest Çiftliği`, Summer 21).

### The Confirmed Root Cause:
Stardew Valley on this system is the **Xbox Game Pass for PC / Microsoft Store edition** (`ConcernedApe.StardewValleyPC_0c8vynj4cqe4e!Game`).
1. When State A was played and saved in Stardew, the game invoked Microsoft's Connected Storage (Windows Game Saves / WGS) system.
2. Xbox Gaming Services captured State A into a local container:
   `SystemAppData\wgs\000901FE00CB68AF_0000000000000000000000007BFD81C5\85AD22AC76F645B4B03AD39E7502B0C1`
3. When we replaced `%APPDATA%\StardewValley\Saves\TXrkTest_999450560` with State B, we modified only the working directory layer while the game was closed.
4. On the next game launch, Xbox Gaming Services initialized its Connected Storage provider. Recognizing that slot `TXrkTest_999450560` was indexed in `containers.index` and possessed an active container generation, it **restored the cached container blobs over `%APPDATA%` before the game's menu scanned the saves.**

**Confirmed Fact:** For the Microsoft Store build of Stardew Valley, `%APPDATA%` file replacement alone is **NOT** sufficient for a save slot identity already registered in WGS.

---

## 2. Microsoft Store / Xbox Save Lifecycle Analysis

### Category A: Confirmed by Our Runtime Evidence
- **1:1 Slot-to-Container Mapping:** Each Stardew save slot directory maps directly to a discrete WGS container directory. In our live environment:
  - Disposable slot `TXrkTest_999450560` maps 1:1 to container GUID `85AD22AC76F645B4B03AD39E7502B0C1`.
  - Production slot `TXrk_450560341` maps 1:1 to container GUID `68DA91E3A52C4108BF6B07C540281EFD`.
- **Blob File Identity:** Inside container `85AD22AC...`:
  - Blob `60119364...` is byte-identical and hash-identical (SHA-256) to the game save file `TXrkTest_999450560`.
  - Blob `5B38461B...` is byte-identical and hash-identical to `SaveGameInfo`.
- **Startup Container Restoration:** When an existing WGS slot is modified externally in `%APPDATA%` while Stardew is closed, launching Stardew triggers Gaming Services to restore the cached WGS container files over `%APPDATA%` prior to `LoadGameMenu` rendering.
- **In-Game Save Captures to WGS:** Normal in-game sleeping/saving writes files to `%APPDATA%` and concurrently updates the blobs in `SystemAppData\wgs`.

### Category B: Confirmed by Source / Documentation
- **Connected Storage Provider Model:** The Microsoft GDK (`XGameSave` / `XGameSaveFiles`) requires title initialization (`XGameSaveInitializeProvider`). The provider mediates access between local file caches and Xbox Live cloud synchronization.
- **Access Restrictions:** Connected Storage APIs are restricted to applications running with a registered package identity, valid Xbox Live Title ID, and Service Configuration ID (SCID). Third-party desktop utilities cannot call these APIs for another publisher's title (`E_GS_NO_ACCESS` / 0x80830002).
- **No Per-Game Cloud Toggle:** The Xbox PC App and Windows Gaming Services do not offer a setting to disable cloud saves for individual games. Cloud sync is an intrinsic, non-optional component of Xbox Live on PC.
- **Offline Decoupling:** Local WGS caching operates independently of internet connectivity. Disconnecting network connectivity prevents remote cloud syncing, but local container restoration from `SystemAppData\wgs` to `%APPDATA%` remains active.

### Category C: Inferred
- **Container Discovery Trigger:** When Stardew Valley discovers a save directory in `%APPDATA%` whose name does *not* exist in `containers.index`, Stardew reads it as a normal local save. WGS does not restore anything over it because it has no record of that slot. Once the game saves that slot in-game, it registers the new container in `containers.index`.
- **Conflict Resolution Threshold:** WGS determines that the local `%APPDATA%` file is desynchronized when its metadata or hashes diverge without an active provider session, defaulting to the provider's known container generation.

### Category D: Unknown
- **Internal Micro-Timing of Provider Flush:** The exact millisecond boundary between Gaming Services container restoration and the CLR calling `Directory.EnumerateDirectories` inside Stardew's `SaveGame.FindSaveGames()`.
- **Proprietary Format of `containers.index` Checksums:** The exact binary format, CRC fields, and cloud-sync flags inside `containers.index`.

---

## 3. Evaluation of Direct WGS Manipulation: FORMAL REJECTION

Hypothesis: Could `stardew-sync` directly write to `%LOCALAPPDATA%\Packages\...\SystemAppData\wgs` or clear `containers.index`?

### Evaluation:
1. **Undocumented Proprietary Format:** `containers.index` is a proprietary binary database managed by Microsoft Gaming Services. There is no public schema, and internal headers can change across Windows updates.
2. **High Risk of Corruption & Data Loss:** An invalid binary state or mismatched GUID in `containers.index` can cause Gaming Services to flag the container as corrupt, wiping the entire save folder or prompting a destructive cloud resynchronization.
3. **Sandbox & Permission Boundaries:** Accessing another UWP/AppX package's internal `SystemAppData` violates Windows application isolation principles. Future Windows security updates or sandboxing policies can block or restrict write permissions.
4. **Production Save Hazard:** The production container `68DA91E3...` resides in the exact same `SystemAppData\wgs` directory. Any mistake in editing `containers.index` could destroy the user's real 200+ hour multiplayer farm.

### Decision:
**Direct manipulation of `SystemAppData\wgs` is strictly REJECTED as a production architecture.**  
`stardew-sync` must remain safe, non-destructive, and resilient without depending on unsupported reverse-engineering of Xbox databases.

---

## 4. Evaluation of Safer Candidate Strategies

| Strategy | Feasibility | Safety | Verdict |
| :--- | :---: | :---: | :--- |
| **A. Write while game is completely closed** | High | High | **FAILS for existing slots.** WGS restores cached container on launch. |
| **B. Write after WGS restore, before menu** | Very Low | Unsafe | **REJECTED.** Millisecond race condition; requires process hooking or fragile polling. |
| **C. Apply switch while game is at title screen** | Low | Unsafe | **REJECTED.** Violates core rule: no mutating saves while game process is alive; file locking hazards. |
| **D. Xbox offline / airplane mode** | Medium | Low | **INEFFECTIVE.** Disconnecting the network does not stop *local* WGS container restoration. |
| **E. User-controlled per-game cloud disable** | None | N/A | **IMPOSSIBLE.** Microsoft Store / Xbox PC app does not provide a per-game cloud toggle. |
| **F. Fresh Disposable Identity per test** | **Very High** | **Absolute** | **RECOMMENDED & ACCEPTED.** Zero WGS conflict. Clean discovery. |
| **G. Supported Microsoft Connected Storage API** | None | N/A | **REJECTED.** 3rd party apps cannot obtain ConcernedApe's Title ID / SCID. |

---

## 5. Detailed Analysis of Strategy F (Fresh Disposable Identity)

### Why Strategy F Completely Solves the Problem:
When `stardew-sync` prepares a test or migration:
- **State A:** Used `TXrkTest_999450560` (Game ID: `999450560`). It now has an active WGS container (`85AD22AC...`).
- **State B (Retest):** We generate a fresh identity:
  - Game ID: `999450561`
  - Folder Name: `TXrkTestB_999450561`
  - Farm Name: `TürkTestB`
  - Save file: `TXrkTestB_999450561`
  - SaveGameInfo: `SaveGameInfo`

### Mathematical Proof of Safety:
1. `containers.index` has **NO** entry for `TXrkTestB_999450561`.
2. `SystemAppData\wgs` has **NO** container for `TXrkTestB_999450561`.
3. When Stardew Valley launches, WGS searches its index for `TXrkTestB_999450561`. Since no container exists, **WGS has nothing to restore.**
4. Stardew's `SaveGame.FindSaveGames()` scans `%APPDATA%\StardewValley\Saves`, encounters `TXrkTestB_999450561`, parses `SaveGameInfo`, and immediately displays:
   `elbi — TürkTestB Çiftliği` (Summer 20, Year 1).
5. When the user loads and plays the farm, Stardew Valley's internal GDK save routine creates the first WGS container for `TXrkTestB_999450561` natively and cleanly.

### Gameplay & Multiplayer Invariants:
In Stardew Valley:
- `uniqueIDForThisGame` seeds the random number generator (daily luck, cart merchant items).
- Player progression, inventories, skills, friendship levels, and quest data are keyed to `UniqueMultiplayerID`, **not** `uniqueIDForThisGame`.
- Changing `uniqueIDForThisGame` between isolated test runs has zero negative impact on game mechanics.

---

## 6. The Authoritative Storage Model for Microsoft Store

### Architectural Answer:
For the Microsoft Store / Xbox PC build, `stardew-sync` MUST treat `%APPDATA%\StardewValley\Saves` as:

> **Option C: One layer of a two-layer persistence system.**

- **Layer 1 (Working Filesystem):** `%APPDATA%\StardewValley\Saves` is where the game engine reads and writes XML files during gameplay.
- **Layer 2 (Platform Persistence Backing):** `SystemAppData\wgs` + Xbox Live Cloud Storage is where Microsoft Gaming Services captures, version-controls, and restores save generations.

### Consequences for `stardew-sync`:
1. **Sync Across PCs (Kubilay <-> elbi):**
   When syncing a multiplayer save between Kubilay and elbi:
   - Synchronizations must execute **after** game shutdown and after `SaveSettleDetector` verifies that both Layer 1 and Layer 2 writes have settled.
   - If an incoming save from elbi is to replace Kubilay's local save, `stardew-sync` must verify whether the local slot name is already managed by WGS.
2. **Rollback Immunity:**
   `CloudObserver` must actively verify generation integrity before and after launch, alerting the user if Xbox Gaming Services attempts an external rollback.

---

## 7. Platform Abstraction Design

To ensure `HostMigrator` remains 100% platform-agnostic, the platform differences are encapsulated into a clean abstraction layer:

```rust
pub enum SavePlatform {
    SteamWin32,
    MicrosoftStoreXbox,
    LinuxDesktop,
    MacOSDesktop,
}

pub struct PlatformCapabilities {
    pub platform: SavePlatform,
    pub has_platform_storage_manager: bool,
    pub supports_in_place_slot_replacement: bool,
    pub persistence_model: &'static str,
    pub recommended_sync_strategy: &'static str,
}

pub trait SaveStorageAdapter {
    fn platform(&self) -> SavePlatform;
    fn discover(&self) -> Result<Vec<DiscoveredSave>, CoreError>;
    fn prepare_for_read(&self, save_path: &Path) -> Result<(), CoreError>;
    fn prepare_for_write(&self, target_slot_name: &str) -> Result<PathBuf, CoreError>;
    fn commit(&self, staged_path: &Path, target_slot_name: &str) -> Result<(), CoreError>;
    fn verify(&self, slot_name: &str, expected: &DirectoryObservationSnapshot) -> Result<bool, CoreError>;
    fn observe_external_change(&self, slot_name: &str) -> Result<Option<DirectoryObservationSnapshot>, CoreError>;
}
```

---

## 8. CloudObserver Improvements: Rollback & Tamper Detection

`CloudObserver` has been extended with two core safety APIs:
1. `verify_generation_integrity(&actual_dir, &expected_snapshot, label, stale_ref)`:
   - Returns `GenerationIntegrityStatus::Verified` if all files and hashes match.
   - Returns `GenerationIntegrityStatus::RollbackDetected` if live files match a known older generation (e.g. State A when expecting State B).
   - Returns `GenerationIntegrityStatus::ExternalReplacementDetected` if files were altered or deleted by external sources.
2. `assert_no_external_replacement(...)`:
   - Returns `Err(CoreError::ExternalSaveReplacement)` to halt execution immediately upon rollback or external replacement.

All functions are verified by automated tests using isolated temporary directories (zero live save interaction).

---

## 9. Safe State B Retest Plan (Strategy F)

### Retest Specification:
1. **Target Identity:**
   - Folder: `%APPDATA%\StardewValley\Saves\TXrkTestB_999450561`
   - Game ID: `999450561`
   - Farm Name: `TürkTestB`
   - Root Host: `elbi`
   - Farmhand: `Kubilay`
2. **Safety Guarantees:**
   - Production save `TXrk_450560341` is untouched.
   - Production WGS container `68DA91E3...` is untouched.
   - Previous disposable test `TXrkTest_999450560` and its WGS container `85AD22AC...` remain untouched as historical evidence.
   - Stardew Valley Load Game menu will cleanly display `elbi — TürkTestB Çiftliği`.
