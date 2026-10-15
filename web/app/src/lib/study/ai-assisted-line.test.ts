/**
 * @vitest-environment jsdom
 */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { describe, expect, it } from 'vitest';
import { hosted } from '$lib/card/frame-host';
import type { CardView, Head, UndoOffer } from '$lib/engine/protocol';
import type { StudyClient } from './review';
import ReviewScreen from './ReviewScreen.svelte';

// SPEC-388 R6, R7, R9, R10, A11 to A14; ADR-402 D2. A card whose note carries the ai-assisted tag
// shows one plain line above the card, on both sides and below the late line when both show, and
// an unmarked card shows none; every locale holds the line and the course sentence, the sentence
// keeps the literal tag, and no text names a maker or a model.
const APP = resolve(import.meta.dirname, '../../..');

/** The two keys every locale holds: the review's line and the progress screen's sentence. */
const KEYS = ['study_ai_assisted', 'progress_ai_assisted'] as const;
/** The line in English, as SPEC-388 R6 words it. */
const LINE = 'This card was made with AI assistance.';
/** The sentence in English, as SPEC-388 R8 words it. */
const SENTENCE = 'Cards made with AI assistance carry the tag ai-assisted.';
/** Each key's English text. */
const ENGLISH: Record<(typeof KEYS)[number], string> = {
  study_ai_assisted: LINE,
  progress_ai_assisted: SENTENCE
};
/** The tag the course sentence keeps, in every locale (R9). */
const TAG = 'ai-assisted';
/** The late line in English, as SPEC-376 R5 words it: the line shows below it when both show. */
const LATE = 'This card was due on an earlier day, so this review does not count toward the streak for that day.';
/** A run of Latin-script letters: one word of a text, as R10 reads it. */
const LATIN_WORD = /\p{Script=Latin}+/gu;
/** The words for AI that may begin with a capital letter anywhere in a text (R10). */
const AI_WORDS = ['AI', 'IA'];

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

/** What a locale's text owes the two keys and lacks, each named with its locale. */
function textProblems(locale: string, text: string): string[] {
  const held = JSON.parse(text) as Record<string, unknown>;
  const problems: string[] = [];
  for (const key of KEYS) {
    const value = held[key];
    if (typeof value !== 'string' || value === '') {
      problems.push(`${locale} lacks ${key}`);
      continue;
    }
    if (locale === 'en' && value !== ENGLISH[key]) problems.push(`en's ${key} is not the SPEC's English text`);
    if (locale !== 'en' && value.includes(ENGLISH[key])) problems.push(`${locale}'s ${key} holds the English text`);
    if (key === 'progress_ai_assisted' && !value.includes(TAG)) problems.push(`${locale}'s ${key} lacks the tag "${TAG}"`);
  }
  return problems;
}

/** What names a maker or a model in `text`, named with `where`: any ASCII digit, and any Latin word
 * that begins with a capital letter but the words for AI and the text's first word (R10). */
function makerProblems(where: string, text: string): string[] {
  const problems: string[] = [];
  if (/[0-9]/.test(text)) problems.push(`${where} holds a digit`);
  for (const match of text.matchAll(LATIN_WORD)) {
    const word = match[0];
    if (/^\p{Lu}/u.test(word) && !AI_WORDS.includes(word) && match.index !== 0) {
      problems.push(`${where} holds the capital word "${word}"`);
    }
  }
  return problems;
}

/** What a locale's two texts hold that names a maker or a model, or the key it lacks. */
function localeMakerProblems(locale: string, text: string): string[] {
  const held = JSON.parse(text) as Record<string, unknown>;
  return KEYS.flatMap((key) => {
    const value = held[key];
    if (typeof value !== 'string' || value === '') return [`${locale} lacks ${key}`];
    return makerProblems(`${locale}'s ${key}`, value);
  });
}

/** A card view as the engine reads it, on time and unmarked unless `extra` says otherwise. */
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
    assisted: false,
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
  const doc = new DOMParser().parseFromString(hosted(frame().getAttribute('srcdoc') ?? '') ?? '', 'text/html');
  return { title: frame().title, text: doc.body.textContent?.trim() ?? '' };
}

/** Whether `first` comes before `second` in the page. */
function before(first: Node, second: Node): boolean {
  return (first.compareDocumentPosition(second) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
}

/** The status line, the line and the card frame, in the order the page holds them. */
function order(line: HTMLElement): boolean[] {
  return [before(screen.getByRole('status'), line), before(line, frame()), frame().contains(line)];
}

describe('the ai-assisted line', () => {
  it('a marked card shows the line above the card', async () => {
    const client = new FakeClient([
      head(view(1, { assisted: true })),
      head(view(2, { assisted: true, late: true })),
      head(null)
    ]);
    render(ReviewScreen, { client: async () => client });
    await settle();

    // the question side: the line, below the status line, above the card frame and outside it
    const question = screen.queryByText(LINE);
    expect(question, 'a marked card shows the line on its question side').not.toBeNull();
    expect(order(question as HTMLElement)).toEqual([true, true, false]);
    expect(shown()).toEqual({ title: "The card's question", text: 'question 1' });

    // the answer side: the line stays where it was
    await fireEvent.click(screen.getByRole('button', { name: 'Show answer' }));
    const answer = screen.queryByText(LINE);
    expect(answer, 'a marked card shows the line on its answer side').not.toBeNull();
    expect(order(answer as HTMLElement)).toEqual([true, true, false]);
    expect(shown()).toEqual({ title: "The card's answer", text: 'answer 1' });

    // a marked card past its due day: the line below the late line, both above the card
    await fireEvent.click(screen.getByRole('button', { name: 'Good <10m' }));
    await settle();
    const late = screen.queryByText(LATE);
    const line = screen.queryByText(LINE);
    expect([late, line].map((shownLine) => shownLine !== null)).toEqual([true, true]);
    expect([before(late as HTMLElement, line as HTMLElement), ...order(line as HTMLElement)]).toEqual([
      true,
      true,
      true,
      false
    ]);
    expect(shown()).toEqual({ title: "The card's question", text: 'question 2' });
  });

  it('an unmarked card shows no line', async () => {
    const client = new FakeClient([head(view(1)), head(view(2, { assisted: true })), head(view(3)), head(null)]);
    render(ReviewScreen, { client: async () => client });
    await settle();

    // an unmarked card: its question, and no line
    expect([shown(), screen.queryByText(LINE)]).toEqual([{ title: "The card's question", text: 'question 1' }, null]);
    await fireEvent.click(screen.getByRole('button', { name: 'Show answer' }));
    expect([shown(), screen.queryByText(LINE)]).toEqual([{ title: "The card's answer", text: 'answer 1' }, null]);

    // a marked card, then an unmarked card: the unmarked card shows its question, and no line
    await fireEvent.click(screen.getByRole('button', { name: 'Good <10m' }));
    await settle();
    expect(shown()).toEqual({ title: "The card's question", text: 'question 2' });
    await fireEvent.click(screen.getByRole('button', { name: 'Show answer' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Good <10m' }));
    await settle();
    expect([shown(), screen.queryByText(LINE)]).toEqual([{ title: "The card's question", text: 'question 3' }, null]);
  });
});

describe('every locale holds both AI-assisted texts', () => {
  it('every locale holds both AI-assisted texts', () => {
    const found = Object.fromEntries(locales().map((locale) => [locale, textProblems(locale, messages(locale))]));
    expect(found).toEqual(Object.fromEntries(locales().map((locale) => [locale, []])));

    // each plant is refused by name
    const line = 'Esta tarjeta se hizo con ayuda de IA.';
    const sentence = 'Las tarjetas hechas con ayuda de IA llevan la etiqueta ai-assisted.';
    expect(textProblems('es', JSON.stringify({ study_ai_assisted: line, progress_ai_assisted: sentence }))).toEqual([]);
    expect(textProblems('ja', JSON.stringify({ tagline: 'x' }))).toEqual([
      'ja lacks study_ai_assisted',
      'ja lacks progress_ai_assisted'
    ]);
    expect(
      textProblems('es', JSON.stringify({ study_ai_assisted: line, progress_ai_assisted: sentence.replace(TAG, 'IA') }))
    ).toEqual([`es's progress_ai_assisted lacks the tag "${TAG}"`]);
    expect(textProblems('fr', JSON.stringify(ENGLISH))).toEqual([
      "fr's study_ai_assisted holds the English text",
      "fr's progress_ai_assisted holds the English text"
    ]);
    expect(textProblems('en', JSON.stringify({ ...ENGLISH, study_ai_assisted: 'This card was made with help.' }))).toEqual([
      "en's study_ai_assisted is not the SPEC's English text"
    ]);

    examined('locale files under messages', locales());
    expect(locales()).toEqual(['en', 'es', 'fr', 'ja', 'ko', 'zh-Hans', 'zh-Hant']);
  });
});

describe('the AI-assisted texts name no maker', () => {
  it('the AI-assisted texts name no maker', () => {
    const found = Object.fromEntries(locales().map((locale) => [locale, localeMakerProblems(locale, messages(locale))]));
    expect(found).toEqual(Object.fromEntries(locales().map((locale) => [locale, []])));

    // each plant is refused by name, and a text that keeps the rule passes
    expect(makerProblems('plant', 'This card was made with Examplecorp assistance.')).toEqual([
      'plant holds the capital word "Examplecorp"'
    ]);
    expect(makerProblems('plant', 'This card was made with AI 4 assistance.')).toEqual(['plant holds a digit']);
    expect(makerProblems('control', "Cette carte a été créée avec l'aide de l'IA.")).toEqual([]);
    expect(makerProblems('control', 'このカードはAIの支援を受けて作成されました。')).toEqual([]);

    const texts = locales().flatMap((locale) => {
      const held = JSON.parse(messages(locale)) as Record<string, unknown>;
      return KEYS.map((key) => held[key]).filter((value) => typeof value === 'string');
    });
    examined('texts of both keys in every locale', texts);
    expect(texts).toHaveLength(14);
  });
});
