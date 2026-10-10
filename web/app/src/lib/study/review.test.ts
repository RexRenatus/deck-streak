/**
 * @vitest-environment jsdom
 */
import { describe, expect, it } from 'vitest';
import { EngineError } from '$lib/engine/client';
import type { CardView, Head, UndoOffer } from '$lib/engine/protocol';
import { Review, step, type Effect, type Phase, type ReviewEvent, type ReviewState, type StudyClient } from './review';

// SPEC-350 R2, R7, R10, A11, A12; ADR-361. The review's machine shows a card's question, reveals
// its answer, rates the shown card with the time since its question showed, and moves on; while a
// request is in flight every gesture fires nothing; and a card the frame refuses keeps its controls.
const COUNTS = { new: 1, learning: 0, review: 2 };
const ESCAPES = '</style><base href="https://cards.example/">';

function view(id: number, extra: Partial<CardView> = {}): CardView {
  return {
    id: BigInt(id),
    ordinal: 0,
    flag: 0,
    question: `<p>question ${id}</p>`,
    answer: `<p>answer ${id}</p>`,
    css: '.card { color: navy; }',
    labels: ['<1m', '<6m', '<10m', '4d'],
    undo: null,
    late: false,
    withheld: false,
    ...extra
  };
}

/** The offer of the review's own last answer: card 1, answered Good at step 7, back to new. */
const OFFER: UndoOffer = { offer: { card: 1n, step: 7, text: 'question 1', grade: 'good', returns: 'new' } };

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty`).toBeGreaterThan(0);
  return items;
}

/** Lets every answer that is not held arrive. */
function flush(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

function head(card: CardView | null): Head {
  return { counts: COUNTS, card };
}

/** The engine as the review sees it: each call recorded, each answer queued, held or refused. */
class FakeClient implements StudyClient {
  calls: string[] = [];
  heads: Head[];
  refusals: Record<string, EngineError> = {};
  #gate: Promise<void> | null = null;
  #open: () => void = () => undefined;

  constructor(heads: Head[]) {
    this.heads = heads;
  }

  /** Holds every answer until `release`. */
  hold(): void {
    this.#gate = new Promise((resolve) => (this.#open = resolve));
  }

  release(): void {
    this.#gate = null;
    this.#open();
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

  /** What the offer answers: the record of the review's own last answer, or why there is none. */
  offered: UndoOffer = OFFER;

  undoOffer(): Promise<UndoOffer> {
    this.calls.push('undo-offer');
    return this.#answer('undo-offer', () => this.offered);
  }

  async #answer<T>(op: string, value: () => T): Promise<T> {
    if (this.#gate !== null) await this.#gate;
    const refusal = this.refusals[op];
    if (refusal !== undefined) {
      delete this.refusals[op];
      throw refusal;
    }
    return value();
  }
}

describe('the review', () => {
  it('the review shows, reveals, rates and moves on', async () => {
    const client = new FakeClient([
      head(view(1)),
      head(view(2, { undo: 'answer' })),
      head(view(1, { ordinal: 1 })),
      head(null)
    ]);
    let time = 1000;
    let changes = 0;
    const review = new Review(
      async () => client,
      () => time,
      () => changes++
    );

    review.start();
    expect(review.phase).toBe('loading');
    await review.settled();
    expect([review.phase, review.side, review.view?.id]).toEqual(['question', 'question', 1n]);
    expect(review.counts).toEqual(COUNTS);
    expect(review.controls).toEqual(['show-answer', 'bury', 'flag']);
    expect(review.status).toBeNull();

    // a grade on the question side fires nothing; show answer reveals the answer
    review.act('good');
    expect(review.phase).toBe('question');
    review.act('show-answer');
    expect([review.phase, review.side]).toEqual(['answer', 'answer']);
    expect(review.controls).toEqual(['again', 'good', 'bury', 'flag']);

    // a rating names the shown card, its grade and the milliseconds since its question showed
    time = 3500.4;
    review.act('good');
    expect(review.phase).toBe('busy');
    await review.settled();
    expect(client.calls).toEqual(['card', 'rate 1 3 2500', 'card']);
    expect([review.phase, review.side, review.view?.id]).toEqual(['question', 'question', 2n]);

    // the review's own last answer can be undone: undo shows; the flag acts on the shown card and keeps its side
    expect(review.controls).toEqual(['show-answer', 'undo', 'bury', 'flag']);
    review.act('flag');
    expect(review.phase).toBe('busy');
    await review.settled();
    expect([review.phase, review.view?.flag]).toEqual(['question', 1]);
    review.act('show-answer');
    review.act('flag');
    await review.settled();
    expect([review.phase, review.side]).toEqual(['answer', 'answer']);

    // undo asks for the offer of the rated answer, its confirmation returns the rated card, and bury
    // moves on from it
    review.act('undo');
    await review.settled();
    expect(review.phase).toBe('confirming');
    review.act('undo');
    await review.settled();
    expect([review.phase, review.view?.id, review.view?.ordinal]).toEqual(['question', 1n, 1]);
    // with no undoable action, undo fires nothing
    review.act('undo');
    expect(review.phase).toBe('question');
    review.act('bury');
    await review.settled();
    expect(client.calls).toEqual([
      'card',
      'rate 1 3 2500',
      'card',
      'flag 2',
      'flag 2',
      'undo-offer',
      'undo 1 7',
      'card',
      'bury 1',
      'card'
    ]);

    // the deck is done: nothing shows, and the status region says so
    expect([review.phase, review.view, review.controls, review.status]).toEqual(['done', null, [], 'done']);
    expect(changes).toBeGreaterThan(0);
  });

  it('a gesture during a request fires nothing', async () => {
    const client = new FakeClient([head(view(1)), head(view(2)), head(view(2)), head(view(3))]);
    const review = new Review(
      async () => client,
      () => 0,
      () => undefined
    );
    review.start();
    // a gesture while the card loads fires nothing
    review.act('show-answer');
    await review.settled();
    expect(review.phase).toBe('question');

    review.act('show-answer');
    client.hold();
    review.act('good');
    for (const action of ['good', 'again', 'bury', 'flag', 'show-answer'] as const) review.act(action);
    await flush();
    expect([review.phase, client.calls]).toEqual(['busy', ['card', 'rate 1 3 0']]);
    client.release();
    await review.settled();
    expect([review.phase, client.calls]).toEqual(['question', ['card', 'rate 1 3 0', 'card']]);

    // a card the engine no longer shows is refused as not-shown, and the queue's head loads again
    review.act('show-answer');
    client.refusals.rate = new EngineError('not-shown', 'not the shown card');
    review.act('again');
    await review.settled();
    expect([review.phase, review.status, client.calls.slice(3)]).toEqual([
      'question',
      'not-shown',
      ['rate 2 1 0', 'card']
    ]);

    // any other refusal is announced, fires nothing more, and a retry asks again
    client.refusals.bury = new EngineError('collection-busy', 'another tab has the collection');
    review.act('bury');
    await review.settled();
    expect([review.phase, review.status, review.controls]).toEqual(['refused', 'collection-busy', []]);
    review.act('show-answer');
    review.act('good');
    await flush();
    expect(client.calls.slice(5)).toEqual(['bury 2']);
    review.retry();
    await review.settled();
    expect([review.phase, review.view?.id, review.status, client.calls.slice(5)]).toEqual([
      'question',
      3n,
      null,
      ['bury 2', 'card']
    ]);

    // the step table itself drops every gesture in flight
    for (const action of ['again', 'good', 'bury', 'flag', 'undo', 'show-answer'] as const) {
      expect(step({ phase: 'busy', side: 'answer' }, action), action).toEqual({
        state: { phase: 'busy', side: 'answer' },
        effect: 'none'
      });
    }
  });

  it('a refused card keeps its answer controls', async () => {
    const client = new FakeClient([head(view(1, { css: ESCAPES })), head(view(2))]);
    const review = new Review(
      async () => client,
      () => 0,
      () => undefined
    );
    review.start();
    await review.settled();

    // the frame refuses the card; its controls stay, and the status region says why
    expect(review.controls).toEqual(['show-answer', 'bury', 'flag']);
    expect([review.phase, review.escaped, review.status]).toEqual(['question', true, 'escaped']);
    review.act('show-answer');
    expect(review.controls).toEqual(['again', 'good', 'bury', 'flag']);
    expect([review.phase, review.escaped, review.status]).toEqual(['answer', true, 'escaped']);
    review.act('good');
    await review.settled();
    expect(client.calls).toEqual(['card', 'rate 1 3 0', 'card']);

    // the next card renders, and the status region is quiet again
    expect([review.phase, review.escaped, review.status]).toEqual(['question', false, null]);
  });

  // Ruling 317 OQ1. A grade moves the side to the question as the request leaves, so the screen
  // reads the face, the card and side last shown, which a request in flight and the next card's
  // load keep, and which a refusal and a done deck clear.
  it('the frame keeps its card while a request is in flight', async () => {
    const client = new FakeClient([head(view(1)), head(view(2)), head(view(3, { css: ESCAPES })), head(null)]);
    const review = new Review(
      async () => client,
      () => 0,
      () => undefined
    );
    review.start();
    expect([review.phase, review.face, review.controls]).toEqual(['loading', null, []]);
    await review.settled();
    expect(review.face).toEqual({ view: view(1), side: 'question' });

    // a held rating keeps the rated card's answer and its controls, though the side has moved on
    review.act('show-answer');
    client.hold();
    review.act('good');
    await flush();
    expect([review.phase, review.side, review.face, review.controls]).toEqual([
      'busy',
      'question',
      { view: view(1), side: 'answer' },
      ['again', 'good', 'bury', 'flag']
    ]);
    // the rating lands and the next card's load is held: the rated card's answer stays
    client.release();
    client.hold();
    await flush();
    expect([review.phase, review.face, review.escaped]).toEqual(['loading', { view: view(1), side: 'answer' }, false]);
    client.release();
    await review.settled();
    expect(review.face).toEqual({ view: view(2), side: 'question' });

    // a held flag keeps the card on screen, and its answer brings the new flag
    client.hold();
    review.act('flag');
    await flush();
    expect([review.phase, review.face]).toEqual(['busy', { view: view(2), side: 'question' }]);
    client.release();
    await review.settled();
    expect(review.face).toEqual({ view: view(2, { flag: 1 }), side: 'question' });

    // a refused bury clears the card, and the status region speaks instead
    client.refusals.bury = new EngineError('collection-busy', 'another tab has the collection');
    review.act('bury');
    await review.settled();
    expect([review.phase, review.face, review.escaped, review.controls]).toEqual(['refused', null, false, []]);

    // a retry onto a card the frame refuses shows that card, refused, with its controls
    review.retry();
    await review.settled();
    expect([review.face, review.escaped, review.controls]).toEqual([
      { view: view(3, { css: ESCAPES }), side: 'question' },
      true,
      ['show-answer', 'bury', 'flag']
    ]);

    // the deck is done: nothing shows
    review.act('show-answer');
    review.act('good');
    await review.settled();
    expect([review.phase, review.face, review.escaped, review.controls]).toEqual(['done', null, false, []]);
  });

  // Mutation coverage: each case holds behaviour the criteria's tests do not reach.
  it('the table has these cells and no others', () => {
    const cells: [phase: Phase, event: ReviewEvent, to: Phase, effect: Effect][] = [
      ['loading', 'view', 'question', 'none'],
      ['loading', 'empty', 'done', 'none'],
      ['loading', 'refusal', 'refused', 'none'],
      ['question', 'show-answer', 'answer', 'none'],
      ['question', 'undo', 'busy', 'offer'],
      ['question', 'bury', 'busy', 'bury'],
      ['question', 'flag', 'busy', 'flag'],
      ['answer', 'again', 'busy', 'rate'],
      ['answer', 'good', 'busy', 'rate'],
      ['answer', 'undo', 'busy', 'offer'],
      ['answer', 'bury', 'busy', 'bury'],
      ['answer', 'flag', 'busy', 'flag'],
      ['busy', 'settled', 'loading', 'card'],
      ['busy', 'not-shown', 'loading', 'card'],
      ['busy', 'refusal', 'refused', 'none'],
      ['busy', 'offered', 'confirming', 'none'],
      ['busy', 'not-offered', 'question', 'none'],
      ['busy', 'undo-refused', 'loading', 'card'],
      ['confirming', 'undo', 'busy', 'undo'],
      ['confirming', 'keep', 'question', 'none'],
      ['confirming', 'show-answer', 'question', 'none'],
      ['confirming', 'again', 'question', 'none'],
      ['confirming', 'good', 'question', 'none'],
      ['confirming', 'bury', 'question', 'none'],
      ['confirming', 'flag', 'question', 'none'],
      ['confirming', 'replay', 'question', 'none'],
      ['refused', 'retry', 'loading', 'card']
    ];
    for (const [phase, event, to, effect] of cells) {
      expect(step({ phase, side: 'question' }, event), `${phase} ${event}`).toMatchObject({
        state: { phase: to },
        effect
      });
    }
    examined('cells', cells);
    // and no others: Hard and Easy have no cell on the answer side, so they fire nothing there
    for (const gone of ['hard', 'easy']) {
      const state: ReviewState = { phase: 'answer', side: 'answer' };
      expect(step(state, gone as unknown as ReviewEvent), gone).toStrictEqual({ state, effect: 'none' });
    }
    // a new card shows its question, whatever side the last one ended on
    expect(step({ phase: 'loading', side: 'answer' }, 'view').state).toStrictEqual({ phase: 'question', side: 'question' });
    // a flag's reply returns to the side the review was on
    expect(step({ phase: 'busy', side: 'answer' }, 'flagged')).toStrictEqual({
      state: { phase: 'answer', side: 'answer' },
      effect: 'none'
    });
    // while it asks, an action that is kept moves no side: a grade on the answer stays on the answer
    expect(step({ phase: 'confirming', side: 'answer' }, 'good')).toStrictEqual({
      state: { phase: 'answer', side: 'answer' },
      effect: 'none'
    });
  });

  it('a fresh review is loading its first card, and starting it says so once', async () => {
    const client = new FakeClient([head(view(1))]);
    let changes = 0;
    const review = new Review(
      async () => client,
      () => 0,
      () => changes++
    );
    expect([review.phase, review.side]).toEqual(['loading', 'question']);
    review.start();
    expect(changes).toBe(1);
    await review.settled();
  });

  it('an undo before any card fires nothing', async () => {
    const client = new FakeClient([head(view(1))]);
    let changes = 0;
    const review = new Review(
      async () => client,
      () => 0,
      () => changes++
    );
    expect(() => review.act('undo')).not.toThrow();
    expect([client.calls, changes]).toEqual([[], 0]);
  });

  it('a client that fails with no code is announced as an engine failure', async () => {
    const review = new Review(
      () => Promise.reject(new Error('the worker would not start')),
      () => 0,
      () => undefined
    );
    review.start();
    await review.settled();
    expect([review.phase, review.status]).toEqual(['refused', 'engine-failed']);
  });

  it('a card whose answer alone escapes the frame is refused only on its answer', async () => {
    const escaping = view(1, {
      answer: '<form><math><mtext></form><form><mglyph><style></math><link rel="preconnect" href="https://cards.example/">'
    });
    const review = new Review(
      async () => new FakeClient([head(escaping)]),
      () => 0,
      () => undefined
    );
    review.start();
    await review.settled();
    expect([review.side, review.escaped, review.status]).toEqual(['question', false, null]);
    review.act('show-answer');
    expect([review.side, review.escaped, review.status]).toEqual(['answer', true, 'escaped']);
  });

  it('the frame shows the faces the engine completed', async () => {
    // SPEC-350 A24, ADR-361 D12: the review asks for the shown card's faces and the frame shows
    // their text, with the core's data: URLs inline; a side with replay clips shows Replay, which
    // stays on its side and asks nothing of the engine; a done deck asks for no faces
    const IMAGE = 'data:image/gif;base64,R0lGODlhAQABAAAAACw=';
    const speech = { kind: 'speech' as const, text: 'der Hund', language: 'de-DE', rate: 0.5 };
    const faced = {
      question: { text: `<p>the dog</p><img src="${IMAGE}">`, css: '', autoplay: [speech], replay: [speech], omitted: [] },
      answer: { text: '<p>der Hund</p>', css: '', autoplay: [], replay: [], omitted: ['bark.ogg'] },
      wanted: []
    };
    class FacesClient extends FakeClient {
      faces(card: bigint) {
        this.calls.push(`faces ${card}`);
        return Promise.resolve(faced);
      }
    }
    const client = new FacesClient([head(view(1)), head(null)]);
    const review = new Review(
      async () => client,
      () => 0,
      () => undefined
    );
    review.start();
    await review.settled();
    expect(review.face?.view.question).toBe(`<p>the dog</p><img src="${IMAGE}">`);
    expect(review.controls).toEqual(['show-answer', 'replay', 'bury', 'flag']);
    review.act('replay');
    expect([review.phase, review.side]).toEqual(['question', 'question']);
    review.act('show-answer');
    expect([review.face?.side, review.face?.view.answer]).toEqual(['answer', '<p>der Hund</p>']);
    expect(review.controls).toEqual(['again', 'good', 'bury', 'flag']);
    review.act('good');
    await review.settled();
    expect(review.phase).toBe('done');
    expect(client.calls).toEqual(['card', 'faces 1', 'rate 1 3 0', 'card']);

    // replay is a cell of each side, and of no other phase
    for (const side of ['question', 'answer'] as const) {
      expect(step({ phase: side, side }, 'replay')).toStrictEqual({ state: { phase: side, side }, effect: 'replay' });
    }
    expect(step({ phase: 'busy', side: 'answer' }, 'replay')).toStrictEqual({
      state: { phase: 'busy', side: 'answer' },
      effect: 'none'
    });
  });

  // SPEC-371 R9, A23 to A26; ADR-382. The press asks for the offer, the review asks the learner,
  // and only the confirmation writes, naming the offered card and step.
  it('an undo press asks first and writes nothing until confirmed', async () => {
    const client = new FakeClient([head(view(2, { undo: 'answer' })), head(view(1))]);
    const review = new Review(
      async () => client,
      () => 0,
      () => undefined
    );
    review.start();
    await review.settled();
    review.act('undo');
    await review.settled();
    // the press read the offer and wrote nothing: the review asks
    expect([review.phase, client.calls]).toEqual(['confirming', ['card', 'undo-offer']]);
    expect(review.offer).toEqual(OFFER.offer);
    // the confirmation is the Undo action again; it writes the offered card and step, and only it
    review.act('undo');
    await review.settled();
    expect([review.phase, review.view?.id, review.offer, client.calls]).toEqual([
      'question',
      1n,
      null,
      ['card', 'undo-offer', 'undo 1 7', 'card']
    ]);
  });

  it('while it asks, any other action keeps the answer and is not carried out', async () => {
    const client = new FakeClient(Array.from({ length: 8 }, () => head(view(2, { undo: 'answer' }))));
    const review = new Review(
      async () => client,
      () => 0,
      () => undefined
    );
    review.start();
    await review.settled();
    const others = examined('other actions', ['again', 'good', 'bury', 'flag', 'replay', 'show-answer'] as const);
    for (const action of others) {
      review.act('undo');
      await review.settled();
      expect(review.phase, action).toBe('confirming');
      review.act(action);
      await review.settled();
      expect([review.phase, review.side, review.offer], action).toEqual(['question', 'question', null]);
    }
    // "Keep it" and Escape keep it too
    review.act('undo');
    await review.settled();
    review.keep();
    await review.settled();
    expect([review.phase, review.side, review.offer]).toEqual(['question', 'question', null]);
    // only reads were sent: one offer per press, and no write
    expect(client.calls).toEqual(['card', ...Array.from({ length: others.length + 1 }, () => 'undo-offer')]);
  });

  it('a synced answer is not offered and says so', async () => {
    const client = new FakeClient([head(view(2, { undo: 'synced' })), head(view(2, { undo: 'answer' }))]);
    const review = new Review(
      async () => client,
      () => 0,
      () => undefined
    );
    review.start();
    await review.settled();
    review.act('undo');
    await review.settled();
    // the view already says the answer synced: the press announces it and sends nothing
    expect([review.phase, review.status, client.calls]).toEqual(['question', 'undo-synced', ['card']]);
    // an offer the Worker declines as synced says the same, back on the side, with nothing written
    const second = new FakeClient([head(view(2, { undo: 'answer' }))]);
    second.offered = { offer: null, why: 'synced' };
    const declined = new Review(
      async () => second,
      () => 0,
      () => undefined
    );
    declined.start();
    await declined.settled();
    declined.act('undo');
    await declined.settled();
    expect([declined.phase, declined.status, declined.offer, second.calls]).toEqual([
      'question',
      'undo-synced',
      null,
      ['card', 'undo-offer']
    ]);
  });

  it('a refused confirmation reloads the card and says why', async () => {
    const client = new FakeClient([head(view(2, { undo: 'answer' })), head(view(3))]);
    const review = new Review(
      async () => client,
      () => 0,
      () => undefined
    );
    review.start();
    await review.settled();
    review.act('undo');
    await review.settled();
    client.refusals.undo = new EngineError('not-undoable', 'not-undoable: something changed after the answer');
    review.act('undo');
    await review.settled();
    // the write was refused: the next card loads, and the notice says why
    expect([review.phase, review.view?.id, review.status, client.calls]).toEqual([
      'question',
      3n,
      'not-undoable',
      ['card', 'undo-offer', 'undo 1 7', 'card']
    ]);
  });

  // MUTATION COVERAGE, SPEC-371 R9: a refused confirmation names its code, whichever of the two the
  // engine gives.
  it('a confirmation refused as synced reloads the card and says so', async () => {
    const client = new FakeClient([head(view(2, { undo: 'answer' })), head(view(3))]);
    const review = new Review(
      async () => client,
      () => 0,
      () => undefined
    );
    review.start();
    await review.settled();
    review.act('undo');
    await review.settled();
    client.refusals.undo = new EngineError('undo-synced', 'undo-synced: the answer has synced');
    review.act('undo');
    await review.settled();
    expect([review.phase, review.view?.id, review.status, client.calls]).toEqual([
      'question',
      3n,
      'undo-synced',
      ['card', 'undo-offer', 'undo 1 7', 'card']
    ]);
  });

  it('an offer the Worker declines for another reason says the answer can no longer be undone', async () => {
    const client = new FakeClient([head(view(2, { undo: 'answer' }))]);
    client.offered = { offer: null, why: 'none' };
    const review = new Review(
      async () => client,
      () => 0,
      () => undefined
    );
    review.start();
    await review.settled();
    review.act('undo');
    await review.settled();
    expect([review.phase, review.status, review.offer, client.calls]).toEqual([
      'question',
      'not-undoable',
      null,
      ['card', 'undo-offer']
    ]);
  });

  it('a synced press on the answer side announces itself and tells the screen', async () => {
    const client = new FakeClient([head(view(2, { undo: 'synced' }))]);
    let changes = 0;
    const review = new Review(
      async () => client,
      () => 0,
      () => changes++
    );
    review.start();
    await review.settled();
    review.act('show-answer');
    const before = changes;
    review.act('undo');
    expect([review.phase, review.side, review.status, changes - before, client.calls]).toEqual([
      'answer',
      'answer',
      'undo-synced',
      1,
      ['card']
    ]);
  });

  it('a synced press while a request is in flight announces nothing', async () => {
    const client = new FakeClient([head(view(2, { undo: 'synced' })), head(view(3))]);
    let changes = 0;
    const review = new Review(
      async () => client,
      () => 0,
      () => changes++
    );
    review.start();
    await review.settled();
    client.hold();
    review.act('bury');
    const before = changes;
    review.act('undo');
    expect([review.phase, review.status, changes - before]).toEqual(['busy', null, 0]);
    client.release();
    await review.settled();
    expect([review.phase, review.view?.id, review.status]).toEqual(['question', 3n, null]);
  });
});
