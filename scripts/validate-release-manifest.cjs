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
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));

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

  // 5. Verify actual installer file exists and matches
  const expectedInstallerPath = path.join(rootDir, 'target', 'release', 'bundle', 'nsis', decodedFilename);
  if (!fs.existsSync(expectedInstallerPath)) {
    throw new Error(`Actual installer artifact not found at: ${expectedInstallerPath}`);
  }
  const installerStats = fs.statSync(expectedInstallerPath);
  console.log(`✓ Installer file exists: ${expectedInstallerPath} (${installerStats.size} bytes)`);

  // 6. Verify signature structure & trusted comment
  const sigRaw = Buffer.from(winPlatform.signature, 'base64').toString('utf8');
  if (!sigRaw.includes('untrusted comment:') || !sigRaw.includes('trusted comment:')) {
    throw new Error('Signature is not a valid Minisign signature format');
  }
  if (!sigRaw.includes(`version:${version}`)) {
    throw new Error(`Signature trusted comment does not contain version:${version}`);
  }
  if (!sigRaw.includes(`file:${decodedFilename}`)) {
    throw new Error(`Signature trusted comment does not match filename file:${decodedFilename}`);
  }
  console.log('✓ Minisign signature format, version binding, and asset name verified.');

  // 7. Verify .sig file on disk matches manifest signature
  const sigPath = path.join(rootDir, 'target', 'release', 'bundle', 'nsis', `${decodedFilename}.sig`);
  if (fs.existsSync(sigPath)) {
    const diskSig = fs.readFileSync(sigPath, 'utf8').trim();
    if (diskSig !== winPlatform.signature.trim()) {
      throw new Error('Signature in manifest does not match .sig file on disk');
    }
    console.log('✓ Disk signature file exactly matches manifest signature.');
  }

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
