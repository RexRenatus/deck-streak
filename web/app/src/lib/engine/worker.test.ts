import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { IDBFactory } from 'fake-indexeddb';
import { describe, expect, it, vi } from 'vitest';
import {
  Bus,
  ORIGIN,
  ReleaseService,
  fixedCrypto,
  loginAnswering,
  readStored,
  sentinel,
  standIn
} from './credential-stand-in.test.support';
import type { EngineModule, SessionDeps } from './session';
import type { CredentialGlobals } from './worker';

/** The module under test, loaded inside each test. It starts itself when it loads, so a fault in
 * that start must fail a test: a static import that throws fails the file's load, which reports
 * no test at all, and StrykerJS reads a mutant nothing reported on as one that survived. */
const worker = () => import('./worker');

// SPEC-338 A13: the Worker's entry. The file names are derived here from the crate's manifest, as
// wasm-bindgen derives them, never read from the code under test.

/** The repository root, found by walking up to the workspace file. A fixed `../../../../../` would
 * miss it when StrykerJS runs this file from its sandbox, which sits below `web/app/.stryker-tmp`. */
function repositoryRoot(from: string): string {
  for (let dir = from; ; dir = dirname(dir)) {
    if (existsSync(join(dir, 'pnpm-workspace.yaml'))) return dir;
    if (dirname(dir) === dir) throw new Error(`no pnpm-workspace.yaml above ${from}`);
  }
}

const MANIFEST = readFileSync(
  join(repositoryRoot(import.meta.dirname), 'crates', 'web-engine', 'Cargo.toml'),
  'utf8'
);
const CRATE = /^name = "([a-z-]+)"$/m.exec(MANIFEST)?.[1] ?? '';
const STEM = CRATE.replaceAll('-', '_');

class FakeScope {
  posted: unknown[] = [];
  #listeners: ((event: MessageEvent) => void)[] = [];
  postMessage(message: unknown) {
    this.posted.push(message);
  }
  addEventListener(type: 'message', listener: (event: MessageEvent) => void) {
    expect(type).toBe('message');
    this.#listeners.push(listener);
  }
  async send(data: unknown, origin = '') {
    for (const listener of this.#listeners) listener(new MessageEvent('message', { data, origin }));
    // the session answers on a later turn: let it
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
}

const NO_BROWSER: SessionDeps = {
  lock: async () => 'held',
  storage: async () => null,
  load: () => Promise.reject(new Error('no engine here'))
};

/** A module as wasm-bindgen's web target shapes it: a default export that instantiates. */
function bindings() {
  const inits: unknown[] = [];
  const module = {
    default: async (options: unknown) => {
      inits.push(options);
    }
  } as unknown as EngineModule & { default: (options: unknown) => Promise<void> };
  return { inits, module };
}

describe('the Worker entry', () => {
  it('the worker serves the session on its own scope', async () => {
    const { ENGINE_BASE, ENGINE_BINDINGS, ENGINE_MODULE, browserDeps, serve } = await worker();
    const scope = new FakeScope();
    serve(scope, NO_BROWSER, 'https://app.example');
    await scope.send({ id: 4, op: 'next' });
    await scope.send({ id: 5, op: 'drop' });
    expect(scope.posted).toEqual([
      { id: 4, ok: false, code: 'not-open', message: 'next before open' },
      { id: 5, ok: false, code: 'bad-request', message: 'unknown operation drop' }
    ]);

    // and loads the module the build ships, from where the page serves it
    expect(STEM).toBe('deck_streak_web_engine');
    expect([ENGINE_BINDINGS, ENGINE_MODULE, ENGINE_BASE]).toEqual([
      `${STEM}.js`,
      `${STEM}_bg.wasm`,
      '/engine/'
    ]);
    const imported: string[] = [];
    const { inits, module } = bindings();
    const deps = browserDeps(new URL('https://app.example/engine/'), async (url) => {
      imported.push(url);
      return module;
    });
    expect(await deps.load()).toBe(module);
    expect(imported).toEqual([`https://app.example/engine/${STEM}.js`]);
    expect(inits).toEqual([{ module_or_path: new URL(`https://app.example/engine/${STEM}_bg.wasm`) }]);
  });

  it('the worker ignores a message from another origin', async () => {
    // SPEC-338 A20: a sender of another origin is not answered; the empty origin a dedicated
    // Worker's channel carries, and the Worker's own origin, are
    const { serve } = await worker();
    const scope = new FakeScope();
    serve(scope, NO_BROWSER, 'https://app.example');
    await scope.send({ id: 1, op: 'next' }, 'https://other.example');
    expect(scope.posted).toEqual([]);
    await scope.send({ id: 2, op: 'next' });
    await scope.send({ id: 3, op: 'next' }, 'https://app.example');
    expect(scope.posted).toEqual([
      { id: 2, ok: false, code: 'not-open', message: 'next before open' },
      { id: 3, ok: false, code: 'not-open', message: 'next before open' }
    ]);
  });

  it('starts only in a dedicated Worker, serving from the origin root', async () => {
    const { start } = await worker();
    expect(start({})).toBe(false);
    const scope = Object.assign(new FakeScope(), {
      DedicatedWorkerGlobalScope: class {},
      location: { href: 'https://app.example/assets/worker-abc.js', origin: 'https://app.example' },
      navigator: {
        locks: { request: async (_name: string, _options: object, grant: (lock: object) => unknown) => grant({}) },
        storage: { getDirectory: async () => ({}) }
      },
      FileSystemFileHandle: class {
        createSyncAccessHandle() {}
      }
    });
    const imported: string[] = [];
    const { module } = bindings();
    const engine = Object.assign(module, {
      install_storage: async () => 0,
      init: () => undefined,
      open: () => JSON.stringify({ existed: true, notes: 2 })
    });
    expect(
      start(scope, async (url) => {
        imported.push(url);
        return engine;
      })
    ).toBe(true);
    await scope.send({ id: 1, op: 'open' });
    expect(scope.posted).toEqual([{ id: 1, ok: true, value: { existed: true, notes: 2 } }]);
    expect(imported).toEqual([`https://app.example/engine/${STEM}.js`]);
  });

  it('the lock is held for the Worker life, or answers busy or unsupported', async () => {
    const { takeLock } = await worker();
    const asked: [string, object][] = [];
    let kept: unknown;
    const locks = (granted: boolean) =>
      ({
        request: (name: string, options: object, grant: (lock: object | null) => unknown) => {
          asked.push([name, options]);
          kept = grant(granted ? { name } : null);
          return Promise.resolve();
        }
      }) as unknown as LockManager;

    expect(await takeLock('deck-streak-collection', locks(true))).toBe('held');
    expect(asked).toEqual([['deck-streak-collection', { ifAvailable: true }]]);
    // the grant's promise never settles, so the lock lives as long as the Worker
    expect(kept).toBeInstanceOf(Promise);
    const settled = await Promise.race([kept, new Promise((resolve) => setTimeout(resolve, 20, 'pending'))]);
    expect(settled).toBe('pending');
    expect(await takeLock('deck-streak-collection', locks(false))).toBe('busy');
    expect(kept).toBeUndefined();
    expect(await takeLock('deck-streak-collection', undefined)).toBe('unsupported');
  });

  it('the storage probe names why OPFS is refused', async () => {
    const { probeStorage } = await worker();
    const handle = { createSyncAccessHandle() {} };
    const refused = new Error('An error occurred while getting the directory handle');
    const cases: [string, Parameters<typeof probeStorage>, string | null][] = [
      ['usable', [{ getDirectory: async () => ({}) as FileSystemDirectoryHandle }, handle], null],
      ['refused', [{ getDirectory: () => Promise.reject(refused) }, handle], `OPFS refused: ${refused.message}`],
      ['refused by a value', [{ getDirectory: () => Promise.reject('denied') }, handle], 'OPFS refused: denied'],
      ['no getDirectory', [{}, handle], 'this browser has no origin private file system'],
      ['no storage manager', [undefined, handle], 'this browser has no origin private file system'],
      ['no access handle', [{ getDirectory: async () => ({}) as FileSystemDirectoryHandle }, {}], 'this browser has no SyncAccessHandle'],
      ['no file handle', [{ getDirectory: async () => ({}) as FileSystemDirectoryHandle }, undefined], 'this browser has no SyncAccessHandle']
    ];
    expect(cases.length).toBeGreaterThan(0);
    for (const [name, args, expected] of cases) {
      expect([name, await probeStorage(...args)]).toEqual([name, expected]);
    }
  });

  it("the browser's side reads the Worker's own Web Locks and storage, and names what is missing", async () => {
    // written after green, to kill the mutants StrykerJS found that no test observed
    const { browserDeps } = await worker();
    const base = new URL('https://app.example/engine/');
    const load = () => Promise.reject(new Error('no engine here'));
    const bare = browserDeps(base, load, {});
    expect(await bare.lock('deck-streak-collection')).toBe('unsupported');
    expect(await bare.storage()).toBe('this browser has no origin private file system');
    const directory = async () => ({}) as FileSystemDirectoryHandle;
    const noHandle = browserDeps(base, load, { navigator: { storage: { getDirectory: directory } } });
    expect(await noHandle.storage()).toBe('this browser has no SyncAccessHandle');
    const asked: string[] = [];
    const locks = {
      request: (name: string, _options: object, grant: (lock: object | null) => unknown) => {
        asked.push(name);
        grant(null);
        return Promise.resolve();
      }
    } as unknown as LockManager;
    expect(await browserDeps(base, load, { navigator: { locks } }).lock('deck-streak-collection')).toBe(
      'busy'
    );
    expect(asked).toEqual(['deck-streak-collection']);
  });

  it('the default loader imports the bindings from the base it is given', async () => {
    // written after green: nothing is served at this base, so the import's own refusal names the
    // URL it was asked for, which only a real import of the bindings' URL can do
    const { browserDeps } = await worker();
    const base = new URL('file:///nowhere-deck-streak-engine/engine/');
    await expect(browserDeps(base).load()).rejects.toThrow(`nowhere-deck-streak-engine/engine/${STEM}.js`);
  });

  it('the module serves itself when a dedicated Worker loads it', async () => {
    // written after green: the module's own last line starts it, on the Worker's global scope
    const scope = new FakeScope();
    vi.stubGlobal('DedicatedWorkerGlobalScope', class {});
    vi.stubGlobal('location', { href: 'https://app.example/assets/worker-abc.js', origin: 'https://app.example' });
    vi.stubGlobal('addEventListener', scope.addEventListener.bind(scope));
    vi.stubGlobal('postMessage', scope.postMessage.bind(scope));
    try {
      vi.resetModules();
      await worker();
      await scope.send({ id: 1, op: 'next' });
      expect(scope.posted).toEqual([{ id: 1, ok: false, code: 'not-open', message: 'next before open' }]);
    } finally {
      vi.unstubAllGlobals();
      vi.resetModules();
    }
  });

  it('the media directory is read through the injected storage', async () => {
    // SPEC-350 A24, ADR-361 D12: the Worker reads media from one flat directory at the origin's
    // root, through the storage its own scope gives it, no further than each limit
    const { MEDIA_DIRECTORY, browserDeps } = await worker();
    expect(MEDIA_DIRECTORY).toBe('deck-streak-media');
    const absent = () => Promise.reject(new DOMException('absent', 'NotFoundError'));
    const media = {
      getDirectoryHandle: absent,
      getFileHandle: async (name: string) =>
        name === 'cat.mp3' ? { getFile: async () => new Blob([new Uint8Array([1, 2, 3])]) } : absent()
    };
    const root = {
      getDirectoryHandle: async (name: string) => (name === 'deck-streak-media' ? media : absent()),
      getFileHandle: absent
    };
    const storage = { getDirectory: async () => root as unknown as FileSystemDirectoryHandle };
    const load = () => Promise.reject(new Error('no engine here'));
    const deps = browserDeps(new URL('https://app.example/engine/'), load, { navigator: { storage } });
    expect(
      await deps.media?.([
        { name: 'cat.mp3', limit: 2n },
        { name: 'gone.ogg', limit: 2n }
      ])
    ).toEqual([{ name: 'cat.mp3', bytes: new Uint8Array([1, 2]) }]);
  });

  it('a scope with no navigator reads no media, and throws nothing', async () => {
    const { browserDeps } = await worker();
    const load = () => Promise.reject(new Error('no engine here'));
    const deps = browserDeps(new URL('https://app.example/engine/'), load, {});
    expect(deps.media).toBeTypeOf('function');
    expect(await deps.media?.([{ name: 'cat.mp3', limit: 2n }])).toEqual([]);
  });

  it("the credential store reads the Worker's own database, fetch, crypto and channel", async () => {
    // SPEC-363 R9 to R14: the store is built over the scope's globals, and fetch is called on the
    // scope itself, as a browser's fetch must be
    const { browserCredential } = await worker();
    const database = new IDBFactory();
    const release = new ReleaseService();
    const bus = new Bus();
    const called: unknown[] = [];
    const scope: CredentialGlobals = {
      location: { origin: ORIGIN },
      indexedDB: database,
      crypto: fixedCrypto,
      fetch(input, init) {
        called.push(this);
        return release.fetch(input, init);
      },
      BroadcastChannel: function (name: string) {
        return bus.join(name);
      } as unknown as CredentialGlobals['BroadcastChannel']
    };
    const user = sentinel('sync', 'user');
    const store = browserCredential(scope, async () => standIn());
    expect(await store.obtain(loginAnswering(sentinel('host', 'key')).login, user, sentinel('pass', 'word'))).toBe(
      'held'
    );
    expect(await readStored(database)).toEqual({ generation: 1n, sealed: expect.objectContaining({ user }) });
    expect(called).toEqual([scope]);
    expect(release.urls).toEqual([`${ORIGIN}/api/sync/seal-key`]);
    expect(bus.names).toEqual(['deck-streak-credential']);
  });

  it('the session and the credential store share one module load', async () => {
    // SPEC-363: wasm-bindgen's init must not run twice at once, so one load serves both
    const { once, start } = await worker();
    let loads = 0;
    const shared = once(async () => (loads += 1));
    expect(await Promise.all([shared(), shared()])).toEqual([1, 1]);
    let tries = 0;
    const refused = once(() => Promise.reject(new Error(`refused ${(tries += 1)}`)));
    await expect(refused()).rejects.toThrow('refused 1');
    await expect(refused()).rejects.toThrow('refused 1');

    const answered: unknown[] = [];
    let heard = () => {};
    const replies = (count: number) =>
      new Promise<void>((resolve) => {
        heard = () => answered.length >= count && resolve();
        heard();
      });
    const scope = Object.assign(new FakeScope(), {
      postMessage: (reply: unknown) => {
        answered.push(reply);
        heard();
      },
      DedicatedWorkerGlobalScope: class {},
      location: { href: `${ORIGIN}/assets/worker-abc.js`, origin: ORIGIN },
      navigator: {
        locks: { request: async (_name: string, _options: object, grant: (lock: object) => unknown) => grant({}) },
        storage: { getDirectory: async () => ({}) }
      },
      FileSystemFileHandle: class {
        createSyncAccessHandle() {}
      },
      indexedDB: new IDBFactory(),
      crypto: fixedCrypto,
      fetch: new ReleaseService().fetch,
      BroadcastChannel: function (name: string) {
        return new Bus().join(name);
      }
    });
    const imported: string[] = [];
    const { inits, module } = bindings();
    const engine = Object.assign(module, standIn(), {
      install_storage: async () => 0,
      init: () => undefined,
      open: () => JSON.stringify({ existed: false, notes: 0 })
    });
    expect(
      start(scope, async (url) => {
        imported.push(url);
        return engine;
      })
    ).toBe(true);
    await scope.send({ id: 1, op: 'credential-status' });
    await scope.send({ id: 2, op: 'open' });
    await replies(2);
    expect(answered).toEqual([
      { id: 1, ok: true, value: 'absent' },
      { id: 2, ok: true, value: { existed: false, notes: 0 } }
    ]);
    expect(imported).toEqual([`${ORIGIN}/engine/${STEM}.js`]);
    expect(inits).toHaveLength(1);
  });

  it("the sync keeps its key in the credential operations' store and sends it only to the origin's sync route", async () => {
    // SPEC-364 R17, R18: one store serves the credential operations and the sync, over one module
    const { start } = await worker();
    const answered: unknown[] = [];
    let heard = () => {};
    const replies = (count: number) =>
      new Promise<void>((resolve) => {
        heard = () => answered.length >= count && resolve();
        heard();
      });
    const scope = Object.assign(new FakeScope(), {
      postMessage: (reply: unknown) => {
        answered.push(reply);
        heard();
      },
      DedicatedWorkerGlobalScope: class {},
      location: { href: `${ORIGIN}/assets/worker-abc.js`, origin: ORIGIN },
      navigator: {
        locks: { request: async (_name: string, _options: object, grant: (lock: object) => unknown) => grant({}) },
        storage: { getDirectory: async () => ({}) }
      },
      FileSystemFileHandle: class {
        createSyncAccessHandle() {}
      },
      indexedDB: new IDBFactory(),
      crypto: fixedCrypto,
      fetch: new ReleaseService().fetch,
      BroadcastChannel: function (name: string) {
        return new Bus().join(name);
      }
    });
    const hostKey = sentinel('host', 'key', 'worker');
    const sent: string[][] = [];
    const { inits, module } = bindings();
    const engine = Object.assign(module, standIn(), {
      install_storage: async () => 0,
      init: () => undefined,
      open: () => JSON.stringify({ existed: false, notes: 0 }),
      sync_login: (endpoint: string, user: string, password: string) => {
        sent.push(['login', endpoint, user, password]);
        return hostKey;
      },
      sync_collection: (key: string, endpoint: string) => {
        sent.push(['sync', key, endpoint]);
        return 1;
      },
      handshake: () => undefined
    });
    expect(start(scope, async () => engine)).toBe(true);
    await scope.send({ id: 1, op: 'open' });
    await scope.send({ id: 2, op: 'sync-login', user: 'a user', password: 'a password' });
    await scope.send({ id: 3, op: 'credential-status' });
    await scope.send({ id: 4, op: 'sync' });
    await replies(4);
    expect(answered).toEqual([
      { id: 1, ok: true, value: { existed: false, notes: 0 } },
      { id: 2, ok: true, value: 'held' },
      { id: 3, ok: true, value: 'held' },
      { id: 4, ok: true, value: { status: 'held', required: 'normal-sync' } }
    ]);
    expect(sent).toEqual([
      ['login', `${ORIGIN}/anki-sync/`, 'a user', 'a password'],
      ['sync', hostKey, `${ORIGIN}/anki-sync/`]
    ]);
    expect(inits).toHaveLength(1);
  });

  it('the worker reads the statement through its own fetch and hands the engine its bytes before each sync', async () => {
    // SPEC-374 R22 to R24: the Worker reads the statement at its own origin and the engine decides
    const { start } = await worker();
    const answered: unknown[] = [];
    const statements: unknown[] = [];
    let heard = () => {};
    const replies = (count: number) =>
      new Promise<void>((resolve) => {
        heard = () => answered.length >= count && resolve();
        heard();
      });
    const scope = Object.assign(new FakeScope(), {
      postMessage: (reply: unknown) => {
        answered.push(reply);
        heard();
      },
      DedicatedWorkerGlobalScope: class {},
      location: { href: `${ORIGIN}/assets/worker-abc.js`, origin: ORIGIN },
      navigator: {
        locks: { request: async (_name: string, _options: object, grant: (lock: object) => unknown) => grant({}) },
        storage: { getDirectory: async () => ({}) }
      },
      FileSystemFileHandle: class {
        createSyncAccessHandle() {}
      },
      indexedDB: new IDBFactory(),
      crypto: fixedCrypto,
      fetch: async (input: string, init: RequestInit) => {
        if (input !== `${ORIGIN}/api/sync/minimum-client`) return new ReleaseService().fetch(input, init);
        statements.push([input, init.credentials, init.redirect]);
        return new Response('{"minimum_client_level":1}', { status: 200 });
      },
      BroadcastChannel: function (name: string) {
        return new Bus().join(name);
      }
    });
    const hostKey = sentinel('host', 'key', 'worker');
    const sent: string[][] = [];
    const { inits, module } = bindings();
    const engine = Object.assign(module, standIn(), {
      install_storage: async () => 0,
      init: () => undefined,
      open: () => JSON.stringify({ existed: false, notes: 0 }),
      sync_login: (endpoint: string, user: string, password: string) => {
        sent.push(['login', endpoint, user, password]);
        return hostKey;
      },
      sync_collection: (key: string, endpoint: string) => {
        sent.push(['sync', key, endpoint]);
        return 1;
      },
      handshake: (statement?: Uint8Array) => {
        sent.push(['handshake', statement === undefined ? 'nothing' : new TextDecoder().decode(statement)]);
        return undefined;
      }
    });
    expect(start(scope, async () => engine)).toBe(true);
    await scope.send({ id: 1, op: 'open' });
    await scope.send({ id: 2, op: 'sync-login', user: 'a user', password: 'a password' });
    await scope.send({ id: 3, op: 'sync' });
    await replies(3);
    expect(answered).toEqual([
      { id: 1, ok: true, value: { existed: false, notes: 0 } },
      { id: 2, ok: true, value: 'held' },
      { id: 3, ok: true, value: { status: 'held', required: 'normal-sync' } }
    ]);
    expect(sent).toEqual([
      ['handshake', '{"minimum_client_level":1}'],
      ['login', `${ORIGIN}/anki-sync/`, 'a user', 'a password'],
      ['handshake', '{"minimum_client_level":1}'],
      ['sync', hostKey, `${ORIGIN}/anki-sync/`]
    ]);
    expect(statements).toEqual([
      [`${ORIGIN}/api/sync/minimum-client`, 'omit', 'manual'],
      [`${ORIGIN}/api/sync/minimum-client`, 'omit', 'manual']
    ]);
  });

  it("the choice reads the snapshot through the worker's own fetch and sends only to the origin's sync route", async () => {
    // SPEC-377 R5, R6: the choice takes its key from the credential operations' store, reads the
    // snapshot answer at the Worker's own origin, and hands the engine the origin's sync route
    const { start } = await worker();
    const answered: unknown[] = [];
    const snapshots: unknown[] = [];
    let heard = () => {};
    const replies = (count: number) =>
      new Promise<void>((resolve) => {
        heard = () => answered.length >= count && resolve();
        heard();
      });
    const scope = Object.assign(new FakeScope(), {
      postMessage: (reply: unknown) => {
        answered.push(reply);
        heard();
      },
      DedicatedWorkerGlobalScope: class {},
      location: { href: `${ORIGIN}/assets/worker-abc.js`, origin: ORIGIN },
      navigator: {
        locks: { request: async (_name: string, _options: object, grant: (lock: object) => unknown) => grant({}) },
        storage: { getDirectory: async () => ({}) }
      },
      FileSystemFileHandle: class {
        createSyncAccessHandle() {}
      },
      indexedDB: new IDBFactory(),
      crypto: fixedCrypto,
      fetch: async (input: string, init: RequestInit) => {
        if (input !== `${ORIGIN}/api/sync/snapshot`) return new ReleaseService().fetch(input, init);
        snapshots.push([input, init.credentials, init.redirect]);
        return new Response('{"found":true,"age_seconds":30}', { status: 200 });
      },
      BroadcastChannel: function (name: string) {
        return new Bus().join(name);
      }
    });
    const hostKey = sentinel('host', 'key', 'worker');
    const sent: unknown[][] = [];
    const counts = { upload: { reviews: 1, cards: 1, notes: 1 }, download: null };
    const { module } = bindings();
    const engine = Object.assign(module, standIn(), {
      install_storage: async () => 0,
      init: () => undefined,
      open: () => JSON.stringify({ existed: false, notes: 0 }),
      sync_login: (endpoint: string, user: string, password: string) => {
        sent.push(['login', endpoint, user, password]);
        return hostKey;
      },
      full_sync_count: async (key: string, endpoint: string, required: number) => {
        sent.push(['count', key, endpoint, required]);
        return JSON.stringify(counts);
      },
      full_sync_confirm: async (direction: number, key: string, endpoint: string, found: boolean) => {
        sent.push(['confirm', direction, key, endpoint, found]);
        return JSON.stringify({ outcome: 'written' });
      },
      handshake: () => undefined
    });
    expect(start(scope, async () => engine)).toBe(true);
    await scope.send({ id: 1, op: 'open' });
    await scope.send({ id: 2, op: 'sync-login', user: 'a user', password: 'a password' });
    await scope.send({ id: 3, op: 'choice-count' });
    await scope.send({ id: 4, op: 'choice-confirm', direction: 'upload' });
    await replies(4);
    expect(answered).toEqual([
      { id: 1, ok: true, value: { existed: false, notes: 0 } },
      { id: 2, ok: true, value: 'held' },
      { id: 3, ok: true, value: { status: 'held', counts, snapshot: { found: true, age: 30 } } },
      { id: 4, ok: true, value: { status: 'held', outcome: 'written' } }
    ]);
    expect(sent).toEqual([
      ['login', `${ORIGIN}/anki-sync/`, 'a user', 'a password'],
      ['count', hostKey, `${ORIGIN}/anki-sync/`, 0],
      ['confirm', 0, hostKey, `${ORIGIN}/anki-sync/`, true]
    ]);
    expect(snapshots).toEqual([
      [`${ORIGIN}/api/sync/snapshot`, 'same-origin', 'manual'],
      [`${ORIGIN}/api/sync/snapshot`, 'same-origin', 'manual']
    ]);
  });
});
