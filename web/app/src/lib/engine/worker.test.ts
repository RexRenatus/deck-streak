import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import type { EngineModule, SessionDeps } from './session';
import {
  ENGINE_BASE,
  ENGINE_BINDINGS,
  ENGINE_MODULE,
  browserDeps,
  probeStorage,
  serve,
  start,
  takeLock
} from './worker';

// SPEC-338 A13: the Worker's entry. The file names are derived here from the crate's manifest, as
// wasm-bindgen derives them, never read from the code under test.
const MANIFEST = readFileSync(
  new URL('../../../../../crates/web-engine/Cargo.toml', import.meta.url),
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
});
