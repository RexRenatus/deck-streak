/**
 * @vitest-environment jsdom
 */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { describe, expect, it } from 'vitest';
import type { CardView, Head, UndoOffer } from '$lib/engine/protocol';
import type { StudyClient } from './review';
import ReviewScreen from './ReviewScreen.svelte';

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

/** A card view as the engine reads it, on time unless `extra` says otherwise. */
function view(id: number, extra: Partial<CardView> = {}): CardView {
  return {
    id: BigInt(id),
    ordinal: 0,
    flag: 0,
    question: `<p>question ${id}</p>`,
    answer: `<p>answer ${id}</p>`,
    css: '',
    labels: ['<1m', '<6m', '<10m', '4d'],
    undo: null,
    late: false,
    withheld: false,
    ...extra
  };
}

function head(card: CardView | null): Head {
  return { counts: { new: 0, learning: 0, review: 3 }, card };
}

/** The engine as the screen sees it: the heads queued in order, and every other call answered. */
class FakeClient implements StudyClient {
  heads: Head[];

  constructor(heads: Head[]) {
    this.heads = heads;
  }

  card(): Promise<Head> {
    return Promise.resolve(this.heads.shift() as Head);
  }

  rate(): Promise<null> {
    return Promise.resolve(null);
  }

  bury(): Promise<null> {
    return Promise.resolve(null);
  }

  flag(): Promise<number> {
    return Promise.resolve(1);
  }

  undo(): Promise<null> {
    return Promise.resolve(null);
  }

  undoOffer(): Promise<UndoOffer> {
    return Promise.resolve({ offer: null, why: 'none' });
  }
}

/** Lets every answer arrive and every timer of zero run, then lets Svelte update the page. */
async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

/** The card frame, and what its document shows: its title and its body's text. */
function frame(): HTMLIFrameElement {
  return document.querySelector('iframe') as HTMLIFrameElement;
}

function shown(): { title: string; text: string } {
  const doc = new DOMParser().parseFromString(frame().getAttribute('srcdoc') ?? '', 'text/html');
  return { title: frame().title, text: doc.body.textContent?.trim() ?? '' };
}

/** Whether `first` comes before `second` in the page. */
function before(first: Node, second: Node): boolean {
  return (first.compareDocumentPosition(second) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
}

/** The status line, the late line and the card frame, in the order the page holds them. */
function order(line: HTMLElement): boolean[] {
  return [before(screen.getByRole('status'), line), before(line, frame()), frame().contains(line)];
}

describe('the late-review line', () => {
  it('a late card shows the line above the card', async () => {
    const client = new FakeClient([head(view(1, { late: true })), head(null)]);
    render(ReviewScreen, { client: async () => client });
    await settle();

    // the question side: the line, below the status line, above the card frame and outside it
    const question = screen.queryByText(ENGLISH);
    expect(question, 'a late card shows the line on its question side').not.toBeNull();
    expect(order(question as HTMLElement)).toEqual([true, true, false]);
    expect(shown()).toEqual({ title: "The card's question", text: 'question 1' });

    // the answer side: the line stays where it was
    await fireEvent.click(screen.getByRole('button', { name: 'Show answer' }));
    const answer = screen.queryByText(ENGLISH);
    expect(answer, 'a late card shows the line on its answer side').not.toBeNull();
    expect(order(answer as HTMLElement)).toEqual([true, true, false]);
    expect(shown()).toEqual({ title: "The card's answer", text: 'answer 1' });
  });

  it('a card on time shows no line', async () => {
    const client = new FakeClient([head(view(1)), head(view(2, { late: true })), head(view(3)), head(null)]);
    render(ReviewScreen, { client: async () => client });
    await settle();

    // an on-time card: its question, and no line
    expect([shown(), screen.queryByText(ENGLISH)]).toEqual([{ title: "The card's question", text: 'question 1' }, null]);
    await fireEvent.click(screen.getByRole('button', { name: 'Show answer' }));
    expect([shown(), screen.queryByText(ENGLISH)]).toEqual([{ title: "The card's answer", text: 'answer 1' }, null]);

    // a late card, then an on-time card: the on-time card shows its question, and no line
    await fireEvent.click(screen.getByRole('button', { name: 'Good <10m' }));
    await settle();
    expect(shown()).toEqual({ title: "The card's question", text: 'question 2' });
    await fireEvent.click(screen.getByRole('button', { name: 'Show answer' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Good <10m' }));
    await settle();
    expect([shown(), screen.queryByText(ENGLISH)]).toEqual([{ title: "The card's question", text: 'question 3' }, null]);
  });
});

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
