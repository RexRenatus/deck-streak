/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it } from 'vitest';
import { getLocale, locales, overwriteGetLocale } from '$lib/paraglide/runtime.js';
import { init } from '../hooks.client';

// SPEC-028 R6, A10. Han unification makes the glyphs follow the language tag, so Chinese always
// names its script, and `<html lang>` is the active locale's: the CJK stylesheet's :lang() rules
// and a screen reader's voice both read it.
const ORIGINAL = getLocale;

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

describe('the page language', () => {
  afterEach(() => {
    overwriteGetLocale(ORIGINAL);
  });

  it('a Chinese locale names its script and the page language follows the locale', async () => {
    const chinese = locales.filter((locale) => new Intl.Locale(locale).language === 'zh');
    expect(chinese.map((locale) => [locale, new Intl.Locale(locale).script])).toEqual([
      ['zh-Hans', 'Hans'],
      ['zh-Hant', 'Hant']
    ]);

    const pages: string[][] = [];
    for (const locale of examined('locales', [...locales])) {
      overwriteGetLocale(() => locale);
      await init();
      pages.push([locale, document.documentElement.lang]);
    }

    expect(pages).toEqual(locales.map((locale) => [locale, locale]));
  });
});
