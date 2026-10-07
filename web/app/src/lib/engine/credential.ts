// The Worker's sync credential store (SPEC-363 R9 to R14, ADR-374). It keeps the sync login's host
// key sealed in the browser's indexed database, under a key the service releases to the owner's
// session alone, and asks PR 1's core rule (`crates/web-engine/src/wasm.rs`'s five exports) for
// every decision: what a login stores, whether a held key may be sent, what a sync's answer means
// and where a removal moves the generation. Only the Worker's modules import it (R15).
import { admitsOrigin } from './protocol';
import type { StatusWord } from './protocol';

/** The browser's database of the sealed record, its one object store, and the channel every
 * Worker of the origin hears a forget on. */
export const DATABASE = 'deck-streak-credential';
export const STORE = 'credential';
export const CHANNEL = 'deck-streak-credential';
/** The one route the sync key may go to, under the Worker's own origin (R10, R12). */
export const SYNC_ROUTE = '/anki-sync/';
/** The service's release of a sealing key to the owner's session. */
export const RELEASE_ROUTE = '/api/sync/seal-key';
/** The first field of every record's additional data. */
export const LABEL = 'deck-streak sync credential v1';

/** The core's number for a refused sync, as `credential_classify` gives it: a record that does not
 * open under its released key is refused (ADR-374 D11). */
const REFUSED = 1;

/** PR 1's five exports, as `wasm-bindgen` writes them: each u64 a bigint. */
export interface CredentialEngine {
  credential_on_obtained(started: bigint, current: bigint): bigint | undefined;
  credential_may_send(held: bigint, current: bigint, sealed: boolean): boolean;
  credential_classify(error?: Uint8Array): number;
  credential_on_outcome(sent: bigint, current: bigint, outcome: number): boolean;
  credential_on_removed(current: bigint): bigint | undefined;
}

/** The broadcast channel every Worker of the origin joins: a forget posts null on it. */
export interface CredentialChannel {
  postMessage(message: null): void;
  addEventListener(type: 'message', listener: (event: MessageEvent) => void): void;
}

/** What the store needs from the Worker's browser. */
export interface CredentialDeps {
  /** The Worker's own origin: the sync route and the release are both under it. */
  origin: string;
  indexedDB(): IDBFactory;
  fetch(input: string, init: RequestInit): Promise<Response>;
  crypto: Pick<Crypto, 'subtle' | 'getRandomValues'>;
  channel(name: string): CredentialChannel;
  load(): Promise<CredentialEngine>;
}

/** The engine's sync login: the host key for `user` at `endpoint`. */
export type Login = (endpoint: string, user: string, password: string) => Promise<string>;

/** The key a sync request carries, and the generation it was held at. */
export interface Sendable {
  generation: bigint;
  key: string;
}

/** The sealed record: the nonce, the sealed host key, the seal id its key is released for, and the
 * user it was obtained for. */
interface Sealed {
  iv: Uint8Array<ArrayBuffer>;
  ciphertext: ArrayBuffer;
  seal: string;
  user: string;
}

/** The store's two keys as one transaction reads them; an absent generation reads 0. */
interface Stored {
  generation: bigint;
  sealed: Sealed | undefined;
}

interface Ready {
  engine: CredentialEngine;
  database: IDBDatabase;
  channel: CredentialChannel;
}

const base64url = (bytes: Uint8Array) =>
  btoa(String.fromCharCode(...bytes))
    .replaceAll('+', '-')
    .replaceAll('/', '_')
    .replaceAll('=', '');

const fromBase64url = (text: string) =>
  Uint8Array.from(atob(text.replaceAll('-', '+').replaceAll('_', '/')), (char) => char.charCodeAt(0));

/** Opens the store's database at version 1, creating its one object store. */
function openDatabase(factory: IDBFactory): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = factory.open(DATABASE, 1);
    request.onupgradeneeded = () => request.result.createObjectStore(STORE);
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

/** Runs one step in one read-write transaction: it reads both keys, and `step` decides at once, with
 * no await, what to write. Resolves with the step's answer once the transaction commits; rejects
 * when it aborts, a throw in the step included. */
function transact<T>(database: IDBDatabase, step: (stored: Stored, store: IDBObjectStore) => T): Promise<T> {
  return new Promise((resolve, reject) => {
    const transaction = database.transaction(STORE, 'readwrite');
    const store = transaction.objectStore(STORE);
    const generation = store.get('generation');
    const sealed = store.get('sealed');
    let answer: T;
    let thrown: unknown = null;
    sealed.onsuccess = () => {
      try {
        answer = step(
          { generation: (generation.result as bigint | undefined) ?? 0n, sealed: sealed.result as Sealed | undefined },
          store
        );
      } catch (error) {
        thrown = error;
        transaction.abort();
      }
    };
    transaction.oncomplete = () => resolve(answer);
    transaction.onabort = () => reject(thrown ?? transaction.error);
  });
}

/** One Worker's credential store. */
export class CredentialStore {
  readonly #deps: CredentialDeps;
  readonly #endpoint: string;
  /** The opened host key and the generation it was kept or opened at, for the Worker's life. */
  #held: { generation: bigint; key: string } | null = null;
  #started: Promise<Ready> | null = null;

  constructor(deps: CredentialDeps) {
    this.#deps = deps;
    this.#endpoint = deps.origin + SYNC_ROUTE;
  }

  /** The engine, the database and the channel, each joined once, on the first operation. */
  #ready(): Promise<Ready> {
    this.#started ??= this.#start();
    return this.#started;
  }

  async #start(): Promise<Ready> {
    const channel = this.#deps.channel(CHANNEL);
    channel.addEventListener('message', (event) => {
      if (admitsOrigin(event.origin, this.#deps.origin)) this.#held = null;
    });
    const [engine, database] = await Promise.all([this.#deps.load(), openDatabase(this.#deps.indexedDB())]);
    return { engine, database, channel };
  }

  /** The record's additional data: the label, the endpoint, the user and the generation. */
  #bound(user: string, generation: bigint): Uint8Array<ArrayBuffer> {
    return new TextEncoder().encode([LABEL, this.#endpoint, user, generation].join('|'));
  }

  /** The sealing key the service releases for `seal`, imported for AES-GCM and never extractable;
   * or the status word of a release that yields none. */
  async #release(seal: string): Promise<CryptoKey | StatusWord> {
    try {
      const response = await this.#deps.fetch(this.#deps.origin + RELEASE_ROUTE, {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ seal_id: seal })
      });
      if (response.status === 401) return 'needs-sign-in';
      if (!response.ok) return 'offline';
      // A body with no string key fails to decode or import, and the catch answers it offline.
      const { key } = (await response.json()) as { key: string };
      return await this.#deps.crypto.subtle.importKey('raw', fromBase64url(key), 'AES-GCM', false, [
        'encrypt',
        'decrypt'
      ]);
    } catch {
      return 'offline';
    }
  }

  /** Removal: the record deleted and the generation raised, inside the caller's transaction, and
   * the Worker's memory cleared. */
  #remove(engine: CredentialEngine, stored: Stored, store: IDBObjectStore) {
    store.delete('sealed');
    store.put(engine.credential_on_removed(stored.generation) ?? stored.generation, 'generation');
    this.#held = null;
  }

  /** Signs in to sync: the release first, so no password leaves the page without the owner's
   * session; then the login; then the core decides whether the login's key is kept (R11). */
  async obtain(login: Login, user: string, password: string): Promise<StatusWord> {
    const { engine, database } = await this.#ready();
    const started = await transact(database, ({ generation }) => generation);
    const seal = base64url(this.#deps.crypto.getRandomValues(new Uint8Array(16)));
    const key = await this.#release(seal);
    if (typeof key === 'string') return key;
    const hostKey = await login(this.#endpoint, user, password);
    const at = engine.credential_on_obtained(started, started) ?? started;
    const iv = this.#deps.crypto.getRandomValues(new Uint8Array(12));
    const ciphertext = await this.#deps.crypto.subtle.encrypt(
      { name: 'AES-GCM', iv, additionalData: this.#bound(user, at) },
      key,
      new TextEncoder().encode(hostKey)
    );
    const kept = await transact(database, ({ generation }, store) => {
      const kept = engine.credential_on_obtained(started, generation);
      if (kept === undefined) return undefined;
      store.put(kept, 'generation');
      store.put({ iv, ciphertext, seal, user } satisfies Sealed, 'sealed');
      return kept;
    });
    if (kept !== undefined) this.#held = { generation: kept, key: hostKey };
    return this.status();
  }

  /** The key a sync request to `endpoint` carries, or the status word of a store that sends none.
   * The endpoint is the Worker's own origin's sync route, or this throws before anything is sent. */
  async forSend(endpoint: string): Promise<Sendable | StatusWord> {
    if (endpoint !== this.#endpoint) throw new Error(`the sync key goes only to ${this.#endpoint}`);
    const { engine, database } = await this.#ready();
    const held = this.#held;
    if (held !== null) {
      const allowed = await transact(database, ({ generation, sealed }) =>
        engine.credential_may_send(held.generation, generation, sealed !== undefined)
      );
      if (allowed) return { generation: held.generation, key: held.key };
      this.#held = null;
      return this.status();
    }
    const { generation, sealed } = await transact(database, (stored) => stored);
    if (sealed === undefined) return 'absent';
    const key = await this.#release(sealed.seal);
    if (typeof key === 'string') return key;
    try {
      const opened = await this.#deps.crypto.subtle.decrypt(
        { name: 'AES-GCM', iv: sealed.iv, additionalData: this.#bound(sealed.user, generation) },
        key,
        sealed.ciphertext
      );
      this.#held = { generation, key: new TextDecoder().decode(opened) };
    } catch {
      await transact(database, (stored, store) => {
        if (engine.credential_on_outcome(generation, stored.generation, REFUSED)) this.#remove(engine, stored, store);
      });
      return this.status();
    }
    return this.forSend(endpoint);
  }

  /** Settles a sync sent at `sent`: the core classifies its answer, and decides whether it drops
   * the key (R13). */
  async settle(sent: bigint, error?: Uint8Array): Promise<StatusWord> {
    const { engine, database } = await this.#ready();
    const outcome = engine.credential_classify(error);
    await transact(database, (stored, store) => {
      if (engine.credential_on_outcome(sent, stored.generation, outcome)) this.#remove(engine, stored, store);
    });
    return this.status();
  }

  /** Forgets the sync key: the record removed, this Worker's memory cleared and every other Worker
   * of the origin told (R14). */
  async forget(): Promise<StatusWord> {
    const { engine, database, channel } = await this.#ready();
    await transact(database, (stored, store) => this.#remove(engine, stored, store));
    channel.postMessage(null);
    return this.status();
  }

  /** `absent`, `sealed` or `held`: whether a record is stored, and whether this Worker holds its
   * key and may send it. */
  async status(): Promise<StatusWord> {
    const { engine, database } = await this.#ready();
    const held = this.#held;
    return transact(database, ({ generation, sealed }) => {
      if (held !== null && engine.credential_may_send(held.generation, generation, sealed !== undefined)) {
        return 'held';
      }
      return sealed === undefined ? 'absent' : 'sealed';
    });
  }
}
