<#
.SYNOPSIS
    Automated Local Release Builder & Signer for Stardew Sync
    Author: bazq
    Generates verified, signed Windows NSIS installer and updater manifest.
#>

[CmdletBinding()]
param(
    [string]$ReleaseNotes = "Stardew Sync Public Release with signed native auto-updater."
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$rootDir = Split-Path -Parent $scriptDir

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host " [bazq-release] Stardew Sync Reproducible Release Pipeline  " -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

# 1. Version validation
$pkgVersion = (Get-Content (Join-Path $rootDir "package.json") -Raw | ConvertFrom-Json).version
$tauriVersion = (Get-Content (Join-Path $rootDir "src-tauri\tauri.conf.json") -Raw | ConvertFrom-Json).version

if ($pkgVersion -ne $tauriVersion) {
    Write-Error "Version mismatch: package.json ($pkgVersion) != tauri.conf.json ($tauriVersion)"
    exit 1
}
Write-Host "[1/7] Target Release Version: v$pkgVersion" -ForegroundColor Green

# 2. Locate signing credentials outside repository
$keyPath = Join-Path $env:USERPROFILE ".stardew-sync-keys\stardew-sync.key"
$passPath = Join-Path $env:USERPROFILE ".stardew-sync-secrets\signing.pass"

if (-not (Test-Path $keyPath)) {
    Write-Error "Private signing key not found at: $keyPath"
    exit 1
}
if (-not (Test-Path $passPath)) {
    Write-Error "Signing passphrase secret not found at: $passPath"
    exit 1
}
$pass = (Get-Content -Path $passPath -Raw).Trim()
Write-Host "[2/7] Signing credentials verified in secure external secret stores." -ForegroundColor Green

# 3. Quality Assurance: Rust Tests & TypeScript Build
Write-Host "[3/7] Running test suites (cargo test)..." -ForegroundColor Yellow
Push-Location (Join-Path $rootDir "src-tauri")
try {
    & cargo test
    if ($LASTEXITCODE -ne 0) {
        throw "cargo test failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
}

Write-Host "[4/7] Running frontend compilation (npm run build)..." -ForegroundColor Yellow
Push-Location $rootDir
try {
    & npm run build
    if ($LASTEXITCODE -ne 0) {
        throw "npm run build failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
}

# 4. Build Windows NSIS Bundle
Write-Host "[5/7] Building Windows NSIS installer bundle..." -ForegroundColor Yellow
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Path $keyPath -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $pass

Push-Location $rootDir
try {
    & npx tauri build --bundles nsis
    if ($LASTEXITCODE -ne 0) {
        throw "tauri build failed with exit code $LASTEXITCODE"
    }
} finally {
    $env:TAURI_SIGNING_PRIVATE_KEY = $null
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $null
    Pop-Location
}

# 5. Dynamically locate built installer
$nsisDir = Join-Path $rootDir "target\release\bundle\nsis"
$installer = Get-ChildItem -Path $nsisDir -Filter "*$pkgVersion*_x64-setup.exe" | Select-Object -First 1

if (-not $installer) {
    Write-Error "Could not find built installer for version $pkgVersion in $nsisDir"
    exit 1
}

$installerName = $installer.Name
$installerPath = $installer.FullName
Write-Host "✓ Built installer: $installerName" -ForegroundColor Green

# 6. Sign installer artifact
Write-Host "[6/7] Signing installer artifact with password-protected Ed25519 Minisign key..." -ForegroundColor Yellow
Push-Location $rootDir
try {
    & npx tauri signer sign -f $keyPath -p $pass --app-version $pkgVersion $installerPath
    if ($LASTEXITCODE -ne 0) {
        throw "tauri signer sign failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
}

$sigFile = Join-Path $nsisDir "$installerName.sig"
if (-not (Test-Path $sigFile)) {
    Write-Error "Signature file not generated at: $sigFile"
    exit 1
}
$signatureBase64 = (Get-Content -Path $sigFile -Raw).Trim()

# 7. Generate latest.json manifest
Write-Host "[7/7] Generating release manifest (latest.json)..." -ForegroundColor Yellow
$encodedName = [System.Uri]::EscapeDataString($installerName)
$downloadUrl = "https://github.com/bazq3D/stardew-sync/releases/download/v$pkgVersion/$encodedName"
$pubDate = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

$manifestObject = [ordered]@{
    version   = $pkgVersion
    notes     = $ReleaseNotes
    pub_date  = $pubDate
    platforms = [ordered]@{
        "windows-x86_64" = [ordered]@{
            signature = $signatureBase64
            url       = $downloadUrl
        }
        "windows-x86_64-nsis" = [ordered]@{
            signature = $signatureBase64
            url       = $downloadUrl
        }
    }
}

$manifestJson = $manifestObject | ConvertTo-Json -Depth 5
$latestJsonPath = Join-Path $nsisDir "latest.json"
Set-Content -Path $latestJsonPath -Value $manifestJson -Encoding UTF8

$templatePath = Join-Path $rootDir "docs\updater-release-template.json"
Set-Content -Path $templatePath -Value $manifestJson -Encoding UTF8
Write-Host "✓ Manifest written to $latestJsonPath and $templatePath" -ForegroundColor Green

# 8. Run validation
Write-Host "`nRunning post-build manifest and signature validation..." -ForegroundColor Yellow
& node (Join-Path $rootDir "scripts\validate-release-manifest.cjs")
if ($LASTEXITCODE -ne 0) {
    Write-Error "Post-build release validation failed!"
    exit 1
}

# 9. Output SHA-256 Checksums
Write-Host "`n================== RELEASE ARTIFACT CHECKSUMS ==================" -ForegroundColor Cyan
Get-ChildItem -Path $nsisDir -Filter "*$pkgVersion*" | ForEach-Object {
    $hash = (Get-FileHash -Path $_.FullName -Algorithm SHA256).Hash
    [PSCustomObject]@{
        Name   = $_.Name
        Length = $_.Length
        SHA256 = $hash
    }
} | Format-Table -AutoSize
Write-Host "================================================================`n" -ForegroundColor Cyan

Write-Host "Release v$pkgVersion generated and verified successfully!" -ForegroundColor Green
Write-Host "Artifacts ready in: $nsisDir" -ForegroundColor Green
Write-Host "(No artifacts have been published or pushed to GitHub)" -ForegroundColor Yellow
