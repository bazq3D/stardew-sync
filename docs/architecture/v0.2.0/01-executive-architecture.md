# Stardew Sync v0.2.0 — Executive Architecture & System Map

**Document Version:** 1.0.0  
**Author:** bazq  
**Project:** `stardew-sync` ([https://github.com/bazq3D/stardew-sync](https://github.com/bazq3D/stardew-sync))  
**Target Milestone:** v0.2.0  
**Status:** ARCHITECTURAL SPECIFICATION & TECHNICAL BLUEPRINT

---

## 1. System Vision & Problem Statement

Stardew Valley multiplayer co-op is fundamentally **host-authoritative**. The world progression, farm buildings, weather, date, shared wallet, and mine elevator floors live exclusively inside the host computer's save files (`%APPDATA%\StardewValley\Saves`). Farmhands own only their assigned cabin and character profile within that save.

When two friends or partners (e.g. on separate Windows PCs) want to alternate who hosts the farm:
1. They must manually copy files across USB drives or messaging apps.
2. They easily desynchronize or overwrite newer farm progress with older copies.
3. They cannot easily swap who plays as the primary farmhouse character without editing complex raw XML.
4. Microsoft Store / Xbox PC players suffer from opaque Windows Game Saves (WGS) cloud restores overwriting external file swaps.

**Stardew Sync v0.2.0** evolves the application from a read-only save inspector into a **safe, local-first Windows companion** that automates save discovery, monitoring, snapshotting, character inspection, device pairing, and encrypted peer-to-peer synchronization without centralized save storage or game corruption hazards.

```
┌────────────────────────────────────────────────────────────────────────┐
│                      Stardew Sync Client (PC A)                        │
│                                                                        │
│  ┌───────────────────────┐             ┌────────────────────────────┐  │
│  │   React + TypeScript  │◄──IPC─────►│    Tauri 2 Core (Rust)     │  │
│  │      (Frontend)       │             │         (Native Engine)    │  │
│  └───────────────────────┘             └──────┬──────────────┬──────┘  │
│                                               │              │         │
│          ┌────────────────────────────────────┘              │         │
│          ▼                                                   ▼         │
│   ┌──────────────┐   ┌───────────────┐   ┌──────────────┐ ┌─────────┐  │
│   │ Save Monitor │   │ Sync Engine   │   │ P2P Network  │ │ SysTray │  │
│   │ (notify +    │   │ (Generations, │   │ (Transport,  │ │ & Auto  │  │
│   │ Settle Det.) │   │  Snapshots)   │   │  Crypto)     │ │ Startup │  │
│   └──────┬───────┘   └───────┬───────┘   └──────┬───────┘ └─────────┘  │
│          │                   │                  │                      │
└──────────┼───────────────────┼──────────────────┼──────────────────────┘
           │                   │                  │
           ▼                   ▼                  │ End-to-End
     Stardew Saves        Versioned Snapshots     │ Encrypted
     (%APPDATA%)          (%LOCALAPPDATA%)        │ Direct Transport
                                                  │ (LAN / Relay)
                                                  ▼
                                       ┌───────────────────────┐
                                       │   Stardew Sync (PC B) │
                                       │   (Trusted Paired PC) │
                                       └───────────────────────┘
```

---

## 2. Reusable v0.1.3 Components vs. Required New Infrastructure

Our audit of the current v0.1.3 codebase reveals substantial, high-quality production foundation that will be reused directly without rewrites:

### Reusable v0.1.3 Components:
1. **XML Parser & Schema Restorer (`core/save/parser.rs`):**
   - Lossless DOM parsing with `xmltree`.
   - Polymorphic `.NET` attribute restoration (`xsi:type`, `xsi:nil`, namespaces).
   - UTF-8 BOM injection matching native `.NET` `StreamWriter`.
2. **Deterministic Fingerprinting (`core/save/fingerprint.rs`):**
   - Full farmer fingerprinting and migration-stable fingerprinting.
   - Zero-diff validation for non-swapped XML world nodes.
3. **Save Validation Engine (`core/save/validator.rs`):**
   - Validates root tags, unique player IDs, XML well-formedness, and cabin references.
4. **Offline Host Migration Core (`core/save/host_migrator.rs`):**
   - Tested functional DOM transformation swapping `<player>` and `<farmhand>`.
5. **Transactional Staging & Crash Recovery (`core/transaction/`):**
   - `StagingArea` (isolated temporary unpacking directory).
   - `SafeReplacer` (pre-flight checks, backup snapshot, transactional atomic rename).
   - `RecoveryManager` (detects interrupted operations and recovers baseline).
6. **Process & Settle Detection (`core/process/monitor.rs`):**
   - `SystemProcessChecker` for `Stardew Valley.exe` and `StardewModdingAPI.exe`.
   - `SaveSettleDetector` (watches directory size and timestamps to confirm the game has finished writing).
7. **Updater Security & Reservation Machine (`commands/mod.rs`):**
   - CSPRNG UUID v4 token-owned update reservations.
   - Atomic state transitions (`STATE_IDLE`, `STATE_SAVE_OPERATION`, `STATE_UPDATE_RESERVED`).
   - Decoupled app updates independent of game status.
8. **Internationalization & Presentation System (`src/i18n/`, `src/styles/`):**
   - Full English and Turkish locale dictionary parity, persistent language selector, dark/light theme tokens.

### Required New Infrastructure for v0.2.0:
1. **Windows System Tray & Autostart Integration:**
   - Background daemon mode, close-to-tray window event interception, `tauri-plugin-autostart`.
2. **Background Update Poller:**
   - Periodic non-intrusive update polling respecting `ActiveOperationGuard` and gameplay status.
3. **Filesystem Watcher & Live Revision Tracker:**
   - Background filesystem watcher (`notify` crate) coupled to `SaveSettleDetector`.
4. **Monotonic Generation Engine:**
   - `sync_manifest.json` generation tracker to detect ahead, behind, and divergent (conflict) farm states.
5. **Cryptographic Identity & Pairing Subsystem:**
   - Local Ed25519 identity keypair + X25519 ECDH exchange keypair.
   - Pairing protocol (short codes, mutual authentication, signed certificates).
6. **Encrypted P2P Network Transport:**
   - Local LAN discovery (mDNS / UDP beacon) and encrypted relay fallback for internet connectivity.
7. **Versioned Multi-Snapshot Architecture:**
   - Hierarchical snapshot storage with categorized triggers and retention policies.
8. **Characters Inspection Engine:**
   - Deep read-only extraction of skills, inventories, friendships, and playtime from `<Farmer>` nodes.

---

## 3. Core Architectural Principles

1. **Strict Default-Deny Save Safety:**
   - The application will **never** overwrite or touch a live save while Stardew Valley is running or during file writing.
   - Saves are staged in isolated scratch folders, verified against schemas and hashes, and swapped atomically.
2. **Monotonic Progression Invariant:**
   - A save can only be fast-forwarded if the incoming revision is a verified direct descendant of the local revision. Divergent progression is flagged as a conflict for user resolution; stale saves are rejected.
3. **Zero Plaintext Transmission:**
   - No save file, farm metadata, or character data is ever transmitted unencrypted over the wire.
4. **Local-First Privacy:**
   - No player data, save blobs, or telemetry are uploaded to central game databases. Synchronization is strictly peer-to-peer or via end-to-end encrypted relay.
5. **Fail-Closed Concurrency:**
   - Any failure (network disconnect, disk error, unexpected process start) immediately aborts the active pipeline and restores the pre-operation snapshot.
