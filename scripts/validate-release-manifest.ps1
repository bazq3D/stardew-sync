<#
.SYNOPSIS
    Automated Release Manifest & Asset Validation for Stardew Sync
    Author: bazq
#>

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$rootDir = Split-Path -Parent $scriptDir

Write-Host "[bazq-validator] Running Stardew Sync release validation..." -ForegroundColor Cyan

# Execute node validator
& node (Join-Path $scriptDir "validate-release-manifest.cjs")
if ($LASTEXITCODE -ne 0) {
    Write-Error "Release validation failed."
    exit 1
}

Write-Host "[bazq-validator] PowerShell release check completed successfully." -ForegroundColor Green
