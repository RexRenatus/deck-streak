import { IDBFactory } from 'fake-indexeddb';
import { describe, expect, it } from 'vitest';
import { CredentialStore } from './credential';
import {
  Bus,
  ENDPOINT,
  NETWORK_ERROR,
  ReleaseService,
  SYNC_AUTH_ERROR,
  engineError,
  sentinel,
  standIn,
  workerDeps
} from './credential-stand-in.test.support';
import { Sync, readStatement, type Fetch, type SyncEngine, type SyncStore } from './sync';

// SPEC-364 B4 (R18, ADR-375 D17): each sync takes its key from the store at the send and settles
// that send whatever the engine answers, so every StartSend of `tla/SyncCredential` ends Accepted,
// Refused or Failed.

/** The engine's sync exports over the core's stand-in classifier: the login answers `hostKey`, and
 * each sync records the key and endpoint it was sent with, then answers `answer`. */
function syncEngine(hostKey: string) {
  const classifier = standIn();
  const sent: [string, string][] = [];
  const engine = {
    ...classifier,
    sent,
    /** What the next sync answers: the engine's number, or a throw. */
    answer: (): number => 1,
    sync_login: (_endpoint: string, _user: string, _password: string) => hostKey,
    handshake: (_statement?: Uint8Array): string | undefined => undefined,
    sync_collection(key: string, endpoint: string) {
      sent.push([key, endpoint]);
      return engine.answer();
    }
  };
  return engine satisfies SyncEngine;
}

/** The Worker's store, recording each step the sync asks of it in order. */
function recording(store: CredentialStore): SyncStore & { steps: string[]; forget(): Promise<unknown> } {
  const steps: string[] = [];
  return {
    steps,
    obtain: (login, user, password) => {
      steps.push('obtain');
      return store.obtain(login, user, password);
    },
    forSend: (endpoint) => {
      steps.push('forSend');
      return store.forSend(endpoint);
    },
    settle: (sent, error) => {
      steps.push(`settle ${sent} ${error === undefined ? 'accepted' : `with ${error.length} bytes`}`);
      return store.settle(sent, error);
    },
    status: () => store.status(),
    forget: () => {
      steps.push('forget');
      return store.forget();
    }
  };
}

/** A store over a fresh database, the stand-in release and `engine`, and its sync. */
function worker(engine: ReturnType<typeof syncEngine>) {
  const store = recording(new CredentialStore(workerDeps(new IDBFactory(), new ReleaseService(), new Bus(), engine)));
  const admit: Fetch = async () => new Response('{"minimum_client_level":1}', { status: 200 });
  return { store, sync: new Sync(store, async () => engine, ENDPOINT, admit) };
}

describe("the worker's sync", () => {
  it('each sync takes its key from the store and settles', async () => {
    const hostKey = sentinel('host', 'key', 'sync');
    const engine = syncEngine(hostKey);
    const { store, sync } = worker(engine);
    expect(await sync.login(sentinel('sync', 'user'), sentinel('pass', 'word'))).toBe('held');

    // two syncs, each sent with the key the store gave at that send and settled as accepted
    expect(await sync.sync()).toEqual({ status: 'held', required: 'normal-sync' });
    engine.answer = () => 4;
    expect(await sync.sync()).toEqual({ status: 'held', required: 'full-upload' });
    // a forgotten key is not sent: the store answers for the sync, and the engine is not called
    await store.forget();
    expect(await sync.sync()).toEqual({ status: 'absent', required: null });
    expect(engine.sent).toEqual([
      [hostKey, ENDPOINT],
      [hostKey, ENDPOINT]
    ]);

    // the engine's refusal settles with its bytes and drops the key; its lost network keeps it
    expect(await sync.login(sentinel('sync', 'user'), sentinel('pass', 'word'))).toBe('held');
    engine.answer = () => {
      throw engineError(NETWORK_ERROR);
    };
    expect(await sync.sync()).toEqual({ status: 'held', required: null });
    engine.answer = () => {
      throw engineError(SYNC_AUTH_ERROR);
    };
    expect(await sync.sync()).toEqual({ status: 'absent', required: null });
    const bytes = engineError(SYNC_AUTH_ERROR).length;
    expect(store.steps).toEqual([
      'obtain',
      'forSend',
      'settle 1 accepted',
      'forSend',
      'settle 1 accepted',
      'forget',
      'forSend',
      'obtain',
      'forSend',
      `settle 3 with ${bytes} bytes`,
      'forSend',
      `settle 3 with ${bytes} bytes`
    ]);
  });

  it("a send that throws anything but the engine's bytes still settles", async () => {
    const engine = syncEngine(sentinel('host', 'key', 'throw'));
    const { store, sync } = worker(engine);
    expect(await sync.login(sentinel('sync', 'user'), sentinel('pass', 'word'))).toBe('held');

    // a throw that is not the engine's bytes is thrown again, after its send settled with no bytes
    const thrown = new TypeError('a synthetic failure');
    engine.answer = () => {
      throw thrown;
    };
    await expect(sync.sync()).rejects.toBe(thrown);
    // a trap too: the session ends on it, and the send it interrupted is settled first
    const trap = new WebAssembly.RuntimeError('unreachable');
    engine.answer = () => {
      throw trap;
    };
    await expect(sync.sync()).rejects.toBe(trap);
    expect(store.steps).toEqual(['obtain', 'forSend', 'settle 1 with 0 bytes', 'forSend', 'settle 1 with 0 bytes']);
    // settled as a failure, the key is kept
    expect(await store.status()).toBe('held');
  });

  it('a login the engine refuses answers needs-sign-in, and any other failure offline', async () => {
    const engine = syncEngine(sentinel('host', 'key', 'login'));
    const { store, sync } = worker(engine);
    const user = sentinel('sync', 'user');
    const password = sentinel('pass', 'word');

    // the engine's refusal: its bytes classify as a refused key
    engine.sync_login = () => {
      throw engineError(SYNC_AUTH_ERROR);
    };
    expect(await sync.login(user, password)).toBe('needs-sign-in');
    // the engine's lost network: its bytes classify as anything but a refusal
    engine.sync_login = () => {
      throw engineError(NETWORK_ERROR);
    };
    expect(await sync.login(user, password)).toBe('offline');
    // a throw that is not the engine's bytes is offline, and the classifier is never asked of it,
    // even one that would call everything a refusal
    const asked: unknown[] = [];
    engine.credential_classify = (error) => {
      asked.push(error);
      return 1;
    };
    engine.sync_login = () => {
      throw new TypeError('a synthetic failure');
    };
    expect(await sync.login(user, password)).toBe('offline');
    expect(asked).toEqual([]);
    // no failed login leaves a key with the store
    expect(store.steps).toEqual(['obtain', 'obtain', 'obtain']);
    expect(await store.status()).toBe('absent');
  });

  it('a trap in the login is thrown again, for the session to end on', async () => {
    const engine = syncEngine(sentinel('host', 'key', 'trap'));
    const { store, sync } = worker(engine);
    const trap = new WebAssembly.RuntimeError('unreachable');
    engine.sync_login = () => {
      throw trap;
    };
    await expect(sync.login(sentinel('sync', 'user'), sentinel('pass', 'word'))).rejects.toBe(trap);
    expect(store.steps).toEqual(['obtain']);
    expect(await store.status()).toBe('absent');
  });

  it('the statement is read at the same origin with no credential, no cache and no redirect followed', async () => {
    const url = 'https://deck.test/api/sync/minimum-client';
    const calls: [string, RequestInit][] = [];
    const answering = (answer: () => Promise<Response> | Response): Fetch => async (input, init) => {
      calls.push([input, init]);
      return answer();
    };
    const body = '{"minimum_client_level":1}';
    expect(await readStatement(answering(() => new Response(body, { status: 200 })), url)).toEqual(
      new TextEncoder().encode(body)
    );
    expect(calls).toEqual([
      [url, { method: 'GET', credentials: 'omit', redirect: 'manual', cache: 'no-store', signal: expect.any(AbortSignal) }]
    ]);

    const exactly = 'a'.repeat(1024);
    expect(await readStatement(answering(() => new Response(exactly, { status: 200 })), url)).toEqual(
      new TextEncoder().encode(exactly)
    );
    expect(await readStatement(answering(() => new Response('a'.repeat(1025), { status: 200 })), url)).toEqual(
      new Uint8Array()
    );
    expect(await readStatement(answering(() => new Response('not found', { status: 404 })), url)).toEqual(
      new Uint8Array()
    );
    expect(
      await readStatement(
        answering(() => new Response(null, { status: 307, headers: { Location: 'https://deck.test/elsewhere' } })),
        url
      )
    ).toEqual(new Uint8Array());
    const opaque = { status: 0, type: 'opaqueredirect', arrayBuffer: async () => new ArrayBuffer(0) } as unknown as Response;
    expect(await readStatement(answering(() => opaque), url)).toEqual(new Uint8Array());
    const unreadable = {
      status: 200,
      arrayBuffer: async (): Promise<ArrayBuffer> => {
        throw new TypeError('the body could not be read');
      }
    } as unknown as Response;
    expect(await readStatement(answering(() => unreadable), url)).toEqual(new Uint8Array());
    const thrown: Fetch = async () => {
      throw new TypeError('a network error');
    };
    expect(await readStatement(thrown, url)).toBeUndefined();
  });

  it('a statement the core refuses stops the login and the sync before the store and says why', async () => {
    const below =
      'This version of DeckStreak is older than the oldest the sync service accepts. Update DeckStreak to sync.';
    const text = '{"minimum_client_level":9007199254740991}';
    const engine = syncEngine(sentinel('host', 'key', 'stop'));
    const handed: string[] = [];
    let refusal: string | undefined = below;
    engine.handshake = (statement?: Uint8Array) => {
      handed.push(statement === undefined ? 'nothing' : new TextDecoder().decode(statement));
      return refusal;
    };
    const logins: unknown[] = [];
    const syncs: unknown[] = [];
    engine.sync_login = (...args: [string, string, string]) => {
      logins.push(args);
      return 'never';
    };
    const sync_collection = engine.sync_collection;
    engine.sync_collection = (key: string, endpoint: string) => {
      syncs.push([key, endpoint]);
      return sync_collection(key, endpoint);
    };
    const asked: string[] = [];
    const reading: Fetch = async (input) => {
      asked.push(input);
      return new Response(text, { status: 200 });
    };
    const store = recording(new CredentialStore(workerDeps(new IDBFactory(), new ReleaseService(), new Bus(), engine)));
    const sync = new Sync(store, async () => engine, ENDPOINT, reading);

    await expect(sync.login('a user', 'a password')).rejects.toThrow(new Error(below));
    await expect(sync.sync()).rejects.toThrow(new Error(below));
    expect(store.steps).toEqual([]);
    expect(logins).toEqual([]);
    expect(syncs).toEqual([]);
    expect(handed).toEqual([text, text]);
    expect(asked).toEqual(['https://app.example/api/sync/minimum-client', 'https://app.example/api/sync/minimum-client']);

    refusal = undefined;
    engine.sync_login = () => sentinel('host', 'key', 'stop');
    expect(await sync.login('a user', 'a password')).toBe('held');
    expect(store.steps).toEqual(['obtain']);
  });
});
