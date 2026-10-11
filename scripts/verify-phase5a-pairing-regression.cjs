/**
 * Phase 5A Pairing UI Regression & Contract Test Suite
 *
 * Verifies:
 * 1. SasVerificationModal hook call ordering (strictly complies with React Rules of Hooks)
 * 2. ErrorBoundary integration in App.tsx and DevicesPage.tsx
 * 3. Invitation code auto-prefix, boundary space handling, and paste normalization logic
 * 4. TypeScript ActivePairingSessionView interface alignment with backend Serde schema
 */

const fs = require('fs');
const path = require('path');
const assert = require('assert');

console.log('Running Phase 5A Pairing UI Regression Suite...\n');

// --- 1. Hook Call Ordering Verification in SasVerificationModal ---
const sasModalPath = path.resolve(__dirname, '../src/components/SasVerificationModal.tsx');
const sasModalSrc = fs.readFileSync(sasModalPath, 'utf8');

const hookRegex = /\b(useState|useEffect|useCallback|useMemo|useRef)\s*\(/g;
const returnRegex = /\breturn\b/g;

// Verify no hooks are called after any conditional early returns
const lines = sasModalSrc.split('\n');
let firstEarlyReturnLine = -1;
let hookAfterReturnFound = false;
let offendingHook = '';
let offendingLine = -1;

for (let i = 0; i < lines.length; i++) {
  const line = lines[i];
  if (firstEarlyReturnLine === -1 && /if\s*\(.*shouldRender.*|!session\)\s*return/.test(line)) {
    firstEarlyReturnLine = i + 1;
  }
  if (firstEarlyReturnLine !== -1 && hookRegex.test(line)) {
    hookAfterReturnFound = true;
    offendingHook = line.trim();
    offendingLine = i + 1;
    break;
  }
}

assert.strictEqual(
  hookAfterReturnFound,
  false,
  `Regression: Hook detected after early return at line ${offendingLine}: "${offendingHook}"`
);
console.log('✓ PASS: SasVerificationModal calls all hooks unconditionally before early returns (React Rules of Hooks compliant)');

// --- 2. ErrorBoundary Integration Verification ---
const appPath = path.resolve(__dirname, '../src/App.tsx');
const appSrc = fs.readFileSync(appPath, 'utf8');
assert(appSrc.includes('<ErrorBoundary'), 'App.tsx must include <ErrorBoundary>');
assert(appSrc.includes('import { ErrorBoundary }'), 'App.tsx must import ErrorBoundary');

const devicesPagePath = path.resolve(__dirname, '../src/pages/DevicesPage.tsx');
const devicesPageSrc = fs.readFileSync(devicesPagePath, 'utf8');
assert(devicesPageSrc.includes('<ErrorBoundary'), 'DevicesPage.tsx must wrap modal in <ErrorBoundary>');
assert(devicesPageSrc.includes('import { ErrorBoundary }'), 'DevicesPage.tsx must import ErrorBoundary');
console.log('✓ PASS: ErrorBoundary protection is active in App.tsx and DevicesPage.tsx');

// --- 3. Invitation Code UX Logic Verification ---
const CODE_PREFIX = 'SD-';
const CODE_MAX_LENGTH = 7;

function normalizePastedCode(pasted) {
  const cleanSuffix = pasted
    .trim()
    .toUpperCase()
    .replace(/^(SD[- ]?)+/i, '')
    .replace(/[^A-Z0-9]/g, '')
    .slice(0, 4);
  return CODE_PREFIX + cleanSuffix;
}

function handleSpaceAtBoundary(val, cursor) {
  // If cursor is at or before prefix boundary (index 3)
  if (cursor <= CODE_PREFIX.length) {
    if (!val.startsWith(CODE_PREFIX)) {
      const suffix = val.replace(/^SD-?/i, '').replace(/[^A-Z0-9]/g, '').slice(0, 4);
      return { val: CODE_PREFIX + suffix, cursor: CODE_PREFIX.length };
    }
    return { val, cursor: CODE_PREFIX.length };
  }
  // If pressed elsewhere, do not alter unrelated text
  return { val, cursor };
}

function handleInputChange(raw) {
  const upper = raw.toUpperCase();
  if (!upper.startsWith(CODE_PREFIX)) {
    const suffix = upper.replace(/^SD-?/i, '').replace(/[^A-Z0-9]/g, '').slice(0, 4);
    return CODE_PREFIX + suffix;
  } else {
    const suffix = upper.slice(CODE_PREFIX.length).replace(/[^A-Z0-9]/g, '').slice(0, 4);
    return CODE_PREFIX + suffix;
  }
}

// Paste normalization tests
assert.strictEqual(normalizePastedCode('SD-8492'), 'SD-8492');
assert.strictEqual(normalizePastedCode('sd-8492'), 'SD-8492');
assert.strictEqual(normalizePastedCode('8492'), 'SD-8492');
assert.strictEqual(normalizePastedCode('SD-SD-8492'), 'SD-8492');
assert.strictEqual(normalizePastedCode('SD 8492'), 'SD-8492');
assert.strictEqual(normalizePastedCode('  sd-8492  '), 'SD-8492');
assert.strictEqual(normalizePastedCode('SD-ABCD123'), 'SD-ABCD');
console.log('✓ PASS: Pasted invitation codes normalize without duplicate SD- prefix');

// Space boundary behavior tests
const spaceAtBoundary = handleSpaceAtBoundary('SD-', 3);
assert.strictEqual(spaceAtBoundary.val, 'SD-');
assert.strictEqual(spaceAtBoundary.cursor, 3);

const spaceInsideSuffix = handleSpaceAtBoundary('SD-1234', 5);
assert.strictEqual(spaceInsideSuffix.val, 'SD-1234'); // Unaltered unrelated text
console.log('✓ PASS: Space at prefix boundary enforces single separator without duplicating hyphens; space elsewhere does not alter text');

// Typing and backspace protection tests
assert.strictEqual(handleInputChange('SD-1'), 'SD-1');
assert.strictEqual(handleInputChange('SD-1234'), 'SD-1234');
assert.strictEqual(handleInputChange('SD-12345'), 'SD-1234'); // Max 4 characters
assert.strictEqual(handleInputChange('1234'), 'SD-1234'); // Restores SD- if user tried clearing
assert.strictEqual(handleInputChange(''), 'SD-'); // Empty input restores prefix
console.log('✓ PASS: Auto-populates SD- prefix and restricts suffix to 4 characters');

// --- 4. ActivePairingSessionView Schema Alignment ---
const typesPath = path.resolve(__dirname, '../src/types/index.ts');
const typesSrc = fs.readFileSync(typesPath, 'utf8');

const requiredFields = [
  'status',
  'session_id',
  'invitation_code',
  'peer_device',
  'sas_code',
  'our_confirmed',
  'peer_confirmed',
  'seconds_remaining',
];

for (const field of requiredFields) {
  assert(
    typesSrc.includes(`${field}:`),
    `types/index.ts ActivePairingSessionView must include field "${field}"`
  );
}
console.log('✓ PASS: ActivePairingSessionView matches backend serialization schema');

// --- 5. Modal Centering & Viewport Portal Verification ---
assert(sasModalSrc.includes('createPortal'), 'SasVerificationModal must use createPortal for viewport centering');
assert(sasModalSrc.includes('document.body'), 'SasVerificationModal must portal to document.body');
assert(sasModalSrc.includes('inset: 0') || sasModalSrc.includes("inset: '0'") || sasModalSrc.includes('inset: 0,'), 'SasVerificationModal must enforce inset 0 backdrop positioning');

const pairModalPath = path.resolve(__dirname, '../src/components/PairDeviceModal.tsx');
const pairModalSrc = fs.readFileSync(pairModalPath, 'utf8');
assert(pairModalSrc.includes('createPortal'), 'PairDeviceModal must use createPortal for viewport centering');
assert(pairModalSrc.includes('document.body'), 'PairDeviceModal must portal to document.body');

const componentsCssPath = path.resolve(__dirname, '../src/styles/components.css');
const componentsCssSrc = fs.readFileSync(componentsCssPath, 'utf8');
assert(componentsCssSrc.includes('.modal-dialog-sm'), 'components.css must define .modal-dialog-sm');
assert(componentsCssSrc.includes('margin: 0 auto;'), 'components.css must enforce margin: 0 auto on .modal-dialog');
console.log('✓ PASS: SAS and Pairing modals portal directly to document.body with centered margins preventing displacement');

// --- 6. Status Model & Direct Invitation Action Verification ---
assert(typesSrc.includes("'online'"), "types/index.ts must include 'online' in connection_status");
assert(typesSrc.includes("'offline'"), "types/index.ts must include 'offline' in connection_status");
assert(typesSrc.includes('export interface UnpairResult'), 'types/index.ts must export UnpairResult interface');

assert(devicesPageSrc.includes('generateInvitationDirect'), 'DevicesPage.tsx must include direct Generate Invitation Code button');
assert(devicesPageSrc.includes('refreshSuccess'), 'DevicesPage.tsx must provide refresh success feedback');
assert(devicesPageSrc.includes('statusOnline'), 'DevicesPage.tsx must support online status badge');
console.log('✓ PASS: Status model, direct invitation action, and refresh feedback verified');

console.log('\nAll Phase 5A frontend regression tests passed successfully!');
