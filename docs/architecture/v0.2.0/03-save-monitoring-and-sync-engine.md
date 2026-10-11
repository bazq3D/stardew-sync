# Stardew Sync v0.2.0 — Save Monitoring & Synchronization Engine

**Document Version:** 1.0.0  
**Author:** bazq  
**Target Milestone:** v0.2.0  
**Status:** TECHNICAL ARCHITECTURE SPECIFICATION

---

## 1. Filesystem Watcher & Save Settle Detection

In Stardew Valley, when a player sleeps in-game, the engine writes multiple files in sequence:
1. Writes temporary buffers.
2. Renames existing save to `SaveGame_old`.
3. Writes new primary `SaveGame`.
4. Writes new companion `SaveGameInfo`.
5. On Microsoft Store / Xbox edition, flushes updates to `SystemAppData\wgs` containers.

Attempting to read or synchronize during this write sequence results in a **partially written, corrupt save**.

```
[Save Write Starts]
       │
       ▼
 [File Created / Modified Events] ──► (notify crate detects filesystem event)
       │
       ▼
 [SaveSettleDetector Engaged]
   - Resets debounce timer to 4.0 seconds on each event
   - Polls directory size and file modified timestamps
   - Checks SystemProcessChecker: is game process active?
       │
       ▼
 [Settle Invariant Satisfied]
   - Zero filesystem events for 4.0 consecutive seconds
   - File size stable across consecutive polls
   - SaveGame and SaveGameInfo exist and are non-empty
       │
       ▼
[Generation Computation & Sync Proposal Triggered]
```

### Invariant Checks:
- `SaveSettleDetector` ensures that no hash computation or network notification occurs until the filesystem has remained completely quiescent for **4.0 seconds**.
- `ActiveOperationGuard` ensures no incoming sync can be applied while a local save is settling.

---

## 2. Monotonic Generation Chain & Conflict Matrix

To eliminate out-of-order writes, accidental rollbacks, and silent overwrites, every state update produces a cryptographically signed **Generation Manifest** (`sync_manifest.json`):

```json
{
  "farm_id": "TXrk_450560341",
  "generation_number": 15,
  "parent_generation_hash": "A7B4...91F",
  "current_generation_hash": "E3B0...855",
  "timestamp": "2026-10-10T14:30:00Z",
  "saved_by_device_id": "7c8e3124-b15c-482a-9f5b-59d436c645bc",
  "in_game_date": "Winter, Day 12 (Year 1)",
  "funds": 48250,
  "play_time_hours": 18.9,
  "files": {
    "TXrk_450560341": "D8F1...4A2",
    "SaveGameInfo": "92C3...7B1"
  },
  "signature": "ED25519_SIGNATURE_STRING"
}
```

### Sync State Resolution Matrix

| Local Generation | Remote Generation | State | System Action | User UX |
|---|---|---|---|---|
| $G_{local} = G_{remote}$ | Hash matches | **SYNCED** | Idle. No transfer needed. | Green "Up to Date" badge. |
| $G_{local} < G_{remote}$ | Local is ancestor of Remote | **OUTDATED** | Prepare pre-sync snapshot; stage incoming revision. | "Update Available from [Device B]" button with date/time. |
| $G_{local} > G_{remote}$ | Remote is ancestor of Local | **AHEAD** | Offer to push update to remote device. | "You have newer progress. Ready to sync." |
| $G_{local}$ and $G_{remote}$ have different hashes for same generation, or diverged ancestors | Different hashes | **CONFLICT (Divergent Fork)** | **STRICT BLOCK.** Zero automatic overwrite. | Amber "Sync Conflict" card. Shows side-by-side comparison (Dates, Funds, Playtime) and lets user choose which farm to keep or fork into a new slot. |
| Remote unreachable | N/A | **UNAVAILABLE** | Idle. Monitor local changes only. | Gray "Peer Offline" indicator. |

---

## 3. Transactional Staging & Application Pipeline

Applying a remote save must be atomic and reversible:

```
[Incoming Save Archive Received]
              │
              ▼
    1. Pre-Flight Safety Gate
       - Is Stardew Valley running? (MUST BE FALSE)
       - Is a local save settling? (MUST BE FALSE)
       - Is ActiveOperationGuard free? (Lock acquired)
              │
              ▼
    2. Immutable Pre-Sync Snapshot
       - Full baseline copy of local save created in %LOCALAPPDATA%
       - Verified with SHA-256 before proceeding
              │
              ▼
    3. Isolated Staging Extraction
       - Unpack archive into scratch/staging/{txn_id}
       - Run SaveValidator (XML well-formedness, player ID, schema)
       - Verify SHA-256 matches remote Generation Manifest
              │
              ▼
    4. Atomic File Replacement (SafeReplacer)
       - Transactional replacement using atomic file replace / moves
       - If any step errors: Rollback instantly from Step 2 snapshot
              │
              ▼
    5. Final Verification & Lock Release
       - Confirm target directory contains valid primary save and SaveGameInfo
       - Release ActiveOperationGuard
```

---

## 4. Microsoft Store / Xbox Connected Storage (WGS) Resilience

- **Strategy F (Fresh Identity Migration):**
  - When performing a host rotation on a Microsoft Store installation, the application creates a fresh slot folder and game ID (e.g. `TXrk_450560342`).
  - Because this slot has no prior record in `containers.index`, Xbox Gaming Services treats it as a brand-new save and will never restore an older cloud generation over it.
- **Settle Observation:**
  - After closing the Xbox edition, Gaming Services flushes container indexes. Stardew Sync waits for this flush to settle before generating sync manifests.
