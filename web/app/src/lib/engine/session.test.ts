import { describe, expect, it } from 'vitest';
import type { ChoiceConfirmed, ChoiceCounted, Reply, Unsynced } from './protocol';
import { Session, type EngineModule, type LockAnswer, type SessionDeps } from './session';

// SPEC-338 A8 to A11: the Worker's session over a fake engine and a fake browser. The fake engine
// keeps its cards and an undo journal, so what the session forwards is judged by what it changes.
interface Card {
  queue: number;
  type: number;
  due: number;
  ivl: number;
  reps: number;
  lapses: number;
}

class FakeEngine implements EngineModule {
  calls: unknown[][] = [];
  cards = new Map<bigint, Card>();
  journal: [bigint, Card][] = [];
  existed = false;
  installRefusal: string | null = null;
  failures: Partial<Record<keyof EngineModule, unknown>> = {};
  panic: string | undefined = undefined;
  /** SPEC-350: each language list `init` received, the decks `deck_tree` answers, and the card the
   * last card view showed, which alone a rating, bury or flag reaches, as the module keeps it. */
  languages: string[][] = [];
  decks: unknown[] = [];
  kept: bigint | null = null;
  flags = new Map<bigint, number>();

  #call(name: keyof EngineModule, ...args: unknown[]) {
    this.calls.push([name, ...args]);
    if (name in this.failures) throw this.failures[name];
  }

  async install_storage() {
    this.#call('install_storage');
    if (this.installRefusal !== null) throw this.installRefusal;
    return 0;
  }
  init(languages: string[]) {
    this.#call('init');
    this.languages.push(languages);
  }
  open() {
    this.#call('open');
    return JSON.stringify({ existed: this.existed, notes: this.cards.size });
  }
  close() {
    this.#call('close');
  }
  seed(count: number) {
    this.#call('seed', count);
    for (let at = 0; at < count; at++) {
      this.cards.set(BigInt(1001 + at), { queue: 0, type: 0, due: at, ivl: 0, reps: 0, lapses: 0 });
    }
    return count;
  }
  next_card() {
    this.#call('next_card');
    return [...this.cards].find(([, card]) => card.reps === 0)?.[0];
  }
  /** SPEC-371 R7: the undo names the answer the offer named, by its card and its step. */
  undo(card: bigint, step: number) {
    this.#call('undo', card, step);
    const [id, held] = this.journal.pop()!;
    this.cards.set(id, held);
  }
  /** An offer the module answers in place of the journal's, as JSON, when a test sets one: the
   * review's last bury or flag (SPEC-383 A27). */
  offered: object | undefined;
  /** The offer of the last answer the journal holds, its card's id a decimal string, or none. */
  undo_offer() {
    this.#call('undo_offer');
    if (this.offered !== undefined) return JSON.stringify(this.offered);
    const last = this.journal.at(-1);
    if (last === undefined) return JSON.stringify({ offer: null, why: 'none' });
    const [id] = last;
    const offer = { card: String(id), step: this.journal.length, text: `front ${id}`, grade: 'good', returns: 'new' };
    return JSON.stringify({ offer });
  }
  snapshot(id: bigint) {
    this.#call('snapshot', id);
    const card = this.cards.get(id);
    if (!card) return 'null';
    return JSON.stringify([Number(id), card.queue, card.type, card.due, card.ivl, card.reps, card.lapses]);
  }
  last_panic() {
    this.calls.push(['last_panic']);
    return this.panic;
  }
  deck_tree() {
    this.#call('deck_tree');
    return JSON.stringify(this.decks);
  }
  set_current_deck(deck: bigint) {
    this.#call('set_current_deck', deck);
  }
  current_card() {
    this.#call('current_card');
    const due = [...this.cards].filter(([, card]) => card.reps === 0 && card.queue >= 0);
    const counts = { new: due.length, learning: 0, review: 0 };
    this.kept = due[0]?.[0] ?? null;
    if (this.kept === null) return JSON.stringify({ counts, card: null });
    const id = this.kept;
    const card = {
      id: String(id),
      ordinal: 1,
      flag: this.flags.get(id) ?? 0,
      question: `front ${id}`,
      answer: `back ${id}`,
      css: '.card { color: black; }',
      labels: ['<1m', '<6m', '<10m', '4d'],
      undo: this.journal.length > 0 ? 'Answer Card' : ''
    };
    return JSON.stringify({ counts, card });
  }
  /** The module's refusal of a card other than the kept one, thrown as its string. */
  #shown(card: bigint) {
    if (card !== this.kept) throw 'not-shown: the card is not the one on screen';
  }
  rate(card: bigint, rating: number, ms: number) {
    this.#call('rate', card, rating, ms);
    this.#shown(card);
    const held = this.cards.get(card)!;
    this.journal.push([card, { ...held }]);
    this.cards.set(card, { ...held, queue: 2, type: 2, ivl: rating, reps: 1 });
    this.kept = null;
  }
  bury(card: bigint) {
    this.#call('bury', card);
    this.#shown(card);
    this.cards.set(card, { ...this.cards.get(card)!, queue: -3 });
    this.kept = null;
  }
  flag(card: bigint) {
    this.#call('flag', card);
    this.#shown(card);
    const flag = this.flags.get(card) === 1 ? 0 : 1;
    this.flags.set(card, flag);
    return flag;
  }
  /** SPEC-350 R14: the names the core wants on the next faces call, each with its limit. Each face's
   * text names the files it was given, so what the session passed is judged by the reply. */
  wants: { name: string; limit: bigint }[] = [];
  faces(card: bigint, names: string[], contents: Uint8Array[]) {
    this.#call('faces', card, names, contents);
    this.#shown(card);
    const face = (side: string) => ({ text: [side, ...names].join(' '), css: '', autoplay: [], replay: [], omitted: [] });
    return { question: face('question'), answer: face('answer'), wanted: this.wants.filter((ask) => !names.includes(ask.name)) };
  }
  /** The module's linear memory in 64 KiB pages: it grows with the collection, as a real one does. */
  memory_pages() {
    this.#call('memory_pages');
    return 17 + this.cards.size;
  }
}

/** A fake browser: `held` is the origin's Web Locks, shared by every tab's session given it. */
function browser(
  engine: FakeEngine,
  options: { held?: Set<string>; lock?: LockAnswer; storage?: string; load?: Error } = {}
) {
  const log: string[] = [];
  const deps: SessionDeps = {
    lock: async (name) => {
      log.push(`lock ${name}`);
      if (options.lock) return options.lock;
      const held = options.held ?? new Set<string>();
      if (held.has(name)) return 'busy';
      held.add(name);
      return 'held';
    },
    storage: async () => {
      log.push('storage');
      return options.storage ?? null;
    },
    load: async () => {
      log.push('load');
      if (options.load) throw options.load;
      return engine;
    }
  };
  return { log, session: new Session(deps) };
}

const refusal = (id: number | null, code: string, message: string): Reply =>
  ({ id, ok: false, code, message }) as Reply;
const OPENED = ['lock deck-streak-collection', 'storage', 'load'];

describe('the Worker session', () => {
  it('a malformed or unknown request is refused before the engine loads', async () => {
    const cases: [unknown, Reply][] = [
      [null, refusal(null, 'bad-request', 'a request is an object with an id and an op')],
      ['open', refusal(null, 'bad-request', 'a request is an object with an id and an op')],
      [[1, 'open'], refusal(null, 'bad-request', 'a request is an object with an id and an op')],
      [{ op: 'open' }, refusal(null, 'bad-request', "a request's id is a whole number from 0")],
      [{ id: -1, op: 'open' }, refusal(null, 'bad-request', "a request's id is a whole number from 0")],
      [{ id: 1.5, op: 'open' }, refusal(null, 'bad-request', "a request's id is a whole number from 0")],
      [{ id: '1', op: 'open' }, refusal(null, 'bad-request', "a request's id is a whole number from 0")],
      [{ id: 7, op: 'run_method' }, refusal(7, 'bad-request', 'unknown operation run_method')],
      [{ id: 7, op: 'toString' }, refusal(7, 'bad-request', 'unknown operation toString')],
      [{ id: 7 }, refusal(7, 'bad-request', 'unknown operation undefined')],
      [{ id: 0, op: 'open', sql: 'delete from cards' }, refusal(0, 'bad-request', 'open takes no sql')],
      [{ id: 2, op: 'next', card: 1n }, refusal(2, 'bad-request', 'next takes no card')],
      [{ id: 2, op: 'rate', card: 1001n, rating: 5, ms: 0 }, refusal(2, 'bad-request', "rate's rating is malformed")],
      [{ id: 2, op: 'rate', card: 1001n, rating: 0, ms: 0 }, refusal(2, 'bad-request', "rate's rating is malformed")],
      [{ id: 2, op: 'rate', card: 1001n, rating: '3', ms: 0 }, refusal(2, 'bad-request', "rate's rating is malformed")],
      [{ id: 2, op: 'rate', card: 1001n, rating: 3 }, refusal(2, 'bad-request', "rate's ms is malformed")],
      [{ id: 2, op: 'rate', card: 1001n, rating: 3, ms: -1 }, refusal(2, 'bad-request', "rate's ms is malformed")],
      [{ id: 2, op: 'rate', card: 1001n, rating: 3, ms: 2.5 }, refusal(2, 'bad-request', "rate's ms is malformed")],
      [{ id: 2, op: 'rate', card: 1001n, rating: 3, ms: 2 ** 32 }, refusal(2, 'bad-request', "rate's ms is malformed")],
      [{ id: 2, op: 'seed', count: 0 }, refusal(2, 'bad-request', "seed's count is malformed")],
      [{ id: 2, op: 'seed', count: 2 ** 32 }, refusal(2, 'bad-request', "seed's count is malformed")],
      [{ id: 2, op: 'seed', count: 1.5 }, refusal(2, 'bad-request', "seed's count is malformed")],
      [{ id: 2, op: 'snapshot', card: 1001 }, refusal(2, 'bad-request', "snapshot's card is malformed")],
      [{ id: 2, op: 'snapshot', card: 0n }, refusal(2, 'bad-request', "snapshot's card is malformed")],
      [{ id: 2, op: 'snapshot', card: 2n ** 63n }, refusal(2, 'bad-request', "snapshot's card is malformed")]
    ];
    expect(cases.length).toBeGreaterThan(0);
    for (const [request, expected] of cases) {
      const engine = new FakeEngine();
      const { log, session } = browser(engine);
      expect([request, await session.handle(request)]).toEqual([request, expected]);
      expect([request, log, engine.calls]).toEqual([request, [], []]);
    }
    // and while the collection is open, a malformed request reaches no engine call
    const engine = new FakeEngine();
    const { session } = browser(engine);
    await session.handle({ id: 1, op: 'open' });
    expect(await session.handle({ id: 2, op: 'undo', all: true })).toEqual(
      refusal(2, 'bad-request', 'undo takes no all')
    );
    expect(engine.calls).toEqual([['install_storage'], ['init'], ['open']]);
    // each argument's edges pass the parse and reach the session's state, here not yet open
    const fresh = browser(new FakeEngine());
    for (const request of [
      { id: 3, op: 'rate', card: 1001n, rating: 3, ms: 2 ** 32 - 1 },
      { id: 4, op: 'seed', count: 2 ** 32 - 1 },
      { id: 5, op: 'snapshot', card: 2n ** 63n - 1n },
      { id: 6, op: 'rate', card: 1001n, rating: 1, ms: 0 },
      { id: 7, op: 'seed', count: 1 },
      { id: 8, op: 'snapshot', card: 1n },
      { id: 9, op: 'undo-offer' },
      { id: 10, op: 'undo', card: 1n, step: 0 },
      { id: Number.MAX_SAFE_INTEGER, op: 'undo', card: 2n ** 63n - 1n, step: 2 ** 32 - 1 }
    ]) {
      expect(await fresh.session.handle(request)).toEqual(
        refusal(request.id, 'not-open', `${request.op} before open`)
      );
    }
    expect(fresh.log).toEqual([]);
  });

  it('a second tab is refused while the first holds the collection', async () => {
    const held = new Set<string>();
    const first = new FakeEngine();
    const second = new FakeEngine();
    const one = browser(first, { held });
    const two = browser(second, { held });

    const opened = await one.session.handle({ id: 1, op: 'open' });
    expect(await two.session.handle({ id: 1, op: 'open' })).toEqual(
      refusal(1, 'collection-busy', 'another tab holds the collection')
    );
    expect(opened).toEqual({ id: 1, ok: true, value: { existed: false, notes: 0 } });
    // the second tab took no storage, loaded no engine, and stays refused
    expect(two.log).toEqual(['lock deck-streak-collection']);
    expect(await two.session.handle({ id: 2, op: 'next' })).toEqual(
      refusal(2, 'collection-busy', 'another tab holds the collection')
    );
    expect(await two.session.handle({ id: 3, op: 'open' })).toEqual(
      refusal(3, 'collection-busy', 'another tab holds the collection')
    );
    expect(two.log).toEqual(['lock deck-streak-collection']);
    expect(second.calls).toEqual([]);
    expect(one.log).toEqual(OPENED);
  });

  it('a context that refuses OPFS is refused with storage-refused', async () => {
    const why = 'OPFS refused: An error occurred while getting the directory handle';
    const engine = new FakeEngine();
    const { log, session } = browser(engine, { storage: why });

    expect(await session.handle({ id: 1, op: 'open' })).toEqual(refusal(1, 'storage-refused', why));
    expect(log).toEqual(['lock deck-streak-collection', 'storage']);
    expect(await session.handle({ id: 2, op: 'seed', count: 1 })).toEqual(
      refusal(2, 'storage-refused', why)
    );
    expect(log).toEqual(['lock deck-streak-collection', 'storage']);
    expect(engine.calls).toEqual([]);

    // a browser with no Web Locks API is refused the same way, before the storage is touched
    const noLocks = browser(new FakeEngine(), { lock: 'unsupported' });
    expect(await noLocks.session.handle({ id: 1, op: 'open' })).toEqual(
      refusal(1, 'storage-refused', 'this browser has no Web Locks API')
    );
    expect(noLocks.log).toEqual(['lock deck-streak-collection']);

    // and a pool the module cannot install leaves the engine uninitialised and the collection shut
    const refusing = new FakeEngine();
    refusing.installRefusal = 'storage-refused: opfs pool: the directory handle was refused';
    const pool = browser(refusing);
    expect(await pool.session.handle({ id: 1, op: 'open' })).toEqual(
      refusal(1, 'storage-refused', 'storage-refused: opfs pool: the directory handle was refused')
    );
    expect(await pool.session.handle({ id: 2, op: 'open' })).toEqual(
      refusal(2, 'storage-refused', 'storage-refused: opfs pool: the directory handle was refused')
    );
    expect(refusing.calls).toEqual([['install_storage']]);
    expect(pool.log).toEqual(OPENED);
  });

  it('the session opens, answers and undoes through the engine', async () => {
    const engine = new FakeEngine();
    const { log, session } = browser(engine);
    const ask = async (request: object) => {
      const reply = await session.handle(request);
      expect(reply.ok, JSON.stringify(reply, (_, v) => (typeof v === 'bigint' ? `${v}n` : v))).toBe(true);
      return reply.ok ? reply.value : undefined;
    };

    expect(await ask({ id: 1, op: 'open' })).toEqual({ existed: false, notes: 0 });
    expect(await ask({ id: 2, op: 'seed', count: 3 })).toBe(3);
    expect(await ask({ id: 3, op: 'next' })).toBe(1001n);
    const before = await ask({ id: 4, op: 'snapshot', card: 1001n });
    expect(before).toEqual({ id: 1001n, queue: 0, type: 0, due: 0, interval: 0, reps: 0, lapses: 0 });
    // the card is shown, then rated: a grade is recorded only on the card shown (SPEC-365 R9)
    expect(((await ask({ id: 5, op: 'card' })) as { card: { id: bigint } }).card.id).toBe(1001n);
    expect(await ask({ id: 6, op: 'rate', card: 1001n, rating: 3, ms: 1200 })).toBeNull();
    expect(await ask({ id: 7, op: 'snapshot', card: 1001n })).toEqual({
      id: 1001n,
      queue: 2,
      type: 2,
      due: 0,
      interval: 3,
      reps: 1,
      lapses: 0
    });
    expect(await ask({ id: 8, op: 'next' })).toBe(1002n);
    // the undo asks for the offer of the review's own last answer, then reverts the answer it named
    // by its card and its step (SPEC-371 R12)
    expect(await ask({ id: 9, op: 'undo-offer' })).toEqual({
      offer: { card: 1001n, step: 1, text: 'front 1001', grade: 'good', returns: 'new' }
    });
    expect(await ask({ id: 10, op: 'undo', card: 1001n, step: 1 })).toBeNull();
    expect(await ask({ id: 11, op: 'snapshot', card: 1001n })).toEqual(before);
    expect(await ask({ id: 12, op: 'snapshot', card: 9n })).toBeNull();
    expect(engine.calls).toEqual([
      ['install_storage'],
      ['init'],
      ['open'],
      ['seed', 3],
      ['next_card'],
      ['snapshot', 1001n],
      ['current_card'],
      ['rate', 1001n, 3, 1200],
      ['snapshot', 1001n],
      ['next_card'],
      ['undo_offer'],
      ['undo', 1001n, 1],
      ['snapshot', 1001n],
      ['snapshot', 9n]
    ]);
    expect(log).toEqual(OPENED);
  });

  // SPEC-371 R12, A28: the module refuses an undo with its reason as a prefix, as it refuses a card
  // it did not show; a synced answer reads `undo-synced`, and every other refusal `not-undoable`.
  it('a refused undo reads as its own error code', async () => {
    const engine = new FakeEngine();
    const { session } = browser(engine);
    expect((await session.handle({ id: 1, op: 'open' })).ok).toBe(true);
    engine.failures.undo = 'undo-synced: the answer has synced';
    expect(await session.handle({ id: 2, op: 'undo', card: 1001n, step: 4 })).toEqual(
      refusal(2, 'undo-synced', 'undo-synced: the answer has synced')
    );
    engine.failures.undo = 'not-undoable: something changed after the answer';
    expect(await session.handle({ id: 3, op: 'undo', card: 1001n, step: 4 })).toEqual(
      refusal(3, 'not-undoable', 'not-undoable: something changed after the answer')
    );
    // an engine refusal leaves the session open
    expect(await session.handle({ id: 4, op: 'snapshot', card: 9n })).toEqual({ id: 4, ok: true, value: null });
  });

  // SPEC-383 R9, A27: the offer of the review's last bury carries its kind and the state the card
  // goes back to, and a flag's its kind and what the undo does to the flag, each with its card's id
  // a bigint; an answer's offer, which names no kind, is as it was.
  it('an offer of a bury or a flag reads its kind', async () => {
    const engine = new FakeEngine();
    const { session } = browser(engine);
    expect((await session.handle({ id: 1, op: 'open' })).ok).toBe(true);
    const offers: [sent: object, read: object][] = [
      [
        { offer: { kind: 'bury', card: '1001', step: 4, text: 'front 1001', returns: 'review' } },
        { offer: { kind: 'bury', card: 1001n, step: 4, text: 'front 1001', returns: 'review' } }
      ],
      [
        { offer: { kind: 'flag', card: '1002', step: 5, text: 'front 1002', flag: 'replaced' } },
        { offer: { kind: 'flag', card: 1002n, step: 5, text: 'front 1002', flag: 'replaced' } }
      ],
      [
        { offer: { card: '1003', step: 6, text: 'front 1003', grade: 'again', returns: 'learning' } },
        { offer: { card: 1003n, step: 6, text: 'front 1003', grade: 'again', returns: 'learning' } }
      ],
      [{ offer: null, why: 'synced' }, { offer: null, why: 'synced' }]
    ];
    let id = 2;
    for (const [sent, read] of offers) {
      engine.offered = sent;
      expect(await session.handle({ id, op: 'undo-offer' })).toEqual({ id, ok: true, value: read });
      id += 1;
    }
    expect(engine.calls.filter(([name]) => name === 'undo_offer')).toHaveLength(offers.length);
  });

  it('the session refuses the queue-head answer as an unknown operation', async () => {
    // SPEC-365 A14: only an owner's press records a grade, through `rate` on the card shown; the
    // queue-head `answer` is no operation, so the session refuses it before the engine sees it.
    const engine = new FakeEngine();
    const { session } = browser(engine);
    expect((await session.handle({ id: 1, op: 'open' })).ok).toBe(true);
    expect((await session.handle({ id: 2, op: 'seed', count: 1 })).ok).toBe(true);
    expect(await session.handle({ id: 3, op: 'answer', rating: 3, ms: 1200 })).toEqual(
      refusal(3, 'bad-request', 'unknown operation answer')
    );
    expect(engine.calls).toEqual([['install_storage'], ['init'], ['open'], ['seed', 1]]);
  });

  it('a request before open answers not-open, and a closed collection reopens on its engine', async () => {
    const engine = new FakeEngine();
    const { log, session } = browser(engine);

    expect(await session.handle({ id: 1, op: 'next' })).toEqual(
      refusal(1, 'not-open', 'next before open')
    );
    expect(log).toEqual([]);
    await session.handle({ id: 2, op: 'open' });
    expect(await session.handle({ id: 3, op: 'open' })).toEqual(
      refusal(3, 'bad-request', 'the collection is already open')
    );
    expect(await session.handle({ id: 4, op: 'close' })).toEqual({ id: 4, ok: true, value: null });
    expect(await session.handle({ id: 5, op: 'undo', card: 1001n, step: 1 })).toEqual(
      refusal(5, 'not-open', 'undo before open')
    );
    engine.existed = true;
    expect(await session.handle({ id: 6, op: 'open' })).toEqual({
      id: 6,
      ok: true,
      value: { existed: true, notes: 0 }
    });
    expect(await session.handle({ id: 7, op: 'next' })).toEqual({ id: 7, ok: true, value: null });
    expect(log).toEqual(OPENED);
    expect(engine.calls).toEqual([
      ['install_storage'],
      ['init'],
      ['open'],
      ['close'],
      ['open'],
      ['next_card']
    ]);
  });

  it('requests are answered one at a time, in the order they came', async () => {
    const engine = new FakeEngine();
    const { session } = browser(engine);
    const replies = await Promise.all([
      session.handle({ id: 1, op: 'open' }),
      session.handle({ id: 2, op: 'seed', count: 1 }),
      session.handle({ id: 3, op: 'next' })
    ]);
    expect(replies.map((reply) => [reply.id, reply.ok])).toEqual([
      [1, true],
      [2, true],
      [3, true]
    ]);
  });

  it('an engine error answers engine-failed, and a trap or a failed start ends the session', async () => {
    // an error the engine returns: the session stays open
    const engine = new FakeEngine();
    const { session } = browser(engine);
    await session.handle({ id: 1, op: 'open' });
    engine.failures.rate = 'engine error 0: no card is queued';
    expect(await session.handle({ id: 2, op: 'rate', card: 1001n, rating: 1, ms: 5 })).toEqual(
      refusal(2, 'engine-failed', 'engine error 0: no card is queued')
    );
    expect(await session.handle({ id: 3, op: 'next' })).toEqual({ id: 3, ok: true, value: null });
    expect(engine.calls).not.toContainEqual(['last_panic']);

    // a trap: the module is spent, and its panic is the message from then on
    engine.panic = 'panicked at rslib/src/undo.rs: the journal is empty';
    engine.failures.undo = new WebAssembly.RuntimeError('unreachable');
    const trapped = refusal(4, 'engine-failed', 'panicked at rslib/src/undo.rs: the journal is empty');
    expect(await session.handle({ id: 4, op: 'undo', card: 1001n, step: 1 })).toEqual(trapped);
    const calls = engine.calls.length;
    expect(await session.handle({ id: 5, op: 'next' })).toEqual({ ...trapped, id: 5 });
    expect(await session.handle({ id: 6, op: 'open' })).toEqual({ ...trapped, id: 6 });
    expect(engine.calls.length).toBe(calls);

    // a trap with no panic recorded says what trapped
    const bare = new FakeEngine();
    const plain = browser(bare);
    await plain.session.handle({ id: 1, op: 'open' });
    bare.failures.next_card = new WebAssembly.RuntimeError('unreachable');
    expect(await plain.session.handle({ id: 2, op: 'next' })).toEqual(
      refusal(2, 'engine-failed', 'unreachable')
    );
    expect(await plain.session.handle({ id: 3, op: 'close' })).toEqual(
      refusal(3, 'engine-failed', 'unreachable')
    );

    // a module that does not load, and one whose engine does not start
    const missing = browser(new FakeEngine(), { load: new Error('fetch failed: 404') });
    expect(await missing.session.handle({ id: 1, op: 'open' })).toEqual(
      refusal(1, 'engine-failed', 'the engine did not load: fetch failed: 404')
    );
    expect(await missing.session.handle({ id: 2, op: 'open' })).toEqual(
      refusal(2, 'engine-failed', 'the engine did not load: fetch failed: 404')
    );
    expect(missing.log).toEqual(OPENED);
    const stillborn = new FakeEngine();
    stillborn.panic = 'panicked at rslib/src/i18n.rs: no translations';
    stillborn.failures.init = new WebAssembly.RuntimeError('unreachable');
    const start = browser(stillborn);
    expect(await start.session.handle({ id: 1, op: 'open' })).toEqual(
      refusal(1, 'engine-failed', 'panicked at rslib/src/i18n.rs: no translations')
    );
    expect(await start.session.handle({ id: 2, op: 'next' })).toEqual(
      refusal(2, 'engine-failed', 'panicked at rslib/src/i18n.rs: no translations')
    );
    expect(stillborn.calls).toEqual([['install_storage'], ['init'], ['last_panic']]);
    // an open that fails without a trap leaves the collection shut and the engine usable
    const shut = new FakeEngine();
    shut.failures.open = 'engine error 4: the collection is corrupt';
    const corrupt = browser(shut);
    expect(await corrupt.session.handle({ id: 1, op: 'open' })).toEqual(
      refusal(1, 'engine-failed', 'engine error 4: the collection is corrupt')
    );
    expect(await corrupt.session.handle({ id: 2, op: 'next' })).toEqual(
      refusal(2, 'not-open', 'next before open')
    );
    delete shut.failures.open;
    expect(await corrupt.session.handle({ id: 3, op: 'open' })).toMatchObject({ id: 3, ok: true });
    expect(corrupt.log).toEqual(OPENED);
  });

  it("the session reports the module's memory in bytes", async () => {
    const engine = new FakeEngine();
    const { session } = browser(engine);
    await session.handle({ id: 1, op: 'open' });
    // the module's pages of 65536 bytes, read when asked, so a reading follows each step
    expect(await session.handle({ id: 2, op: 'memory' })).toEqual({ id: 2, ok: true, value: 17 * 65536 });
    await session.handle({ id: 3, op: 'seed', count: 3 });
    expect(await session.handle({ id: 4, op: 'memory' })).toEqual({ id: 4, ok: true, value: 20 * 65536 });
    expect(engine.calls.filter(([name]) => name === 'memory_pages')).toEqual([['memory_pages'], ['memory_pages']]);
    // memory takes no argument, and before open there is no module to read
    expect(await session.handle({ id: 5, op: 'memory', pages: 1 })).toEqual(
      refusal(5, 'bad-request', 'memory takes no pages')
    );
    const fresh = new FakeEngine();
    const before = browser(fresh);
    expect(await before.session.handle({ id: 1, op: 'memory' })).toEqual(
      refusal(1, 'not-open', 'memory before open')
    );
    expect([before.log, fresh.calls]).toEqual([[], []]);
  });

  it('each study operation reaches its engine call', async () => {
    // SPEC-350 A7: open passes the app's languages to the engine's init, each study operation
    // reaches its export with its ids as bigint, and the module's refusal of a card other than the
    // one it showed answers not-shown and leaves the session open.
    const engine = new FakeEngine();
    engine.decks = [
      {
        id: '1',
        name: 'Default',
        level: 1,
        new: 2,
        learning: 0,
        review: 0,
        children: [{ id: '9007199254740993', name: 'Verbs', level: 2, new: 1, learning: 3, review: 4, children: [] }]
      }
    ];
    const { session } = browser(engine);
    expect(await session.handle({ id: 1, op: 'open', languages: ['ja', 'en'] })).toEqual({
      id: 1,
      ok: true,
      value: { existed: false, notes: 0 }
    });
    expect(engine.languages).toEqual([['ja', 'en']]);
    await session.handle({ id: 2, op: 'seed', count: 2 });
    expect(await session.handle({ id: 3, op: 'decks' })).toEqual({
      id: 3,
      ok: true,
      value: [
        {
          id: 1n,
          name: 'Default',
          level: 1,
          new: 2,
          learning: 0,
          review: 0,
          children: [{ id: 9007199254740993n, name: 'Verbs', level: 2, new: 1, learning: 3, review: 4, children: [] }]
        }
      ]
    });
    expect(await session.handle({ id: 4, op: 'study', deck: 9007199254740993n })).toEqual({ id: 4, ok: true, value: null });
    const view = (id: bigint, flag: number, undo: string) => ({
      id,
      ordinal: 1,
      flag,
      question: `front ${id}`,
      answer: `back ${id}`,
      css: '.card { color: black; }',
      labels: ['<1m', '<6m', '<10m', '4d'],
      undo
    });
    expect(await session.handle({ id: 5, op: 'card' })).toEqual({
      id: 5,
      ok: true,
      value: { counts: { new: 2, learning: 0, review: 0 }, card: view(1001n, 0, '') }
    });
    expect(await session.handle({ id: 6, op: 'flag', card: 1001n })).toEqual({ id: 6, ok: true, value: 1 });
    expect(await session.handle({ id: 7, op: 'rate', card: 1002n, rating: 3, ms: 1500 })).toEqual(
      refusal(7, 'not-shown', 'not-shown: the card is not the one on screen')
    );
    expect(await session.handle({ id: 8, op: 'rate', card: 1001n, rating: 3, ms: 1500 })).toEqual({
      id: 8,
      ok: true,
      value: null
    });
    expect(await session.handle({ id: 9, op: 'card' })).toEqual({
      id: 9,
      ok: true,
      value: { counts: { new: 1, learning: 0, review: 0 }, card: view(1002n, 0, 'Answer Card') }
    });
    expect(await session.handle({ id: 10, op: 'bury', card: 1001n })).toEqual(
      refusal(10, 'not-shown', 'not-shown: the card is not the one on screen')
    );
    expect(await session.handle({ id: 11, op: 'bury', card: 1002n })).toEqual({ id: 11, ok: true, value: null });
    expect(await session.handle({ id: 12, op: 'card' })).toEqual({
      id: 12,
      ok: true,
      value: { counts: { new: 0, learning: 0, review: 0 }, card: null }
    });
    expect(engine.calls).toEqual([
      ['install_storage'],
      ['init'],
      ['open'],
      ['seed', 2],
      ['deck_tree'],
      ['set_current_deck', 9007199254740993n],
      ['current_card'],
      ['flag', 1001n],
      ['rate', 1002n, 3, 1500],
      ['rate', 1001n, 3, 1500],
      ['current_card'],
      ['bury', 1001n],
      ['bury', 1002n],
      ['current_card']
    ]);
    // with no languages, init receives none, and the engine's own rule speaks English
    const plain = new FakeEngine();
    await browser(plain).session.handle({ id: 1, op: 'open' });
    expect(plain.languages).toEqual([[]]);
  });

  it('faces asks the engine twice, the second time with the files it named', async () => {
    // SPEC-350 A24, ADR-361 D12: the first ask carries no file and names the files the core wants;
    // the Worker reads them and asks again, and the second answer is the reply. A face that wants
    // nothing is asked once, a card the engine did not show is refused before any read, and a
    // Worker with no media directory asks again with none.
    const engine = new FakeEngine();
    const read: unknown[] = [];
    const session = new Session({
      lock: async () => 'held',
      storage: async () => null,
      load: async () => engine,
      media: async (wanted) => {
        read.push(wanted);
        return [{ name: 'cat.mp3', bytes: new Uint8Array([1, 2]) }];
      }
    });
    await session.handle({ id: 1, op: 'open' });
    await session.handle({ id: 2, op: 'seed', count: 1 });
    await session.handle({ id: 3, op: 'card' });
    engine.wants = [
      { name: 'cat.mp3', limit: 9n },
      { name: 'gone.png', limit: 9n }
    ];
    const face = (text: string) => ({ text, css: '', autoplay: [], replay: [], omitted: [] });
    expect(await session.handle({ id: 4, op: 'faces', card: 1001n })).toEqual({
      id: 4,
      ok: true,
      value: {
        question: face('question cat.mp3'),
        answer: face('answer cat.mp3'),
        wanted: [{ name: 'gone.png', limit: 9n }]
      }
    });
    expect(read).toEqual([
      [
        { name: 'cat.mp3', limit: 9n },
        { name: 'gone.png', limit: 9n }
      ]
    ]);
    expect(engine.calls.filter(([name]) => name === 'faces')).toEqual([
      ['faces', 1001n, [], []],
      ['faces', 1001n, ['cat.mp3'], [new Uint8Array([1, 2])]]
    ]);

    // a face that wants nothing is asked once, and nothing is read
    engine.wants = [];
    engine.calls = [];
    expect(await session.handle({ id: 5, op: 'faces', card: 1001n })).toEqual({
      id: 5,
      ok: true,
      value: { question: face('question'), answer: face('answer'), wanted: [] }
    });
    expect(engine.calls).toEqual([['faces', 1001n, [], []]]);

    // a card the engine did not show is refused, and nothing is read
    engine.wants = [{ name: 'cat.mp3', limit: 9n }];
    expect(await session.handle({ id: 6, op: 'faces', card: 1002n })).toEqual(
      refusal(6, 'not-shown', 'not-shown: the card is not the one on screen')
    );
    expect(read).toHaveLength(1);

    // a Worker with no media directory asks again with no file
    const bare = new FakeEngine();
    const { session: plain } = browser(bare);
    await plain.handle({ id: 1, op: 'open' });
    await plain.handle({ id: 2, op: 'seed', count: 1 });
    await plain.handle({ id: 3, op: 'card' });
    bare.wants = [{ name: 'cat.mp3', limit: 9n }];
    expect(await plain.handle({ id: 4, op: 'faces', card: 1001n })).toEqual({
      id: 4,
      ok: true,
      value: { question: face('question'), answer: face('answer'), wanted: [{ name: 'cat.mp3', limit: 9n }] }
    });
    expect(bare.calls.filter(([name]) => name === 'faces')).toEqual([
      ['faces', 1001n, [], []],
      ['faces', 1001n, [], []]
    ]);
  });
});

// Mutation coverage for SPEC-363 R15, written after green: the two credential operations reach the
// Worker's credential store and nothing else, before the collection opens too.
describe("the Worker session's credential operations", () => {
  const sessionWith = (credential: SessionDeps['credential']) =>
    new Session({
      lock: async () => 'held',
      storage: async () => null,
      load: async () => new FakeEngine(),
      credential
    });

  it('a Worker with no credential store answers absent', async () => {
    const { log, session } = browser(new FakeEngine());
    expect(await session.handle({ id: 1, op: 'credential-status' })).toEqual({ id: 1, ok: true, value: 'absent' });
    expect(log).toEqual([]);
  });

  it("each credential operation answers its own store's word, before open too", async () => {
    const session = sessionWith({ status: async () => 'held', forget: async () => 'absent' });
    expect(await session.handle({ id: 1, op: 'credential-forget' })).toEqual({ id: 1, ok: true, value: 'absent' });
    expect(await session.handle({ id: 2, op: 'credential-status' })).toEqual({ id: 2, ok: true, value: 'held' });
    // the collection still opens after them
    expect(await session.handle({ id: 3, op: 'open' })).toMatchObject({ id: 3, ok: true });
    expect(await session.handle({ id: 4, op: 'credential-status' })).toEqual({ id: 4, ok: true, value: 'held' });
  });

  it('a store that fails answers storage-refused with its reason', async () => {
    const session = sessionWith({
      status: () => Promise.reject(new Error('the database is gone')),
      forget: async () => 'absent'
    });
    expect(await session.handle({ id: 1, op: 'credential-status' })).toEqual(
      refusal(1, 'storage-refused', 'the database is gone')
    );
  });
});

describe("the Worker session's sync operations", () => {
  // SPEC-364 R17, R18 (ADR-375 D18): both sync operations need an open session, run on the
  // session's queue, and answer the Worker's sync's own value; a trap ends the session
  const opened = async (sync: SessionDeps['sync'], engine = new FakeEngine()) => {
    const session = new Session({
      lock: async () => 'held',
      storage: async () => null,
      load: async () => engine,
      sync
    });
    expect(await session.handle({ id: 0, op: 'open' })).toMatchObject({ id: 0, ok: true });
    return session;
  };

  it('both sync operations need an open session', async () => {
    const asked: string[] = [];
    const session = new Session({
      lock: async () => 'held',
      storage: async () => null,
      load: async () => new FakeEngine(),
      sync: {
        login: async () => {
          asked.push('login');
          return 'held';
        },
        sync: async () => {
          asked.push('sync');
          return { status: 'held', required: 'no-changes' };
        }
      }
    });
    expect(await session.handle({ id: 1, op: 'sync-login', user: 'u', password: 'p' })).toEqual(
      refusal(1, 'not-open', 'sync-login before open')
    );
    expect(await session.handle({ id: 2, op: 'sync' })).toEqual(refusal(2, 'not-open', 'sync before open'));
    expect(asked).toEqual([]);
  });

  it('a Worker with no sync answers as a store with no key', async () => {
    const session = await opened(undefined);
    expect(await session.handle({ id: 1, op: 'sync-login', user: 'u', password: 'p' })).toEqual({
      id: 1,
      ok: true,
      value: 'absent'
    });
    expect(await session.handle({ id: 2, op: 'sync' })).toEqual({
      id: 2,
      ok: true,
      value: { status: 'absent', required: null }
    });
  });

  it("each sync operation answers its sync's own value, the login with the request's user and password", async () => {
    const logins: [string, string][] = [];
    const session = await opened({
      login: async (user, password) => {
        logins.push([user, password]);
        return 'needs-sign-in';
      },
      sync: async () => ({ status: 'sealed', required: 'full-download' })
    });
    expect(await session.handle({ id: 1, op: 'sync-login', user: 'a user', password: 'a password' })).toEqual({
      id: 1,
      ok: true,
      value: 'needs-sign-in'
    });
    expect(logins).toEqual([['a user', 'a password']]);
    expect(await session.handle({ id: 2, op: 'sync' })).toEqual({
      id: 2,
      ok: true,
      value: { status: 'sealed', required: 'full-download' }
    });
  });

  it('a study request waits for a sync on the queue, and is then answered', async () => {
    const order: string[] = [];
    let finish: () => void = () => undefined;
    const session = await opened({
      login: async () => 'held',
      sync: () =>
        new Promise((resolve) => {
          finish = () => {
            order.push('sync settled');
            resolve({ status: 'held', required: 'normal-sync' });
          };
        })
    });
    const synced = session.handle({ id: 1, op: 'sync' }).then((reply) => {
      order.push('sync answered');
      return reply;
    });
    const next = session.handle({ id: 2, op: 'next' }).then((reply) => {
      order.push('next answered');
      return reply;
    });
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(order).toEqual([]);
    finish();
    expect(await synced).toEqual({ id: 1, ok: true, value: { status: 'held', required: 'normal-sync' } });
    expect(await next).toEqual({ id: 2, ok: true, value: null });
    expect(order).toEqual(['sync settled', 'sync answered', 'next answered']);
  });

  it('a trap in a sync ends the session, and any other throw answers engine-failed', async () => {
    const engine = new FakeEngine();
    let thrown: unknown = new TypeError('a synthetic failure');
    const session = await opened(
      {
        login: async () => {
          throw thrown;
        },
        sync: async () => {
          throw thrown;
        }
      },
      engine
    );
    // a throw that is not a trap leaves the session open
    expect(await session.handle({ id: 1, op: 'sync' })).toEqual(refusal(1, 'engine-failed', 'a synthetic failure'));
    expect(await session.handle({ id: 2, op: 'next' })).toEqual({ id: 2, ok: true, value: null });
    // a trap spends the module: its panic is the message from then on
    engine.panic = 'panicked at rslib/src/sync/mod.rs: the sync trapped';
    thrown = new WebAssembly.RuntimeError('unreachable');
    const trapped = refusal(3, 'engine-failed', 'panicked at rslib/src/sync/mod.rs: the sync trapped');
    expect(await session.handle({ id: 3, op: 'sync-login', user: 'u', password: 'p' })).toEqual(trapped);
    expect(await session.handle({ id: 4, op: 'next' })).toEqual({ ...trapped, id: 4 });
    expect(await session.handle({ id: 5, op: 'sync' })).toEqual({ ...trapped, id: 5 });
  });
});

describe("the Worker session's choice operations", () => {
  // SPEC-377 R6: the four choice operations need an open session and answer the Worker's choice's
  // own value; a Worker with no choice refuses each by name, a trap ends the session, and any other
  // throw answers engine-failed
  const OPS = ['choice-count', 'choice-confirm', 'choice-cancel', 'unsynced'] as const;
  const COUNTED: ChoiceCounted = {
    status: 'held',
    counts: { upload: { reviews: 1, cards: 2, notes: 3 }, download: null },
    snapshot: { found: true, age: 60 }
  };
  const CONFIRMED: ChoiceConfirmed = { status: 'held', outcome: 'written' };
  const UNSYNCED: Unsynced = { reviews: 4, changed: true, schema: false };
  const ask = (id: number, op: (typeof OPS)[number]) =>
    op === 'choice-confirm' ? { id, op, direction: 'upload' } : { id, op };
  const recording = (calls: string[], thrown?: () => unknown): NonNullable<SessionDeps['choice']> => {
    const answer = async <T>(call: string, value: T): Promise<T> => {
      calls.push(call);
      if (thrown !== undefined) throw thrown();
      return value;
    };
    return {
      heard: (required) => {
        calls.push(`heard ${required}`);
      },
      count: () => answer('count', COUNTED),
      confirm: (direction) => answer(`confirm ${direction}`, CONFIRMED),
      cancel: () => answer('cancel', undefined),
      unsynced: () => answer('unsynced', UNSYNCED)
    };
  };
  const opened = async (choice: SessionDeps['choice'], engine = new FakeEngine()) => {
    const session = new Session({
      lock: async () => 'held',
      storage: async () => null,
      load: async () => engine,
      choice
    });
    expect(await session.handle({ id: 0, op: 'open' })).toMatchObject({ id: 0, ok: true });
    return session;
  };

  it('each choice operation needs an open session, and a Worker with no choice refuses each by name', async () => {
    const asked: string[] = [];
    const closed = new Session({
      lock: async () => 'held',
      storage: async () => null,
      load: async () => new FakeEngine(),
      choice: recording(asked)
    });
    for (const [index, op] of OPS.entries()) {
      expect(await closed.handle(ask(index + 1, op)), op).toEqual(refusal(index + 1, 'not-open', `${op} before open`));
    }
    expect(asked).toEqual([]);
    const none = await opened(undefined);
    for (const [index, op] of OPS.entries()) {
      expect(await none.handle(ask(index + 1, op)), op).toEqual(
        refusal(index + 1, 'engine-failed', `this Worker has no full-sync choice for ${op}`)
      );
    }
    // the refusal leaves the session open
    expect(await none.handle({ id: 5, op: 'next' })).toEqual({ id: 5, ok: true, value: null });
    console.log(`examined ${OPS.length} choice operations`);
  });

  it("each choice operation answers its choice's own value, the confirm with the request's direction", async () => {
    const calls: string[] = [];
    const session = await opened(recording(calls));
    expect(await session.handle({ id: 1, op: 'choice-count' })).toEqual({ id: 1, ok: true, value: COUNTED });
    expect(await session.handle({ id: 2, op: 'choice-confirm', direction: 'upload' })).toEqual({
      id: 2,
      ok: true,
      value: CONFIRMED
    });
    expect(await session.handle({ id: 3, op: 'choice-confirm', direction: 'download' })).toEqual({
      id: 3,
      ok: true,
      value: CONFIRMED
    });
    expect(await session.handle({ id: 4, op: 'choice-cancel' })).toEqual({ id: 4, ok: true, value: null });
    expect(await session.handle({ id: 5, op: 'unsynced' })).toEqual({ id: 5, ok: true, value: UNSYNCED });
    expect(calls).toEqual(['count', 'confirm upload', 'confirm download', 'cancel', 'unsynced']);
  });

  it('a trap in a choice ends the session, and any other throw answers engine-failed', async () => {
    const engine = new FakeEngine();
    let thrown: unknown = new TypeError('a synthetic failure');
    const calls: string[] = [];
    const session = await opened(
      recording(calls, () => thrown),
      engine
    );
    // a throw that is not a trap leaves the session open
    for (const [index, op] of OPS.entries()) {
      expect(await session.handle(ask(index + 1, op)), op).toEqual(refusal(index + 1, 'engine-failed', 'a synthetic failure'));
    }
    expect(calls).toEqual(['count', 'confirm upload', 'cancel', 'unsynced']);
    expect(await session.handle({ id: 5, op: 'next' })).toEqual({ id: 5, ok: true, value: null });
    // a trap spends the module: its panic is the message from then on
    engine.panic = 'panicked at rslib/src/sync/collection/upload.rs: the choice trapped';
    thrown = new WebAssembly.RuntimeError('unreachable');
    const trapped = refusal(6, 'engine-failed', 'panicked at rslib/src/sync/collection/upload.rs: the choice trapped');
    expect(await session.handle({ id: 6, op: 'choice-confirm', direction: 'download' })).toEqual(trapped);
    expect(await session.handle({ id: 7, op: 'next' })).toEqual({ ...trapped, id: 7 });
    expect(await session.handle({ id: 8, op: 'choice-count' })).toEqual({ ...trapped, id: 8 });
  });
});
