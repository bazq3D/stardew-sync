# Stardew Sync — End-to-End Updater Test Plan (v0.1.1 → v0.1.2)

**Project:** `stardew-sync` ([github.com/bazq3D/stardew-sync](https://github.com/bazq3D/stardew-sync))  
**Application:** Stardew Sync  
**Platform:** Windows x64 (Tauri 2 NSIS + Minisign Ed25519)  
**Author:** `bazq`  
**Status:** Prepared & Awaiting User Approval (Approval-Gated)

---

## 1. Overview & Objective

This plan specifies the controlled end-to-end validation of the Stardew Sync signed update delivery mechanism across two phases:
1. **Initial Upgrade (v0.1.0 → v0.1.1):** One-time manual upgrade using the signed NSIS installer to establish cryptographic trust and configure the official GitHub endpoint.
2. **Native In-App Update (v0.1.1 → v0.1.2):** Full automatic in-app check, user approval, background download, cryptographic signature verification, installer execution, and application restart.

All steps involving binary installation, Git push, or GitHub release publication are **strictly approval-gated**.

---

## 2. Test Execution Sequence

### STEP A: Manual Installation of v0.1.1 over v0.1.0
*Requirement: Explicit User Approval*
1. Close any running instances of `Stardew Sync` and `Stardew Valley`.
2. Locate the signed v0.1.1 installer:
   `target/release/bundle/nsis/Stardew Sync_0.1.1_x64-setup.exe`
3. Execute the installer.
4. **Expected Behavior:**
   - The installer installs in `currentUser` mode without UAC elevation.
   - Files in `D:\Stardew Sync\` are updated to v0.1.1.
   - Windows Registry key `HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Stardew Sync` reflects `DisplayVersion: 0.1.1`.

### STEP B: Verification of v0.1.1 Runtime & Save Discovery
1. Launch `D:\Stardew Sync\stardew-sync.exe`.
2. Inspect the dashboard:
   - App version displays `v0.1.1`.
   - Save discovery finds local saves without errors.
   - In-game calendar date, farm funds, host, and farmhands display dynamically.
   - Safety lock shows all discovered saves as protected (read-only).
3. Inspect Updates page:
   - Installed version reports `v0.1.1`.
   - Cryptographic verification badge shows Minisign Ed25519 configured.
   - Endpoint displays `https://github.com/bazq3D/stardew-sync/releases/latest/download/latest.json`.

### STEP C: Publish Initial v0.1.1 GitHub Release
*Requirement: Explicit User Approval*
1. Push local `main` commits to GitHub:
   ```powershell
   git push origin main
   ```
2. Create release tag `v0.1.1`:
   ```powershell
   git tag v0.1.1
   git push origin v0.1.1
   ```
3. Create GitHub Release `v0.1.1` on `https://github.com/bazq3D/stardew-sync/releases`:
   - Title: `Stardew Sync v0.1.1`
   - Upload asset: `target/release/bundle/nsis/Stardew Sync_0.1.1_x64-setup.exe`
   - Upload asset: `target/release/bundle/nsis/Stardew Sync_0.1.1_x64-setup.exe.sig`
   - Upload asset: `target/release/bundle/nsis/latest.json` (constructed from `docs/updater-release-template.json`)
4. Verify HTTP download:
   Querying `https://github.com/bazq3D/stardew-sync/releases/latest/download/latest.json` returns HTTP 200 with valid JSON.

### STEP D: Prepare v0.1.2 Test Artifact
*Requirement: Explicit User Approval*
1. Execute `scripts/prepare-v0.1.2-test-release.ps1` to bump version declarations to `0.1.2`.
2. Minimal identifiable change:
   - Version: `0.1.2`
   - Release notes: `"Stardew Sync v0.1.2 Update Verification Release. Confirms seamless end-to-end in-app updating."`
3. The build pipeline runs tests, compiles the NSIS installer (`Stardew Sync_0.1.2_x64-setup.exe`), signs it with the password-protected Minisign key, and generates `latest.json` for v0.1.2.

### STEP E: Publish v0.1.2 GitHub Release
*Requirement: Explicit User Approval*
1. Push tag `v0.1.2` to GitHub.
2. Create GitHub Release `v0.1.2` with:
   - `Stardew Sync_0.1.2_x64-setup.exe`
   - `Stardew Sync_0.1.2_x64-setup.exe.sig`
   - Updated `latest.json` announcing v0.1.2.

### STEP F: Check for Updates from Installed v0.1.1 Client
1. In the running v0.1.1 application, navigate to the **Updates** tab.
2. Click **Check for Updates**.
3. **Expected Behavior:**
   - Client queries `latest.json` from GitHub.
   - Tauri compares version `0.1.1` with announced version `0.1.2`.
   - Detects update `v0.1.2`.

### STEP G & H: Confirmation of Update Detection & Approval
1. The UI switches to the "New Release Available" card:
   - Title: `New Release Available: v0.1.2`
   - Release notes displayed accurately.
   - Button displayed: `Download & Install Update`.
2. **Safety Check:** If Stardew Valley is launched while on this screen, the update button is immediately disabled with the `Update Safety Lock Engaged` warning.

### STEP I: Download and Installation Execution
1. Ensure Stardew Valley is closed.
2. Click **Download & Install Update**.
3. **Expected Behavior:**
   - Real-time percentage progress bar fills as chunks download.
   - Status updates: *"Downloading signed update package..."* -> *"Cryptographic signature verified. Launching installer..."*
   - Tauri core verifies the Minisign signature against the public key `262CDD7BAC36A4EC` and verifies that the trusted comment matches `version:0.1.2`.
   - Windows NSIS installer runs silently or in foreground to replace the binary.

### STEP J: Post-Update Restart Verification
1. Application relaunches.
2. Dashboard displays: `App Version: v0.1.2`.
3. Updates page displays: `Installed Application Version: v0.1.2` and `UP TO DATE`.

### STEP K: Data Integrity & Save Protection Audit
1. Verify that `%APPDATA%\StardewValley\Saves\TXrk_450560341` remains bit-for-bit identical to baseline.
2. Verify that local snapshots and configuration files are fully intact.
3. Verify that save discovery continues to function without errors.

---

## 3. Rollback & Emergency Recovery Plan

If any step in the sequence fails:
- **Installer Failure:** Run the previous installer `target/release/bundle/nsis/Stardew Sync_0.1.1_x64-setup.exe` to restore v0.1.1.
- **Corrupt Update Download:** The Tauri updater engine verifies the Minisign signature in memory before touching any files on disk. If a download is corrupt or the signature is invalid, Tauri rejects the file and aborts without modifying the installed application.
- **Save Protection:** The updater does not touch `%APPDATA%\StardewValley\Saves`. All save operations remain strictly read-only throughout.
