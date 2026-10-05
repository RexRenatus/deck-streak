import { afterEach, describe, expect, it } from 'vitest';
import { getLocale, locales, overwriteGetLocale } from '$lib/paraglide/runtime.js';
import { engineLanguages } from './locale';

// SPEC-350 R4, A9; ADR-361. The engine speaks the app's locale: its interval labels, undo label and
// deck names come from the engine's own translations, which name Chinese by region where the app
// names it by script.
const ORIGINAL = getLocale;

/** The engine's language for each of the app's locales, written out rather than derived. */
const EXPECTED: Record<string, string> = {
  en: 'en',
  es: 'es',
  fr: 'fr',
  ja: 'ja',
  ko: 'ko',
  'zh-Hans': 'zh-CN',
  'zh-Hant': 'zh-TW'
};

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

describe('the engine languages', () => {
  afterEach(() => {
    overwriteGetLocale(ORIGINAL);
  });

  it("the engine's languages follow the app's locale", () => {
    const read = locales.map((locale) => [locale, engineLanguages(locale)]);
    expect(read).toEqual(locales.map((locale) => [locale, [EXPECTED[locale]]]));

    // with no locale named, the app's own is read
    for (const locale of locales) {
      overwriteGetLocale(() => locale);
      expect(engineLanguages(), locale).toEqual([EXPECTED[locale]]);
    }

    // every locale the app has is in the table, and the table names no other
    expect([...locales].sort()).toEqual(Object.keys(EXPECTED).sort());
    examined('locales', [...locales]);
  });
});
