/**
 * Release Manifest & Asset Validation Script
 * Verifies version consistency, GitHub URLs, signature integrity, and asset matches.
 * Author: bazq
 */

const fs = require('fs');
const path = require('path');

function validate() {
  const rootDir = path.resolve(__dirname, '..');
  console.log('[bazq-validator] Starting Stardew Sync release validation...');

  // 1. Read version sources
  const packageJson = JSON.parse(fs.readFileSync(path.join(rootDir, 'package.json'), 'utf8'));
  const tauriConf = JSON.parse(fs.readFileSync(path.join(rootDir, 'src-tauri', 'tauri.conf.json'), 'utf8'));
  const cargoToml = fs.readFileSync(path.join(rootDir, 'src-tauri', 'Cargo.toml'), 'utf8');
  const cargoVersionMatch = cargoToml.match(/version\s*=\s*"([^"]+)"/);
  const cargoVersion = cargoVersionMatch ? cargoVersionMatch[1] : null;

  const version = packageJson.version;
  console.log(`[bazq-validator] Target Version: ${version}`);

  if (tauriConf.version !== version) {
    throw new Error(`tauri.conf.json version (${tauriConf.version}) does not match package.json (${version})`);
  }
  if (cargoVersion !== version) {
    throw new Error(`Cargo.toml version (${cargoVersion}) does not match package.json (${version})`);
  }
  console.log('✓ Version consistency verified across package.json, tauri.conf.json, and Cargo.toml.');

  // 2. Read manifest
  const manifestPath = path.join(rootDir, 'docs', 'updater-release-template.json');
  if (!fs.existsSync(manifestPath)) {
    throw new Error(`Manifest not found at ${manifestPath}`);
  }
  const rawManifest = fs.readFileSync(manifestPath, 'utf8').replace(/^\uFEFF/, '');
  const manifest = JSON.parse(rawManifest);

  if (manifest.version !== version) {
    throw new Error(`Manifest version (${manifest.version}) does not match app version (${version})`);
  }
  if (!manifest.notes || manifest.notes.trim().length === 0) {
    throw new Error('Manifest missing release notes');
  }
  if (!manifest.pub_date || isNaN(Date.parse(manifest.pub_date))) {
    throw new Error(`Manifest pub_date is invalid: ${manifest.pub_date}`);
  }
  console.log('✓ Manifest core metadata (version, notes, pub_date) verified.');

  // 3. Verify platform targets
  const platforms = manifest.platforms;
  if (!platforms || (!platforms['windows-x86_64'] && !platforms['windows-x86_64-nsis'])) {
    throw new Error('Manifest missing windows-x86_64 platform configuration');
  }

  if (platforms['windows-x86_64'] && platforms['windows-x86_64-nsis']) {
    if (platforms['windows-x86_64'].url !== platforms['windows-x86_64-nsis'].url) {
      throw new Error('Platform URL mismatch between windows-x86_64 and windows-x86_64-nsis');
    }
    if (platforms['windows-x86_64'].signature !== platforms['windows-x86_64-nsis'].signature) {
      throw new Error('Platform signature mismatch between windows-x86_64 and windows-x86_64-nsis');
    }
    console.log('✓ Platform parity confirmed: windows-x86_64 and windows-x86_64-nsis share identical URL and signature.');
  }

  const winPlatform = platforms['windows-x86_64'] || platforms['windows-x86_64-nsis'];
  if (!winPlatform.url || !winPlatform.signature) {
    throw new Error('Windows platform missing url or signature field');
  }

  // 4. Verify URL structure and GitHub owner
  const expectedRepo = 'https://github.com/bazq3D/stardew-sync';
  const expectedPrefix = `${expectedRepo}/releases/download/v${version}/`;
  if (!winPlatform.url.startsWith(expectedPrefix)) {
    throw new Error(`Manifest URL (${winPlatform.url}) must start with ${expectedPrefix}`);
  }

  const encodedFilename = winPlatform.url.slice(expectedPrefix.length);
  const decodedFilename = decodeURIComponent(encodedFilename);
  console.log(`[bazq-validator] Manifest asset filename: "${decodedFilename}" (URL encoded: "${encodedFilename}")`);

  // 5. Verify actual installer file exists, is non-empty, and is the sole installer for this version
  const bundleDir = path.join(rootDir, 'target', 'release', 'bundle', 'nsis');
  if (!fs.existsSync(bundleDir)) {
    throw new Error(`Required bundle output directory not found: ${bundleDir}`);
  }

  const expectedInstallerPath = path.join(bundleDir, decodedFilename);
  if (!fs.existsSync(expectedInstallerPath)) {
    throw new Error(`Authoritative installer artifact not found at: ${expectedInstallerPath}`);
  }
  const installerStats = fs.statSync(expectedInstallerPath);
  if (installerStats.size === 0) {
    throw new Error(`Installer artifact is empty (0 bytes): ${expectedInstallerPath}`);
  }

  // Reject unexpected additional or ambiguous installers for the release version
  const versionExes = fs.readdirSync(bundleDir).filter(f => f.toLowerCase().endsWith('.exe') && f.includes(version));
  if (versionExes.length !== 1) {
    throw new Error(`Security Halt: Expected exactly 1 installer .exe for version ${version} in bundle, found ${versionExes.length}: ${versionExes.join(', ')}`);
  }
  if (versionExes[0] !== decodedFilename) {
    throw new Error(`Security Halt: Found installer (${versionExes[0]}) does not match canonical manifest filename (${decodedFilename})`);
  }

  if (process.env.CI) {
    const allExes = fs.readdirSync(bundleDir).filter(f => f.toLowerCase().endsWith('.exe'));
    if (allExes.length !== 1) {
      throw new Error(`CI Security Halt: Found multiple installer executables in bundle: ${allExes.join(', ')}`);
    }
  }
  console.log(`✓ Sole authoritative installer for v${version} verified: ${decodedFilename} (${installerStats.size} bytes)`);

  // 6. Verify signature structure & trusted comment
  const sigRaw = Buffer.from(winPlatform.signature, 'base64').toString('utf8');
  if (!sigRaw.includes('untrusted comment:') || !sigRaw.includes('trusted comment:')) {
    throw new Error('Signature is not a valid Minisign signature format');
  }
  if (!sigRaw.includes(`version:${version}`)) {
    throw new Error(`Signature trusted comment does not contain version:${version}`);
  }
  const spaceFilename = decodedFilename.replace(/^Stardew\.Sync_/, 'Stardew Sync_');
  const dotFilename = decodedFilename.replace(/^Stardew Sync_/, 'Stardew.Sync_');
  const hasFileMatch = sigRaw.includes(`file:${decodedFilename}`) || 
                       sigRaw.includes(`file:${spaceFilename}`) || 
                       sigRaw.includes(`file:${dotFilename}`);
  if (!hasFileMatch) {
    throw new Error(`Signature trusted comment does not match filename file:${decodedFilename}`);
  }
  console.log('✓ Minisign signature format, version binding, and asset name verified.');

  // 7. Verify mandatory .sig file on disk matches manifest signature
  const sigPath = path.join(bundleDir, `${decodedFilename}.sig`);
  if (!fs.existsSync(sigPath)) {
    throw new Error(`Mandatory signature file not found on disk: ${sigPath}`);
  }
  const sigStats = fs.statSync(sigPath);
  if (sigStats.size === 0) {
    throw new Error(`Signature file on disk is empty (0 bytes): ${sigPath}`);
  }

  // Reject unexpected additional signature files for the release version
  const versionSigs = fs.readdirSync(bundleDir).filter(f => f.toLowerCase().endsWith('.sig') && f.includes(version));
  if (versionSigs.length !== 1) {
    throw new Error(`Security Halt: Expected exactly 1 .sig file for version ${version} in bundle, found ${versionSigs.length}: ${versionSigs.join(', ')}`);
  }
  if (versionSigs[0] !== `${decodedFilename}.sig`) {
    throw new Error(`Security Halt: Signature file (${versionSigs[0]}) does not match expected (${decodedFilename}.sig)`);
  }

  if (process.env.CI) {
    const allSigs = fs.readdirSync(bundleDir).filter(f => f.toLowerCase().endsWith('.sig'));
    if (allSigs.length !== 1) {
      throw new Error(`CI Security Halt: Found multiple signature files in bundle: ${allSigs.join(', ')}`);
    }
  }

  const diskSig = fs.readFileSync(sigPath, 'utf8').trim();
  if (diskSig !== winPlatform.signature.trim()) {
    throw new Error('Signature in manifest does not match .sig file on disk');
  }
  console.log('✓ Mandatory disk signature file exists, is unique, and exactly matches manifest signature.');

  // 8. Verify public key configured in tauri.conf.json
  const pubkeyB64 = tauriConf.plugins?.updater?.pubkey;
  if (!pubkeyB64) {
    throw new Error('tauri.conf.json missing plugins.updater.pubkey');
  }
  const pubkeyRaw = Buffer.from(pubkeyB64, 'base64').toString('utf8');
  if (!pubkeyRaw.includes('untrusted comment: minisign public key:')) {
    throw new Error('tauri.conf.json pubkey is not a valid Minisign public key');
  }
  console.log('✓ Public key configured in tauri.conf.json is valid Minisign key.');

  // 9. Synchronize validated manifest to target bundle directory for release upload
  const targetLatestJson = path.join(bundleDir, 'latest.json');
  fs.writeFileSync(targetLatestJson, JSON.stringify(manifest, null, 2), 'utf8');
  if (!fs.existsSync(targetLatestJson) || fs.statSync(targetLatestJson).size === 0) {
    throw new Error(`Failed to create non-empty target release manifest at: ${targetLatestJson}`);
  }
  console.log(`✓ Synchronized and verified target release manifest at: ${targetLatestJson}`);

  console.log('\n============================================================');
  console.log(`[bazq-validator] RELEASE VALIDATION PASSED FOR v${version}`);
  console.log('============================================================\n');
}

try {
  validate();
} catch (err) {
  console.error('\n❌ [bazq-validator] RELEASE VALIDATION FAILED:', err.message);
  process.exit(1);
}
