import { IDBFactory } from 'fake-indexeddb';
import { describe, expect, it } from 'vitest';
import { CredentialStore } from './credential';
import type { CredentialDeps } from './credential';
import {
  Bus,
  ENDPOINT,
  MAX,
  NETWORK_ERROR,
  ORIGIN,
  ReleaseService,
  SYNC_AUTH_ERROR,
  engineError,
  fixedCrypto,
  heldLogin,
  loginAnswering,
  readStored,
  sentinel,
  standIn,
  studyEngine,
  workerDeps,
  writeStored
} from './credential-stand-in.test.support';
import type { Reply } from './protocol';
import { Session } from './session';

// SPEC-363 section 7, B1 to B6, B10, B12 and B13: the Worker's credential store over a fake indexed
// database, the service's release and a broadcast bus, each witness of the model replayed as an
// interleaving of two or more Workers of one origin. Every decision is the stand-in's, which answers
// as the core's rule (A2 to A6) does.

const HOST_KEY = sentinel('host', 'key', 'one');
const OTHER_KEY = sentinel('host', 'key', 'two');
const USER = sentinel('sync', 'user');
const OTHER_USER = sentinel('sync', 'other');
const PASSWORD = sentinel('pass', 'word');

/** One origin's browser: its database, the service's release and the channel bus. */
function origin() {
  return { database: new IDBFactory(), release: new ReleaseService(), bus: new Bus() };
}

/** A Worker's store over `world`, on `bus` when it is given, else on the world's own. */
function worker(world: ReturnType<typeof origin>, bus: Bus = world.bus, deps: Partial<CredentialDeps> = {}) {
  return new CredentialStore({ ...workerDeps(world.database, world.release, bus), ...deps });
}

const fromBase64url = (text: string) =>
  Uint8Array.from(atob(text.replaceAll('-', '+').replaceAll('_', '/')), (char) => char.charCodeAt(0));

interface SealedRecord {
  iv: Uint8Array<ArrayBuffer>;
  ciphertext: ArrayBuffer;
  seal: string;
  user: string;
}

/** Opens a stored record as the documented format says, independently of the store: the key the
 * service releases for its seal id, and the additional data spelled here. A record that does not
 * open answers null. */
function opens(...args: Parameters<typeof openRecord>): Promise<string | null> {
  return openRecord(...args).catch(() => null);
}

async function openRecord(
  release: ReleaseService,
  record: SealedRecord,
  endpoint: string,
  user: string,
  generation: bigint
): Promise<string> {
  const key = await crypto.subtle.importKey('raw', fromBase64url(await release.keyFor(record.seal)), 'AES-GCM', false, [
    'decrypt'
  ]);
  const additionalData = new TextEncoder().encode(
    `deck-streak sync credential v1|${endpoint}|${user}|${generation}`
  );
  const opened = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: record.iv, additionalData },
    key,
    record.ciphertext
  );
  return new TextDecoder().decode(opened);
}

/** The fixed Web Crypto, with every key the store imports recorded. */
function watchedCrypto() {
  const imported: CryptoKey[] = [];
  const subtle = new Proxy(crypto.subtle, {
    get(target, name) {
      const value = Reflect.get(target, name) as unknown;
      if (name === 'importKey') {
        return async (...args: Parameters<SubtleCrypto['importKey']>) => {
          const key = await target.importKey(...args);
          imported.push(key);
          return key;
        };
      }
      return typeof value === 'function' ? (value as (...args: unknown[]) => unknown).bind(target) : value;
    }
  });
  return { imported, crypto: { subtle, getRandomValues: fixedCrypto.getRandomValues } };
}

/** The fixed Web Crypto, whose decrypt waits at a gate the test opens; `reached` settles once a
 * decrypt has begun. */
function gatedCrypto() {
  let reach = () => {};
  const reached = new Promise<void>((begun) => (reach = begun));
  let open = () => {};
  const gate = new Promise<void>((opened) => (open = opened));
  const subtle = new Proxy(crypto.subtle, {
    get(target, name) {
      const value = Reflect.get(target, name) as unknown;
      if (name === 'decrypt') {
        return async (...args: Parameters<SubtleCrypto['decrypt']>) => {
          reach();
          await gate;
          return target.decrypt(...args);
        };
      }
      return typeof value === 'function' ? (value as (...args: unknown[]) => unknown).bind(target) : value;
    }
  });
  return { reached, open: () => open(), crypto: { subtle, getRandomValues: fixedCrypto.getRandomValues } };
}

describe("the Worker's credential store", () => {
  it('a login that lands after a forget stores nothing, and the generation stays raised', async () => {
    // B1, the witness a-login-kept-after-a-sign-out: A's login is under way when B forgets
    const world = origin();
    const login = heldLogin(HOST_KEY);
    const a = worker(world);
    const b = worker(world);
    const obtained = a.obtain(login.login, USER, PASSWORD);
    await login.started;
    await b.forget();
    login.land();
    const answer = await obtained;
    expect(await readStored(world.database)).toEqual({ generation: 1n, sealed: undefined });
    expect(answer).toBe('absent');
  });

  it('a worker whose generation is stale sends nothing and clears its memory', async () => {
    // B2, the witness a-send-from-memory-after-a-sign-out: A hears no broadcast, on its own bus
    const world = origin();
    const a = worker(world, new Bus());
    const b = worker(world);
    await a.obtain(loginAnswering(HOST_KEY).login, USER, PASSWORD);
    await b.forget();
    const asked = world.release.urls.length;
    expect(await a.forSend(ENDPOINT)).toBe('absent');
    expect(world.release.urls.slice(asked)).toEqual([]);
    // its memory is clear: once B signs in again, A opens the new record
    await b.obtain(loginAnswering(OTHER_KEY).login, USER, PASSWORD);
    expect(await a.forSend(ENDPOINT)).toEqual({ generation: 3n, key: OTHER_KEY });
  });

  it('a refusal of an older generation keeps the newer record', async () => {
    // B3, the witness a-refusal-that-drops-a-newer-key: A sends at 1, B signs in at 2, A's refusal lands
    const world = origin();
    const a = worker(world);
    const b = worker(world);
    const refused = engineError(SYNC_AUTH_ERROR);
    await a.obtain(loginAnswering(HOST_KEY).login, USER, PASSWORD);
    expect(await a.forSend(ENDPOINT)).toEqual({ generation: 1n, key: HOST_KEY });
    await b.obtain(loginAnswering(OTHER_KEY).login, USER, PASSWORD);
    await a.settle(1n, refused);
    const kept = await readStored(world.database);
    expect(kept.generation).toBe(2n);
    expect(await opens(world.release, kept.sealed as SealedRecord, ENDPOINT, USER, 2n)).toBe(OTHER_KEY);
    expect(await b.forSend(ENDPOINT)).toEqual({ generation: 2n, key: OTHER_KEY });
    // B's own refusal at 2 is the current generation's, and drops it
    const answer = await b.settle(2n, refused);
    expect(await readStored(world.database)).toEqual({ generation: 3n, sealed: undefined });
    expect(answer).toBe('absent');
  });

  it('a network failure keeps the sealed record', async () => {
    // B4, the witness a-network-failure-that-drops-the-key: a failed, an accepted and an unreadable answer
    const world = origin();
    const a = worker(world);
    await a.obtain(loginAnswering(HOST_KEY).login, USER, PASSWORD);
    const record = await readStored(world.database);
    expect(await a.forSend(ENDPOINT)).toEqual({ generation: 1n, key: HOST_KEY });
    const answers: [string, Uint8Array | undefined][] = [
      ['a network error', engineError(NETWORK_ERROR)],
      ['an accepted sync', undefined],
      ['bytes that do not decode', Uint8Array.of(0xff)]
    ];
    for (const [what, error] of answers) {
      const answer = await a.settle(1n, error);
      expect(await readStored(world.database), what).toEqual(record);
      expect(answer, what).toBe('held');
    }
    expect(record).toEqual({ generation: 1n, sealed: expect.objectContaining({ user: USER }) });
  });

  it('a refused or unreachable release keeps the record and reports needs-sign-in or offline', async () => {
    // B5, the witness an-unseal-with-no-session: each fresh Worker asks for the release and is refused
    const world = origin();
    await worker(world).obtain(loginAnswering(HOST_KEY).login, USER, PASSWORD);
    const record = await readStored(world.database);
    const answers: unknown[] = [];
    world.release.session = false;
    answers.push(await worker(world).forSend(ENDPOINT));
    expect(await readStored(world.database)).toEqual(record);
    world.release.session = true;
    world.release.reachable = false;
    answers.push(await worker(world).forSend(ENDPOINT));
    world.release.reachable = true;
    world.release.refusing = 404;
    answers.push(await worker(world).forSend(ENDPOINT));
    world.release.refusing = 429;
    answers.push(await worker(world).forSend(ENDPOINT));
    expect(await readStored(world.database)).toEqual(record);
    expect(answers).toEqual(['needs-sign-in', 'offline', 'offline', 'offline']);
    // the record opens once the session is back
    world.release.refusing = null;
    expect(await worker(world).forSend(ENDPOINT)).toEqual({ generation: 1n, key: HOST_KEY });
    // an obtain with no session, or offline, never sends the password
    const login = loginAnswering(OTHER_KEY);
    world.release.session = false;
    expect(await worker(world).obtain(login.login, USER, PASSWORD)).toBe('needs-sign-in');
    world.release.session = true;
    world.release.reachable = false;
    expect(await worker(world).obtain(login.login, USER, PASSWORD)).toBe('offline');
    expect(login.calls).toEqual([]);
    expect(await readStored(world.database)).toEqual(record);
  });

  it('the sealed record opens only for its own endpoint and user, and its generation', async () => {
    // B6: the record is opened here as the format documents it, and refuses any other binding
    const world = origin();
    const watched = watchedCrypto();
    await worker(world, world.bus, { crypto: watched.crypto }).obtain(
      loginAnswering(HOST_KEY).login,
      USER,
      PASSWORD
    );
    const { generation, sealed } = await readStored(world.database);
    const record = sealed as SealedRecord;
    expect(await opens(world.release, record, ENDPOINT, USER, generation as bigint)).toBe(HOST_KEY);
    const others: [string, string, bigint][] = [
      ['https://elsewhere.example/anki-sync/', USER, 1n],
      [ENDPOINT, OTHER_USER, 1n],
      [ENDPOINT, USER, 2n]
    ];
    for (const [endpoint, user, at] of others) {
      expect(await opens(world.release, record, endpoint, user, at), `${endpoint} ${user} ${at}`).toBeNull();
    }
    // a record whose user is rewritten does not open, and is deleted
    await writeStored(world.database, 'sealed', { ...record, user: OTHER_USER });
    const answer = await worker(world, world.bus, { crypto: watched.crypto }).forSend(ENDPOINT);
    expect(await readStored(world.database)).toEqual({ generation: 2n, sealed: undefined });
    expect(answer).toBe('absent');
    // every key the store imported stays inside Web Crypto
    expect(watched.imported.map((key) => key.extractable)).toEqual([false, false]);
  });

  it('every study op answers the same in each credential state', async () => {
    // B10: five sessions over one studying module, their credential held, absent, sealed, refused
    // its release and offline
    const held = origin();
    const heldStore = worker(held);
    await heldStore.obtain(loginAnswering(HOST_KEY).login, USER, PASSWORD);
    const sealedWith = async (release?: Partial<ReleaseService>) => {
      const world = origin();
      await worker(world).obtain(loginAnswering(HOST_KEY).login, USER, PASSWORD);
      Object.assign(world.release, release);
      return { world, store: worker(world) };
    };
    const refused = await sealedWith({ session: false });
    expect(await refused.store.forSend(ENDPOINT)).toBe('needs-sign-in');
    const offline = await sealedWith({ reachable: false });
    expect(await offline.store.forSend(ENDPOINT)).toBe('offline');
    const states: [string, CredentialStore][] = [
      ['held', heldStore],
      ['absent', worker(origin())],
      ['sealed', (await sealedWith()).store],
      ['needs-sign-in', refused.store],
      ['offline', offline.store]
    ];
    const study: object[] = [
      { op: 'open' },
      { op: 'seed', count: 2 },
      { op: 'next' },
      { op: 'rate', card: 1001n, rating: 3, ms: 1000 },
      { op: 'snapshot', card: 1001n },
      { op: 'undo-offer' },
      { op: 'undo', card: 1001n, step: 1 },
      { op: 'memory' },
      { op: 'decks' },
      { op: 'study', deck: 1n },
      { op: 'card' },
      { op: 'rate', card: 1001n, rating: 3, ms: 500 },
      { op: 'bury', card: 1001n },
      { op: 'flag', card: 1001n },
      { op: 'faces', card: 1001n },
      { op: 'close' }
    ];
    const replies = new Map<string, Reply[]>();
    for (const [state, store] of states) {
      const session = new Session({
        lock: async () => 'held',
        storage: async () => null,
        load: async () => studyEngine(),
        credential: store
      });
      const answered: Reply[] = [];
      for (const [at, body] of study.entries()) answered.push(await session.handle({ id: at + 1, ...body }));
      replies.set(state, answered);
    }
    for (const [state] of states) expect(replies.get(state), state).toEqual(replies.get('held'));
    expect(replies.get('held')?.map((reply) => reply.ok)).toEqual(study.map(() => true));
  });

  it("the key goes only to its own origin's sync route", async () => {
    // B12: a planted foreign endpoint is refused before anything is sent, by a held Worker and a fresh one
    const world = origin();
    const login = loginAnswering(HOST_KEY);
    const a = worker(world);
    await a.obtain(login.login, USER, PASSWORD);
    const asked = world.release.urls.length;
    const foreign = [
      'https://elsewhere.example/anki-sync/',
      `${ORIGIN}/anki-sync`,
      `${ORIGIN}/anki-sync/upload`,
      'http://app.example/anki-sync/'
    ];
    for (const endpoint of foreign) {
      await expect(a.forSend(endpoint), endpoint).rejects.toThrow(`the sync key goes only to ${ENDPOINT}`);
    }
    const fresh = worker(world);
    for (const endpoint of foreign) {
      await expect(fresh.forSend(endpoint), endpoint).rejects.toThrow(`the sync key goes only to ${ENDPOINT}`);
    }
    expect(world.release.urls.slice(asked)).toEqual([]);
    expect(login.calls.map(([endpoint]) => endpoint)).toEqual([ENDPOINT]);
  });

  it('a record that does not open is deleted, and the status reads absent', async () => {
    // B13: the service's secret rotated, so the record's released key no longer opens it
    const world = origin();
    await worker(world).obtain(loginAnswering(HOST_KEY).login, USER, PASSWORD);
    world.release.version = 2;
    const answer = await worker(world).forSend(ENDPOINT);
    expect(await readStored(world.database)).toEqual({ generation: 2n, sealed: undefined });
    expect(answer).toBe('absent');
  });
});

// Mutation coverage, written after green: each case pins one behaviour of the store that B1 to B14
// reach only in part.
describe("the Worker's credential store, case by case", () => {
  it("a forget heard on the channel clears another Worker's memory", async () => {
    const world = origin();
    const a = worker(world);
    const b = worker(world);
    await a.obtain(loginAnswering(HOST_KEY).login, USER, PASSWORD);
    await b.forget();
    await b.obtain(loginAnswering(OTHER_KEY).login, USER, PASSWORD);
    // A heard the forget, so it holds nothing from generation 1 and opens B's record
    expect(await a.forSend(ENDPOINT)).toEqual({ generation: 3n, key: OTHER_KEY });
  });

  it('a message from another origin changes nothing a Worker holds', async () => {
    const world = origin();
    const a = worker(world);
    await a.obtain(loginAnswering(HOST_KEY).login, USER, PASSWORD);
    const asked = world.release.urls.length;
    world.bus.deliver(null, null, 'https://elsewhere.example');
    expect(await a.forSend(ENDPOINT)).toEqual({ generation: 1n, key: HOST_KEY });
    expect(world.release.urls.slice(asked)).toEqual([]);
    // the control: the same message from the Worker's own origin clears its memory, so it asks again
    world.bus.deliver(null, null, ORIGIN);
    expect(await a.forSend(ENDPOINT)).toEqual({ generation: 1n, key: HOST_KEY });
    expect(world.release.urls.slice(asked)).toEqual([`${ORIGIN}/api/sync/seal-key`]);
  });

  it("at the generation's maximum a forget still stops a held Worker", async () => {
    const world = origin();
    expect(await worker(world).status()).toBe('absent');
    await writeStored(world.database, 'generation', MAX - 1n);
    const a = worker(world);
    expect(await a.obtain(loginAnswering(HOST_KEY).login, USER, PASSWORD)).toBe('held');
    // B forgets on a bus A does not hear, and the generation cannot rise past its maximum
    expect(await worker(world, new Bus()).forget()).toBe('absent');
    expect(await a.status()).toBe('absent');
    expect(await a.forSend(ENDPOINT)).toBe('absent');
    const b = worker(world);
    expect(await b.obtain(loginAnswering(OTHER_KEY).login, USER, PASSWORD)).toBe('absent');
    expect(await readStored(world.database)).toEqual({ generation: MAX, sealed: undefined });
  });

  it('a Worker that holds nothing answers absent with no record, and sealed with one', async () => {
    const world = origin();
    const fresh = worker(world);
    expect(await fresh.forSend(ENDPOINT)).toBe('absent');
    expect(await fresh.status()).toBe('absent');
    expect(world.release.urls).toEqual([]);
    await worker(world).obtain(loginAnswering(HOST_KEY).login, USER, PASSWORD);
    expect(await worker(world).status()).toBe('sealed');
  });

  it('a login the core discards leaves the Worker holding nothing', async () => {
    const world = origin();
    const login = heldLogin(HOST_KEY);
    const a = worker(world);
    const b = worker(world);
    const obtained = a.obtain(login.login, USER, PASSWORD);
    await login.started;
    await b.forget();
    expect(await b.obtain(loginAnswering(OTHER_KEY).login, USER, PASSWORD)).toBe('held');
    login.land();
    expect(await obtained).toBe('sealed');
    // A kept nothing in memory, so it opens B's record
    expect(await a.forSend(ENDPOINT)).toEqual({ generation: 2n, key: OTHER_KEY });
  });

  it('a record replaced while another Worker failed to open it is kept', async () => {
    const world = origin();
    await worker(world).obtain(loginAnswering(HOST_KEY).login, USER, PASSWORD);
    world.release.version = 2;
    const gated = gatedCrypto();
    const sent = worker(world, world.bus, { crypto: gated.crypto }).forSend(ENDPOINT);
    await gated.reached;
    expect(await worker(world).obtain(loginAnswering(OTHER_KEY).login, USER, PASSWORD)).toBe('held');
    gated.open();
    expect(await sent).toBe('sealed');
    const kept = await readStored(world.database);
    expect(kept.generation).toBe(2n);
    expect(await opens(world.release, kept.sealed as SealedRecord, ENDPOINT, USER, 2n)).toBe(OTHER_KEY);
  });

  it("the release is asked as JSON with the page's own session, and only an ok answer's key is a key", async () => {
    const world = origin();
    const asked: [string | undefined, RequestCredentials | undefined, string | null][] = [];
    const recorded = (input: string, init: RequestInit) => {
      asked.push([init.method, init.credentials, new Headers(init.headers).get('content-type')]);
      return world.release.fetch(input, init);
    };
    const login = loginAnswering(HOST_KEY);
    expect(await worker(world, world.bus, { fetch: recorded }).obtain(login.login, USER, PASSWORD)).toBe('held');
    expect(asked).toEqual([['POST', 'same-origin', 'application/json']]);
    const answers: [string, () => Promise<Response>][] = [
      [
        'a key with a failed status',
        async () => Response.json({ key: await world.release.keyFor('any') }, { status: 503 })
      ],
      ['no key', async () => Response.json({})],
      ['a key that is not text', async () => Response.json({ key: 7 })]
    ];
    for (const [what, answer] of answers) {
      const other = origin();
      const unsent = loginAnswering(HOST_KEY);
      expect(await worker(other, other.bus, { fetch: answer }).obtain(unsent.login, USER, PASSWORD), what).toBe(
        'offline'
      );
      expect(unsent.calls, what).toEqual([]);
    }
  });

  it('a database at a newer version makes the store reject', async () => {
    const world = origin();
    await new Promise<void>((resolve, reject) => {
      const opened = world.database.open('deck-streak-credential', 2);
      opened.onsuccess = () => {
        opened.result.close();
        resolve();
      };
      opened.onerror = () => reject(opened.error);
    });
    await expect(worker(world).status()).rejects.toMatchObject({ name: 'VersionError' });
  });

  it('a step the core cannot answer aborts its transaction, and the record stands', async () => {
    const world = origin();
    await worker(world).obtain(loginAnswering(HOST_KEY).login, USER, PASSWORD);
    const record = await readStored(world.database);
    expect(record).toEqual({ generation: 1n, sealed: expect.objectContaining({ user: USER }) });
    const engine = {
      ...standIn(),
      credential_on_removed(): bigint | undefined {
        throw new Error('the core did not answer');
      }
    };
    const store = new CredentialStore(workerDeps(world.database, world.release, world.bus, engine));
    // the rejection is the step's own error: `rejects.toThrow` with a message would also pass a null
    const failure = await store.forget().then(
      () => null,
      (error: unknown) => error
    );
    expect(failure).toBeInstanceOf(Error);
    expect((failure as Error).message).toBe('the core did not answer');
    expect(await readStored(world.database)).toEqual(record);
  });
});
