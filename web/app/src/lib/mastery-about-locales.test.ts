import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { locales } from '$lib/paraglide/runtime.js';

// SPEC-408 R4, R5, A3; ADR-422 D4. Both mastery descriptions are message keys in every shipped
// locale's file: present, not empty, not the English text outside English, different from each
// other, and holding the same numbers as the English sentence.
const APP = resolve(import.meta.dirname, '../..');
const KEYS = ['progress_mastery_about', 'law_mastery_about'] as const;

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** One locale's messages, parsed. */
function messages(locale: string): Record<string, unknown> {
  return JSON.parse(readFileSync(join(APP, 'messages', `${locale}.json`), 'utf8')) as Record<string, unknown>;
}

/** The digit runs of a text, sorted. */
function figures(text: string): string[] {
  return (text.match(/\d+/g) ?? []).sort();
}

describe('the mastery descriptions in every locale', () => {
  it('every locale carries both mastery descriptions with the English figures', () => {
    const all = [...locales];
    const english = messages('en');
    const found: Record<string, string[]> = {};
    for (const locale of all) {
      const held = messages(locale);
      const problems: string[] = [];
      for (const key of KEYS) {
        const text = held[key];
        const source = english[key];
        if (typeof text !== 'string' || text.trim() === '') {
          problems.push(`${locale} lacks ${key}`);
          continue;
        }
        if (typeof source !== 'string') continue;
        if (locale !== 'en' && text === source) problems.push(`${locale}'s ${key} holds the English text`);
        if (figures(text).join() !== figures(source).join()) {
          problems.push(`${locale}'s ${key} holds ${figures(text).join() || 'no figure'}, not ${figures(source).join()}`);
        }
      }
      if (held[KEYS[0]] === held[KEYS[1]]) problems.push(`${locale}'s two descriptions are one text`);
      found[locale] = problems;
    }
    expect(found).toEqual(Object.fromEntries(all.map((locale) => [locale, []])));
    expect(figures(String(english.progress_mastery_about))).toEqual(['0']);
    expect(figures(String(english.law_mastery_about))).toEqual(['100', '3', '70']);

    examined('locales', all);
  });
});
