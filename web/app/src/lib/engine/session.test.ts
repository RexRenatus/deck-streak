import { describe, expect, it } from 'vitest';
import type { Reply } from './protocol';
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

  #call(name: keyof EngineModule, ...args: unknown[]) {
    this.calls.push([name, ...args]);
    if (name in this.failures) throw this.failures[name];
  }

  async install_storage() {
    this.#call('install_storage');
    if (this.installRefusal !== null) throw this.installRefusal;
    return 0;
  }
  init() {
    this.#call('init');
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
  answer(rating: number, ms: number) {
    this.#call('answer', rating, ms);
    const [id, card] = [...this.cards].find(([, card]) => card.reps === 0)!;
    this.journal.push([id, { ...card }]);
    this.cards.set(id, { ...card, queue: rating === 1 ? 1 : 2, type: 2, ivl: rating, reps: 1 });
    return id;
  }
  undo() {
    this.#call('undo');
    const [id, card] = this.journal.pop()!;
    this.cards.set(id, card);
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
      [{ id: 2, op: 'answer', rating: 5, ms: 0 }, refusal(2, 'bad-request', "answer's rating is malformed")],
      [{ id: 2, op: 'answer', rating: 0, ms: 0 }, refusal(2, 'bad-request', "answer's rating is malformed")],
      [{ id: 2, op: 'answer', rating: '3', ms: 0 }, refusal(2, 'bad-request', "answer's rating is malformed")],
      [{ id: 2, op: 'answer', rating: 3 }, refusal(2, 'bad-request', "answer's ms is malformed")],
      [{ id: 2, op: 'answer', rating: 3, ms: -1 }, refusal(2, 'bad-request', "answer's ms is malformed")],
      [{ id: 2, op: 'answer', rating: 3, ms: 2.5 }, refusal(2, 'bad-request', "answer's ms is malformed")],
      [{ id: 2, op: 'answer', rating: 3, ms: 2 ** 32 }, refusal(2, 'bad-request', "answer's ms is malformed")],
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
      { id: 3, op: 'answer', rating: 4, ms: 2 ** 32 - 1 },
      { id: 4, op: 'seed', count: 2 ** 32 - 1 },
      { id: 5, op: 'snapshot', card: 2n ** 63n - 1n },
      { id: 6, op: 'answer', rating: 1, ms: 0 },
      { id: 7, op: 'seed', count: 1 },
      { id: 8, op: 'snapshot', card: 1n },
      { id: Number.MAX_SAFE_INTEGER, op: 'undo' }
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
    expect(await ask({ id: 5, op: 'answer', rating: 3, ms: 1200 })).toBe(1001n);
    expect(await ask({ id: 6, op: 'snapshot', card: 1001n })).toEqual({
      id: 1001n,
      queue: 2,
      type: 2,
      due: 0,
      interval: 3,
      reps: 1,
      lapses: 0
    });
    expect(await ask({ id: 7, op: 'next' })).toBe(1002n);
    expect(await ask({ id: 8, op: 'undo' })).toBeNull();
    expect(await ask({ id: 9, op: 'snapshot', card: 1001n })).toEqual(before);
    expect(await ask({ id: 10, op: 'snapshot', card: 9n })).toBeNull();
    expect(engine.calls).toEqual([
      ['install_storage'],
      ['init'],
      ['open'],
      ['seed', 3],
      ['next_card'],
      ['snapshot', 1001n],
      ['answer', 3, 1200],
      ['snapshot', 1001n],
      ['next_card'],
      ['undo'],
      ['snapshot', 1001n],
      ['snapshot', 9n]
    ]);
    expect(log).toEqual(OPENED);
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
    expect(await session.handle({ id: 5, op: 'undo' })).toEqual(
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
    engine.failures.answer = 'engine error 0: no card is queued';
    expect(await session.handle({ id: 2, op: 'answer', rating: 2, ms: 5 })).toEqual(
      refusal(2, 'engine-failed', 'engine error 0: no card is queued')
    );
    expect(await session.handle({ id: 3, op: 'next' })).toEqual({ id: 3, ok: true, value: null });
    expect(engine.calls).not.toContainEqual(['last_panic']);

    // a trap: the module is spent, and its panic is the message from then on
    engine.panic = 'panicked at rslib/src/undo.rs: the journal is empty';
    engine.failures.undo = new WebAssembly.RuntimeError('unreachable');
    const trapped = refusal(4, 'engine-failed', 'panicked at rslib/src/undo.rs: the journal is empty');
    expect(await session.handle({ id: 4, op: 'undo' })).toEqual(trapped);
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
});
