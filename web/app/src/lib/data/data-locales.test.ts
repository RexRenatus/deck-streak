import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

// SPEC-516 R8, B8; ADR-534. Every string the data screen shows is a message key in each of the
// seven locale files: present, not empty, carrying every placeholder the English text carries, and
// never the English text itself outside English. The screen's own keys come first; then the keys
// it shares with the sync screen's backup list and the study screens, which it shows as they are.
const APP = resolve(import.meta.dirname, '../../..');

/** Every key the data screen shows of its own. */
const DATA_KEYS = [
  'data_title',
  'data_json_title',
  'data_json_about',
  'data_json_download',
  'data_json_saved',
  'data_json_failed',
  'data_json_reopen',
  'data_telegram_phone',
  'data_device_title',
  'data_device_about',
  'data_back_up_now',
  'data_backup_made',
  'data_working',
  'data_server_title',
  'data_server_age',
  'data_server_none',
  'data_server_unknown'
] as const;

/** The keys the data screen shows from the backup list it shares with the sync screen, and the
 * study screens' sentence for an engine that stopped. */
const SHARED_KEYS = [
  'sync_backups_title',
  'sync_backups_none',
  'sync_backup_device',
  'sync_backup_server',
  'sync_backup_export',
  'sync_backup_removed',
  'study_refused_engine'
] as const;

/** The source files whose message calls the census reads: the screen and the list it shows. */
const SOURCES = ['src/lib/data/DataScreen.svelte', 'src/lib/sync/BackupList.svelte'];

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** Every locale with a message file, by its name. */
function locales(): string[] {
  const path = join(APP, 'messages');
  if (!existsSync(path)) return [];
  return readdirSync(path)
    .filter((name) => name.endsWith('.json'))
    .map((name) => name.slice(0, -'.json'.length))
    .sort();
}

/** One locale's messages, parsed. */
function messages(locale: string): Record<string, unknown> {
  return JSON.parse(readFileSync(join(APP, 'messages', `${locale}.json`), 'utf8')) as Record<string, unknown>;
}

/** The placeholders a text carries, in order, as `{name}`. */
function placeholders(text: string): string[] {
  return [...text.matchAll(/\{[a-z]+\}/g)].map((found) => found[0]).sort();
}

/** What `held` owes the data screen and lacks, each named with `locale` and its key, judged against
 * the English messages `english`. */
function dataProblems(
  locale: string,
  held: Record<string, unknown>,
  english: Record<string, unknown>,
  keys: readonly string[] = [...DATA_KEYS, ...SHARED_KEYS]
): string[] {
  const problems: string[] = [];
  for (const key of keys) {
    const text = held[key];
    const source = english[key];
    if (typeof text !== 'string' || text.trim() === '') {
      problems.push(`${locale} lacks ${key}`);
      continue;
    }
    if (typeof source === 'string' && placeholders(text).join() !== placeholders(source).join()) {
      problems.push(`${locale}'s ${key} carries ${placeholders(text).join() || 'no placeholder'}, not ${placeholders(source).join()}`);
    }
    if (locale !== 'en' && text === source) problems.push(`${locale}'s ${key} holds the English text`);
  }
  return problems;
}

/** Every message key the screen's sources call, as `m.<key>(`. */
function called(): string[] {
  const keys = SOURCES.flatMap((source) =>
    [...readFileSync(join(APP, source), 'utf8').matchAll(/\bm\.([a-z_]+)\(/g)].map((found) => found[1])
  );
  return [...new Set(keys)].sort();
}

describe('every locale holds the data screen keys', () => {
  it('each of the seven locales holds every key the data screen shows', () => {
    const english = messages('en');
    const found = Object.fromEntries(locales().map((locale) => [locale, dataProblems(locale, messages(locale), english)]));
    expect(found).toEqual(Object.fromEntries(locales().map((locale) => [locale, []])));

    // each plant is refused by name
    const keys = ['data_title', 'data_server_age'];
    const source = { data_title: 'Your data', data_server_age: 'Made {age}' };
    expect(dataProblems('es', { data_title: 'Tus datos', data_server_age: 'Hecha {age}' }, source, keys)).toEqual([]);
    expect(dataProblems('ja', { data_title: 'データ' }, source, keys)).toEqual(['ja lacks data_server_age']);
    expect(dataProblems('fr', { data_title: 'Vos données', data_server_age: 'Faite' }, source, keys)).toEqual([
      "fr's data_server_age carries no placeholder, not {age}"
    ]);
    expect(dataProblems('ko', { data_title: '  ', data_server_age: '{age}' }, source, keys)).toEqual(['ko lacks data_title']);
    expect(dataProblems('zh-Hans', { data_title: 'Your data', data_server_age: '{age}' }, source, keys)).toEqual([
      "zh-Hans's data_title holds the English text"
    ]);

    examined('data key(s)', [...DATA_KEYS]);
    examined('shared key(s)', [...SHARED_KEYS]);
    examined('locale files under messages', locales());
    expect(locales()).toEqual(['en', 'es', 'fr', 'ja', 'ko', 'zh-Hans', 'zh-Hant']);
  });

  it('the keys the census holds are every key the screen and its list call', () => {
    // a string the screen shows outside a message key, or a key the census does not hold, would
    // escape the seven locales: the census's keys are exactly the keys the sources call
    const calls = examined('message call(s) in the data screen and its list', called());
    expect(calls).toEqual([...DATA_KEYS, ...SHARED_KEYS].sort());
  });
});
