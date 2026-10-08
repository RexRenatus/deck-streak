/**
 * @vitest-environment jsdom
 */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

// SPEC-376 R5, R6, R7, A9 to A11; ADR-387 D3. A card the engine marks past its due day shows one
// line above the card, on both sides, and an on-time card shows none; every locale holds the line
// under one key, with its own word for the streak and never the English text.
const APP = resolve(import.meta.dirname, '../../..');

/** The line's key in every locale. */
const KEY = 'study_late_review';
/** The line in English, as SPEC-376 R5 words it. */
const ENGLISH = 'This card was due on an earlier day, so this review does not count toward the streak for that day.';

/** Each locale's word for the streak, as its tagline spells it (R7). */
const STREAK_WORDS: Record<string, string> = {
  en: 'streak',
  es: 'racha',
  fr: 'série',
  ja: '連続記録',
  ko: '연속 기록',
  'zh-Hans': '连续记录',
  'zh-Hant': '連續紀錄'
};

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

/** One locale's message file, as text. */
function messages(locale: string): string {
  return readFileSync(join(APP, 'messages', `${locale}.json`), 'utf8');
}

/** What a locale's text owes the late line and lacks, each named with its locale. */
function lateLineProblems(locale: string, text: string): string[] {
  const held = JSON.parse(text) as Record<string, unknown>;
  const line = held[KEY];
  if (typeof line !== 'string' || line === '') return [`${locale} lacks ${KEY}`];
  const problems: string[] = [];
  const word = STREAK_WORDS[locale];
  if (word === undefined) problems.push(`${locale} has no streak word to hold`);
  else if (!line.includes(word)) problems.push(`${locale}'s ${KEY} lacks its streak word "${word}"`);
  if (locale !== 'en' && line.includes(ENGLISH)) problems.push(`${locale}'s ${KEY} holds the English text`);
  return problems;
}

describe('every locale holds the late line', () => {
  it('every locale holds the late line', () => {
    const found = Object.fromEntries(locales().map((locale) => [locale, lateLineProblems(locale, messages(locale))]));
    expect(found).toEqual(Object.fromEntries(locales().map((locale) => [locale, []])));

    // the streak words are the taglines' own
    for (const locale of locales()) {
      const tagline = (JSON.parse(messages(locale)) as Record<string, string>).tagline;
      expect(tagline, `${locale}'s tagline`).toContain(STREAK_WORDS[locale]);
    }

    // each plant is refused by name
    const spanish = 'Esta tarjeta vencía un día anterior, así que este repaso no cuenta para la racha de ese día.';
    expect(lateLineProblems('es', JSON.stringify({ [KEY]: spanish }))).toEqual([]);
    expect(lateLineProblems('ja', JSON.stringify({ tagline: 'x' }))).toEqual([`ja lacks ${KEY}`]);
    expect(lateLineProblems('es', JSON.stringify({ [KEY]: spanish.replace('racha', 'streak') }))).toEqual([
      `es's ${KEY} lacks its streak word "racha"`
    ]);
    expect(lateLineProblems('fr', JSON.stringify({ [KEY]: ENGLISH }))).toEqual([
      `fr's ${KEY} lacks its streak word "série"`,
      `fr's ${KEY} holds the English text`
    ]);

    examined('locale files under messages', locales());
    expect(locales()).toEqual(['en', 'es', 'fr', 'ja', 'ko', 'zh-Hans', 'zh-Hant']);
  });
});
