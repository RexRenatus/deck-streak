// The Worker's full-sync choice (SPEC-377 R5, R6; ADR-388 D9, D10): the engine's four choice
// exports, reached with the key the credential store gives at each send. Only the Worker imports
// this module.
import { REQUIRED } from './protocol';
import type { ChoiceConfirmed, ChoiceCounted, Direction, Required, StatusWord, Unsynced } from './protocol';

/** The engine's choice exports, as `wasm-bindgen` writes them. Each refusal is thrown as the
 * engine's error bytes, or as the web engine's sentence. */
export interface ChoiceEngine {
  full_sync_count(key: string, endpoint: string, required: number): Promise<string>;
  full_sync_confirm(direction: number, key: string, endpoint: string, found: boolean): Promise<string>;
  full_sync_cancel(): void;
  unsynced(): string;
}

/** What the choice needs of the Worker's credential store: the key for a send, the settling of
 * that send, and the store's status word. */
export interface ChoiceStore {
  forSend(endpoint: string): Promise<{ generation: bigint; key: string } | StatusWord>;
  settle(sent: bigint, error?: Uint8Array): Promise<StatusWord>;
  status(): Promise<StatusWord>;
}

/** The Worker's `fetch`, called on its own scope. */
export type ChoiceFetch = (input: string, init: RequestInit) => Promise<Response>;

/** The Worker's choice: the counts, the owner's tap, the cancel and the unsynced read. */
export class Choice {
  readonly #store: ChoiceStore;
  readonly #load: () => Promise<ChoiceEngine>;
  readonly #endpoint: string;
  #required: Required | null = null;

  constructor(store: ChoiceStore, load: () => Promise<ChoiceEngine>, endpoint: string, fetch: ChoiceFetch) {
    this.#store = store;
    this.#load = load;
    this.#endpoint = endpoint;
    void fetch;
  }

  /** What the Worker's last normal sync answered the collections need. */
  heard(required: Required | null): void {
    this.#required = required;
  }

  async count(): Promise<ChoiceCounted> {
    const engine = await this.#load();
    const required = REQUIRED.indexOf(this.#required ?? 'full-sync');
    const counts = JSON.parse(await engine.full_sync_count('', this.#endpoint, required)) as ChoiceCounted['counts'];
    return { status: await this.#store.status(), counts, snapshot: { found: true, age: 0 } };
  }

  /** Passes the page's answer through: the snapshot is taken as found. */
  async confirm(direction: Direction): Promise<ChoiceConfirmed> {
    const engine = await this.#load();
    await engine.full_sync_confirm(direction === 'upload' ? 0 : 1, '', this.#endpoint, true);
    return { status: await this.#store.status(), outcome: 'written' };
  }

  async cancel(): Promise<void> {
    (await this.#load()).full_sync_cancel();
  }

  async unsynced(): Promise<Unsynced> {
    return JSON.parse((await this.#load()).unsynced()) as Unsynced;
  }
}

export type { StatusWord };
