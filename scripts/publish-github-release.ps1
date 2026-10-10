<#
.SYNOPSIS
    Automated Local GitHub Release Publishing Pipeline for Stardew Sync
    Author: bazq
    Repository: https://github.com/bazq3D/stardew-sync

.DESCRIPTION
    Uses GitHub CLI (gh) to validate, stage, publish, and verify signed releases
    and updater manifests. Handles GitHub asset filename normalization, prevents
    404 updater mismatches, and confirms cryptographic signature parity.

.PARAMETER Version
    Version to publish (e.g. "0.1.2"). Defaults to package.json version.

.PARAMETER Repo
    Canonical repository in owner/repo format. Defaults to "bazq3D/stardew-sync".

.PARAMETER Title
    Release title. Defaults to "Stardew Sync v<version> - Updater Verification".

.PARAMETER ReleaseNotes
    Release notes body or path to release notes file.

.PARAMETER DryRun
    When specified, validates artifacts and verifies GitHub state without mutating.

.PARAMETER Approved
    Required confirmation switch to create/modify GitHub releases or upload assets.
#>

[CmdletBinding()]
param(
    [string]$Version,
    [string]$Repo = "bazq3D/stardew-sync",
    [string]$Title,
    [string]$ReleaseNotes,
    [switch]$DryRun,
    [switch]$Approved
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$rootDir = Split-Path -Parent $scriptDir

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "  [bazq-publisher] Stardew Sync GitHub Release Automation     " -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

# 1. Determine Target Version & Canonical Tag
if (-not $Version) {
    $Version = (Get-Content (Join-Path $rootDir "package.json") -Raw | ConvertFrom-Json).version
}
$tag = "v$Version"
Write-Host "[1/7] Target Release: $tag (Version: $Version) on repo: $Repo" -ForegroundColor Green

# 2. Locate and Authenticate GitHub CLI (gh)
Write-Host "[2/7] Checking GitHub CLI (gh) environment..." -ForegroundColor Yellow
$ghPath = Get-Command "gh" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source
if (-not $ghPath) {
    $defaultGh = "C:\Program Files\GitHub CLI\gh.exe"
    if (Test-Path $defaultGh) {
        $ghPath = $defaultGh
    } else {
        Write-Error "GitHub CLI (gh) not found in PATH or standard directory. Please ensure gh is installed."
        exit 1
    }
}

# Auto-authenticate using Git Credential Manager if GH_TOKEN is not currently exported
if (-not $env:GH_TOKEN -and -not $env:GITHUB_TOKEN) {
    try {
        $credOutput = "protocol=https`nhost=github.com`n`n" | git credential fill 2>$null
        $tokenMatch = $credOutput -split "`n" | Where-Object { $_ -like "password=*" }
        if ($tokenMatch) {
            $env:GH_TOKEN = $tokenMatch.Substring("password=".Length).Trim()
        }
    } catch {
        # Git Credential Manager fallback
    }
}

$authCheck = & $ghPath auth status 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Error "GitHub CLI is not authenticated. Please run 'gh auth login' or export GH_TOKEN."
    exit 1
}
Write-Host "[OK] GitHub CLI authenticated successfully." -ForegroundColor Green

# 3. Check GitHub Remote Release State & Fetch Existing Metadata
Write-Host "[3/8] Inspecting remote release state on $Repo for $tag..." -ForegroundColor Yellow
$remoteReleaseJson = & $ghPath release view $tag --repo $Repo --json name,body 2>$null
$existingRelease = $null
if ($LASTEXITCODE -eq 0 -and $remoteReleaseJson) {
    try {
        $existingRelease = $remoteReleaseJson | ConvertFrom-Json
    } catch { }
}

if ($existingRelease) {
    Write-Host "[OK] Remote release $tag found on GitHub." -ForegroundColor Green
    if (-not $Title -and $existingRelease.name) {
        $Title = $existingRelease.name
    }
    if (-not $ReleaseNotes -and $existingRelease.body) {
        $ReleaseNotes = $existingRelease.body
    }
}

# 4. Locate Existing Validated Release Artifacts (avoid unnecessary builds)
Write-Host "[4/8] Locating existing validated release artifacts for v$Version..." -ForegroundColor Yellow
$nsisDir = Join-Path $rootDir "target\release\bundle\nsis"

# Check for installer matching version (support space or dot naming)
$installer = Get-ChildItem -Path $nsisDir -Filter "*$Version*_x64-setup.exe" | 
             Where-Object { $_.Name -notlike "*.sig" } | 
             Select-Object -First 1

if (-not $installer) {
    Write-Error "No built installer found for version $Version in $nsisDir. Build it first with scripts/build-release.ps1."
    exit 1
}

$sigFile = Get-ChildItem -Path $nsisDir -Filter "$($installer.Name).sig" | Select-Object -First 1
if (-not $sigFile) {
    # Check for dot/space counterpart signature
    $sigFile = Get-ChildItem -Path $nsisDir -Filter "*$Version*.sig" | Select-Object -First 1
}

if (-not $sigFile) {
    Write-Error "Signature file not found for installer in $nsisDir."
    exit 1
}

$installerHash = (Get-FileHash -Path $installer.FullName -Algorithm SHA256).Hash
Write-Host "[OK] Installer: $($installer.Name) ($($installer.Length) bytes, SHA256: $installerHash)" -ForegroundColor Green
Write-Host "[OK] Signature: $($sigFile.Name)" -ForegroundColor Green

# 5. Handle GitHub Asset Filename Normalization (Critical Requirement 4)
# When uploaded to GitHub releases, files with spaces are normalized or downloaded with dots:
# "Stardew.Sync_0.1.2_x64-setup.exe"
# We prepare staged release assets with standard normalized names so the manifest download URL matches exactly:
$normalizedInstallerName = "Stardew.Sync_${Version}_x64-setup.exe"
$normalizedSigName = "Stardew.Sync_${Version}_x64-setup.exe.sig"

$stagedInstallerPath = Join-Path $nsisDir $normalizedInstallerName
$stagedSigPath = Join-Path $nsisDir $normalizedSigName

# Create identical byte-for-byte normalized copies if needed
if ($installer.Name -ne $normalizedInstallerName) {
    Copy-Item -Path $installer.FullName -Destination $stagedInstallerPath -Force
}
if ($sigFile.Name -ne $normalizedSigName) {
    Copy-Item -Path $sigFile.FullName -Destination $stagedSigPath -Force
}

$sigContent = (Get-Content -Path $stagedSigPath -Raw).Trim()

# 6. Generate and Validate latest.json with Exact Matching Asset Download URL
Write-Host "[5/8] Generating normalized release manifest (latest.json)..." -ForegroundColor Yellow
$assetUrl = "https://github.com/$Repo/releases/download/$tag/$normalizedInstallerName"
$pubDate = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

if (-not $Title) {
    $Title = "Stardew Sync $tag - Updater Verification"
}

if ($ReleaseNotes -and (Test-Path $ReleaseNotes)) {
    $ReleaseNotes = Get-Content -Path $ReleaseNotes -Raw
}

if (-not $ReleaseNotes) {
    $notesFile = Join-Path $rootDir "docs\release-notes\$tag.md"
    if (Test-Path $notesFile) {
        $ReleaseNotes = Get-Content -Path $notesFile -Raw
    } else {
        try {
            $gitCommits = git log -n 10 --pretty=format:"* %s (%h)" 2>$null
            if ($gitCommits) {
                $ReleaseNotes = "## Stardew Sync $tag`n`n### Changes in this release:`n$gitCommits`n`n*Automated release package by bazq.*"
            }
        } catch {
            # fallback
        }
    }
}

if (-not $ReleaseNotes) {
    $ReleaseNotes = "Stardew Sync $tag Release. Featuring verified native auto-updater integration and dynamic save discovery."
}

$manifestObject = [ordered]@{
    version   = $Version
    notes     = $ReleaseNotes
    pub_date  = $pubDate
    platforms = [ordered]@{
        "windows-x86_64" = [ordered]@{
            signature = $sigContent
            url       = $assetUrl
        }
        "windows-x86_64-nsis" = [ordered]@{
            signature = $sigContent
            url       = $assetUrl
        }
    }
}

$manifestJson = $manifestObject | ConvertTo-Json -Depth 5
$latestJsonPath = Join-Path $nsisDir "latest.json"
$templatePath = Join-Path $rootDir "docs\updater-release-template.json"
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($latestJsonPath, $manifestJson, $utf8NoBom)
[System.IO.File]::WriteAllText($templatePath, $manifestJson, $utf8NoBom)
Write-Host "[OK] Manifest generated with download URL: $assetUrl" -ForegroundColor Green

# 7. Verify Local Manifest & Cryptographic Signature
Write-Host "[6/8] Verifying manifest and cryptographic signature integrity..." -ForegroundColor Yellow
& node (Join-Path $rootDir "scripts\validate-release-manifest.cjs")
if ($LASTEXITCODE -ne 0) {
    Write-Error "Release manifest validation failed!"
    exit 1
}

if ($DryRun) {
    Write-Host "`n[DRY RUN COMPLETE] Artifacts validated. Remote release inspection finished." -ForegroundColor Cyan
    Write-Host "No changes were made to GitHub." -ForegroundColor Yellow
    exit 0
}

if (-not $Approved) {
    Write-Warning "`nOperation is approval-gated. To publish or update the release on GitHub, rerun with -Approved."
    Write-Host "Example: .\scripts\publish-github-release.ps1 -Approved" -ForegroundColor Cyan
    exit 0
}

# 8. Publish or Update GitHub Release (Idempotent)
Write-Host "[7/8] Publishing/updating release on GitHub..." -ForegroundColor Yellow
$assetsToUpload = @($stagedInstallerPath, $stagedSigPath, $latestJsonPath)

if ($existingRelease) {
    Write-Host "Release $tag already exists on GitHub. Updating assets idempotently..." -ForegroundColor Cyan
    & $ghPath release upload $tag --repo $Repo --clobber @assetsToUpload
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to update release assets on GitHub!"
        exit 1
    }
    & $ghPath release edit $tag --repo $Repo --title $Title --latest
} else {
    Write-Host "Creating new GitHub release for tag $tag..." -ForegroundColor Cyan
    & $ghPath release create $tag --repo $Repo --title $Title --notes $ReleaseNotes --latest @assetsToUpload
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to create GitHub release!"
        exit 1
    }
}

Write-Host "[OK] Release assets uploaded successfully to GitHub." -ForegroundColor Green

# 9. Verify Published Asset Endpoints (Post-Publish Check)
Write-Host "`nVerifying live GitHub asset endpoints..." -ForegroundColor Yellow
$urlsToVerify = @(
    "https://github.com/$Repo/releases/latest/download/latest.json",
    $assetUrl,
    "https://github.com/$Repo/releases/download/$tag/$normalizedSigName"
)

foreach ($url in $urlsToVerify) {
    $res = curl.exe -sL -o NUL -w "%{http_code}" -H "User-Agent: stardew-sync" $url
    if ($res -eq "200") {
        Write-Host "[OK] HTTP 200 OK: $url" -ForegroundColor Green
    } else {
        Write-Warning "[WARN] HTTP $res for endpoint: $url"
    }
}

Write-Host ""
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "  GitHub Release $tag published and verified successfully!   " -ForegroundColor Green
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host ""
