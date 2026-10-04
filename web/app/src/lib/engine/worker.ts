// The web engine's Worker entry (SPEC-338 R3, ADR-348): the session served on the Worker's own
// scope, over the browser's Web Locks, origin private file system and the module the build ships.
import type { LockAnswer, SessionDeps } from './session';
import { Session } from './session';

/** The bindings and module `wasm-bindgen --target web` writes for `deck-streak-web-engine`. */
export const ENGINE_BINDINGS = '';
export const ENGINE_MODULE = '';
/** Where the page's server serves them, from the origin's root. */
export const ENGINE_BASE = '';

/** The Worker's scope as the session needs it. */
export interface WorkerScope {
  postMessage(message: unknown): void;
  addEventListener(type: 'message', listener: (event: MessageEvent) => void): void;
}

export type ImportModule = (url: string) => Promise<unknown>;

export function serve(scope: WorkerScope, deps: SessionDeps): Session {
  void scope;
  return new Session(deps);
}

export async function takeLock(name: string, locks?: LockManager): Promise<LockAnswer> {
  void name;
  void locks;
  return 'held';
}

export async function probeStorage(
  storage?: Partial<Pick<StorageManager, 'getDirectory'>>,
  handle?: object
): Promise<string | null> {
  void storage;
  void handle;
  return null;
}

export function browserDeps(base: URL, importModule?: ImportModule): SessionDeps {
  void base;
  void importModule;
  return {
    lock: (name) => takeLock(name),
    storage: () => probeStorage(),
    load: () => Promise.reject(new Error('not built'))
  };
}

/** Serves the session when `scope` is a dedicated Worker's; true when it did. */
export function start(scope: object = globalThis, importModule?: ImportModule): boolean {
  void scope;
  void importModule;
  return false;
}

start();
