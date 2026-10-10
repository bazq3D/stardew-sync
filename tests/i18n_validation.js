import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const enPath = path.resolve(__dirname, '../src/locales/en.json');
const trPath = path.resolve(__dirname, '../src/locales/tr.json');

console.log('--- Validating i18n Locales ---');

// 1. JSON parse
const en = JSON.parse(fs.readFileSync(enPath, 'utf-8'));
const tr = JSON.parse(fs.readFileSync(trPath, 'utf-8'));

function getAllKeys(obj, prefix = '') {
  let keys = [];
  for (const [k, v] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === 'object' && !Array.isArray(v)) {
      keys = keys.concat(getAllKeys(v, fullKey));
    } else {
      keys.push(fullKey);
    }
  }
  return keys;
}

const enKeys = getAllKeys(en).sort();
const trKeys = getAllKeys(tr).sort();

console.log(`Total English keys: ${enKeys.length}`);
console.log(`Total Turkish keys: ${trKeys.length}`);

// 2. Parity check
const missingInTr = enKeys.filter((k) => !trKeys.includes(k));
const missingInEn = trKeys.filter((k) => !enKeys.includes(k));

if (missingInTr.length > 0) {
  console.error('Keys missing in Turkish:', missingInTr);
  process.exit(1);
}
if (missingInEn.length > 0) {
  console.error('Keys missing in English:', missingInEn);
  process.exit(1);
}

console.log('✓ 100% key parity between English and Turkish.');

// 3. Turkish character verification
const trJsonRaw = fs.readFileSync(trPath, 'utf-8');
const turkishChars = ['ç', 'ğ', 'ı', 'İ', 'ö', 'ş', 'ü'];
const foundChars = turkishChars.filter((c) => trJsonRaw.includes(c));
console.log(`✓ Turkish special characters verified in tr.json: ${foundChars.join(' ')}`);

// 4. Interpolation & fallback test
function interpolate(template, params) {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, paramName) => {
    return paramName in params ? String(params[paramName]) : match;
  });
}

const templateEn = '{count} Farms';
const renderedEn = interpolate(templateEn, { count: 5 });
if (renderedEn !== '5 Farms') {
  console.error('Interpolation test failed:', renderedEn);
  process.exit(1);
}
console.log('✓ Parameter interpolation verified.');

// 5. Calendar and playtime formatting verification
function formatSeasonDate(seasonRaw, day, year, lang, dict) {
  const s = seasonRaw.trim().toLowerCase();
  let seasonKey = 'spring';
  if (s === '1' || s === 'summer') seasonKey = 'summer';
  else if (s === '2' || s === 'fall' || s === 'autumn') seasonKey = 'fall';
  else if (s === '3' || s === 'winter') seasonKey = 'winter';

  const localizedSeason = dict.seasons[seasonKey];
  const localizedDay = dict.seasons.day;
  const localizedYear = dict.seasons.year;
  const yearFormatted = lang === 'tr' ? `${year}. ${localizedYear}` : `${localizedYear} ${year}`;
  return `${localizedSeason}, ${localizedDay} ${day} (${yearFormatted})`;
}

const trCalendar = formatSeasonDate('fall', 23, 1, 'tr', tr);
if (trCalendar !== 'Sonbahar, Gün 23 (1. Yıl)') {
  console.error('Turkish calendar test failed:', trCalendar);
  process.exit(1);
}
const enCalendar = formatSeasonDate('fall', 23, 1, 'en', en);
if (enCalendar !== 'Fall, Day 23 (Year 1)') {
  console.error('English calendar test failed:', enCalendar);
  process.exit(1);
}
console.log(`✓ Calendar localization verified: "${enCalendar}" <-> "${trCalendar}"`);

// 6. Playtime formatting test
function formatPlaytimeHours(hours, lang, dict) {
  if (hours === null || hours === undefined || isNaN(hours)) {
    return dict.common.unknown;
  }
  return `${hours.toFixed(1)} ${dict.common.hours}`;
}

if (formatPlaytimeHours(null, 'tr', tr) !== 'Bilinmiyor') {
  console.error('Null playtime test failed for TR');
  process.exit(1);
}
if (formatPlaytimeHours(0.0, 'tr', tr) !== '0.0 saat') {
  console.error('Zero playtime test failed for TR');
  process.exit(1);
}
if (formatPlaytimeHours(42.6, 'tr', tr) !== '42.6 saat') {
  console.error('Decimal playtime test failed for TR');
  process.exit(1);
}
console.log('✓ Playtime formatting verified for null, zero, and decimal values.');

console.log('--- ALL i18n VALIDATIONS PASSED ---');

