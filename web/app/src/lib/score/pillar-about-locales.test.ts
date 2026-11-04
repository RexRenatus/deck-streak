import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { locales } from '#lib/paraglide/runtime.js';

// SPEC-606 R3, A3; ADR-627 D4. Every text the score breakdown shows is a message key in every
// shipped locale's file, present and not empty. Each pillar sentence holds the English sentence's
// figures, is not the English text outside English, and differs from the locale's other sentences.
// The keys are read from the breakdown's own source, so a text it gains is judged too.
const APP = resolve(import.meta.dirname, '../../..');
const SOURCE = join(APP, 'src', 'lib', 'score', 'ScoreBreakdown.svelte');

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

/** Every message key the breakdown's source reads, once each, in the order it first reads them. */
function breakdownKeys(): string[] {
  const source = readFileSync(SOURCE, 'utf8');
  return [...new Set([...source.matchAll(/m\.([a-z0-9_]+)/g)].map((match) => match[1]))];
}

describe('the score breakdown texts in every locale', () => {
  it('every locale carries every text the score breakdown shows with the English figures', () => {
    const keys = breakdownKeys();
    const about = keys.filter((key) => key.endsWith('_about'));
    const all = [...locales];
    const english = messages('en');
    const found: Record<string, string[]> = {};
    for (const locale of all) {
      const held = messages(locale);
      const problems: string[] = [];
      for (const key of keys) {
        const text = held[key];
        if (typeof text !== 'string' || text.trim() === '') problems.push(`${locale} lacks ${key}`);
      }
      for (const key of about) {
        const text = held[key];
        const source = english[key];
        if (typeof text !== 'string' || typeof source !== 'string') continue;
        if (locale !== 'en' && text === source) problems.push(`${locale}'s ${key} holds the English text`);
        if (figures(text).join() !== figures(source).join()) {
          problems.push(`${locale}'s ${key} holds ${figures(text).join() || 'no figure'}, not ${figures(source).join()}`);
        }
      }
      const sentences = about.map((key) => held[key]);
      if (new Set(sentences).size !== sentences.length) problems.push(`${locale}'s pillar sentences repeat a text`);
      found[locale] = problems;
    }
    expect(found).toEqual(Object.fromEntries(all.map((locale) => [locale, []])));

    // the breakdown reads its title, the list's name, five labels, the gap line and five sentences
    console.log(`examined ${keys.length} breakdown text(s)`);
    expect(keys.length, `the breakdown reads ${keys.length} texts, fewer than 13`).toBeGreaterThanOrEqual(13);
    expect([...about].sort()).toEqual([
      'pillar_consistency_about',
      'pillar_mastery_about',
      'pillar_retention_about',
      'pillar_volume_about',
      'pillar_workload_about'
    ]);

    examined('locales', all);
  });
});
