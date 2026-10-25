/**
 * @vitest-environment jsdom
 */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { CardView, Head, StatusWord, Synced, Unsynced, UndoOffer } from '$lib/engine/protocol';
import StudyReview from '../../routes/study/review/+page.svelte';
import { Review, type StudyClient } from './review';
import ReviewScreen from './ReviewScreen.svelte';
import type { UpdateClient } from './update-now';

// SPEC-409 R10, R11, A13, A14; ADR-423 D4, D5. Every locale holds the four words of the one-tap
// sync offer under one set of keys, with its own text and no word for the streak, and the study
// code's shipped sync calls are the session's one and the offer's one.
const routes = vi.hoisted(() => ({ client: vi.fn() }));
vi.mock('$lib/study/engine', () => ({ studyEngine: () => ({ client: routes.client }) }));

afterEach(() => {
  cleanup();
  routes.client.mockReset();
});

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

// ---- the screen's offer: A1 to A12 ----

/** The offer's lines in English, as SPEC-409 R6 and R10 word them. */
const PROMPT = ENGLISH.study_update_prompt;
const DONE = 'Sync finished.';
const OFFLINE = 'The sync server cannot be reached right now. Your reviews stay on this device.';
const ABSENT = 'You are not signed in to sync.';
const ENDED = 'Your sign-in has ended. Sign in again to sync.';
const FULL =
  'The server and this device changed in ways a normal sync cannot merge. Choose which copy to keep.';
const STOPPED = 'The study engine stopped. Try again.';

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
    assisted: false,
    ...extra
  };
}

function head(card: CardView | null): Head {
  return { counts: { new: 0, learning: 0, review: 3 }, card };
}

/** The engine as the review sees it: the heads queued in order, each rate counted, or refused. */
class FakeStudy implements StudyClient {
  heads: Head[];
  rates: bigint[] = [];
  refuse = false;
  buried: bigint[] = [];

  constructor(heads: Head[]) {
    this.heads = heads;
  }

  card(): Promise<Head> {
    return Promise.resolve(this.heads.shift() as Head);
  }

  rate(card: bigint): Promise<null> {
    if (this.refuse) return Promise.reject(new Error('refused'));
    this.rates.push(card);
    return Promise.resolve(null);
  }

  bury(card: bigint): Promise<null> {
    this.buried.push(card);
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

/** The sync client as the offer sees it: each call counted in order, answered from fixed values,
 * or held pending until the test lets it answer. */
class FakeUpdate implements UpdateClient {
  calls: string[] = [];
  answer: Synced | Error = { status: 'held', required: null };
  count: Unsynced | Error = { reviews: 0, changed: false, schema: false };
  hold = false;
  release: (() => void) | null = null;

  sync(): Promise<Synced> {
    this.calls.push('sync');
    const answer = this.answer;
    const settle = () => (answer instanceof Error ? Promise.reject(answer) : Promise.resolve(answer));
    if (!this.hold) return settle();
    return new Promise<void>((resolve) => (this.release = resolve)).then(settle);
  }

  unsynced(): Promise<Unsynced> {
    this.calls.push('unsynced');
    return this.count instanceof Error ? Promise.reject(this.count) : Promise.resolve(this.count);
  }
}

/** Lets every answer arrive and every timer of zero run, then lets Svelte update the page. */
async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

/** Shows the screen over `study`, with `update` when given. */
async function open(study: FakeStudy, update?: FakeUpdate): Promise<void> {
  render(ReviewScreen, { client: async () => study, ...(update === undefined ? {} : { update: async () => update }) });
  await settle();
}

/** Shows the answer of the card on screen and grades it Good. */
async function answerGood(): Promise<void> {
  await fireEvent.click(screen.getByRole('button', { name: 'Show answer' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Good <10m' }));
  await settle();
}

/** Whether `first` comes before `second` in the page. */
function before(first: Node, second: Node): boolean {
  return (first.compareDocumentPosition(second) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
}

/** The status regions the page holds, in order; the offer's is the last. */
function regions(): HTMLElement[] {
  return [...document.querySelectorAll<HTMLElement>('[role="status"]')];
}

function tap(): HTMLButtonElement {
  return screen.getByRole('button', { name: 'Sync now' });
}

describe('the one-tap sync offer', () => {
  it('a late review raises the offer below the controls', async () => {
    const study = new FakeStudy([head(view(1, { late: true })), head(view(2)), head(null)]);
    const update = new FakeUpdate();
    await open(study, update);
    expect(screen.queryByText(PROMPT)).toBeNull();

    await answerGood();
    expect(study.rates).toEqual([1n]);
    const line = screen.getByText(PROMPT);
    expect(screen.getAllByRole('button').map((button) => button.textContent?.trim()).filter((name) => name === 'Sync now' || name === 'Not now')).toEqual([
      'Sync now',
      'Not now'
    ]);
    // after the control bar and before the nav
    expect(before(screen.getByRole('button', { name: 'Flag' }), line)).toBe(true);
    expect(before(line, screen.getByRole('link', { name: 'Back to decks' }))).toBe(true);
    // the focus is where the answer left it, on the screen's region, and never on the offer
    expect(document.activeElement).toBe(document.querySelector('section'));
    expect(update.calls).toEqual([]);
  });

  it('an on-time review and an unanswered late card raise no offer', async () => {
    const study = new FakeStudy([head(view(1)), head(view(2, { late: true })), head(view(3)), head(null)]);
    await open(study, new FakeUpdate());
    await answerGood();
    expect(study.rates).toEqual([1n]);
    // the late card is shown, and not answered
    expect(screen.queryByRole('button', { name: 'Show answer' })).not.toBeNull();
    expect(screen.queryByText(ENGLISH.study_update_prompt)).toBeNull();
    expect(screen.queryByRole('button', { name: 'Not now' })).toBeNull();

    // its first answer raises it
    await answerGood();
    expect(study.rates).toEqual([1n, 2n]);
    expect(screen.queryByText(PROMPT)).not.toBeNull();
  });

  it('not now hides the offer for the screen and syncs nothing', async () => {
    const study = new FakeStudy([head(view(1, { late: true })), head(view(2, { late: true })), head(view(3, { late: true })), head(null)]);
    const update = new FakeUpdate();
    await open(study, update);
    await answerGood();
    expect(screen.queryByText(PROMPT)).not.toBeNull();

    await fireEvent.click(screen.getByRole('button', { name: 'Not now' }));
    await settle();
    expect(screen.queryByText(PROMPT)).toBeNull();
    expect(screen.queryByRole('button', { name: 'Sync now' })).toBeNull();

    // a later late review raises nothing
    await answerGood();
    expect(study.rates).toEqual([1n, 2n]);
    expect(screen.queryByText(PROMPT)).toBeNull();
    expect(update.calls).toEqual([]);

    // pressed while a sync runs, it hides the offer and the sync ends without a line
    cleanup();
    const held = new FakeStudy([head(view(1, { late: true })), head(null)]);
    const pending = new FakeUpdate();
    pending.hold = true;
    await open(held, pending);
    await answerGood();
    await fireEvent.click(tap());
    await settle();
    expect(pending.calls).toEqual(['sync']);

    await fireEvent.click(screen.getByRole('button', { name: 'Not now' }));
    (pending.release as () => void)();
    await settle();
    expect(pending.calls).toEqual(['sync', 'unsynced']);
    expect([screen.queryByText(DONE), screen.queryByText(PROMPT), screen.queryByText('Syncing…')]).toEqual([null, null, null]);
    expect(regions().at(-1)?.textContent?.trim()).toBe('');
  });

  it('one tap syncs once and shows the result', async () => {
    const study = new FakeStudy([head(view(1, { late: true })), head(view(2)), head(null)]);
    const update = new FakeUpdate();
    update.hold = true;
    await open(study, update);
    await answerGood();
    const focus = document.activeElement;

    await fireEvent.click(tap());
    await settle();
    expect(update.calls).toEqual(['sync']);
    expect(screen.queryByText('Syncing…')).not.toBeNull();
    expect(tap().disabled).toBe(true);

    (update.release as () => void)();
    await settle();
    expect(update.calls).toEqual(['sync', 'unsynced']);
    expect(screen.queryByText(DONE)).not.toBeNull();
    expect(screen.queryByText('Reviews waiting to sync: 0')).not.toBeNull();
    expect(screen.queryByText('Syncing…')).toBeNull();
    expect(tap().disabled).toBe(false);
    expect(document.activeElement).toBe(focus);
  });

  it('an offline answer says so and lets the learner retry', async () => {
    const study = new FakeStudy([head(view(1, { late: true })), head(null)]);
    const update = new FakeUpdate();
    update.answer = { status: 'offline', required: null };
    update.count = { reviews: 3, changed: false, schema: false };
    await open(study, update);
    await answerGood();

    await fireEvent.click(tap());
    await settle();
    expect(screen.queryByText(OFFLINE)).not.toBeNull();
    expect(screen.queryByText('Reviews waiting to sync: 3')).not.toBeNull();
    expect(screen.queryByRole('link', { name: 'Open the sync page' })).toBeNull();
    expect(tap().disabled).toBe(false);

    await fireEvent.click(tap());
    await settle();
    expect(update.calls).toEqual(['sync', 'unsynced', 'sync', 'unsynced']);
  });

  it('a sync with no sign-in links to the sync page', async () => {
    const lines: [StatusWord, string][] = [
      ['absent', ABSENT],
      ['needs-sign-in', ENDED]
    ];
    for (const [status, line] of lines) {
      const study = new FakeStudy([head(view(1, { late: true })), head(null)]);
      const update = new FakeUpdate();
      update.answer = { status, required: null };
      update.count = { reviews: 2, changed: false, schema: false };
      await open(study, update);
      await answerGood();
      await fireEvent.click(tap());
      await settle();
      expect(screen.queryByText(line), status).not.toBeNull();
      expect(screen.getByRole('link', { name: 'Open the sync page' }).getAttribute('href'), status).toBe('/sync');
      expect(screen.queryByText('Reviews waiting to sync: 2'), status).not.toBeNull();
      cleanup();
    }
  });

  it('a full sync required sends the learner to the sync page', async () => {
    const study = new FakeStudy([head(view(1, { late: true })), head(null)]);
    const update = new FakeUpdate();
    update.answer = { status: 'held', required: 'full-sync' };
    update.count = { reviews: 4, changed: false, schema: false };
    await open(study, update);
    await answerGood();
    await fireEvent.click(tap());
    await settle();
    expect(screen.queryByText(FULL)).not.toBeNull();
    expect(screen.queryByText(DONE)).toBeNull();
    expect(screen.getByRole('link', { name: 'Open the sync page' }).getAttribute('href')).toBe('/sync');
    expect(screen.queryByText('Reviews waiting to sync: 4')).not.toBeNull();
    // the review makes no full-sync call: the client it was given holds none
    expect(update.calls).toEqual(['sync', 'unsynced']);
  });

  it('a refused sync says the engine stopped and lets the learner retry', async () => {
    for (const refuse of ['sync', 'unsynced']) {
      const study = new FakeStudy([head(view(1, { late: true })), head(null)]);
      const update = new FakeUpdate();
      if (refuse === 'sync') update.answer = new Error('stopped');
      else update.count = new Error('stopped');
      await open(study, update);
      await answerGood();
      await fireEvent.click(tap());
      await settle();
      expect(screen.queryByText(STOPPED), refuse).not.toBeNull();
      expect(screen.queryByText(/Reviews waiting to sync/), refuse).toBeNull();
      expect(tap().disabled, refuse).toBe(false);
      cleanup();
    }
  });

  it('the review goes on while a sync is pending', async () => {
    const study = new FakeStudy([head(view(1, { late: true })), head(view(2)), head(null)]);
    const update = new FakeUpdate();
    update.hold = true;
    await open(study, update);
    await answerGood();
    await fireEvent.click(tap());
    await settle();
    expect(update.calls).toEqual(['sync']);

    // the next card shows and its rate reaches the client, with the sync still pending
    expect(screen.queryByRole('button', { name: 'Show answer' })).not.toBeNull();
    await answerGood();
    expect(study.rates).toEqual([1n, 2n]);
    expect(update.calls).toEqual(['sync']);
  });

  it('the offer is reachable by touch keyboard and screen reader', async () => {
    const study = new FakeStudy([head(view(1, { late: true })), head(null)]);
    await open(study, new FakeUpdate());
    // the region is in the page, empty, before the offer shows
    expect(regions().length).toBe(2);
    expect(regions().at(-1)?.textContent?.trim()).toBe('');

    await answerGood();
    const buttons = ['Sync now', 'Not now'].map((name) => screen.getByRole('button', { name }) as HTMLButtonElement);
    for (const button of buttons) {
      expect(button.tagName).toBe('BUTTON');
      expect(button.type).toBe('button');
      expect(button.classList.contains('min-h-11')).toBe(true);
    }
    expect(regions().at(-1)?.textContent).toContain(PROMPT);

    // a screen given no update client holds no region and no offer
    cleanup();
    await open(new FakeStudy([head(view(1, { late: true })), head(null)]));
    await answerGood();
    expect(regions().length).toBe(1);
    expect(screen.queryByText(PROMPT)).toBeNull();
    expect(screen.queryByRole('button', { name: 'Sync now' })).toBeNull();
  });

  it('the review route gives the screen the engine sync', async () => {
    const study = new FakeStudy([head(view(1, { late: true })), head(null)]);
    const update = new FakeUpdate();
    routes.client.mockResolvedValue(Object.assign(study, { sync: () => update.sync(), unsynced: () => update.unsynced() }));
    render(StudyReview);
    await settle();
    await answerGood();
    expect(screen.queryByText(PROMPT)).not.toBeNull();
    await fireEvent.click(tap());
    await settle();
    expect(update.calls).toEqual(['sync', 'unsynced']);
    expect(screen.queryByText(DONE)).not.toBeNull();
  });
});

describe('the review counts only settled late answers', () => {
  it('the review counts only settled late answers', async () => {
    const study = new FakeStudy([
      head(view(1, { late: true })),
      head(view(2)),
      head(view(3, { late: true })),
      head(view(4, { late: true })),
      head(null)
    ]);
    const review = new Review(async () => study, () => 0, () => undefined);
    const rate = async (): Promise<void> => {
      review.act('show-answer');
      review.act('good');
      await review.settled();
    };
    review.start();
    await review.settled();
    expect(review.lateReviews).toBe(0);

    await rate();
    expect(study.rates).toEqual([1n]);
    expect(review.lateReviews).toBe(1);

    // an on-time rate counts none
    await rate();
    expect(study.rates).toEqual([1n, 2n]);
    expect(review.lateReviews).toBe(1);

    // a bury of a late card counts none
    review.act('bury');
    await review.settled();
    expect(study.buried).toEqual([3n]);
    expect(review.lateReviews).toBe(1);

    // a refused late rate counts none
    study.refuse = true;
    review.act('show-answer');
    review.act('good');
    await review.settled();
    expect(study.rates).toEqual([1n, 2n]);
    expect(review.lateReviews).toBe(1);
  });
});
