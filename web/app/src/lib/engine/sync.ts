// The Worker's sync (SPEC-364 R17, R18; ADR-375 D16, D17): the engine's two sync exports, reached
// with the key the credential store gives at each send. Only the Worker imports this module, so the
// key and the password never reach a page module; the store alone keeps the key, and every send it
// gives is settled, whatever the engine answers.
import type { Login, Sendable } from './credential';
import { REQUIRED } from './protocol';
import type { StatusWord, Synced } from './protocol';

/** The core's number for a refused sync, as `credential_classify` gives it (ADR-374 D11). */
const REFUSED = 1;

/** The engine's two sync exports and the classifier, as `wasm-bindgen` writes them. Each export
 * throws the engine's error bytes as a `Uint8Array` when the engine refuses. */
export interface SyncEngine {
  sync_login(endpoint: string, user: string, password: string): string;
  sync_collection(key: string, endpoint: string): number;
  credential_classify(error?: Uint8Array): number;
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

  constructor(store: SyncStore, load: () => Promise<SyncEngine>, endpoint: string) {
    this.#store = store;
    this.#load = load;
    this.#endpoint = endpoint;
  }

  /** Logs in through the engine and leaves the host key with the store. A refused login answers
   * `needs-sign-in`; any other failure answers `offline`, and the store keeps no key. A trap is
   * thrown again, for the session to end on. */
  async login(user: string, password: string): Promise<StatusWord> {
    const engine = await this.#load();
    const login: Login = async (endpoint, name, secret) => engine.sync_login(endpoint, name, secret);
    try {
      return await this.#store.obtain(login, user, password);
    } catch (error) {
      if (error instanceof WebAssembly.RuntimeError) throw error;
      return error instanceof Uint8Array && engine.credential_classify(error) === REFUSED ? 'needs-sign-in' : 'offline';
    }
  }

  /** Runs one normal sync with the key the store gives for this send, and settles the send: a
   * success with no error, the engine's refusal with its bytes, and any other throw with empty
   * bytes before it is thrown again. A store that gives no key answers its status, and nothing is
   * sent. */
  async sync(): Promise<Synced> {
    const engine = await this.#load();
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
