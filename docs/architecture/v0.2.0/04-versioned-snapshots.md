# Stardew Sync v0.2.0 — Versioned Snapshot Architecture

**Document Version:** 1.0.0  
**Author:** bazq  
**Target Milestone:** v0.2.0  
**Status:** TECHNICAL ARCHITECTURE SPECIFICATION

---

## 1. Directory Structure & Versioned Storage Model

In v0.1.3, snapshots were stored as a single baseline copy per farm in `production-snapshots/`.  
In v0.2.0, snapshots evolve into a **hierarchical versioned timeline** stored in `%LOCALAPPDATA%\StardewSync\snapshots`:

```
%LOCALAPPDATA%\StardewSync\snapshots\
└── TXrk_450560341\
    ├── 20261007_ORIGINAL_IMPORT_0001\
    │   ├── TXrk_450560341
    │   ├── SaveGameInfo
    │   └── snapshot_manifest.json  (is_protected: true)
    ├── 20261010_143000_PRE_SYNC_b7a2e1\
    │   ├── TXrk_450560341
    │   ├── SaveGameInfo
    │   └── snapshot_manifest.json  (is_protected: false)
    ├── 20261010_160000_MANUAL_4c9f18\
    │   ├── TXrk_450560341
    │   ├── SaveGameInfo
    │   └── snapshot_manifest.json  (is_protected: true)
    └── 20261010_180000_PRE_HOST_ROTATION_e81a3d\
        ├── TXrk_450560341
        ├── SaveGameInfo
        └── snapshot_manifest.json  (is_protected: true)
```

---

## 2. Categorized Snapshot Types & Lifecycle

| Snapshot Type | Trigger Condition | Protection Policy | Retention Engine Behavior |
|---|---|---|---|
| **`OriginalImport`** | First time a farm is discovered and indexed | **Permanently Protected** | Never pruned. Acts as immutable ground truth. |
| **`Manual`** | User explicitly clicks "Create Snapshot" in UI | **User Protected** | Never pruned automatically. Can only be deleted by explicit user action. |
| **`PreSync`** | Created immediately before applying an incoming remote sync | Managed | Eligible for automated retention FIFO pruning (e.g. keep last 10). |
| **`PostSync`** | Created immediately after a remote sync settles cleanly | Managed | Eligible for automated retention FIFO pruning. |
| **`PreHostRotation`** | Created before initiating a host role swap | **Protected** | Retained permanently to allow guaranteed rollback. |
| **`Recovery`** | Created if an unexpected anomaly is detected during runtime | **Protected** | Retained permanently for post-mortem analysis. |

---

## 3. Snapshot Metadata Schema (`snapshot_manifest.json`)

Every snapshot directory contains a self-contained, cryptographically signed manifest:

```json
{
  "snapshot_id": "b7a2e190-84c1-4b2a-89f5-19e84b1d5a7b",
  "created_at": "2026-10-10T14:30:00Z",
  "snapshot_type": "PRE_SYNC",
  "farm_name": "Türk",
  "game_id": "450560341",
  "in_game_date": "Winter, Day 12 (Year 1)",
  "money": 48250,
  "play_time_hours": 18.9,
  "play_time_formatted": "18h 54m (18.9 hours)",
  "host_name": "Kubilay",
  "source_device_name": "Kubilay-Desktop",
  "source_device_id": "7c8e3124-b15c-482a-9f5b-59d436c645bc",
  "is_protected": false,
  "files": [
    {
      "file_name": "TXrk_450560341",
      "sha256": "D8F1B94C8A2019...",
      "size_bytes": 1048576
    },
    {
      "file_name": "SaveGameInfo",
      "sha256": "92C37B1E4A0815...",
      "size_bytes": 32768
    }
  ],
  "package_hash": "A7B4...91F"
}
```

---

## 4. Automated Retention Engine

To prevent unbounded disk consumption over months of gameplay:
1. **Configurable Cap:** User specifies maximum automatic snapshots per farm (default: 10, range: 3–50).
2. **Exclusion Filter:** The engine filters out all snapshots with `is_protected: true`.
3. **FIFO Pruning:** When unpinned snapshots exceed the limit, the oldest `PreSync` / `PostSync` snapshots are safely removed.
4. **Zero Silent Loss:** If disk space is low, the app warns the user rather than purging protected snapshots.

---

## 5. Multi-Step Guarded Restoration Workflow

Restoring a snapshot replaces the active farm save. To ensure 100% safety:
1. **Process Gate:** Verify `Stardew Valley.exe` is completely closed.
2. **Pre-Restore Safety Snapshot:** Automatically capture a `PreRestore` snapshot of the current state before touching anything.
3. **Staging & Validation:** Unpack snapshot files into scratch directory; verify SHA-256 hashes against `snapshot_manifest.json` and validate XML integrity.
4. **Side-by-Side Diff Modal:** Display an interactive preview comparing:
   - Current on-disk date vs Snapshot date.
   - Current funds vs Snapshot funds.
   - Current playtime vs Snapshot playtime.
5. **Explicit Confirmation:** User types or clicks confirmation.
6. **Atomic Replacement:** `SafeReplacer` transactionally swaps files into `%APPDATA%\StardewValley\Saves`.
