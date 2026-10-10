/**
 * Authoritative Release Asset Verification Script
 * Validates installer bytes, signature match, latest.json parity, and exports exact paths.
 * Author: bazq
 */

const fs = require('fs');
const path = require('path');

function verifyReleaseAssets() {
  const rootDir = path.resolve(__dirname, '..');
  const pkg = JSON.parse(fs.readFileSync(path.join(rootDir, 'package.json'), 'utf8'));
  const version = pkg.version;
  const bundleDir = path.join(rootDir, 'target', 'release', 'bundle', 'nsis');

  console.log(`[bazq-asset-verifier] Verifying release bundle for v${version}...`);

  if (!fs.existsSync(bundleDir)) {
    throw new Error(`Bundle output directory not found: ${bundleDir}`);
  }

  // 1. Verify latest.json manifest
  const latestJsonPath = path.join(bundleDir, 'latest.json');
  if (!fs.existsSync(latestJsonPath)) {
    throw new Error(`Mandatory release manifest missing: ${latestJsonPath}`);
  }
  const manifestStats = fs.statSync(latestJsonPath);
  if (manifestStats.size === 0) {
    throw new Error(`Release manifest is empty (0 bytes): ${latestJsonPath}`);
  }

  const manifest = JSON.parse(fs.readFileSync(latestJsonPath, 'utf8'));
  if (manifest.version !== version) {
    throw new Error(`Manifest version (${manifest.version}) does not match package.json (${version})`);
  }

  // 2. Identify canonical installer from manifest
  const platformEntry = manifest.platforms?.['windows-x86_64'];
  if (!platformEntry || !platformEntry.url || !platformEntry.signature) {
    throw new Error('Manifest missing platforms.windows-x86_64 configuration');
  }

  const rawUrl = platformEntry.url;
  const canonicalName = decodeURIComponent(rawUrl.split('/').pop());
  const expectedPrefix = `https://github.com/bazq3D/stardew-sync/releases/download/v${version}/`;
  if (!rawUrl.startsWith(expectedPrefix)) {
    throw new Error(`Manifest download URL (${rawUrl}) does not match canonical release prefix (${expectedPrefix})`);
  }

  // 3. Verify exact installer exists and is non-empty
  const installerPath = path.join(bundleDir, canonicalName);
  if (!fs.existsSync(installerPath)) {
    throw new Error(`Authoritative installer not found on disk: ${installerPath}`);
  }
  const installerStats = fs.statSync(installerPath);
  if (installerStats.size === 0) {
    throw new Error(`Installer file is empty (0 bytes): ${installerPath}`);
  }

  // 4. Verify exact signature file exists, is non-empty, and matches manifest
  const sigPath = path.join(bundleDir, `${canonicalName}.sig`);
  if (!fs.existsSync(sigPath)) {
    throw new Error(`Authoritative signature file not found on disk: ${sigPath}`);
  }
  const sigStats = fs.statSync(sigPath);
  if (sigStats.size === 0) {
    throw new Error(`Signature file is empty (0 bytes): ${sigPath}`);
  }

  const diskSig = fs.readFileSync(sigPath, 'utf8').trim();
  if (diskSig !== platformEntry.signature.trim()) {
    throw new Error('Disk signature does not match manifest signature');
  }

  // 5. Fail closed on unexpected additional or ambiguous installers
  const versionExes = fs.readdirSync(bundleDir).filter(f => f.toLowerCase().endsWith('.exe') && f.includes(version));
  if (versionExes.length !== 1 || versionExes[0] !== canonicalName) {
    throw new Error(`Security Halt: Found ambiguous installer executables for v${version}: ${versionExes.join(', ')}. Expected only: ${canonicalName}`);
  }

  const versionSigs = fs.readdirSync(bundleDir).filter(f => f.toLowerCase().endsWith('.sig') && f.includes(version));
  if (versionSigs.length !== 1 || versionSigs[0] !== `${canonicalName}.sig`) {
    throw new Error(`Security Halt: Found ambiguous signature files for v${version}: ${versionSigs.join(', ')}. Expected only: ${canonicalName}.sig`);
  }

  if (process.env.CI) {
    const allExes = fs.readdirSync(bundleDir).filter(f => f.toLowerCase().endsWith('.exe'));
    if (allExes.length !== 1) {
      throw new Error(`CI Security Halt: Unexpected extra installers present in bundle: ${allExes.join(', ')}`);
    }
  }

  console.log(`✓ Authoritative installer verified: ${canonicalName} (${installerStats.size} bytes)`);
  console.log(`✓ Authoritative signature verified: ${canonicalName}.sig (${sigStats.size} bytes)`);
  console.log(`✓ Authoritative manifest verified:  latest.json (v${manifest.version})`);

  // 6. Export exact paths to GITHUB_OUTPUT for upload
  if (process.env.GITHUB_OUTPUT) {
    const normalizedInstaller = installerPath.replace(/\\/g, '/');
    const normalizedSig = sigPath.replace(/\\/g, '/');
    const normalizedManifest = latestJsonPath.replace(/\\/g, '/');
    const outputLines = [
      `installer=${normalizedInstaller}`,
      `sig=${normalizedSig}`,
      `manifest=${normalizedManifest}`,
      `installer_name=${canonicalName}`,
      ''
    ].join('\n');
    fs.appendFileSync(process.env.GITHUB_OUTPUT, outputLines, 'utf8');
    console.log(`[bazq-asset-verifier] Exported verified asset paths to GITHUB_OUTPUT`);
  }
}

try {
  verifyReleaseAssets();
} catch (err) {
  console.error('\n❌ [bazq-asset-verifier] ASSET VERIFICATION FAILED:', err.message);
  process.exit(1);
}
