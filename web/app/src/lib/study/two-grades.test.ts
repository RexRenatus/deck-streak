import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

// SPEC-366 R9, A11 and A12; ADR-377 D8. The page rates with two grades, Again and Good, and from
// two sites. The cast where the review sends a rating lets any `RATING` value compile, so only a
// read of the text holds it. The census reads text, so a comment counts.
const APP = fileURLToPath(new URL('../../..', import.meta.url));

/** A call of the client's or the engine's `rate`, found by text. */
const RATE_CALL = /\.rate\s*\(/g;

/** The `RATING` declaration a review source holds, whitespace folded. */
const RATING_DECLARATION = /const\s+RATING\b[^=]*=\s*(\{[^}]*\})\s*;/;

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** Every shipped source file under `src`, relative to the app, never a test. */
function sources(): string[] {
  const path = join(APP, 'src');
  if (!existsSync(path)) return [];
  return readdirSync(path, { withFileTypes: true, recursive: true })
    .filter((entry) => entry.isFile() && /\.(?:ts|js|svelte)$/.test(entry.name))
    .filter((entry) => !/\.(?:test|spec)\./.test(entry.name))
    .map((entry) => relative(APP, join(entry.parentPath, entry.name)).split('\\').join('/'))
    .filter((file) => !file.startsWith('src/lib/paraglide/'))
    .sort();
}

/** The `.rate(` calls one text holds. */
function rateCalls(text: string): number {
  return [...text.matchAll(RATE_CALL)].length;
}

/** The text of a `RATING` object, spaces removed, or undefined when the text declares none. */
function ratingOf(text: string): string | undefined {
  return RATING_DECLARATION.exec(text)?.[1].replace(/\s+/g, '');
}

/** Every locale file, relative to the app. */
function locales(): string[] {
  const path = join(APP, 'messages');
  if (!existsSync(path)) return [];
  return readdirSync(path)
    .filter((name) => name.endsWith('.json'))
    .sort()
    .map((name) => `messages/${name}`);
}

/** The message keys a locale text names that are not the two grades: those it holds of Hard and
 * Easy, and whether it holds Again and Good. */
function gradeKeys(text: string): { has: string[]; forbidden: string[] } {
  const keys = Object.keys(JSON.parse(text) as Record<string, unknown>);
  return {
    has: keys.filter((key) => key === 'study_again' || key === 'study_good'),
    forbidden: keys.filter((key) => key === 'study_hard' || key === 'study_easy')
  };
}

describe('the page rates with two grades and from two sites', () => {
  it('the page rates with two grades and from two sites', () => {
    const files = sources();
    const sites: Record<string, number> = {};
    for (const file of files) {
      const count = rateCalls(readFileSync(join(APP, file), 'utf8'));
      if (count > 0) sites[file] = count;
    }

    // the rate is called at exactly two sites
    expect(sites).toEqual({ 'src/lib/engine/session.ts': 1, 'src/lib/study/review.ts': 1 });
    // and the review's grade cell sends exactly Again and Good
    expect(ratingOf(readFileSync(join(APP, 'src/lib/study/review.ts'), 'utf8'))).toBe('{again:1,good:3}');
    examined('shipped source files under src', files);

    // each plant is refused by name
    expect(rateCalls('await client.rate(card, 3, 0);\nawait other\n  .rate (card, 1, 0);')).toBe(2);
    expect(rateCalls('const rate = view.rate; rate(card);')).toBe(0);
    expect(ratingOf('const RATING: Partial<Record<Action, Rating>> = { again: 1, hard: 2, good: 3, easy: 4 };')).toBe(
      '{again:1,hard:2,good:3,easy:4}'
    );
    expect(ratingOf('const RATING: Partial<Record<Action, Rating>> = { again: 1, good: 3 };')).toBe('{again:1,good:3}');
    expect(ratingOf('const RATING = { again: 1, good: 3, easy: 4 };')).toBe('{again:1,good:3,easy:4}');
    expect(ratingOf('const OTHER = 1;')).toBeUndefined();
  });
});

describe('every locale names the two grades and no other', () => {
  it('every locale names the two grades and no other', () => {
    const files = locales();
    const found: Record<string, { has: string[]; forbidden: string[] }> = {};
    for (const file of files) found[file] = gradeKeys(readFileSync(join(APP, file), 'utf8'));

    for (const file of files) {
      expect(found[file].forbidden, `${file} names a grade the review no longer offers`).toEqual([]);
      expect(found[file].has.sort(), `${file} lacks a grade the review offers`).toEqual(['study_again', 'study_good']);
    }
    examined('locale files under messages', files);
    expect(files).toHaveLength(7);

    // each plant is refused by name
    expect(gradeKeys('{"study_again":"a","study_good":"g","study_easy":"e"}')).toEqual({
      has: ['study_again', 'study_good'],
      forbidden: ['study_easy']
    });
    expect(gradeKeys('{"study_again":"a","study_hard":"h"}')).toEqual({
      has: ['study_again'],
      forbidden: ['study_hard']
    });
    expect(gradeKeys('{"study_again":"a","study_good":"g"}').forbidden).toEqual([]);
  });
});
