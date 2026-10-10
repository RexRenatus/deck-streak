import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

// SPEC-377 R11, A15; ADR-388 D11. Every string the sync and choice screens show is a message key
// in each of the seven locale files: present, not empty, carrying every placeholder the English
// text carries, and never the English text itself outside English.
const APP = resolve(import.meta.dirname, '../../..');

/** Every key the sync screen and the choice screen show. */
const SYNC_KEYS = [
  'sync_title',
  'sync_user',
  'sync_password',
  'sync_sign_in',
  'sync_sign_out',
  'sync_now',
  'sync_working',
  'sync_status_absent',
  'sync_status_sealed',
  'sync_status_held',
  'sync_status_needs_sign_in',
  'sync_status_offline',
  'sync_unsynced',
  'sync_in_step',
  'sync_full_required',
  'sync_lost',
  'sync_choose',
  'sync_choice_upload',
  'sync_choice_download',
  'sync_choice_restore',
  'sync_choice_upload_loses',
  'sync_choice_download_loses',
  'sync_choice_snapshot_age',
  'sync_choice_snapshot_waits',
  'sync_choice_cancel',
  'sync_choice_counting',
  'sync_choice_changed_server',
  'sync_choice_changed_device',
  'sync_choice_written',
  'sync_choice_refused',
  'sync_backups_title',
  'sync_backups_none',
  'sync_backup_device',
  'sync_backup_server',
  'sync_backup_export',
  'sync_backup_removed',
  'sync_storage_persisted',
  'sync_storage_not_persisted',
  'sync_storage_unknown',
  'sync_storage_evicted'
] as const;

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

/** What `held` owes the sync screens and lacks, each named with `locale` and its key, judged
 * against the English messages `english`. */
function syncProblems(locale: string, held: Record<string, unknown>, english: Record<string, unknown>): string[] {
  const problems: string[] = [];
  for (const key of SYNC_KEYS) {
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

describe('every locale holds the sync screens keys', () => {
  it('each of the seven locales holds every key the screens use', () => {
    const english = messages('en');
    const found = Object.fromEntries(locales().map((locale) => [locale, syncProblems(locale, messages(locale), english)]));
    expect(found).toEqual(Object.fromEntries(locales().map((locale) => [locale, []])));

    // each plant is refused by name
    const source = Object.fromEntries(SYNC_KEYS.map((key) => [key, `${key} {count}`]));
    const spanish = Object.fromEntries(SYNC_KEYS.map((key) => [key, `es ${key} {count}`]));
    expect(syncProblems('es', spanish, source)).toEqual([]);
    const missing = Object.fromEntries(Object.entries(spanish).filter(([key]) => key !== 'sync_lost'));
    expect(syncProblems('ja', missing, source)).toEqual(['ja lacks sync_lost']);
    expect(syncProblems('fr', { ...spanish, sync_now: 'Synchroniser' }, source)).toEqual([
      "fr's sync_now carries no placeholder, not {count}"
    ]);
    expect(syncProblems('ko', { ...spanish, sync_title: '   ' }, source)).toEqual(['ko lacks sync_title']);
    expect(syncProblems('zh-Hans', { ...spanish, sync_title: source.sync_title }, source)).toEqual([
      "zh-Hans's sync_title holds the English text"
    ]);

    examined('sync key(s)', [...SYNC_KEYS]);
    examined('locale files under messages', locales());
    expect(locales()).toEqual(['en', 'es', 'fr', 'ja', 'ko', 'zh-Hans', 'zh-Hant']);
  });
});
