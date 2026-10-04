import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { describe, expect, it, vi } from 'vitest';
import type { EngineModule, SessionDeps } from './session';

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
});
