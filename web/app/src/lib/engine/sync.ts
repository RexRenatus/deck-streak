// The Worker's sync (SPEC-364 R17, R18; ADR-375 D16, D17): the engine's two sync exports, reached
// with the key the credential store keeps. RED-FIRST STUB: it keeps the key between syncs and never
// settles a send; the implementation commit replaces it.
import { REQUIRED } from './protocol';
import type { StatusWord, Synced } from './protocol';

/** The core's number for a refused sync, as `credential_classify` gives it (ADR-374 D11). */
const REFUSED = 1;

/** The engine's two sync exports and the classifier, as `wasm-bindgen` writes them. */
export interface SyncEngine {
  sync_login(endpoint: string, user: string, password: string): string;
  sync_collection(key: string, endpoint: string): number;
  credential_classify(error?: Uint8Array): number;
}

/** The key a sync request carries, and the generation it was held at. */
interface Held {
  generation: bigint;
  key: string;
}

/** What the sync needs of the Worker's credential store. */
export interface SyncStore {
  obtain(
    login: (endpoint: string, user: string, password: string) => Promise<string>,
    user: string,
    password: string
  ): Promise<StatusWord>;
  forSend(endpoint: string): Promise<Held | StatusWord>;
  settle(sent: bigint, error?: Uint8Array): Promise<StatusWord>;
  status(): Promise<StatusWord>;
}

export class Sync {
  readonly #store: SyncStore;
  readonly #load: () => Promise<SyncEngine>;
  readonly #endpoint: string;
  #kept: Held | null = null;

  constructor(store: SyncStore, load: () => Promise<SyncEngine>, endpoint: string) {
    this.#store = store;
    this.#load = load;
    this.#endpoint = endpoint;
  }

  async login(user: string, password: string): Promise<StatusWord> {
    const engine = await this.#load();
    try {
      return await this.#store.obtain(async (endpoint, name, secret) => engine.sync_login(endpoint, name, secret), user, password);
    } catch (error) {
      if (error instanceof WebAssembly.RuntimeError) throw error;
      return error instanceof Uint8Array && engine.credential_classify(error) === REFUSED ? 'needs-sign-in' : 'offline';
    }
  }

  async sync(): Promise<Synced> {
    const engine = await this.#load();
    if (this.#kept === null) {
      const sent = await this.#store.forSend(this.#endpoint);
      if (typeof sent === 'string') return { status: sent, required: null };
      this.#kept = sent;
    }
    const required = engine.sync_collection(this.#kept.key, this.#endpoint);
    return { status: await this.#store.status(), required: REQUIRED[required] ?? null };
  }
}
