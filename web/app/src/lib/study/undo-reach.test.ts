import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { step, type Phase, type ReviewEvent } from './review';

// SPEC-371 R14, A20 to A22; ADR-382 D9. An undo reaches the engine only from the review and the
// session, the review asks for the offer before it writes, and only its confirmation writes. The
// census reads text, so a comment counts, in SPEC-366 R9's shape.
const APP = fileURLToPath(new URL('../../..', import.meta.url));

/** A call of the client's or the engine's `undo`, found by text. */
const UNDO_CALL = /\.undo\s*\(/g;
/** A call of the client's offer. */
const OFFER_CALL = /\.undoOffer\s*\(/g;
/** A call of the engine's offer. */
const ENGINE_OFFER_CALL = /\.undo_offer\s*\(/g;

/** The undo dialog's fifteen messages (R11). */
const UNDO_KEYS = [
  'study_undo_answer',
  'undo_title',
  'undo_card',
  'undo_answer',
  'undo_returns',
  'undo_returns_new',
  'undo_returns_learning',
  'undo_returns_review',
  'undo_returns_relearning',
  'undo_returns_preview',
  'undo_unsynced',
  'undo_keep',
  'undo_synced',
  'undo_gone',
  'undo_no_text'
];

/** The ten messages of the undo of a bury or a flag (SPEC-383 R12). */
const CHANGE_KEYS = [
  'study_undo_bury',
  'study_undo_flag',
  'undo_bury_title',
  'undo_flag_title',
  'undo_bury_unsynced',
  'undo_flag_added',
  'undo_flag_removed',
  'undo_flag_replaced',
  'undo_change_synced',
  'undo_change_gone'
];

/** Every phase the review may be in, the confirmation's included. */
const PHASES = ['loading', 'question', 'answer', 'busy', 'confirming', 'refused', 'done'];

/** Every event that moves the review: the actions and the outcomes of its requests. */
const EVENTS = [
  'show-answer',
  'again',
  'good',
  'undo',
  'bury',
  'flag',
  'replay',
  'keep',
  'view',
  'empty',
  'settled',
  'flagged',
  'not-shown',
  'refusal',
  'retry',
  'offered',
  'not-offered',
  'undo-refused'
];

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

/** The calls `pattern` finds in one text. */
function calls(pattern: RegExp, text: string): number {
  return [...text.matchAll(pattern)].length;
}

/** Each file that calls `pattern`, with how many times. */
function sites(pattern: RegExp, files: string[]): Record<string, number> {
  const found: Record<string, number> = {};
  for (const file of files) {
    const count = calls(pattern, readFileSync(join(APP, file), 'utf8'));
    if (count > 0) found[file] = count;
  }
  return found;
}

/** The phases that hold a cell whose effect is `undo`, by asking the machine itself. A phase the
 * machine has no row for holds no cell. */
function undoPhases(): string[] {
  const found = new Set<string>();
  for (const phase of PHASES) {
    for (const event of EVENTS) {
      for (const side of ['question', 'answer'] as const) {
        let effect: string;
        try {
          effect = step({ phase: phase as Phase, side }, event as ReviewEvent).effect;
        } catch {
          continue;
        }
        if (effect === 'undo') found.add(phase);
      }
    }
  }
  return [...found].sort();
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

/** The undo keys a locale text lacks, or holds empty. */
function missingKeys(text: string): string[] {
  const messages = JSON.parse(text) as Record<string, unknown>;
  return UNDO_KEYS.filter((key) => typeof messages[key] !== 'string' || messages[key] === '');
}

/** The keys of `keys` a locale text lacks, or holds empty. */
function lackedKeys(text: string, keys: readonly string[]): string[] {
  const messages = JSON.parse(text) as Record<string, unknown>;
  return keys.filter((key) => typeof messages[key] !== 'string' || messages[key] === '');
}

describe('an undo reaches the engine only from the review and the session', () => {
  it('an undo reaches the engine only from the review and the session', () => {
    const files = sources();
    // the positive sites first: the review asks for the offer, and the session asks the engine
    expect(sites(OFFER_CALL, files)).toEqual({ 'src/lib/study/review.ts': 1 });
    expect(sites(ENGINE_OFFER_CALL, files)).toEqual({ 'src/lib/engine/session.ts': 1 });
    expect(sites(UNDO_CALL, files)).toEqual({ 'src/lib/engine/session.ts': 1, 'src/lib/study/review.ts': 1 });
    examined('shipped source files under src', files);
    examined('undoOffer call sites', Object.keys(sites(OFFER_CALL, files)));
    examined('undo_offer call sites', Object.keys(sites(ENGINE_OFFER_CALL, files)));
    examined('undo call sites', Object.keys(sites(UNDO_CALL, files)));

    // each plant is refused by name
    expect(calls(UNDO_CALL, 'await client.undo(card, step);\nengine\n  .undo (card, step);')).toBe(2);
    expect(calls(UNDO_CALL, 'const undo = view.undo; undoOffer(); client.undoOffer();')).toBe(0);
    expect(calls(OFFER_CALL, 'await client.undoOffer();\n// client.undoOffer() in a comment')).toBe(2);
    expect(calls(OFFER_CALL, 'client.undo_offer(); client.undo();')).toBe(0);
    expect(calls(ENGINE_OFFER_CALL, 'engine.undo_offer();\nengine\n  .undo_offer ();')).toBe(2);
    expect(calls(ENGINE_OFFER_CALL, 'engine.undoOffer(); engine.undo();')).toBe(0);
  });
});

describe('only the confirmation holds a cell whose effect is undo', () => {
  it('only the confirmation holds a cell whose effect is undo', () => {
    const phases = undoPhases();
    expect(phases).toEqual(['confirming']);
    examined('phases', PHASES);
    examined('events', EVENTS);
    examined('phases holding an undo cell', phases);
  });
});

describe("every locale holds the undo dialog's fifteen messages", () => {
  it("every locale holds the undo dialog's fifteen messages", () => {
    const files = locales();
    const found: Record<string, string[]> = {};
    for (const file of files) found[file] = missingKeys(readFileSync(join(APP, file), 'utf8'));
    for (const file of files) {
      expect(found[file], `${file} lacks an undo message`).toEqual([]);
    }
    examined('locale files under messages', files);
    expect(files).toHaveLength(7);
    examined('undo keys', UNDO_KEYS);
    expect(UNDO_KEYS).toHaveLength(15);

    // each plant is refused by name
    const full = Object.fromEntries(UNDO_KEYS.map((key) => [key, 'x']));
    expect(missingKeys(JSON.stringify(full))).toEqual([]);
    expect(missingKeys(JSON.stringify({ ...full, undo_title: '' }))).toEqual(['undo_title']);
    const lacking = Object.fromEntries(UNDO_KEYS.filter((key) => key !== 'undo_keep').map((key) => [key, 'x']));
    expect(missingKeys(JSON.stringify(lacking))).toEqual(['undo_keep']);
  });
});

describe('every locale holds the bury and flag undo messages', () => {
  it('every locale holds the bury and flag undo messages', () => {
    // SPEC-383 R12, A28: the ten keys of the undo of a bury or a flag, in every locale
    const files = locales();
    for (const file of files) {
      const lacked = lackedKeys(readFileSync(join(APP, file), 'utf8'), CHANGE_KEYS);
      expect(lacked, `${file} lacks a bury or flag undo message`).toEqual([]);
    }
    examined('locale files under messages', files);
    expect(files).toHaveLength(7);
    examined('bury and flag undo keys', CHANGE_KEYS);
    expect(CHANGE_KEYS).toHaveLength(10);

    // a locale without one message, or with one empty, is refused by the key's name
    const full = Object.fromEntries(CHANGE_KEYS.map((key) => [key, 'x']));
    expect(lackedKeys(JSON.stringify(full), CHANGE_KEYS)).toEqual([]);
    const lacking = Object.fromEntries(
      CHANGE_KEYS.filter((key) => key !== 'undo_flag_replaced').map((key) => [key, 'x'])
    );
    expect(lackedKeys(JSON.stringify(lacking), CHANGE_KEYS)).toEqual(['undo_flag_replaced']);
    expect(lackedKeys(JSON.stringify({ ...full, undo_change_gone: '' }), CHANGE_KEYS)).toEqual([
      'undo_change_gone'
    ]);
  });
});
