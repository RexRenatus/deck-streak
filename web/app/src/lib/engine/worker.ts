// The web engine's Worker entry (SPEC-338 R3, ADR-348): the session served on the Worker's own
// scope, over the browser's Web Locks, origin private file system and the module the build ships.
import { readMedia } from './media';
import { admitsOrigin } from './protocol';
import type { EngineModule, LockAnswer, SessionDeps } from './session';
import { Session } from './session';

/** The bindings and module `wasm-bindgen --target web` writes for `deck-streak-web-engine`. */
export const ENGINE_BINDINGS = 'deck_streak_web_engine.js';
export const ENGINE_MODULE = 'deck_streak_web_engine_bg.wasm';
/** Where the page's server serves them, from the origin's root. */
export const ENGINE_BASE = '/engine/';
/** The origin private file system's directory of the collection's media: flat, at the root, each
 * file under the name the engine stores (SPEC-350 R14, ADR-361 D12). */
export const MEDIA_DIRECTORY = 'deck-streak-media';

/** The Worker's scope as the session needs it. */
export interface WorkerScope {
  postMessage(message: unknown): void;
  addEventListener(type: 'message', listener: (event: MessageEvent) => void): void;
}

/** The parts of a Worker's global scope the browser's side of the session reads. */
interface WorkerGlobals {
  navigator?: { locks?: LockManager; storage?: Partial<Pick<StorageManager, 'getDirectory'>> };
  FileSystemFileHandle?: { prototype: object };
}

export type ImportModule = (url: string) => Promise<unknown>;

/** The default loader: a module import of the bindings' URL, decided when it is called. */
function importModule(url: string): Promise<unknown> {
  return import(/* @vite-ignore */ url);
}

/** Answers each message on `scope` with the session's reply, on the same scope. A dedicated
 * Worker hears only the page that started it; a message that names another origin than `origin`,
 * the Worker's own, is still not heard, and gets no reply (SPEC-338 R13). */
export function serve(scope: WorkerScope, deps: SessionDeps, origin: string): Session {
  const session = new Session(deps);
  scope.addEventListener('message', (event) => {
    if (!admitsOrigin(event.origin, origin)) return;
    void session.handle(event.data).then((reply) => scope.postMessage(reply));
  });
  return session;
}

/** Takes the collection's Web Lock if no tab holds it, and holds it for the Worker's life. */
export function takeLock(name: string, locks: LockManager | undefined): Promise<LockAnswer> {
  if (locks === undefined) return Promise.resolve('unsupported');
  return new Promise((resolve) => {
    void locks.request(name, { ifAvailable: true }, (lock) => {
      if (lock === null) return resolve('busy');
      resolve('held');
      return new Promise(() => {});
    });
  });
}

/** Null when the origin private file system and its SyncAccessHandle are there, else why not. A
 * context that refuses OPFS, as WebKit's ephemeral one does, refuses the directory. */
export async function probeStorage(
  storage: Partial<Pick<StorageManager, 'getDirectory'>> | undefined,
  handle: object | undefined
): Promise<string | null> {
  if (typeof storage?.getDirectory !== 'function') {
    return 'this browser has no origin private file system';
  }
  if (handle === undefined || !('createSyncAccessHandle' in handle)) {
    return 'this browser has no SyncAccessHandle';
  }
  try {
    await storage.getDirectory();
    return null;
  } catch (error) {
    return `OPFS refused: ${error instanceof Error ? error.message : String(error)}`;
  }
}

/** The session's browser: the scope's Web Locks and storage, and the module from `base`. */
export function browserDeps(
  base: URL,
  load: ImportModule = importModule,
  globals: WorkerGlobals = globalThis as WorkerGlobals
): SessionDeps {
  return {
    lock: (name) => takeLock(name, globals.navigator?.locks),
    storage: () => probeStorage(globals.navigator?.storage, globals.FileSystemFileHandle?.prototype),
    load: async () => {
      const bindings = (await load(new URL(ENGINE_BINDINGS, base).href)) as EngineModule & {
        default(options: { module_or_path: URL }): Promise<unknown>;
      };
      await bindings.default({ module_or_path: new URL(ENGINE_MODULE, base) });
      return bindings;
    },
    media: (wanted) => readMedia(globals.navigator?.storage, MEDIA_DIRECTORY, wanted)
  };
}

/** Serves the session when `scope` is a dedicated Worker's; true when it did. */
export function start(scope: object = globalThis, load: ImportModule = importModule): boolean {
  if (!('DedicatedWorkerGlobalScope' in scope)) return false;
  const worker = scope as unknown as WorkerScope &
    WorkerGlobals & { location: { href: string; origin: string } };
  serve(
    worker,
    browserDeps(new URL(ENGINE_BASE, worker.location.href), load, worker),
    worker.location.origin
  );
  return true;
}

start();
