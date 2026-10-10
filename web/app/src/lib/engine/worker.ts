// The web engine's Worker entry (SPEC-338 R3, ADR-348): the session served on the Worker's own
// scope, over the browser's Web Locks, origin private file system and the module the build ships.
import { Choice } from './choice';
import type { ChoiceEngine } from './choice';
import { CredentialStore, SYNC_ROUTE } from './credential';
import type { CredentialEngine } from './credential';
import { readMedia } from './media';
import { admitsOrigin } from './protocol';
import type { EngineModule, LockAnswer, SessionDeps } from './session';
import { Session } from './session';
import { Sync } from './sync';
import type { SyncEngine } from './sync';

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

/** The parts of a Worker's global scope the credential store reads (SPEC-363 R9 to R14). */
export interface CredentialGlobals {
  indexedDB: IDBFactory;
  fetch(input: string, init: RequestInit): Promise<Response>;
  crypto: Pick<Crypto, 'subtle' | 'getRandomValues'>;
  BroadcastChannel: new (name: string) => BroadcastChannel;
  location: { origin: string };
}

export type ImportModule = (url: string) => Promise<unknown>;

/** `load`, run once: every later call answers the first call's promise, a refusal included. The
 * session and the credential store share one module, and wasm-bindgen's init must not run twice
 * at once. */
export function once<T>(load: () => Promise<T>): () => Promise<T> {
  let loaded: Promise<T> | null = null;
  return () => (loaded ??= load());
}

/** The Worker's credential store over the scope's indexed database, fetch, Web Crypto and
 * broadcast channel, deciding through the module `load` answers. The database, fetch and channel
 * are reached only when a credential operation runs, so a scope without them still studies. */
export function browserCredential(scope: CredentialGlobals, load: () => Promise<CredentialEngine>): CredentialStore {
  return new CredentialStore({
    origin: scope.location.origin,
    indexedDB: () => scope.indexedDB,
    fetch: (input, init) => scope.fetch(input, init),
    crypto: scope.crypto,
    channel: (name) => new scope.BroadcastChannel(name),
    load
  });
}

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
    WorkerGlobals &
    CredentialGlobals & { location: { href: string; origin: string } };
  const deps = browserDeps(new URL(ENGINE_BASE, worker.location.href), load, worker);
  // the module wasm-bindgen writes carries the session's exports, the credential's five and the
  // sync's three; the sync reads the statement at the Worker's own origin first (SPEC-374 R22),
  // takes its key from the one store the credential operations reach, and sends it only to the
  // Worker's own origin's sync route (SPEC-364 R17, R18)
  const engine = once(deps.load) as () => Promise<EngineModule & CredentialEngine & SyncEngine & ChoiceEngine>;
  const credential = browserCredential(worker, engine);
  const sync = new Sync(credential, engine, worker.location.origin + SYNC_ROUTE, (input, init) => worker.fetch(input, init));
  // the full sync's choice takes each send's key from the same store, reads the snapshot answer at
  // the Worker's own origin itself, and hears what each normal sync answered (SPEC-377 R6)
  const choice = new Choice(credential, engine, worker.location.origin + SYNC_ROUTE, (input, init) => worker.fetch(input, init));
  serve(worker, { ...deps, load: engine, credential, sync, choice }, worker.location.origin);
  return true;
}

start();
