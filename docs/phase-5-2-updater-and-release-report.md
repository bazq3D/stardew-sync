# Phase 5.2 — Versioning, Secure Auto-Updater & GitHub Release Readiness Report

**Project:** `stardew-sync-p2p`  
**Application:** Stardew Sync  
**Installed Version:** `0.1.0`  
**Target / Packaged Version:** `0.1.1`  
**Author:** `bazq`  
**Date:** 2026-10-09  
**Platform:** Windows x64 (Tauri 2 + React 18 + TypeScript + Rust)

---

## 1. Initial Repository and Release Audit

### Application Version Sources
All version declarations were audited across the codebase:
- `package.json`: Updated from `0.1.0` to `0.1.1`
- `package-lock.json`: Synchronized to `0.1.1`
- `src-tauri/Cargo.toml`: Updated from `0.1.0` to `0.1.1`
- `Cargo.lock`: Synchronized to `0.1.1`
- `src-tauri/tauri.conf.json`: Updated from `0.1.0` to `0.1.1`
- `src-tauri/src/commands/mod.rs`: Replaced hardcoded `"0.1.0"` with `env!("CARGO_PKG_VERSION")` to prevent future version drift.

### Tauri Bundle Identifier & NSIS Configuration
- **Bundle Identifier:** `com.bazq.stardewsync` (Preserved identically to maintain Windows application registry consistency and user data continuity).
- **NSIS Install Mode:** `currentUser` (installs to user application directory without requiring UAC privilege elevation).
- **NSIS Language:** `["English"]`.
- **Application Display Name:** `Stardew Sync`.

### Existing Update Endpoint & Git Remote Audit
- **Git Remote:** `origin` is configured as `https://github.com/bazq3D/stardew-sync-p2p.git`.
- **Endpoint Misconfiguration Identified & Fixed:** `tauri.conf.json` previously referenced `bazq3/stardew-sync-p2p`. This was corrected to `https://github.com/bazq3D/stardew-sync-p2p/releases/latest/download/latest.json`.
- **Publication State:**
  - Local `main` branch is 13 commits ahead of remote `origin/main`.
  - Zero git tags exist on the remote repository.
  - No GitHub release has ever been published (`https://github.com/bazq3D/stardew-sync-p2p/releases/latest/download/latest.json` currently returns HTTP 404 Not Found).
  - Version `0.1.0` has **never** been published publicly.

---

## 2. Security-Critical Signing Audit

### Findings
1. **Old Insecure Key:** In Phase 5.0, an updater keypair was generated into `.updater-key` in the repository root using an empty passphrase.
2. **Repository Cleanliness Check:**
   - Git tracking history was audited across all branches and commits using `git log -p -S` and regex patterns.
   - Result: **Zero private keys, secrets, credentials, or real Stardew Valley save files exist in Git history.**
   - `.updater-key` was ignored by `.gitignore` and was never staged or committed.

### Remediation & Secure Key Generation
1. **Passphrase-Protected Keypair:**
   - Generated a brand-new Ed25519 Minisign keypair protected by a cryptographically strong 256-bit passphrase.
2. **Secure Key Location (Outside Repository):**
   - Private key stored in user profile directory:
     `C:\Users\bazq3\.stardew-sync-keys\stardew-sync.key` (348 bytes, password-encrypted).
   - Passphrase stored in separate restricted-access file:
     `C:\Users\bazq3\.stardew-sync-keys\stardew-sync.password` (44 bytes, protected by Windows NT ACL allowing only current user).
   - Public verification key saved at:
     `C:\Users\bazq3\.stardew-sync-keys\stardew-sync.key.pub` (152 bytes).
3. **Workspace Sanitization:**
   - Removed legacy unencrypted `.updater-key` and `.updater-key.pub` from the project repository root.
4. **Public Verification Key Configured:**
   - Embedded into `src-tauri/tauri.conf.json`:
     ```
     dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDI2MkNERDdCQUMzNkE0RUMKUldUc3BEYXNlOTBzSmkzSjIxcmdZOEg0U1E0UzQ5cmcrcWd5YkpsNmM4N2p4bGF5bWpDUDlxcG8K
     ```
   - Matches Minisign public key ID `262CDD7BAC36A4EC`.

### Safe Backup & Recovery Procedure
- **Backup:** Copy `C:\Users\bazq3\.stardew-sync-keys\` to an encrypted offline storage medium (e.g. BitLocker drive or password manager vault).
- **Recovery:** To sign future releases on CI/CD (GitHub Actions), add the private key content to repository secret `TAURI_SIGNING_PRIVATE_KEY` and the passphrase to `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

---

## 3. Official Tauri 2 Updater & Capability Configuration

### Tauri 2 ACL Capability Configuration
Created `src-tauri/capabilities/default.json` to grant the frontend window legitimate permissions for the Tauri 2 core updater and process plugins:
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default capability for the main desktop window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:app:default",
    "updater:default",
    "process:default"
  ]
}
```
This enables:
- `updater:allow-check`, `updater:allow-download`, `updater:allow-install`, `updater:allow-download-and-install`
- `process:allow-restart`, `process:allow-exit`
- `core:app:allow-version`, `core:app:allow-name`

---

## 4. UI Integration (Updates Page Audit & Enhancements)

The `src/pages/UpdatesPage.tsx` interface was audited and upgraded:
1. **Installed Version:** Dynamically retrieved from `appStatus.app_version` (evaluated via Rust `env!("CARGO_PKG_VERSION")`), eliminating static strings.
2. **Native Tauri 2 Updater Execution:** `handleCheckUpdates()` directly invokes `check()` from `@tauri-apps/plugin-updater`.
3. **Graceful Handling of Unreleased Endpoints:** Because no release manifest exists on GitHub yet, the 404 response is cleanly intercepted and displayed:
   > *"No published release manifest found on GitHub (HTTP 404 at https://github.com/bazq3D/stardew-sync-p2p/releases/latest/download/latest.json). Releases must be published on GitHub before remote updates can be retrieved."*
   No fake success state is presented.
4. **Explicit User Approval:** When an update is detected, a dedicated "New Release Available" card appears with the release notes and a distinct **Download & Install Update** button. Updates are never applied silently.
5. **Download Progress:** Real-time percentage progress bar tracking chunk lengths from download events.
6. **Process Lockout Guard:** If Stardew Valley or SMAPI is running (`processStatus.is_stardew_running`), the update button is disabled and a warning banner advises the user to exit the game to prevent save lock collisions or binary locking.
7. **Upgrade Advisory Card:** Documents the one-time manual upgrade path required for users moving from the v0.1.0 pre-release to v0.1.1.

---

## 5. Windows NSIS Installer & Release Artifacts

The Windows x64 NSIS bundle was built using `npx tauri build --bundles nsis` and signed with the new password-protected key:

| Artifact | Location | Size | SHA-256 Checksum |
| :--- | :--- | :--- | :--- |
| **Windows NSIS Installer** | `target/release/bundle/nsis/Stardew Sync_0.1.1_x64-setup.exe` | 4,019,198 bytes (3.83 MiB) | `B5A3F2D14C9092FBA312432137F688BC68C9BE941B463E9BA54279DE373E52A8` |
| **Minisign Signature** | `target/release/bundle/nsis/Stardew Sync_0.1.1_x64-setup.exe.sig` | 444 bytes | `E6E745DDE185F1EFDA519C48D4CB36241FD04E2BD5608B04C63D8CDEDC2BA084` |
| **Release Binary** | `target/release/stardew-sync.exe` | 13,072,896 bytes (12.46 MiB) | `04ED4AB0545F3DBA73DC45A05CBC7821DB67981C9F1F1B8F805D3F32C38EB3E7` |

### Detached Minisign Signature
```
dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVUc3BEYXNlOTBzSnYzZHRZdHJhdTFnT0RMTmd2QlZFUjBwalF4dUs2ZExRdlkrbW4zRnNNd0s4eDZFdU1MTHVJcldjVmtKVU8xdXNXaS9HcHM1Vmh1SVB2Q3dKajVKY3dVPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkxNTY2ODgzCWZpbGU6U3RhcmRldyBTeW5jXzAuMS4xX3g2NC1zZXR1cC5leGUJdmVyc2lvbjowLjEuMQozempvTS8xUU42dmk3c25YVjlXYndKM0tyOExxcGpuUGhqYzhkRmZuYlByNm1qT3JDU1RhUmZYMlFVRFdyTjVSTHZ3MG4yTStyTUhLaXJ4aU4wVEhCdz09Cg==
```

### Manifest Template (`docs/updater-release-template.json`)
```json
{
  "version": "0.1.1",
  "notes": "Stardew Sync v0.1.1 Public-Ready Release. Featuring dynamic save discovery for any player/farm, accurate in-game calendar date parsing, generic read-only safety model, password-protected cryptographic updater signing, and native Tauri 2 updater integration.",
  "pub_date": "2026-10-09T20:28:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVUc3BEYXNlOTBzSnYzZHRZdHJhdTFnT0RMTmd2QlZFUjBwalF4dUs2ZExRdlkrbW4zRnNNd0s4eDZFdU1MTHVJcldjVmtKVU8xdXNXaS9HcHM1Vmh1SVB2Q3dKajVKY3dVPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkxNTY2ODgzCWZpbGU6U3RhcmRldyBTeW5jXzAuMS4xX3g2NC1zZXR1cC5leGUJdmVyc2lvbjowLjEuMQozempvTS8xUU42dmk3c25YVjlXYndKM0tyOExxcGpuUGhqYzhkRmZuYlByNm1qT3JDU1RhUmZYMlFVRFdyTjVSTHZ3MG4yTStyTUhLaXJ4aU4wVEhCdz09Cg==",
      "url": "https://github.com/bazq3D/stardew-sync-p2p/releases/download/v0.1.1/Stardew.Sync_0.1.1_x64-setup.exe"
    }
  }
}
```

---

## 6. Upgrade Compatibility Assessment from Installed v0.1.0

### Installed v0.1.0 State
- Location: `D:\Stardew Sync\`
- Registry Key: `HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Stardew Sync`
- Display Version: `0.1.0`
- Compiled Public Key: `D9104FFB7A3F651` (unencrypted development key)
- Compiled Endpoint: `https://github.com/bazq3/stardew-sync-p2p/releases/latest/download/latest.json`

### Why In-App Update from v0.1.0 to v0.1.1 Cannot Work
1. **Non-Existent Endpoint:** The v0.1.0 binary points to `bazq3` instead of `bazq3D`. That URL returns 404.
2. **Cryptographic Key Mismatch:** Even if a manifest were hosted at that URL, v0.1.0's updater plugin expects signatures from the old insecure key. Binaries signed with the new secure key would be rejected as untrusted.
3. **No Published Releases:** Neither v0.1.0 nor v0.1.1 has been pushed or published to GitHub.

### One-Time Manual Upgrade Procedure
To transition from v0.1.0 to v0.1.1 safely:
1. Ensure `Stardew Sync` is closed.
2. Execute `target/release/bundle/nsis/Stardew Sync_0.1.1_x64-setup.exe`.
3. The installer detects the existing `currentUser` installation, replaces the application binaries in `D:\Stardew Sync\`, and updates the registry `DisplayVersion` to `0.1.1`.
4. User settings and local save directory data are preserved without modification.
5. All future updates (v0.1.1 onwards) will update automatically in-app via the official GitHub repository.

---

## 7. Verification Results

### 1. Test Suite Pass Rate
- **Commands & Regression Tests:** 13 / 13 PASSED
- **Core Safety & Boundary Tests:** 7 / 7 PASSED
- **Host Migration Tests:** 9 / 9 PASSED
- **Runtime Safety Tests:** 12 / 12 PASSED
- **Total:** **41 / 41 Tests PASSED (100%)**

### 2. Frontend & Compilation
- `npm run build`: Zero errors, bundle created (207.39 kB JS, 11.72 kB CSS).
- `cargo check`: Clean pass on `stardew-sync-core v0.1.1`.
- `cargo test`: Clean pass across all 5 test suites.

### 3. Runtime Verification
- Executed `target/release/stardew-sync.exe` directly (PID 12012).
- Status: `Responding: True`, Working Set: 25.6 MB. Clean shutdown verified.

### 4. Production Save Safety Audit
The user's real Stardew Valley save at `%APPDATA%\StardewValley\Saves\TXrk_450560341` was verified before and after all Phase 5.2 actions:

| File | Baseline SHA-256 | Post-Phase 5.2 SHA-256 | Match |
| :--- | :--- | :--- | :--- |
| `SaveGameInfo` | `ECBDA32EBAA515A1FF5BE17690099C401CB70995F6583AF1AC3B32A908852299` | `ECBDA32EBAA515A1FF5BE17690099C401CB70995F6583AF1AC3B32A908852299` | **EXACT MATCH** |
| `TXrk_450560341` | `C835F8572AEE9ADAE36F3BDDAB512936C6605339D59096954D052D0D0557DEC1` | `C835F8572AEE9ADAE36F3BDDAB512936C6605339D59096954D052D0D0557DEC1` | **EXACT MATCH** |

- **Xbox WGS Storage:** `containers.index` LastWriteTime `9.10.2026 17:40:45` remains untouched.

---

## 8. Required User Actions Before Publishing

Before publishing to GitHub:
1. **Push Commits to Remote:** Push local `main` commits to `https://github.com/bazq3D/stardew-sync-p2p.git`.
2. **Create GitHub Release `v0.1.1`:**
   - Tag: `v0.1.1`
   - Attach: `Stardew Sync_0.1.1_x64-setup.exe`
   - Attach: `latest.json` (constructed from `docs/updater-release-template.json`)
3. **Optional Manual Upgrade:** Run `Stardew Sync_0.1.1_x64-setup.exe` to update the local installation from v0.1.0 to v0.1.1.
