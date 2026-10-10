/**
 * Release Tag Resolver and Version Validator
 * Author: bazq
 */

const fs = require('fs');
const path = require('path');

function resolveReleaseTag() {
  const rootDir = path.resolve(__dirname, '..');
  const pkg = JSON.parse(fs.readFileSync(path.join(rootDir, 'package.json'), 'utf8'));
  const expectedTag = `v${pkg.version}`;

  const ref = process.env.GITHUB_REF || '';
  const inputVersion = (process.env.INPUT_VERSION || '').trim();

  let tag = '';
  if (ref.startsWith('refs/tags/')) {
    tag = ref.replace('refs/tags/', '');
  } else if (inputVersion) {
    tag = inputVersion.startsWith('v') ? inputVersion : `v${inputVersion}`;
  } else if (process.env.RELEASE_TAG) {
    tag = process.env.RELEASE_TAG.trim();
  }

  if (!tag) {
    throw new Error('Failed to resolve release tag from GITHUB_REF, INPUT_VERSION, or RELEASE_TAG!');
  }

  // The release tag MUST match the built application version
  if (tag !== expectedTag) {
    throw new Error(`Release tag mismatch! Resolved tag (${tag}) does not match package.json version (${expectedTag})`);
  }

  console.log(`[bazq-tag-resolver] Release tag confirmed: ${tag} (Application version: ${pkg.version})`);

  // Write to GITHUB_OUTPUT if running inside GitHub Actions
  if (process.env.GITHUB_OUTPUT) {
    const outputLines = [
      `tag=${tag}`,
      `version=${pkg.version}`,
      ''
    ].join('\n');
    fs.appendFileSync(process.env.GITHUB_OUTPUT, outputLines, 'utf8');
    console.log(`[bazq-tag-resolver] Wrote outputs to GITHUB_OUTPUT`);
  }

  return { tag, version: pkg.version };
}

try {
  resolveReleaseTag();
} catch (err) {
  console.error('\n❌ [bazq-tag-resolver] TAG RESOLUTION FAILED:', err.message);
  process.exit(1);
}
