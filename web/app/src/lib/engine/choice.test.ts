import { describe, expect, it } from 'vitest';
import { Choice, type ChoiceEngine, type ChoiceFetch, type ChoiceStore } from './choice';
import type { StatusWord } from './protocol';

// SPEC-377 R5, R6, A7; ADR-388 D9, D10. The Worker's choice takes each send's key from the
// credential store and settles that send on every path, reads the snapshot answer from the
// service's route itself, and refuses an upload when the answer is not found or unknown, before
// the engine is asked.
const ENDPOINT = 'https://app.example/anki-sync/';
const SNAPSHOT = 'https://app.example/api/sync/snapshot';
const COUNTS = { upload: { reviews: 1, cards: 2, notes: 3 }, download: { reviews: 4, cards: 5, notes: 6 } };

/** A store that gives one key per send and records each send and each settling. */
class FakeStore implements ChoiceStore {
  calls: string[] = [];
  word: StatusWord = 'held';
  gives: 'key' | StatusWord = 'key';
  #generation = 0n;

  async forSend(endpoint: string) {
    this.calls.push(`forSend ${endpoint}`);
    if (this.gives !== 'key') return this.gives;
    this.#generation += 1n;
    return { generation: this.#generation, key: `key-${this.#generation}` };
  }

  async settle(sent: bigint, error?: Uint8Array) {
    this.calls.push(`settle ${sent} ${error === undefined ? 'ok' : `[${[...error].join(',')}]`}`);
    return this.word;
  }

  async status() {
    return this.word;
  }
}

/** An engine whose exports record their arguments and answer what the test queued. */
class FakeEngine implements ChoiceEngine {
  calls: string[] = [];
  counted: string | Uint8Array | Error = JSON.stringify(COUNTS);
  confirmed: string | Uint8Array | Error = JSON.stringify({ outcome: 'written' });

  async full_sync_count(key: string, endpoint: string, required: number) {
    this.calls.push(`count ${key} ${endpoint} ${required}`);
    if (typeof this.counted !== 'string') throw this.counted;
    return this.counted;
  }

  async full_sync_confirm(direction: number, key: string, endpoint: string, found: boolean) {
    this.calls.push(`confirm ${direction} ${key} ${endpoint} ${found}`);
    if (typeof this.confirmed !== 'string') throw this.confirmed;
    return this.confirmed;
  }

  full_sync_cancel() {
    this.calls.push('cancel');
  }

  unsynced() {
    this.calls.push('unsynced');
    return JSON.stringify({ reviews: 7, changed: true, schema: false });
  }
}

/** A fetch that answers the snapshot route as `answer` says, and records each request. */
function snapshotFetch(answer: Response | Error, seen: [string, RequestInit][]): ChoiceFetch {
  return async (input, init) => {
    seen.push([input, init]);
    if (answer instanceof Error) throw answer;
    return answer.clone();
  };
}

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
}

function made(answer: Response | Error = json({ found: true, age_seconds: 3600 })) {
  const store = new FakeStore();
  const engine = new FakeEngine();
  const seen: [string, RequestInit][] = [];
  const choice = new Choice(store, async () => engine, ENDPOINT, snapshotFetch(answer, seen));
  return { store, engine, seen, choice };
}

describe("the worker's choice", () => {
  it('every choice send takes its key from the store and settles it', async () => {
    const { store, engine, choice } = made();
    choice.heard('full-upload');
    expect(await choice.count()).toEqual({ status: 'held', counts: COUNTS, snapshot: { found: true, age: 3600 } });
    expect(await choice.confirm('download')).toEqual({ status: 'held', outcome: 'written' });
    expect(await choice.confirm('upload')).toEqual({ status: 'held', outcome: 'written' });
    // the engine got each send's own key and the Worker's endpoint, and the heard requirement
    expect(engine.calls).toEqual([
      `count key-1 ${ENDPOINT} 4`,
      `confirm 1 key-2 ${ENDPOINT} false`,
      `confirm 0 key-3 ${ENDPOINT} true`
    ]);
    expect(store.calls).toEqual([
      `forSend ${ENDPOINT}`,
      'settle 1 ok',
      `forSend ${ENDPOINT}`,
      'settle 2 ok',
      `forSend ${ENDPOINT}`,
      'settle 3 ok'
    ]);

    // an engine refusal settles its send with the engine's bytes, and answers the store's word
    const refused = made();
    refused.choice.heard('full-sync');
    refused.engine.counted = new Uint8Array([1, 2]);
    refused.engine.confirmed = new Uint8Array([3]);
    refused.store.word = 'needs-sign-in';
    expect(await refused.choice.count()).toEqual({ status: 'needs-sign-in', counts: null, snapshot: null });
    expect(await refused.choice.confirm('download')).toEqual({
      status: 'needs-sign-in',
      outcome: 'refused',
      why: 'engine'
    });
    expect(refused.engine.calls).toEqual([`count key-1 ${ENDPOINT} 2`, `confirm 1 key-2 ${ENDPOINT} false`]);
    expect(refused.store.calls).toEqual([`forSend ${ENDPOINT}`, 'settle 1 [1,2]', `forSend ${ENDPOINT}`, 'settle 2 [3]']);

    // any other throw settles with empty bytes and is thrown again
    const broken = made();
    broken.choice.heard('full-download');
    broken.engine.counted = new Error('trap');
    broken.engine.confirmed = new Error('trap again');
    await expect(broken.choice.count()).rejects.toThrow('trap');
    await expect(broken.choice.confirm('download')).rejects.toThrow('trap again');
    expect(broken.engine.calls).toEqual([`count key-1 ${ENDPOINT} 3`, `confirm 1 key-2 ${ENDPOINT} false`]);
    expect(broken.store.calls).toEqual([`forSend ${ENDPOINT}`, 'settle 1 []', `forSend ${ENDPOINT}`, 'settle 2 []']);

    // a store with no key answers its word, and nothing is sent
    const unsent = made();
    unsent.store.gives = 'absent';
    unsent.store.word = 'absent';
    expect(await unsent.choice.count()).toEqual({ status: 'absent', counts: null, snapshot: null });
    expect(await unsent.choice.confirm('download')).toEqual({ status: 'absent', outcome: 'unsent' });
    expect(await unsent.choice.confirm('upload')).toEqual({ status: 'absent', outcome: 'unsent' });
    expect(unsent.engine.calls).toEqual([]);
  });

  it('a refusal the web engine answers is passed on, and a changed count comes back with its side', async () => {
    const { store, engine, choice } = made();
    choice.heard('full-sync');
    engine.confirmed = JSON.stringify({ outcome: 'refused', why: 'not-offered' });
    expect(await choice.confirm('download')).toEqual({ status: 'held', outcome: 'refused', why: 'not-offered' });
    engine.confirmed = JSON.stringify({ outcome: 'changed', why: 'server', ...COUNTS });
    expect(await choice.confirm('upload')).toEqual({ status: 'held', outcome: 'changed', why: 'server', counts: COUNTS });
    engine.confirmed = JSON.stringify({ outcome: 'changed', why: 'device', upload: null, download: COUNTS.download });
    expect(await choice.confirm('download')).toEqual({
      status: 'held',
      outcome: 'changed',
      why: 'device',
      counts: { upload: null, download: COUNTS.download }
    });
    expect(store.calls.filter((call) => call.startsWith('settle'))).toEqual(['settle 1 ok', 'settle 2 ok', 'settle 3 ok']);
    // the cancel and the unsynced read send nothing and take no key
    await choice.cancel();
    expect(await choice.unsynced()).toEqual({ reviews: 7, changed: true, schema: false });
    expect(engine.calls.slice(-2)).toEqual(['cancel', 'unsynced']);
    expect(store.calls).toHaveLength(6);
    // with no normal sync heard, the count asks for the engine's own answer: none required
    const fresh = made();
    await fresh.choice.count();
    expect(fresh.engine.calls).toEqual([`count key-1 ${ENDPOINT} 0`]);
  });

  it('the worker reads the snapshot answer itself, and an unknown answer refuses the upload', async () => {
    // found: the age is shown, and the upload passes the answer the Worker read
    const found = made(json({ found: true, age_seconds: 90 }));
    expect((await found.choice.count()).snapshot).toEqual({ found: true, age: 90 });
    expect(found.seen.map(([url]) => url)).toEqual([SNAPSHOT]);
    const [, init] = found.seen[0];
    expect([init.method, init.credentials, init.redirect, init.cache, init.signal instanceof AbortSignal]).toEqual([
      'GET',
      'same-origin',
      'manual',
      'no-store',
      true
    ]);
    expect(await found.choice.confirm('upload')).toEqual({ status: 'held', outcome: 'written' });
    expect(found.seen).toHaveLength(2);

    // not found, refused, unreadable, redirected, malformed or unreachable: no engine call, no send
    const answers: [string, Response | Error, unknown][] = [
      ['not found', json({ found: false }), { found: false }],
      ['unknown', json({ found: null }), { found: null }],
      ['refused', json({ error: 'no session' }, 401), { found: null }],
      ['redirected', new Response(null, { status: 307, headers: { location: '/elsewhere' } }), { found: null }],
      ['malformed', new Response('not json', { status: 200 }), { found: null }],
      ['an age that is not a number', json({ found: true, age_seconds: '9' }), { found: null }],
      ['a negative age', json({ found: true, age_seconds: -1 }), { found: null }],
      ['a word that is not a boolean', json({ found: 'yes' }), { found: null }],
      ['too long', new Response(`{"found":false,"pad":"${'x'.repeat(2048)}"}`, { status: 200 }), { found: null }],
      ['unreachable', new TypeError('network'), { found: null }]
    ];
    for (const [what, answer, read] of answers) {
      const each = made(answer);
      expect((await each.choice.count()).snapshot, what).toEqual(read);
      expect(await each.choice.confirm('upload'), what).toEqual({ status: 'held', outcome: 'refused', why: 'no-snapshot' });
      expect(each.engine.calls.filter((call) => call.startsWith('confirm')), what).toEqual([]);
      expect(each.store.calls, what).toEqual([`forSend ${ENDPOINT}`, 'settle 1 ok']);
      // a download asks no snapshot and is sent
      expect(await each.choice.confirm('download'), what).toEqual({ status: 'held', outcome: 'written' });
      expect(each.seen, what).toHaveLength(2);
    }
    console.log(`examined ${answers.length} snapshot answers that are not found`);
  });

  it('a found answer is read only at a 200 that states whole seconds from zero, within the cap', async () => {
    // an age of zero is a snapshot made this second
    expect((await made(json({ found: true, age_seconds: 0 })).choice.count()).snapshot).toEqual({ found: true, age: 0 });
    // a body of exactly the cap is read; one byte over is not (the 'too long' answer above)
    const head = '{"found":false,"pad":"';
    const exact = `${head}${'x'.repeat(1024 - head.length - 2)}"}`;
    expect(new TextEncoder().encode(exact).byteLength).toBe(1024);
    expect((await made(new Response(exact, { status: 200 })).choice.count()).snapshot).toEqual({ found: false });
    // a stated answer at any status but 200, and an age with no word that says found, are unknown
    const unknown: [string, Response][] = [
      ['not found at 404', json({ found: false }, 404)],
      ['found at 203', json({ found: true, age_seconds: 5 }, 203)],
      ['an age alone', json({ age_seconds: 5 })],
      ['an age beside a null word', json({ found: null, age_seconds: 5 })]
    ];
    for (const [what, answer] of unknown) {
      expect((await made(answer).choice.count()).snapshot, what).toEqual({ found: null });
    }
    console.log(`examined ${unknown.length + 2} snapshot answers at the bounds`);
  });
});
