# PHASE 5.0 — STARDEW SYNC DESKTOP APPLICATION FOUNDATION REPORT

**Author:** bazq  
**Date:** 2026-10-09  
**Status:** COMPLETE  
**Application:** Stardew Sync (v0.1.0)  
**Target:** Windows x64 (NSIS Installer)

---

## 1. Repository Audit Summary

Prior to changes in Phase 5.0, the repository contained:
- `src-tauri`: A standalone Rust library crate named `stardew_sync_core` containing the save migration engine, parser, validator, and test binaries.
- 28 unit and integration tests across 3 suites (`core_safety_tests`, `host_migration_tests`, `runtime_safety_tests`).
- **Missing components:** No Node.js frontend files (`package.json`, Vite, React, TypeScript), no `tauri.conf.json`, no Tauri 2 plugins, and no application icons.
- **Save Engine Preservation:** All 28 existing tests and migration core modules (`parser`, `validator`, `host_migrator`, `recovery`, `staging`, `guard`) were preserved intact.

---

## 2. Final Application Architecture

The desktop application uses a clean multi-layer separation of concerns:

```
┌────────────────────────────────────────────────────────┐
│                   Stardew Sync UI                      │
│            React 18 + TypeScript + Vite                │
│    (Dashboard, Farms, Backups, Settings, Updates)      │
└───────────────────────────▲────────────────────────────┘
                            │ Tauri IPC (Strictly Typed & Read-Only)
┌───────────────────────────▼────────────────────────────┐
│                  Tauri 2 Backend Bridge                │
│             `src-tauri/src/commands/mod.rs`            │
│   (AppStatus, ProcessStatus, Farms, Snapshots, Update) │
└───────────────────────────▲────────────────────────────┘
                            │ Safe Rust In-Memory Queries
┌───────────────────────────▼────────────────────────────┐
│                 Core Save Engine & Guard               │
│                   `stardew_sync_core`                  │
│       ProductionGuard • Discovery • SHA-256 • XML      │
└────────────────────────────────────────────────────────┘
```

### Safety Boundary Enforcement:
1. **Read-Only Operations:** All Tauri commands exposed to the UI (`get_app_status`, `get_process_status`, `discover_farms`, `get_farm_metadata`, `list_snapshots`, `verify_snapshot_integrity`, `check_for_updates`) are strictly read-only.
2. **Zero Write Exposure:** No endpoints exist in the Tauri backend to write, overwrite, delete, or restore save files.
3. **Directory Traversal Protection:** All path arguments (`folder_name`, `snapshot_folder`) strictly reject traversal sequences (`..`, `/`, `\`) and are verified against canonical directories.
4. **ProductionGuard Active:** Production save identity (`TXrk_450560341`, Game ID `450560341`) is hard-coded as write-protected. Direct writes to Microsoft Store / Xbox Connected Storage (WGS) remain prohibited.

---

## 3. Desktop UI Implementation

The frontend UI follows modern desktop standards with Stardew-inspired aesthetics:
- **Theme:** Dark mode (Forest Slate `#0c120e`) and Light mode (Parchment Meadow `#f4f7f4`), calm green accents (`#38a169`), gold farmer accents (`#ecc94b`).
- **Typography:** Google Fonts `Outfit` and `JetBrains Mono` for cryptographic checksums and IDs.
- **Custom Branding:** Minimalist app icon featuring a seedling intertwined with synchronization arrows generated specifically for Stardew Sync (zero copyrighted game assets).
- **Screens Implemented:**
  1. **Dashboard:** Displays application version, game process detection pill (live running/closed status), current production farm card (`Türk Çiftliği`), active host (`Kubilay`) and farmhand (`elbi`), and an explicit P2P sync offline notice.
  2. **Farms:** Discovers local saves in `%APPDATA%\StardewValley\Saves`, clearly separating the protected production farm from disposable test saves (`TürkTest`, `TürkTestB`). Clicking any card opens a non-destructive detailed XML inspector modal.
  3. **Backups:** Discovers immutable local snapshots, shows creation timestamps, file lists, and provides a real `Verify SHA-256` button that verifies integrity against live files. Restores to production are explicitly disabled.
  4. **Settings:** Theme selection (Dark / Light), detected save paths (%APPDATA% and Xbox WGS), system diagnostics, and bazq engineering standards.
  5. **Updates:** Official Tauri 2 updater interface with Minisign Ed25519 public key verification and safety locks preventing updates while the game is active.

---

## 4. Windows Packaging (NSIS Installer)

Configured and built via Tauri 2 and NSIS (`makensis` v3.13):
- **Installer Mode:** `currentUser` (user-level installation without requiring UAC admin elevation).
- **Application Identifier:** `com.bazq.stardewsync`
- **Application Name:** `Stardew Sync`
- **Version:** `0.1.0`
- **Icon:** Custom multi-resolution Windows ICO (`src-tauri/icons/icon.ico`).
- **Uninstall Behavior:** Clean removal of application binaries without modifying or deleting the user's Stardew Valley save directory.
- **Generated Installer Artifact:**
  `C:\Users\bazq3\Desktop\stardew-sync-p2p\target\release\bundle\nsis\Stardew Sync_0.1.0_x64-setup.exe` (3.81 MiB)
- **Standalone Binary:**
  `C:\Users\bazq3\Desktop\stardew-sync-p2p\target\release\stardew-sync.exe`

---

## 5. Signed Automatic Updates Readiness

- **Algorithm:** Minisign Ed25519 (Tauri 2 native updater).
- **Public Key:** Configured in `src-tauri/tauri.conf.json`:
  `dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEQ5MTA0RkZCN0ZBM0Y2NTEKUldSUjlxTi8rMDhRMlhpL3ZrSGRBVmYrblZPcERMNUVqQWpSOWdENElSUEtOYWQxRlhQMDAzSFQK`
- **Private Key Security:** The private key was generated locally into `.updater-key`, added to `.gitignore`, and is never committed to Git.
- **Distribution Target:** Configured for GitHub Releases:
  `https://github.com/bazq3/stardew-sync-p2p/releases/latest/download/latest.json`
- **Generated Signature for v0.1.0:**
  `target\release\bundle\nsis\Stardew Sync_0.1.0_x64-setup.exe.sig`
- **Release Manifest Template:** Saved to `docs/updater-release-template.json`.

---

## 6. Safety & Test Verification Results

### Automated Test Suites:
- `commands_tests`: 7 / 7 passed (New: verified DTOs, security traversal rejection, process detection, discovery).
- `core_safety_tests`: 7 / 7 passed (Preserved original).
- `host_migration_tests`: 9 / 9 passed (Preserved original).
- `runtime_safety_tests`: 12 / 12 passed (Preserved original).
- **Total Test Count:** **35 / 35 PASSED (100%)**.

### TypeScript & Frontend Build:
- `npm run build`: Zero errors, zero warnings.
- Production assets bundled into `dist/`.

### Production Farm Protection Verification:
- Baseline SHA-256 (pre-Phase 5.0):
  - `TXrk_450560341`: `C835F8572AEE9ADAE36F3BDDAB512936C6605339D59096954D052D0D0557DEC1`
  - `SaveGameInfo`: `ECBDA32EBAA515A1FF5BE17690099C401CB70995F6583AF1AC3B32A908852299`
- Post-Phase 5.0 SHA-256:
  - `TXrk_450560341`: `C835F8572AEE9ADAE36F3BDDAB512936C6605339D59096954D052D0D0557DEC1`
  - `SaveGameInfo`: `ECBDA32EBAA515A1FF5BE17690099C401CB70995F6583AF1AC3B32A908852299`
- **Verification Result:** **Identical. Zero bytes modified.**
- **WGS Connected Storage:** Untouched throughout Phase 5.0.

---

## 7. Known Blockers & Remaining Manual Configuration

1. **Remote Repository Deployment:** GitHub Releases endpoint `https://github.com/bazq3/stardew-sync-p2p/releases/` has not yet been populated with published release assets. Once approved to release, the signed installer `.exe`, `.sig`, and `latest.json` should be uploaded to GitHub Releases.
2. **Private Signing Key Storage:** The private signing key (`.updater-key`) remains on the developer workstation and should be stored in GitHub Repository Secrets (`TAURI_SIGNING_PRIVATE_KEY`) for CI/CD automated release builds.

---

## 8. Suggested Next Phase

**Phase 6.0: P2P Transport & Two-PC Pairing Engine**
- Implement device pairing exchange (QR code / pairing string) between Kubilay's PC and elbi's PC.
- Implement encrypted payload transport (AES-256-GCM + Ed25519 signed manifests).
- Implement transactional staging and atomic save replacement on the remote PC.
- Real 2-PC co-op gameplay validation with host promotion.
