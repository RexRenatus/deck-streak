/**
 * @vitest-environment jsdom
 */
import { describe, expect, it } from 'vitest';
import { EngineError } from '$lib/engine/client';
import type { CardView, Head } from '$lib/engine/protocol';
import { Review, step, type StudyClient } from './review';

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
    undo: '',
    ...extra
  };
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

  undo(): Promise<null> {
    this.calls.push('undo');
    return this.#answer('undo', () => null);
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
      head(view(2, { undo: 'Undo Answer Card' })),
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
    expect(review.controls).toEqual(['again', 'hard', 'good', 'easy', 'bury', 'flag']);

    // a rating names the shown card, its grade and the milliseconds since its question showed
    time = 3500.4;
    review.act('good');
    expect(review.phase).toBe('busy');
    await review.settled();
    expect(client.calls).toEqual(['card', 'rate 1 3 2500', 'card']);
    expect([review.phase, review.side, review.view?.id]).toEqual(['question', 'question', 2n]);

    // the engine names an undoable action: undo shows; the flag acts on the shown card and keeps its side
    expect(review.controls).toEqual(['show-answer', 'undo', 'bury', 'flag']);
    review.act('flag');
    expect(review.phase).toBe('busy');
    await review.settled();
    expect([review.phase, review.view?.flag]).toEqual(['question', 1]);
    review.act('show-answer');
    review.act('flag');
    await review.settled();
    expect([review.phase, review.side]).toEqual(['answer', 'answer']);

    // undo returns the rated card, and bury moves on from it
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
      'undo',
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
    for (const action of ['good', 'again', 'easy', 'bury', 'flag', 'show-answer'] as const) review.act(action);
    await flush();
    expect([review.phase, client.calls]).toEqual(['busy', ['card', 'rate 1 3 0']]);
    client.release();
    await review.settled();
    expect([review.phase, client.calls]).toEqual(['question', ['card', 'rate 1 3 0', 'card']]);

    // a card the engine no longer shows is refused as not-shown, and the queue's head loads again
    review.act('show-answer');
    client.refusals.rate = new EngineError('not-shown', 'not the shown card');
    review.act('hard');
    await review.settled();
    expect([review.phase, review.status, client.calls.slice(3)]).toEqual([
      'question',
      'not-shown',
      ['rate 2 2 0', 'card']
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
    for (const action of ['again', 'hard', 'good', 'easy', 'bury', 'flag', 'undo', 'show-answer'] as const) {
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
    expect(review.controls).toEqual(['again', 'hard', 'good', 'easy', 'bury', 'flag']);
    expect([review.phase, review.escaped, review.status]).toEqual(['answer', true, 'escaped']);
    review.act('easy');
    await review.settled();
    expect(client.calls).toEqual(['card', 'rate 1 4 0', 'card']);

    // the next card renders, and the status region is quiet again
    expect([review.phase, review.escaped, review.status]).toEqual(['question', false, null]);
  });
});
