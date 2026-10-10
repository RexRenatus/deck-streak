import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { locales } from '$lib/paraglide/runtime.js';

// SPEC-381 R9, A15; ADR-392 D3. Every string the AI-and-your-decks screen and its link show is a
// message key in each of the seven locale files: present, not empty, carrying every placeholder the
// English text carries, and never the English text itself outside English. The English words are
// R9's.
const MESSAGES = fileURLToPath(new URL('../../../messages/', import.meta.url));

/** Every key the screen and the deck list's link show, written out rather than derived. */
const AI_DECKS_KEYS = [
  'ai_decks_title',
  'ai_decks_lead',
  'ai_decks_switch',
  'ai_decks_kept_by',
  'ai_decks_load_failed',
  'ai_decks_save_failed',
  'ai_decks_retry',
  'ai_decks_back'
] as const;

/** R9's English words, and the two the screen's way back and its retry carry. */
const ENGLISH: Record<(typeof AI_DECKS_KEYS)[number], string> = {
  ai_decks_title: 'AI and your decks',
  ai_decks_lead:
    "Turn on a deck's switch to keep its cards away from AI features. Studying works the same either way.",
  ai_decks_switch: 'Keep {name} away from AI',
  ai_decks_kept_by: 'Kept away because {parent} is.',
  ai_decks_load_failed: 'These settings could not be loaded.',
  ai_decks_save_failed: 'That change was not saved. Try again.',
  ai_decks_retry: 'Try again',
  ai_decks_back: 'Back to decks'
};

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** One locale's messages, parsed. */
function messages(locale: string): Record<string, unknown> {
  return JSON.parse(readFileSync(`${MESSAGES}${locale}.json`, 'utf8')) as Record<string, unknown>;
}

/** The placeholders a text carries, sorted, as `{name}`. */
function placeholders(text: string): string[] {
  return [...text.matchAll(/\{[a-z]+\}/g)].map((found) => found[0]).sort();
}

/** What `held` owes the screen and lacks, each named with `locale` and its key, judged against
 * the English messages `english`. */
function problems(locale: string, held: Record<string, unknown>, english: Record<string, unknown>): string[] {
  const found: string[] = [];
  for (const key of AI_DECKS_KEYS) {
    const text = held[key];
    const source = english[key];
    if (typeof text !== 'string' || text.trim() === '') {
      found.push(`${locale} lacks ${key}`);
      continue;
    }
    if (typeof source === 'string' && placeholders(text).join() !== placeholders(source).join()) {
      found.push(`${locale}'s ${key} carries ${placeholders(text).join() || 'no placeholder'}, not ${placeholders(source).join()}`);
    }
    if (locale !== 'en' && text === source) found.push(`${locale}'s ${key} holds the English text`);
  }
  return found;
}

describe('the AI-and-your-decks words', () => {
  it('every locale holds every AI-and-your-decks key', () => {
    const english = messages('en');
    const all = [...locales];
    const found = Object.fromEntries(all.map((locale) => [locale, problems(locale, messages(locale), english)]));
    expect(found).toEqual(Object.fromEntries(all.map((locale) => [locale, []])));

    // the English words are R9's
    expect(Object.fromEntries(AI_DECKS_KEYS.map((key) => [key, english[key]]))).toEqual(ENGLISH);

    // each plant is refused by name
    const source = Object.fromEntries(AI_DECKS_KEYS.map((key) => [key, `${key} {name}`]));
    const french = Object.fromEntries(AI_DECKS_KEYS.map((key) => [key, `fr ${key} {name}`]));
    expect(problems('fr', french, source)).toEqual([]);
    const missing = Object.fromEntries(Object.entries(french).filter(([key]) => key !== 'ai_decks_kept_by'));
    expect(problems('ja', missing, source)).toEqual(['ja lacks ai_decks_kept_by']);
    expect(problems('ko', { ...french, ai_decks_switch: 'Garder hors de l’IA' }, source)).toEqual([
      "ko's ai_decks_switch carries no placeholder, not {name}"
    ]);
    expect(problems('es', { ...french, ai_decks_title: '  ' }, source)).toEqual(['es lacks ai_decks_title']);
    expect(problems('zh-Hant', { ...french, ai_decks_lead: source.ai_decks_lead }, source)).toEqual([
      "zh-Hant's ai_decks_lead holds the English text"
    ]);

    examined('AI-and-your-decks key(s)', [...AI_DECKS_KEYS]);
    expect(examined('locales', all).sort()).toEqual(['en', 'es', 'fr', 'ja', 'ko', 'zh-Hans', 'zh-Hant']);
  });
});
