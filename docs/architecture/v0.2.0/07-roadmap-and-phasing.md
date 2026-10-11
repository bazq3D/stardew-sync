# Stardew Sync v0.2.0 — Phased Roadmap, Threat Model & Release Scope

**Document Version:** 1.0.0  
**Author:** bazq  
**Target Milestone:** v0.2.0  
**Status:** TECHNICAL ARCHITECTURE SPECIFICATION & RELEASE PLAN

---

## 1. Feature Feasibility Matrix

| Requested Feature | Technical Feasibility | Risk Level | Target Release Scope | Justification & Architectural Boundary |
|---|---|---|---|---|
| **1. Windows Startup & System Tray** | **High** | Low | **v0.2.0 Core** | Standard Tauri 2 plugins (`tauri-plugin-autostart`), non-privileged HKCU registry, clean process lifecycle. |
| **2. Automatic Update Checker** | **High** | Low | **v0.2.0 Core** | Background timer with process-detection gates, deduplication, and notification API. No silent installs. |
| **3. Save Settle & Revision Monitor** | **High** | Medium | **v0.2.0 Core** | `notify` crate + existing `SaveSettleDetector` + Monotonic Generation manifests (`sync_manifest.json`). Zero auto-overwrites. |
| **4. Two-PC Pairing & LAN/Relay P2P Sync** | **Medium-High** | Medium-High | **v0.2.0 Core** | Ed25519 identity, X25519 key exchange, 6-char pairing codes, LAN mDNS + E2EE WebSocket Relay fallback. |
| **5. Versioned Multi-Snapshot History** | **High** | Low | **v0.2.0 Core** | Hierarchical snapshot timeline in `%LOCALAPPDATA%`, `snapshot_manifest.json`, retention pruning, guarded restore. |
| **6. My Characters Page (Read-Only)** | **High** | Low | **v0.2.0 Core** | Read-only XML extraction of skills, gear, friendships, and playtime from `<player>` and cabin `<farmhand>` nodes. |
| **7. Character Transfer Feasibility Study** | **High** (Research) | N/A (Doc only) | **v0.2.0 Core** | Complete research documentation and sanitization strategy delivered; production XML modification deferred. |
| **8. Safe Host Rotation (Two-PC Integration)** | **Medium** | High | **v0.2.0 Experimental** | Node swap engine exists (`HostMigrator`). Wire into UI with multi-step validation; requires explicit user approval and disposable saves. |
| **9. Production Cross-Farm Character Writing** | **Low-Medium** | High | **Deferred (v0.3.0)** | High risk of world-flag corruption (`mailReceived`, marriage, horse GUIDs). Requires dedicated sanitized profile engine. |

---

## 2. Release Scope vs. Deferred Features

### Target Scope for v0.2.0 Release:
1. **Windows System Tray & Autostart:** Minimize to tray on close, startup with Windows (opt-in), clean exit.
2. **Background Update Checker:** Non-intrusive polling, gameplay-aware suppression, notification deduplication.
3. **Save Monitoring & Generation Tracker:** `notify` watcher, settle detection, `sync_manifest.json`, conflict identification.
4. **Device Pairing & P2P Sync Transport:** Ed25519/X25519 identity, pairing wizard, direct LAN transfer + E2EE relay.
5. **Versioned Snapshot History:** Timeline of snapshots, categorized types, automatic retention pruning, guarded restore.
6. **My Characters Inspector (Read-Only):** Full character page displaying stats, skills, inventory summary, relationships.
7. **Comprehensive Host Rotation Wizard:** Multi-step guided rotation using `HostMigrator` with Strategy F for Xbox.

### Deferred to Future Releases (v0.2.x / v0.3.0):
- **Live In-Memory Game Hooking:** Directly injecting saves into a running game process is strictly rejected.
- **Cross-Farm Character Migration Tool:** Transferring characters between unrelated farms is deferred until the Sanitized Profile Transformation is prototyped and validated on synthetic saves.
- **Direct WGS Binary Modification:** Manipulating `SystemAppData\wgs` internal binary blobs directly is permanently rejected due to Xbox cloud corruption risks.

---

## 3. Security & Save-Safety Threat Model

```
┌────────────────────────────────────────────────────────────────────────┐
│                        THREAT MODEL & MITIGATIONS                      │
├────────────────────────────────────────────────────────────────────────┤
│ THREAT 1: Stale Save Overwriting Newer Progress (ABA Race)             │
│ Mitigation: Monotonic Generation Chain (generation_number + hashes).   │
│ Stale generation numbers are rejected immediately at the protocol      │
│ layer. Divergent generations trigger hard conflict locks.              │
├────────────────────────────────────────────────────────────────────────┤
│ THREAT 2: Overwriting Save While Stardew Valley is Saving              │
│ Mitigation: Dual-Gate Protection:                                      │
│ 1. SystemProcessChecker verifies Stardew Valley.exe is dormant.         │
│ 2. SaveSettleDetector requires 4.0 seconds of zero file modifications. │
├────────────────────────────────────────────────────────────────────────┤
│ THREAT 3: Corrupted or Malicious Save Archive from Network             │
│ Mitigation: Full Isolated Staging Pipeline:                            │
│ Archive unpacked in temporary scratch folder; SaveValidator verifies   │
│ XML schema, player IDs, and checksums before touching %APPDATA%.       │
├────────────────────────────────────────────────────────────────────────┤
│ THREAT 4: Xbox WGS Cloud Save Reversion                                │
│ Mitigation: Strategy F (Fresh Identity Migration):                     │
│ Generate clean slot and Game ID on host rotation so Xbox Gaming       │
│ Services treats it as a brand-new container.                           │
├────────────────────────────────────────────────────────────────────────┤
│ THREAT 5: Man-in-the-Middle on Sync Transport                          │
│ Mitigation: End-to-End Encryption:                                    │
│ AES-256-GCM authenticated encryption using keys derived via mutual     │
│ X25519 Diffie-Hellman. Relays handle only opaque encrypted blobs.      │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Phased Implementation Sequence

```
Phase 1: Foundation, Tray & Background Daemon
  ├── Add tauri-plugin-autostart & tray icon menu
  ├── Implement window minimize-to-tray & clean exit handler
  └── Implement background periodic update checker with process gating

Phase 2: Characters Page (Read-Only) & Navigation Expansion
  ├── Extend parser.rs to extract skills, inventory, relationships, playtime
  ├── Create My Characters page in React UI (responsive, dark/light theme)
  └── Update navigation sidebar (8-tab layout) with English/Turkish i18n

Phase 3: Versioned Snapshots & Retention Engine
  ├── Refactor backup manager for hierarchical snapshot timelines
  ├── Implement snapshot_manifest.json with categorized types & pinning
  ├── Build automated FIFO retention engine
  └── Create multi-step guarded snapshot restore modal

Phase 4: Save Monitoring & Monotonic Generation Tracker
  ├── Integrate notify crate with SaveSettleDetector for %APPDATA%
  ├── Implement sync_manifest.json monotonic generation computation
  └── Build Sync Activity page with real-time state badges (Synced/Outdated/Conflict)

Phase 5: Two-PC Device Pairing & Encrypted Transport
  ├── Implement local Ed25519/X25519 identity generation & storage
  ├── Build pairing wizard (6-char code, mutual ECDH, pairing certificates)
  ├── Implement LAN discovery (mDNS/UDP) + E2EE WebSocket Relay fallback
  └── Build chunked streaming protocol with SHA-256 verification

Phase 6: Host Rotation Integration & Two-PC Validation Bench
  ├── Wire HostMigrator into UI with side-by-side preview and Strategy F
  ├── Execute two-PC synthetic validation suite on isolated test fixtures
  └── End-to-end integration and final stabilization
```

---

## 5. Testing & Validation Strategy

1. **Unit & Property Tests (Cargo Test):**
   - Generation chain advancement, ancestor verification, and conflict detection.
   - Chunk reassembly and checksum integrity verification.
   - Character parser tolerance across Stardew 1.5, 1.6, and modded XML schemas.
   - Retention engine FIFO pruning while preserving protected snapshots.
2. **Synthetic Concurrency Harness:**
   - Multi-threaded simulations of concurrent save writes, network drops, and delayed packets.
3. **Two-PC Validation Bench:**
   - Testing on two real Windows machines using **synthetic disposable saves** (`TXrkTestB_999450561`).
   - Zero access to live production saves.
