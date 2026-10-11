/**
 * @vitest-environment jsdom
 */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

// SPEC-409 R10, R11, A13, A14; ADR-423 D4, D5. Every locale holds the four words of the one-tap
// sync offer under one set of keys, with its own text and no word for the streak, and the study
// code's shipped sync calls are the session's one and the offer's one.
const APP = resolve(import.meta.dirname, '../../..');

/** The offer's four keys, with the English as SPEC-409 R10 words it. */
const ENGLISH: Record<string, string> = {
  study_update_prompt: 'Your reviews stay on this device until they sync. You can sync them now.',
  study_update_dismiss: 'Not now',
  study_update_done: 'Sync finished.',
  study_update_open_sync: 'Open the sync page'
};

/** Each locale's word for the streak, as its tagline spells it (R10). */
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

/** What a locale's text owes the offer's words and lacks, each named with its locale. */
function offerProblems(locale: string, text: string): string[] {
  const held = JSON.parse(text) as Record<string, unknown>;
  const problems: string[] = [];
  for (const [key, english] of Object.entries(ENGLISH)) {
    const value = held[key];
    if (typeof value !== 'string' || value === '') {
      problems.push(`${locale} lacks ${key}`);
      continue;
    }
    if (locale === 'en' && value !== english) problems.push(`en's ${key} is not the English text`);
    if (locale !== 'en' && value === english) problems.push(`${locale}'s ${key} holds the English text`);
    const word = STREAK_WORDS[locale];
    if (word === undefined) problems.push(`${locale} has no streak word to refuse`);
    else if (value.includes(word)) problems.push(`${locale}'s ${key} holds its streak word "${word}"`);
    if (/[!！]/.test(value)) problems.push(`${locale}'s ${key} holds an exclamation mark`);
  }
  return problems;
}

describe('every locale holds the offer words', () => {
  it('every locale holds the offer words', () => {
    const found = Object.fromEntries(locales().map((locale) => [locale, offerProblems(locale, messages(locale))]));
    expect(found).toEqual(Object.fromEntries(locales().map((locale) => [locale, []])));

    // the streak words are the taglines' own
    for (const locale of locales()) {
      const tagline = (JSON.parse(messages(locale)) as Record<string, string>).tagline;
      expect(tagline, `${locale}'s tagline`).toContain(STREAK_WORDS[locale]);
    }

    // each plant is refused by name
    const spanish = {
      study_update_prompt: 'Tus repasos se quedan en este dispositivo hasta que se sincronizan. Puedes sincronizarlos ahora.',
      study_update_dismiss: 'Ahora no',
      study_update_done: 'Sincronización terminada.',
      study_update_open_sync: 'Abrir la página de sincronización'
    };
    expect(offerProblems('es', JSON.stringify(spanish))).toEqual([]);
    expect(offerProblems('es', JSON.stringify({ ...spanish, study_update_done: undefined }))).toEqual([
      'es lacks study_update_done'
    ]);
    expect(offerProblems('es', JSON.stringify({ ...spanish, study_update_dismiss: 'Not now' }))).toEqual([
      "es's study_update_dismiss holds the English text"
    ]);
    expect(offerProblems('es', JSON.stringify({ ...spanish, study_update_done: 'Mantén la racha.' }))).toEqual([
      `es's study_update_done holds its streak word "racha"`
    ]);
    expect(offerProblems('fr', JSON.stringify({ ...ENGLISH, study_update_done: 'Terminé !' }))).toEqual([
      "fr's study_update_prompt holds the English text",
      "fr's study_update_dismiss holds the English text",
      "fr's study_update_done holds an exclamation mark",
      "fr's study_update_open_sync holds the English text"
    ]);
    expect(offerProblems('ja', JSON.stringify({ ...ENGLISH, study_update_done: '完了！' }))).toContain(
      "ja's study_update_done holds an exclamation mark"
    );

    examined('locale files under messages', locales());
    expect(locales()).toEqual(['en', 'es', 'fr', 'ja', 'ko', 'zh-Hans', 'zh-Hant']);
  });
});

const SCREENS = ['src/lib/study', 'src/routes/study'];

/** Every shipped source file under `dir`, relative to the app, never a test; a directory the tree
 * does not hold contributes none. */
function sources(dir: string): string[] {
  const path = join(APP, dir);
  if (!existsSync(path)) return [];
  return readdirSync(path, { withFileTypes: true, recursive: true })
    .filter((entry) => entry.isFile() && /\.(?:ts|js|svelte)$/.test(entry.name))
    .filter((entry) => !/\.(?:test|spec)\./.test(entry.name))
    .map((entry) => relative(APP, join(entry.parentPath, entry.name)).split('\\').join('/'))
    .sort();
}

/** How many sync calls one text holds, comments counted. */
function syncCalls(text: string): number {
  return [...text.matchAll(/\.sync\s*\(/g)].length;
}

/** The sync calls per file, for the files that hold any. */
function census(texts: Record<string, string>): Record<string, number> {
  const found: Record<string, number> = {};
  for (const [file, text] of Object.entries(texts)) {
    const count = syncCalls(text);
    if (count > 0) found[file] = count;
  }
  return found;
}

/** The sync calls SPEC-409 R11 admits: the session's one and the offer's one. */
const ADMITTED: Record<string, number> = { 'src/lib/study/engine.ts': 1, 'src/lib/study/update-now.ts': 1 };

/** What a census owes R11 and lacks, each named by file. */
function callProblems(found: Record<string, number>): string[] {
  const problems: string[] = [];
  for (const [file, count] of Object.entries(found)) {
    if (!(file in ADMITTED)) problems.push(`${file} holds ${count} sync call(s) and is not admitted`);
    else if (count !== ADMITTED[file]) problems.push(`${file} holds ${count} sync call(s), not ${ADMITTED[file]}`);
  }
  for (const file of Object.keys(ADMITTED)) if (!(file in found)) problems.push(`${file} holds no sync call`);
  return problems;
}

describe('the tap is the one new sync call in the study code', () => {
  it('the tap is the one new sync call in the study code', () => {
    const files = SCREENS.flatMap(sources);
    const texts = Object.fromEntries(files.map((file) => [file, readFileSync(join(APP, file), 'utf8')]));
    expect(census(texts)).toEqual(ADMITTED);
    expect(callProblems(census(texts))).toEqual([]);

    // each plant is refused by name
    const base = { 'src/lib/study/engine.ts': 'void client.sync();', 'src/lib/study/update-now.ts': 'await c.sync();' };
    expect(callProblems(census(base))).toEqual([]);
    expect(callProblems(census({ ...base, 'src/lib/study/extra.ts': 'await client.sync();' }))).toEqual([
      'src/lib/study/extra.ts holds 1 sync call(s) and is not admitted'
    ]);
    expect(callProblems(census({ ...base, 'src/lib/study/review.ts': 'await client\n  .sync ();' }))).toEqual([
      'src/lib/study/review.ts holds 1 sync call(s) and is not admitted'
    ]);
    expect(callProblems(census({ ...base, 'src/lib/study/update-now.ts': 'a.sync(); b.sync();' }))).toEqual([
      'src/lib/study/update-now.ts holds 2 sync call(s), not 1'
    ]);
    expect(callProblems(census({ 'src/lib/study/engine.ts': 'void client.sync();' }))).toEqual([
      'src/lib/study/update-now.ts holds no sync call'
    ]);

    examined('shipped source files under the study modules and routes', files);
    expect(files).toContain('src/lib/study/ReviewScreen.svelte');
  });
});
