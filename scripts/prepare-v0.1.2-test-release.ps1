<#
.SYNOPSIS
    Prepares and builds v0.1.2 test release artifact for end-to-end update testing.
    Author: bazq
    REQUIRES EXPLICIT USER APPROVAL.
#>

[CmdletBinding()]
param(
    [switch]$Approved
)

$ErrorActionPreference = "Stop"

if (-not $Approved) {
    Write-Warning "This script is part of Step D in docs/updater-test-plan.md."
    Write-Warning "It must only be run AFTER v0.1.1 has been installed and published to GitHub."
    Write-Warning "To execute, run with the -Approved switch: .\scripts\prepare-v0.1.2-test-release.ps1 -Approved"
    exit 1
}

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$rootDir = Split-Path -Parent $scriptDir

Write-Host "[bazq-updater-test] Preparing v0.1.2 test release artifact..." -ForegroundColor Cyan

# 1. Bump version declarations to 0.1.2
$filesToBump = @(
    (Join-Path $rootDir "package.json"),
    (Join-Path $rootDir "package-lock.json"),
    (Join-Path $rootDir "src-tauri\tauri.conf.json"),
    (Join-Path $rootDir "src-tauri\Cargo.toml")
)

foreach ($file in $filesToBump) {
    $content = Get-Content -Path $file -Raw
    $updated = $content.Replace('"0.1.1"', '"0.1.2"').Replace('version = "0.1.1"', 'version = "0.1.2"')
    Set-Content -Path $file -Value $updated -NoNewline
}

# 2. Invoke reproducible build pipeline
& (Join-Path $scriptDir "build-release.ps1") -ReleaseNotes "Stardew Sync v0.1.2 Update Verification Release. Confirms seamless end-to-end in-app updating."
if ($LASTEXITCODE -ne 0) {
    Write-Error "Failed to build and sign v0.1.2 test release!"
    exit 1
}

Write-Host "`nv0.1.2 test release is ready for publication (Step E)!" -ForegroundColor Green
