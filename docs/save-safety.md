# Stardew Valley Save Safety & Invariants Specification

## 1. Absolute Priority: Save Safety Invariants

The existing farm owned by Kubilay and Elbi contains real, irreplaceable progress. The sync system must enforce strict invariants where safety takes absolute precedence over speed or automation.

### Invariant 1: Never Mutate the Sole Copy
The application must **never** perform in-place edits on a live save file.
- Before reading or transforming a save, a full local snapshot must be created and verified.
- Transformations and downloads occur inside isolated temporary staging directories.
- If an operation fails, the live save remains completely untouched.

### Invariant 2: Immutable Original Import Snapshot
The very first time the farm is imported into `stardew-sync`:
- A bit-for-bit archive is recorded in `%LOCALAPPDATA%\stardew-sync\backups\`.
- Marked with `is_protected = 1`.
- Excluded from all automatic retention pruning policies.
- Acts as the immutable baseline recovery point.

### Invariant 3: Zero In-Place Overwrites During Game Execution
The application must **never** touch, replace, or stage save files while `Stardew Valley.exe` or `StardewModdingAPI.exe` is running.
- File system locks by the game engine will cause corrupted partial writes.
- The process monitor must verify the game process is terminated and wait for the settle period (3,000 ms) before scanning or modifying saves.

### Invariant 4: No Silent Conflict Resolution
When divergent versions exist ($V_{parent} \to V_{A}$ and $V_{parent} \to V_{B}$):
- The application must **never** use "newest timestamp wins" to delete or overwrite either branch.
- Both branches must be preserved locally and in cloud storage.
- The user must be presented with an explicit conflict resolution dialog with backup checkpoints clearly visible.

### Invariant 5: Complete Transactional Replacement
All local save file replacements (from cloud sync, backup restore, or host migration) must follow the transactional pipeline:
$$\text{Prepare Staging} \to \text{Validate Staging XML} \to \text{Snapshot Live Save} \to \text{Atomic Swap} \to \text{Verify Live Save}$$
If any step fails, the system immediately triggers an automatic rollback to the pre-operation state.

### Invariant 6: Bidirectional Player State Preservation
During host migration ($A \to B$ or $B \to A$):
- **Name, UniqueMultiplayerID, Inventory items, Experience points, Skills, Professions, Friendships, Quests, Mail, and Clothes** must remain 100% intact.
- The application must assert that both players still exist in the output XML with matching item counts and IDs before writing to disk.

---

## 2. Failure Scenarios & Mitigations

| Failure Scenario | Risk | System Mitigation |
| :--- | :--- | :--- |
| **PC Crashes During Save Replace** | Corrupted / half-written save folder. | Changes are staged in a `.tmp` sibling directory. Replacement uses Windows `MoveFileEx` with transactional atomic rename. A rollback recovery manifest is written before swapping so a restarted client completes recovery. |
| **Network Loss Mid-Download** | Broken/partial archive in live save folder. | Archives download to `%TEMP%`. Checksums (SHA-256) and decryption tags are validated *before* any extraction. If verification fails, staging is deleted and live save is untouched. |
| **Stardew Launches Mid-Sync** | Simultaneous write locks and corrupted saves. | Native process monitoring holds an exclusive lock on the sync transaction; if Stardew starts, sync aborts immediately and live save is restored from the safety snapshot. |
| **Malformed XML from Corrupt Mod / Crash** | Game crashes on title screen load. | Strict XML validation (`quick-xml`) ensures balanced tags, valid root elements, and player node integrity before any file is saved or uploaded. |
| **Disk Full During Backup** | Incomplete backup followed by replacement. | Pre-flight disk space check requires at least $3 \times$ save size available. Backup archive integrity is checked before proceeding with any operation. |
| **Antivirus Lock on Save File** | File access denied during atomic swap. | Retry loop with exponential backoff (up to 5 retries over 5 seconds). If file lock persists, operation is aborted cleanly with zero modifications. |

---

## 3. Transactional Replacement Protocol

```
               [ Start Operation: Host Switch / Sync / Restore ]
                                       │
                                       ▼
                     ┌───────────────────────────────────┐
                     │ 1. PRE-FLIGHT CHECKS              │
                     │    - Process check (Game stopped) │
                     │    - Disk space check             │
                     │    - File lock test               │
                     └─────────────────┬─────────────────┘
                                       │ OK
                                       ▼
                     ┌───────────────────────────────────┐
                     │ 2. CREATE SAFETY SNAPSHOT         │
                     │    - Copy live save to backup dir │
                     │    - Calculate SHA-256            │
                     │    - Verify archive readability   │
                     └─────────────────┬─────────────────┘
                                       │ Verified
                                       ▼
                     ┌───────────────────────────────────┐
                     │ 3. STAGING EXECUTION              │
                     │    - Prepare changes in .tmp dir  │
                     │    - Run host migration / unpack  │
                     └─────────────────┬─────────────────┘
                                       │
                                       ▼
                     ┌───────────────────────────────────┐
                     │ 4. STRICT VALIDATION              │
                     │    - Well-formed XML parse        │
                     │    - Check <player> name & ID     │
                     │    - Check <farmhand> name & ID   │
                     │    - Check SaveGameInfo matches   │
                     └─────────────────┬─────────────────┘
                                       │
                         Pass? ────────┴──────── Fail?
                           │                       │
                           ▼                       ▼
            ┌───────────────────────────┐    ┌───────────────────────────┐
            │ 5. ATOMIC SWAP            │    │ ABORT & CLEANUP           │
            │    - Rename live to .old  │    │ - Remove staging .tmp     │
            │    - Rename .tmp to live  │    │ - Report validation error │
            │    - Delete .old          │    │ - Live save untouched     │
            └──────────────┬────────────┘    └───────────────────────────┘
                           │
                           ▼
            ┌───────────────────────────┐
            │ 6. POST-VERIFY            │
            │    - Verify live files on │
            │      disk match expected  │
            └───────────────────────────┘
```

---

## 4. Host Migration Invariant Checks

Before any transformed save file is committed to disk, the validator must run the following assertions against the transformed XML DOM:

```rust
// Pseudocode for Host Migration Invariant Checks
fn validate_migration(
    original_save: &ParsedSave,
    transformed_save: &ParsedSave,
    expected_host_name: &str,
    expected_farmhand_name: &str
) -> Result<(), SafetyError> {
    // 1. Root Player check
    let new_host = &transformed_save.player;
    assert_eq!(new_host.name, expected_host_name, "Host name mismatch");
    assert_eq!(new_host.home_location, "FarmHouse", "Host homeLocation must be FarmHouse");
    
    // 2. Cabin Farmhand check
    let new_farmhand = transformed_save.find_farmhand(expected_farmhand_name)
        .ok_or(SafetyError::MissingFarmhand)?;
    assert_ne!(new_farmhand.home_location, "FarmHouse", "Farmhand homeLocation cannot be FarmHouse");

    // 3. ID and Inventory Preservation
    assert_eq!(new_host.unique_id, original_save.find_player(expected_host_name).unique_id);
    assert_eq!(new_host.inventory.len(), original_save.find_player(expected_host_name).inventory.len());
    assert_eq!(new_farmhand.unique_id, original_save.find_player(expected_farmhand_name).unique_id);
    assert_eq!(new_farmhand.inventory.len(), original_save.find_player(expected_farmhand_name).inventory.len());

    // 4. World continuity
    assert_eq!(transformed_save.year, original_save.year);
    assert_eq!(transformed_save.season, original_save.season);
    assert_eq!(transformed_save.day, original_save.day);

    Ok(())
}
```
