import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { afterEach, describe, expect, it } from 'vitest';
import { getLocale, locales, overwriteGetLocale } from '$lib/paraglide/runtime.js';
import { STATUS_KEYS, statusText } from './refusal';

// SPEC-350 R5, R7, R10, R11; ADR-361. Each status the review announces, every engine refusal, the
// card the frame refused and the done deck, names its own message, and the message is in every
// locale the app has.
const ORIGINAL = getLocale;
const MESSAGES = fileURLToPath(new URL('../../../messages/', import.meta.url));

/** Each status and its message's key, written out rather than derived. */
const EXPECTED = {
  'bad-request': 'study_refused_bad_request',
  'collection-busy': 'study_refused_collection_busy',
  'storage-refused': 'study_refused_storage',
  'engine-failed': 'study_refused_engine',
  'not-open': 'study_refused_not_open',
  'not-shown': 'study_refused_not_shown',
  'undo-synced': 'undo_synced',
  'not-undoable': 'undo_gone',
  escaped: 'study_card_escaped',
  withheld: 'study_card_withheld',
  done: 'study_done'
};

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

function messages(locale: string): Record<string, string> {
  return JSON.parse(readFileSync(`${MESSAGES}${locale}.json`, 'utf8')) as Record<string, string>;
}

describe('the review status', () => {
  afterEach(() => {
    overwriteGetLocale(ORIGINAL);
  });

  it('each refusal names its own message in every locale', () => {
    expect(STATUS_KEYS).toEqual(EXPECTED);
    const keys = Object.values(EXPECTED);
    expect(new Set(keys).size).toBe(keys.length);

    for (const locale of examined('locales', [...locales])) {
      const table = messages(locale);
      overwriteGetLocale(() => locale);
      for (const [status, key] of Object.entries(EXPECTED)) {
        expect(table[key]?.length ?? 0, `${locale} ${key}`).toBeGreaterThan(0);
        expect(statusText(status as keyof typeof EXPECTED), `${locale} ${status}`).toBe(table[key]);
      }
    }
  });
});
