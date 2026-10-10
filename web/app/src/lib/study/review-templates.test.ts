/**
 * @vitest-environment jsdom
 */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { EngineError } from '$lib/engine/client';
import type { CardView, Faces, Head, UndoOffer } from '$lib/engine/protocol';
import { telegram } from '$lib/telegram.svelte';
import type { StudyClient } from './review';
import ReviewScreen from './ReviewScreen.svelte';

// SPEC-372 R4, A1 to A4; ADR-383 D4, D5. The review screen shows the template cards the engine
// rendered: a cloze card's two deletions, a reversed card's answer that opens with its question
// where its template says FrontSide, and each note type's own CSS. The fake client serves the
// golden file's render, which crates/engine-core/tests/review_templates.rs compares with the
// engine's: the head card carries the golden ordinal and CSS and two sides of its own, and the faces
// carry the golden texts. Each side is read from the frame's document as the frame's parser reads
// it, and the frame CSS each note type should show is written by hand in the golden file, so no
// expectation is computed by the frame's own code.

/** One card of the golden file's render. */
interface GoldenCard {
  note: string;
  ordinal: number;
  question: string;
  answer: string;
  face_css: string;
  card_css: string;
}

/** The golden file: the fixture's inputs and frame CSS, written by hand, and the engine's render. */
interface Golden {
  notetypes: { name: string; stock: string; css: string; templates: { name: string; question: string; answer: string }[] }[];
  notes: { name: string; notetype: string; fields: string[] }[];
  frame_css: Record<string, string>;
  cards: GoldenCard[];
}

// read beside this file, as docs-mermaid.test.ts reads its JSON: under jsdom the global URL is the
// page's, which node:fs refuses as a file URL
const GOLDEN = JSON.parse(readFileSync(resolve(import.meta.dirname, 'review-templates.golden.json'), 'utf8')) as Golden;
const COUNTS = { new: 1, learning: 0, review: 2 };
const LABELS = ['<1m', '<6m', '<10m', '4d'];
const ESCAPES = '</style><base href="https://cards.example/">';
/** The body's classes for each ordinal, written out. */
const CLASSES: Record<number, string> = { 0: 'card card1', 1: 'card card2' };

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** The note type a golden card's note has. */
function notetype(card: GoldenCard): string {
  const note = GOLDEN.notes.find((written) => written.name === card.note);
  expect(note, `the golden file has the note ${card.note}`).toBeDefined();
  return (note as Golden['notes'][number]).notetype;
}

/** A head card for golden card `index`: its id, the golden ordinal and CSS, and two sides of its own. */
function view(index: number, extra: Partial<CardView> = {}): CardView {
  const card = GOLDEN.cards[index];
  return {
    id: BigInt(index + 1),
    ordinal: card.ordinal,
    flag: 0,
    question: '<p>the head card question</p>',
    answer: '<p>the head card answer</p>',
    css: card.card_css,
    labels: LABELS,
    undo: null,
    late: false,
    withheld: false,
    ...extra
  };
}

function head(card: CardView | null): Head {
  return { counts: COUNTS, card };
}

/** The engine as the screen sees it: each call recorded, each answer queued or refused once, and
 * the faces of card `n` the golden file's card `n - 1`. */
class FakeClient implements StudyClient {
  calls: string[] = [];
  heads: Head[];
  refusals: Record<string, EngineError> = {};

  constructor(heads: Head[]) {
    this.heads = heads;
  }

  card(): Promise<Head> {
    this.calls.push('card');
    return this.#answer('card', () => this.heads.shift() as Head);
  }

  rate(card: bigint, rating: number, ms: number): Promise<null> {
    this.calls.push(`rate ${card} ${rating} ${ms}`);
    return this.#answer('rate', () => null);
  }

  bury(card: bigint): Promise<null> {
    this.calls.push(`bury ${card}`);
    return this.#answer('bury', () => null);
  }

  flag(card: bigint): Promise<number> {
    this.calls.push(`flag ${card}`);
    return this.#answer('flag', () => 1);
  }

  undo(card?: bigint, step?: number): Promise<null> {
    this.calls.push(card === undefined ? 'undo' : `undo ${card} ${step}`);
    return this.#answer('undo', () => null);
  }

  undoOffer(): Promise<UndoOffer> {
    this.calls.push('undo-offer');
    return this.#answer('undo-offer', () => ({ offer: null, why: 'none' }) as UndoOffer);
  }

  faces(card: bigint): Promise<Faces> {
    this.calls.push(`faces ${card}`);
    const golden = GOLDEN.cards[Number(card) - 1];
    const side = (text: string) => ({ text, css: golden.face_css, autoplay: [], replay: [], omitted: [] });
    return this.#answer('faces', () => ({ question: side(golden.question), answer: side(golden.answer), wanted: [] }));
  }

  async #answer<T>(op: string, value: () => T): Promise<T> {
    const refusal = this.refusals[op];
    if (refusal !== undefined) {
      delete this.refusals[op];
      throw refusal;
    }
    return value();
  }
}

class FakeSentinel {
  released = 0;

  release(): Promise<void> {
    this.released += 1;
    return Promise.resolve();
  }

  addEventListener(_type: 'release', _listener: () => void): void {}
}

class FakeWakeLock {
  requests = 0;
  #pending: ((sentinel: FakeSentinel) => void)[] = [];

  request(_type: 'screen'): Promise<FakeSentinel> {
    this.requests += 1;
    return new Promise((resolve) => this.#pending.push(resolve));
  }
}

let visibility: DocumentVisibilityState = 'visible';
/** The animation frames asked for and not yet run or cancelled, by id. */
const frames = new Map<number, FrameRequestCallback>();
let frameIds = 0;
let cancelled: number[] = [];
let lock: FakeWakeLock;
let now = 1000;

/** Lets every answer arrive and every timer of zero run, then lets Svelte update the page. */
async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

/** The card frame. */
function frame(): HTMLIFrameElement {
  return document.querySelector('iframe') as HTMLIFrameElement;
}

function status(): string {
  return screen.getByRole('status').textContent?.trim() ?? '';
}

function button(name: string | RegExp): HTMLButtonElement {
  return screen.getByRole('button', { name }) as HTMLButtonElement;
}

beforeEach(() => {
  visibility = 'visible';
  frames.clear();
  frameIds = 0;
  cancelled = [];
  lock = new FakeWakeLock();
  now = 1000;
  vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
    frameIds += 1;
    frames.set(frameIds, callback);
    return frameIds;
  });
  vi.stubGlobal('cancelAnimationFrame', (id: number) => {
    cancelled.push(id);
    frames.delete(id);
  });
  vi.spyOn(performance, 'now').mockImplementation(() => now);
  Object.defineProperty(navigator, 'wakeLock', { configurable: true, value: lock });
  Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => visibility });
  localStorage.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  // the default page has no gamepad API at all, as jsdom has none
  Reflect.deleteProperty(navigator, 'getGamepads');
  telegram.colorScheme = 'light';
});

/** A card text as the frame's parser reads it: the body it makes of the text alone. */
function asTheFrameReads(text: string): string {
  return new DOMParser().parseFromString('<!doctype html><body>' + text, 'text/html').body.innerHTML;
}

/** What the frame shows for one side of one golden card. */
interface Observation {
  name: string;
  card: GoldenCard;
  side: 'question' | 'answer';
  srcdoc: string | null;
  refused: string | null;
  status: string;
  /** The frame's document as its parser reads the srcdoc. */
  doc: Document;
  body: string;
  classes: string;
  style: string | null;
}

function observe(card: GoldenCard, side: 'question' | 'answer'): Observation {
  const srcdoc = frame().getAttribute('srcdoc');
  const doc = new DOMParser().parseFromString(srcdoc ?? '', 'text/html');
  return {
    name: `${card.note} ordinal ${card.ordinal} ${side}`,
    card,
    side,
    srcdoc,
    refused: frame().getAttribute('data-card-refused'),
    status: status(),
    doc,
    body: doc.body.innerHTML,
    classes: doc.body.className,
    style: doc.querySelector('style')?.textContent ?? null
  };
}

/** Renders the review over every golden card and observes each side: the question, then Show
 * answer and the answer, then Good. */
async function walk(): Promise<Observation[]> {
  const client = new FakeClient([...GOLDEN.cards.map((_, index) => head(view(index))), head(null)]);
  const shown = render(ReviewScreen, { client: async () => client });
  const observations: Observation[] = [];
  for (const card of GOLDEN.cards) {
    await settle();
    observations.push(observe(card, 'question'));
    await fireEvent.click(button('Show answer'));
    observations.push(observe(card, 'answer'));
    await fireEvent.click(button('Good <10m'));
    await settle();
  }
  shown.unmount();
  return observations;
}

describe('the review frame shows the template cards as the engine rendered them', () => {
  it("a cloze card's question and answer reach the frame as the engine rendered them, with its card's classes", async () => {
    const cloze = (await walk()).filter((seen) => seen.card.note === 'cloze');
    for (const seen of cloze) {
      expect(seen.body, `${seen.name}: the body`).toBe(asTheFrameReads(seen.card[seen.side]));
      expect(seen.classes, `${seen.name}: the classes`).toBe(CLASSES[seen.card.ordinal]);
    }
    const questions = cloze.filter((seen) => seen.side === 'question');
    for (const seen of questions) {
      expect(seen.doc.body.querySelector('.cloze'), `${seen.name}: a .cloze element`).not.toBeNull();
    }
    examined('cloze cards', questions);
  });

  it("a reversed card's answer opens with its question where its template says FrontSide", async () => {
    const reversed = (await walk()).filter((seen) => seen.card.note === 'reversed');
    const answers = reversed.filter((seen) => seen.side === 'answer');
    for (const answer of answers) {
      const question = reversed.find((seen) => seen.card === answer.card && seen.side === 'question') as Observation;
      expect(answer.body, `${answer.name}: the body`).toBe(asTheFrameReads(answer.card.answer));
      expect(answer.body.startsWith(question.body), `${answer.name}: opens with ${question.body}`).toBe(true);
      expect(answer.doc.body.querySelector('hr#answer'), `${answer.name}: the rule`).not.toBeNull();
    }
    examined('reversed cards', answers);
  });

  it("the note type's css reaches the frame's style element with only its newlines normalised", async () => {
    const observations = await walk();
    for (const seen of observations) {
      expect(seen.style, `${seen.name}: the style element`).toBe(GOLDEN.frame_css[notetype(seen.card)]);
      if (seen.card.note === 'cloze') {
        expect(seen.style, `${seen.name}: the cloze rule`).toContain('.cloze {');
      }
    }
    const cloze = GOLDEN.notetypes.find((written) => written.name === 'Template frame cloze');
    expect(cloze?.css, 'the cloze css input carries a CR LF').toContain('\r\n');
    expect(cloze?.css, 'the cloze css input carries a lone CR').toMatch(/\r(?!\n)/);
    examined('style elements', observations);
  });

  it("the frame's document check refuses none of the template cards", async () => {
    const observations = await walk();
    for (const seen of observations) {
      expect(
        { refused: seen.refused, srcdoc: seen.srcdoc !== null, status: seen.status },
        `${seen.name}: data-card-refused, srcdoc and status`
      ).toEqual({ refused: null, srcdoc: true, status: '' });
    }
    examined('faces', observations);

    // the control: the same cloze card with CSS that ends its style element is refused
    const client = new FakeClient([head(view(0, { css: GOLDEN.cards[0].card_css + ESCAPES })), head(null)]);
    render(ReviewScreen, { client: async () => client });
    await settle();
    expect(frame().getAttribute('data-card-refused'), 'the control is refused').toBe('escaped');
  });
});
