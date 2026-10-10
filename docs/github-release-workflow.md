# Stardew Sync — GitHub Release Automation & Publishing Guide

> **Author:** bazq  
> **Repository:** `bazq3D/stardew-sync`  
> **Framework:** Tauri 2 (Rust) + React (TypeScript) + NSIS  

---

## 1. Overview & Architecture

Stardew Sync uses native in-app updates powered by the Tauri 2 updater plugin (`tauri-plugin-updater`).
The client periodically queries the canonical GitHub release endpoint:
`https://github.com/bazq3D/stardew-sync/releases/latest/download/latest.json`

To distribute updates safely and reliably from the local development machine without risk of breaking installed clients or corrupting Stardew Valley saves, the publishing workflow enforces strict automated checks:

1. **Authentication**: Uses GitHub CLI (`gh`) with automated token resolution from the Windows Git Credential Manager.
2. **Standard Version Tagging**: Consistent tag naming (`vX.Y.Z`).
3. **Artifact Reuse**: Avoids unnecessary and redundant Cargo compilation if verified release artifacts already exist in `target/release/bundle/nsis/`.
4. **GitHub Asset Normalization Compliance**: GitHub replaces spaces in asset filenames with dots (e.g. `Stardew Sync_...` becomes `Stardew.Sync_...`). The automation stages normalized files so manifest URLs and uploaded filenames match 100%.
5. **Full Cryptographic Validation**: Validates Minisign Ed25519 signatures, version comments, and exact byte parity before touching GitHub.
6. **Idempotence & Safety Gates**: Requires explicit `-Approved` switch before modifying remote releases; supports safe `-DryRun` mode.
7. **Post-Publication Live Verification**: Probes all published release endpoints via HTTP HEAD/GET to guarantee HTTP 200 OK availability.
8. **Stardew Valley Save Protection**: The release automation operates entirely within the repository workspace and never touches `%APPDATA%\StardewValley\Saves` or Xbox WGS directories.

---

## 2. Release Assets & Filename Normalization

When Tauri builds a Windows NSIS bundle, it produces:
```
target/release/bundle/nsis/
├── Stardew Sync_0.1.2_x64-setup.exe
└── Stardew Sync_0.1.2_x64-setup.exe.sig
```

### The GitHub Normalization Issue
When files containing spaces are uploaded to GitHub releases via API or web, GitHub converts spaces to dots or serves them normalized:
- Uploaded filename: `Stardew.Sync_0.1.2_x64-setup.exe`
- If `latest.json` specifies `Stardew%20Sync_0.1.2_x64-setup.exe`, GitHub returns **HTTP 404 Not Found**.

### The Solution Implemented
The release automation stages normalized asset copies:
- `Stardew.Sync_0.1.2_x64-setup.exe`
- `Stardew.Sync_0.1.2_x64-setup.exe.sig`
- `latest.json` (pointing directly to `https://github.com/bazq3D/stardew-sync/releases/download/v0.1.2/Stardew.Sync_0.1.2_x64-setup.exe`)

Both `Stardew Sync_...` and `Stardew.Sync_...` share identical SHA-256 binary contents, preserving cryptographic signature validity.

---

## 3. Cryptographic Signature Verification

Tauri's updater uses **Minisign** Ed25519 signatures.

- **Public Key** (configured in [src-tauri/tauri.conf.json](file:///c:/Users/bazq3/Desktop/stardew-sync-p2p/src-tauri/tauri.conf.json)):
  ```
  dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDExMjM1QzVBOUI3QURFM0MKUlVScGRIZlR3RER1b3Z2NFR5dkZqY2J1S1NlUG1yZDJpUjlXOHJicjRBNk52aFVlYTRlS1BBMFEK
  ```
  *(Key ID: `11235C5A9B7ADE3C`)*

- **Signature Properties Verified by `validate-release-manifest.cjs` and `tests/updater_signing_tests.rs`**:
  - `untrusted comment`: standard Minisign header.
  - `trusted comment`: binds `timestamp`, `file:Stardew.Sync_<version>_x64-setup.exe`, and `version:<version>`.
  - Cryptographic validation via `minisign-verify` crate against the exact installer binary bytes.

---

## 4. Automation Script Usage

The primary script is [scripts/publish-github-release.ps1](file:///c:/Users/bazq3/Desktop/stardew-sync-p2p/scripts/publish-github-release.ps1).

### Syntax
```powershell
.\scripts\publish-github-release.ps1 `
    [-Version <string>] `
    [-Repo <string>] `
    [-Title <string>] `
    [-ReleaseNotes <string>] `
    [-DryRun] `
    [-Approved]
```

### Parameters
| Parameter | Default | Description |
|---|---|---|
| `-Version` | Version from `package.json` | Release version to target (e.g. `0.1.2`). |
| `-Repo` | `bazq3D/stardew-sync` | Canonical GitHub repository. |
| `-Title` | Auto-generated | Release title on GitHub. |
| `-ReleaseNotes` | Auto-extracted from Git log or docs | Release notes body or path to Markdown notes file. |
| `-DryRun` | `$false` | Runs preflight checks, generates manifest, and validates signatures without mutating GitHub. |
| `-Approved` | `$false` | **Required safety gate**. Without this switch, no changes are pushed to GitHub. |

### Examples

#### Safe Dry-Run (No remote changes)
```powershell
powershell -ExecutionPolicy Bypass -File scripts\publish-github-release.ps1 -DryRun
```

#### Publish / Update Release (With explicit approval)
```powershell
powershell -ExecutionPolicy Bypass -File scripts\publish-github-release.ps1 -Approved
```

#### Publish Specific Version with Custom Title
```powershell
powershell -ExecutionPolicy Bypass -File scripts\publish-github-release.ps1 `
    -Version "0.1.2" `
    -Title "Stardew Sync v0.1.2 - Updater Verification" `
    -Approved
```

---

## 5. Inspection of Existing GitHub Release `v0.1.2`

A thorough inspection of the remote release on GitHub (`bazq3D/stardew-sync`) revealed two critical issues that break in-app auto-updating:

### Defect 1: Manifest Asset URL Mismatch (HTTP 404)
- **Manifest on GitHub (`latest.json`)**:
  `"url": "https://github.com/bazq3D/stardew-sync/releases/download/v0.1.2/Stardew%20Sync_0.1.2_x64-setup.exe"`
- **Actual File on GitHub**:
  `Stardew.Sync_0.1.2_x64-setup.exe`
- **Result**: Client requests result in **`HTTP 404 Not Found`**.

### Defect 2: Cryptographic Signature Mismatch (`Err(InvalidSignature)`)
- **Remote Installer Asset on GitHub**:
  - Size: `4,022,022` bytes
  - SHA-256: `ED256F196C9A0976F18E470CEB0F17E9373EB99A8AF5E389D6FA534A53D961CA`
  - Uploaded at: `2026-10-09T19:10:59Z`
- **Remote Signature Asset on GitHub (`.sig`)**:
  - Uploaded at: `2026-10-09T18:58:53Z`
  - Minisign Verification against the remote installer: **`Err(InvalidSignature)`**
- **Local Validated Installer**:
  - Size: `4,020,567` bytes
  - SHA-256: `26129A60B254F7C49FC302E707C683FD73A32CDD39D3855B92C7FF28837CBF80`
  - Minisign Verification: **`Ok(())`** (100% cryptographic match with the `.sig` file)
- **Result**: The installer file currently uploaded on GitHub was compiled from a different commit/build than the signature, so even if the 404 were bypassed, the installed client's updater would reject it as untrusted.

---

## 6. Required Corrections for `v0.1.2`

To make the v0.1.2 update succeed end-to-end for installed clients:

1. **Upload Validated Local Installer**: Replace the mismatched remote installer with the local `4,020,567`-byte binary (`26129A60B254F7C49FC302E707C683FD73A32CDD39D3855B92C7FF28837CBF80`).
2. **Upload Validated Signature**: Ensure `Stardew.Sync_0.1.2_x64-setup.exe.sig` matches.
3. **Upload Corrected `latest.json`**: Ensure download URLs reference `https://github.com/bazq3D/stardew-sync/releases/download/v0.1.2/Stardew.Sync_0.1.2_x64-setup.exe`.
4. **Set as Latest Release**: Mark `v0.1.2` as `--latest` on GitHub so `/releases/latest/download/latest.json` serves it.
5. **Verify Live URLs**: Check HTTP 200 OK status on all published asset links.
