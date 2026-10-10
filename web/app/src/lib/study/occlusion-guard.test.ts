/**
 * @vitest-environment jsdom
 */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

// SPEC-380 R5, R6, R8, R11, A9 to A12; ADR-391. A card the engine marks withheld, because its
// question holds an image occlusion mask this app does not draw, shows one line where the card's
// lines are shown, and no card; it offers no reveal and no grade, and a bury moves on. Every locale
// holds the line under one key, with its own word for bury and never the English text.
const APP = resolve(import.meta.dirname, '../../..');

/** The line's key in every locale. */
const KEY = 'study_card_withheld';
/** The line in English, as SPEC-380 R6 words it. */
const ENGLISH =
  'This image occlusion card cannot be shown here, because this app does not draw its masks. You can still bury or flag it.';
/** The key of each locale's own word for bury, which its line holds (R6). */
const BURY = 'study_bury';

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

/** What a locale's text owes the withheld line and lacks, each named with its locale. */
function withheldLineProblems(locale: string, text: string): string[] {
  const held = JSON.parse(text) as Record<string, unknown>;
  const line = held[KEY];
  if (typeof line !== 'string' || line === '') return [`${locale} lacks ${KEY}`];
  const problems: string[] = [];
  const word = held[BURY];
  if (typeof word !== 'string' || word === '') problems.push(`${locale} has no ${BURY} word to hold`);
  else if (!line.toLocaleLowerCase(locale).includes(word.toLocaleLowerCase(locale)))
    problems.push(`${locale}'s ${KEY} lacks its bury word "${word}"`);
  if (locale === 'en' && line !== ENGLISH) problems.push(`en's ${KEY} is not the English text of R6`);
  if (locale !== 'en' && line.includes(ENGLISH)) problems.push(`${locale}'s ${KEY} holds the English text`);
  return problems;
}

describe('every locale holds the withheld line', () => {
  it('every locale holds the withheld line', () => {
    const found = Object.fromEntries(locales().map((locale) => [locale, withheldLineProblems(locale, messages(locale))]));
    expect(found).toEqual(Object.fromEntries(locales().map((locale) => [locale, []])));

    // each plant is refused by name
    const spanish =
      'Esta tarjeta de oclusión de imagen no se puede mostrar aquí, porque esta app no dibuja sus máscaras. Aún puedes enterrarla o marcarla.';
    expect(withheldLineProblems('es', JSON.stringify({ [KEY]: spanish, [BURY]: 'Enterrar' }))).toEqual([]);
    expect(withheldLineProblems('ja', JSON.stringify({ [BURY]: '延期' }))).toEqual([`ja lacks ${KEY}`]);
    expect(
      withheldLineProblems('es', JSON.stringify({ [KEY]: spanish.replace('enterrarla', 'ocultarla'), [BURY]: 'Enterrar' }))
    ).toEqual([`es's ${KEY} lacks its bury word "Enterrar"`]);
    expect(withheldLineProblems('fr', JSON.stringify({ [KEY]: ENGLISH, [BURY]: 'Enfouir' }))).toEqual([
      `fr's ${KEY} lacks its bury word "Enfouir"`,
      `fr's ${KEY} holds the English text`
    ]);
    expect(withheldLineProblems('en', JSON.stringify({ [KEY]: 'You can still bury it.', [BURY]: 'Bury' }))).toEqual([
      `en's ${KEY} is not the English text of R6`
    ]);
    expect(withheldLineProblems('ko', JSON.stringify({ [KEY]: '이 카드는 보류할 수 있습니다.' }))).toEqual([
      `ko has no ${BURY} word to hold`
    ]);

    examined('locale files under messages', locales());
    expect(locales()).toEqual(['en', 'es', 'fr', 'ja', 'ko', 'zh-Hans', 'zh-Hant']);
  });
});
