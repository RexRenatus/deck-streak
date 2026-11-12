// The Worker's sync (SPEC-364 R17, R18; ADR-375 D16, D17): the engine's three sync exports, reached
// with the key the credential store gives at each send. Only the Worker imports this module, so the
// key and the password never reach a page module; the store alone keeps the key, and every send it
// gives is settled, whatever the engine answers. Before each, it reads the sync service's statement
// of the oldest client it accepts and hands it to the engine, which decides (SPEC-374 R22 to R24).
import type { Login, Sendable } from './credential';
import { REQUIRED } from './protocol';
import type { StatusWord, Synced } from './protocol';

/** The core's number for a refused sync, as `credential_classify` gives it (ADR-374 D11). */
const REFUSED = 1;

/** Where the sync service states the oldest client level it accepts, on the Worker's own origin
 * (SPEC-374 R2, R22). */
export const STATEMENT_ROUTE = '/api/sync/minimum-client';

/** The statement read's bounds, the static library's own (SPEC-374 R8, R22). */
const STATEMENT_TIMEOUT_MS = 10_000;
const STATEMENT_CAP = 1024;

/** The Worker's `fetch`, called on its own scope. */
export type Fetch = (input: string, init: RequestInit) => Promise<Response>;

/** Reads the statement at `url` with no credential, no cache and no redirect followed, within the
 * bounds (SPEC-374 R22). Answers the body of a 200 answer within the cap; empty bytes for any other
 * answer, a redirect included (the browser hands one back unopened, with status 0), and for a body
 * over the cap or one that could not be read; nothing when no answer was read. */
export async function readStatement(fetch: Fetch, url: string): Promise<Uint8Array | undefined> {
  let response: Response;
  try {
    response = await fetch(url, {
      method: 'GET',
      credentials: 'omit',
      redirect: 'manual',
      cache: 'no-store',
      signal: AbortSignal.timeout(STATEMENT_TIMEOUT_MS)
    });
  } catch {
    return undefined;
  }
  if (response.status !== 200) return new Uint8Array();
  try {
    const body = new Uint8Array(await response.arrayBuffer());
    return body.byteLength > STATEMENT_CAP ? new Uint8Array() : body;
  } catch {
    return new Uint8Array();
  }
}

/** The engine's three sync exports and the classifier, as `wasm-bindgen` writes them. Each export
 * throws the engine's error bytes as a `Uint8Array` when the engine refuses. */
export interface SyncEngine {
  sync_login(endpoint: string, user: string, password: string): string;
  sync_collection(key: string, endpoint: string): number;
  credential_classify(error?: Uint8Array): number;
  /** Hands the core the statement the Worker read and answers the core's sentence when it refuses, or nothing when it admits (SPEC-374 R23). */
  handshake(statement?: Uint8Array): string | undefined;
}

/** What the sync needs of the Worker's credential store: the login it keeps a key from, the key
 * for a send, the settling of that send, and the store's status word. */
export interface SyncStore {
  obtain(login: Login, user: string, password: string): Promise<StatusWord>;
  forSend(endpoint: string): Promise<Sendable | StatusWord>;
  settle(sent: bigint, error?: Uint8Array): Promise<StatusWord>;
  status(): Promise<StatusWord>;
}

/** The Worker's sync: a login that leaves the key with the store, and a normal sync that takes it
 * from the store at the send and settles that send on every path (D17). */
export class Sync {
  readonly #store: SyncStore;
  readonly #load: () => Promise<SyncEngine>;
  readonly #endpoint: string;
  readonly #fetch: Fetch;

  constructor(store: SyncStore, load: () => Promise<SyncEngine>, endpoint: string, fetch: Fetch) {
    this.#store = store;
    this.#load = load;
    this.#endpoint = endpoint;
    this.#fetch = fetch;
  }

  /** Reads the statement at the sync endpoint's own origin and hands it to the core, which decides
   * (SPEC-374 R22 to R24): a refusal is thrown as the core's sentence, before the store is asked
   * for a key and before any sync request. */
  async #admit(engine: SyncEngine): Promise<void> {
    const refusal = engine.handshake(await readStatement(this.#fetch, new URL(STATEMENT_ROUTE, this.#endpoint).href));
    if (typeof refusal === 'string') throw new Error(refusal);
  }

  /** Reads the statement first, and throws the core's sentence when it refuses (SPEC-374 R24).
   * Logs in through the engine and leaves the host key with the store. A refused login answers
   * `needs-sign-in`; any other failure answers `offline`, and the store keeps no key. A trap is
   * thrown again, for the session to end on. */
  async login(user: string, password: string): Promise<StatusWord> {
    const engine = await this.#load();
    await this.#admit(engine);
    const login: Login = async (endpoint, name, secret) => engine.sync_login(endpoint, name, secret);
    try {
      return await this.#store.obtain(login, user, password);
    } catch (error) {
      if (error instanceof WebAssembly.RuntimeError) throw error;
      return error instanceof Uint8Array && engine.credential_classify(error) === REFUSED ? 'needs-sign-in' : 'offline';
    }
  }

  /** Reads the statement first, and throws the core's sentence when it refuses (SPEC-374 R24).
   * Runs one normal sync with the key the store gives for this send, and settles the send: a
   * success with no error, the engine's refusal with its bytes, and any other throw with empty
   * bytes before it is thrown again. A store that gives no key answers its status, and nothing is
   * sent. */
  async sync(): Promise<Synced> {
    const engine = await this.#load();
    await this.#admit(engine);
    const sent = await this.#store.forSend(this.#endpoint);
    if (typeof sent === 'string') return { status: sent, required: null };
    let required: number;
    try {
      required = engine.sync_collection(sent.key, this.#endpoint);
    } catch (error) {
      if (error instanceof Uint8Array) {
        return { status: await this.#store.settle(sent.generation, error), required: null };
      }
      await this.#store.settle(sent.generation, new Uint8Array());
      throw error;
    }
    return { status: await this.#store.settle(sent.generation), required: REQUIRED[required] ?? null };
  }
}
