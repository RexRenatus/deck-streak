/**
 * @vitest-environment jsdom
 */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { describe, expect, it } from 'vitest';
import type { CardView, Head, UndoOffer } from '$lib/engine/protocol';
import { Review, type StudyClient } from './review';
import ReviewScreen from './ReviewScreen.svelte';

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

/** The late line in English (SPEC-376 R5), which a withheld card never shows. */
const LATE = 'This card was due on an earlier day, so this review does not count toward the streak for that day.';

/** A card view as the engine reads it: shown, on time, with nothing to undo, unless `extra` says
 * otherwise. */
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

/** A card view the engine withheld: its question, answer and note CSS are empty (SPEC-380 R3). */
function withheld(id: number, extra: Partial<CardView> = {}): CardView {
  return view(id, { withheld: true, question: '', answer: '', css: '', ...extra });
}

function head(card: CardView | null): Head {
  return { counts: { new: 0, learning: 0, review: 3 }, card };
}

/** The engine as the review sees it: each call recorded, the heads queued in order. */
class FakeClient implements StudyClient {
  calls: string[] = [];
  heads: Head[];

  constructor(heads: Head[]) {
    this.heads = heads;
  }

  card(): Promise<Head> {
    this.calls.push('card');
    return Promise.resolve(this.heads.shift() as Head);
  }

  rate(card: bigint, rating: number, ms: number): Promise<null> {
    this.calls.push(`rate ${card} ${rating} ${ms}`);
    return Promise.resolve(null);
  }

  bury(card: bigint): Promise<null> {
    this.calls.push(`bury ${card}`);
    return Promise.resolve(null);
  }

  flag(card: bigint): Promise<number> {
    this.calls.push(`flag ${card}`);
    return Promise.resolve(1);
  }

  undo(card: bigint, step: number): Promise<null> {
    this.calls.push(`undo ${card} ${step}`);
    return Promise.resolve(null);
  }

  undoOffer(): Promise<UndoOffer> {
    this.calls.push('undo-offer');
    return Promise.resolve({ offer: null, why: 'none' });
  }
}

/** Lets every answer arrive and every timer of zero run, then lets Svelte update the page. */
async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

/** What the status region says. */
function status(): string {
  return screen.getByRole('status').textContent?.trim() ?? '';
}

/** The card frame and what its document shows, or `null` when the page draws no frame. */
function shown(): { title: string; text: string } | null {
  const frame = document.querySelector('iframe');
  if (frame === null) return null;
  const doc = new DOMParser().parseFromString(frame.getAttribute('srcdoc') ?? '', 'text/html');
  return { title: frame.title, text: doc.body.textContent?.trim() ?? '' };
}

describe('the withheld occlusion question', () => {
  it('a withheld card shows the line and no card', async () => {
    const client = new FakeClient([head(withheld(1, { late: true })), head(null)]);
    render(ReviewScreen, { client: async () => client });
    await settle();

    // the line, in the status region where the card's lines are shown; no card frame, and no late
    // line though the card is late
    expect({ status: status(), frame: shown(), late: screen.queryByText(LATE) }).toEqual({
      status: ENGLISH,
      frame: null,
      late: null
    });
    expect(client.calls).toEqual(['card']);
  });

  it('a withheld card offers no reveal and no grade', async () => {
    // the machine, which every key, button, remote and stick reaches through its one handler
    const client = new FakeClient([head(withheld(1, { undo: 'answer' })), head(null)]);
    const review = new Review(
      async () => client,
      () => 0,
      () => {}
    );
    review.start();
    await review.settled();
    expect(review.controls, 'a withheld card offers a reveal or a grade').toEqual(['undo', 'bury', 'flag']);

    // a reveal, a grade and a replay change nothing and send nothing
    for (const action of ['show-answer', 'again', 'good', 'replay'] as const) review.act(action);
    await review.settled();
    expect([review.phase, review.status, review.controls, client.calls]).toEqual([
      'withheld',
      'withheld',
      ['undo', 'bury', 'flag'],
      ['card']
    ]);

    // a flag marks the card it named and keeps it withheld
    review.act('flag');
    await review.settled();
    expect([review.phase, review.view?.flag, review.status, client.calls]).toEqual([
      'withheld',
      1,
      'withheld',
      ['card', 'flag 1']
    ]);

    // an undo with nothing to offer says so and returns to the withheld card, then the line again
    review.act('undo');
    await review.settled();
    expect([review.phase, review.status, client.calls.at(-1)]).toEqual(['withheld', 'not-undoable', 'undo-offer']);
    review.act('show-answer');
    await review.settled();
    expect([review.phase, review.status, client.calls.length]).toEqual(['withheld', 'withheld', 3]);

    // the page: no reveal and no grade button, and no key sends one
    const page = new FakeClient([head(withheld(2)), head(null)]);
    render(ReviewScreen, { client: async () => page });
    await settle();
    const offered = ['Show answer', /^Again/, /^Good/, 'Replay'].map((name) => screen.queryByRole('button', { name }));
    expect(offered).toEqual([null, null, null, null]);
    for (const key of [' ', 'Enter', '1', '3', 'r', 'u', '-']) await fireEvent.keyDown(window, { key });
    await fireEvent.keyDown(window, { key: '1', ctrlKey: true });
    await settle();
    expect({ calls: page.calls, status: status(), frame: shown() }).toEqual({
      calls: ['card'],
      status: ENGLISH,
      frame: null
    });
  });

  it('burying a withheld card moves on and grades nothing', async () => {
    const client = new FakeClient([head(withheld(1)), head(view(2)), head(null)]);
    render(ReviewScreen, { client: async () => client });
    await settle();

    await fireEvent.click(screen.getByRole('button', { name: 'Bury' }));
    await settle();
    expect({ calls: client.calls, frame: shown(), status: status() }).toEqual({
      calls: ['card', 'bury 1', 'card'],
      frame: { title: "The card's question", text: 'question 2' },
      status: ''
    });
  });
});

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
