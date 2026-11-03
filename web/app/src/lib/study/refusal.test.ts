import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, describe, expect, it } from 'vitest';
import { getLocale, locales, overwriteGetLocale } from '#lib/paraglide/runtime.js';
import { noVoiceText, STATUS_KEYS, statusText } from './refusal';
import type { Status } from './review';

// SPEC-350 R5, R7, R10, R11; ADR-361. Each status the review announces, every engine refusal, the
// card the frame refused and the done deck, names its own message, and the message is in every
// locale the app has. SPEC-506 R5; ADR-524: so is the line a clip with no local voice leaves.
const ORIGINAL = getLocale;
const MESSAGES = fileURLToPath(new URL('../../../messages/', import.meta.url));

/**
 * The repository's root, found by walking up to its pnpm workspace. A fixed count of `..` would miss
 * it when StrykerJS runs this file from its sandbox, which sits below `web/app/.stryker-tmp`.
 */
function repositoryRoot(from: string): string {
  for (let dir = from; ; dir = dirname(dir)) {
    if (existsSync(join(dir, 'pnpm-workspace.yaml'))) return dir;
    if (dirname(dir) === dir) throw new Error(`no pnpm-workspace.yaml above ${from}`);
  }
}

/** Each status and its message's key, written out rather than derived. */
const EXPECTED = {
  'bad-request': 'study_refused_bad_request',
  'collection-busy': 'study_refused_collection_busy',
  'storage-refused': 'study_refused_storage',
  'engine-failed': 'study_refused_engine',
  'not-open': 'study_refused_not_open',
  'not-shown': 'study_refused_not_shown',
  'undo-synced': 'undo_synced',
  'not-undoable': 'undo_gone',
  // SPEC-505 R6, A2: each frame check's reason has its own message
  'class-list': 'study_card_refused_class_list',
  'style-closed': 'study_card_refused_style_closed',
  'reparse-element': 'study_card_refused_reparse_element',
  'reparse-srcset': 'study_card_refused_reparse_srcset',
  'reparse-target': 'study_card_refused_reparse_target',
  'body-attributes': 'study_card_refused_body_attributes',
  'host-readback': 'study_card_refused_host_readback',
  done: 'study_done'
};

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

function messages(locale: string): Record<string, string> {
  return JSON.parse(readFileSync(`${MESSAGES}${locale}.json`, 'utf8')) as Record<string, string>;
}

describe('the review status', () => {
  afterEach(() => {
    overwriteGetLocale(ORIGINAL);
  });

  it('each refusal names its own message in every locale', () => {
    expect(STATUS_KEYS).toEqual(EXPECTED);
    const keys = Object.values(EXPECTED);
    expect(new Set(keys).size).toBe(keys.length);

    for (const locale of examined('locales', [...locales])) {
      const table = messages(locale);
      overwriteGetLocale(() => locale);
      for (const [status, key] of Object.entries(EXPECTED)) {
        expect(table[key]?.length ?? 0, `${locale} ${key}`).toBeGreaterThan(0);
        expect(statusText(status as keyof typeof EXPECTED), `${locale} ${status}`).toBe(table[key]);
      }
      // the one sentence for every refused card is gone, replaced by the reasons' own
      expect(Object.keys(table), locale).not.toContain('study_card_escaped');
    }
  });

  it('the no voice line is in every locale', () => {
    // the English line, written here
    expect(messages('en').study_no_local_voice).toBe('No voice on this device for {language}.');

    const seven = examined('locales', [...locales]);
    expect(seven).toHaveLength(7);
    for (const locale of seven) {
      const line = messages(locale).study_no_local_voice ?? '';
      // each locale names the language, in its own words
      expect(line, `${locale} study_no_local_voice`).toContain('{language}');
      if (locale !== 'en') expect(line, `${locale} is the English line`).not.toBe(messages('en').study_no_local_voice);
      overwriteGetLocale(() => locale);
      expect(noVoiceText('ko-KR'), locale).toBe(line.replace('{language}', 'ko-KR'));
    }
  });

  // SPEC-505 R5. The status type is the closed set: the engine's codes, the seven reasons and a
  // done deck, and no other word. The app's check reads the directive below, and a status type that
  // opened would leave it unused, which the check reports.
  it('no other word is a status', () => {
    // @ts-expect-error the one word every refused card used to announce is no status (SPEC-505 R5)
    const word: Status = 'escaped';
    expect(Object.keys(STATUS_KEYS)).not.toContain(word);
    expect(Object.keys(STATUS_KEYS)).toEqual(Object.keys(EXPECTED));
  });

  // SPEC-505 R7, A4; ADR-523 D5. The iPhone and iPad card face shows its refusal in DeckStreak's words,
  // through the words file, and never prints the error's Swift case name.
  it('the iPhone and iPad card face words a refusal through its mapper, never a Swift case name', () => {
    const swift = join(repositoryRoot(import.meta.dirname), 'ios', 'App', 'Sources');
    const face = readFileSync(join(swift, 'CardFaceView.swift'), 'utf8');
    const words = readFileSync(join(swift, 'CardRefusalWords.swift'), 'utf8');
    const judged = (text: string) => ({
      printsTheCase: /String\(\s*describing:/.test(text),
      wordsTheRefusal: /CardRefusalWords\.text\(\s*of:\s*error\s*\)/.test(text),
      announcesIt: /CardRefusalWords\.announce\(/.test(text)
    });
    expect(judged(face)).toEqual({ printsTheCase: false, wordsTheRefusal: true, announcesIt: true });
    // the words file maps every case with a switch over the refusal, and prints no case either
    expect(words).toMatch(/switch refusal \{/);
    expect(words).toMatch(/case \.ruleListDidNotCompile:/);
    expect(judged(words).printsTheCase).toBe(false);

    // the control: the card face as it was, printing the case, is refused
    const planted = face.replace(/refusal = CardRefusalWords\.text\(of: error\)/, 'refusal = String(describing: error)');
    expect(planted).not.toBe(face);
    expect(judged(planted).printsTheCase).toBe(true);
    console.log('examined 2 Swift files');
  });
});
