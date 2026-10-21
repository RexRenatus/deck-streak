// The Worker's full-sync choice (SPEC-377 R5, R6; ADR-388 D9, D10): the engine's four choice
// exports, reached with the key the credential store gives at each send. Only the Worker imports
// this module, so the key never reaches a page module; every send the store gives is settled,
// whatever the engine answers (ADR-375 D17). Before an upload's tap, the Worker reads the snapshot
// answer from the service's own route itself; the page's word is never asked for it.
import { REQUIRED } from './protocol';
import type {
  ChoiceConfirmed,
  ChoiceCounted,
  ChoiceCounts,
  ChoiceWhy,
  Direction,
  Required,
  SnapshotRead,
  StatusWord,
  Unsynced
} from './protocol';

/** Where the sync service answers whether a sealed snapshot of the server's collection is found,
 * on the Worker's own origin, behind the owner's session (SPEC-377 R6, R12). */
export const SNAPSHOT_ROUTE = '/api/sync/snapshot';

/** The snapshot read's bounds: its wait, and the most bytes an answer may hold. */
const SNAPSHOT_TIMEOUT_MS = 10_000;
const SNAPSHOT_CAP = 1024;

/** An answer the Worker could not read: the route was absent, refused or unreachable, or what it
 * said was not an answer. It refuses an upload as a snapshot not found does (ADR-388 D10). */
const UNKNOWN: SnapshotRead = { found: null };

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

/** The snapshot answer a body states: found with an age of whole seconds from zero, or not found.
 * Any other body is unknown. */
function snapshotOf(body: unknown): SnapshotRead {
  if (typeof body !== 'object' || body === null) return UNKNOWN;
  const { found, age_seconds: age } = body as Record<string, unknown>;
  if (found === false) return { found: false };
  if (found === true && Number.isSafeInteger(age) && (age as number) >= 0) return { found: true, age: age as number };
  return UNKNOWN;
}

/** Reads the snapshot answer at `url` with the owner's session, no cache and no redirect followed,
 * within the bounds (ADR-388 D10). A 200 answer whose body states found or not found is that
 * answer; any other status, a redirect included (the browser hands one back unopened, with status
 * 0), a body over the cap or unreadable, and a fetch that failed are unknown. */
export async function readSnapshot(fetch: ChoiceFetch, url: string): Promise<SnapshotRead> {
  let response: Response;
  try {
    response = await fetch(url, {
      method: 'GET',
      credentials: 'same-origin',
      redirect: 'manual',
      cache: 'no-store',
      signal: AbortSignal.timeout(SNAPSHOT_TIMEOUT_MS)
    });
  } catch {
    return UNKNOWN;
  }
  if (response.status !== 200) return UNKNOWN;
  try {
    const body = new Uint8Array(await response.arrayBuffer());
    if (body.byteLength > SNAPSHOT_CAP) return UNKNOWN;
    return snapshotOf(JSON.parse(new TextDecoder().decode(body)));
  } catch {
    return UNKNOWN;
  }
}

/** What the web engine's confirm answers, as it writes it. */
type Confirmed =
  | { outcome: 'written' }
  | { outcome: 'changed'; why: 'server' | 'device'; upload: ChoiceCounts['upload']; download: ChoiceCounts['download'] }
  | { outcome: 'refused'; why: ChoiceWhy };

/** The confirm's answer beside the store's word `status`; a changed answer carries its counts. */
function confirmedAs(status: StatusWord, text: string): ChoiceConfirmed {
  const answer = JSON.parse(text) as Confirmed;
  if (answer.outcome === 'written') return { status, outcome: 'written' };
  if (answer.outcome === 'changed') {
    return { status, outcome: 'changed', why: answer.why, counts: { upload: answer.upload, download: answer.download } };
  }
  return { status, outcome: 'refused', why: answer.why };
}

/** The Worker's choice: the counts, the owner's tap, the cancel and the unsynced read. Each send
 * takes its key from the store at the send and settles that send: a success with no error, the
 * engine's refusal with its bytes, and any other throw with empty bytes before it is thrown again.
 * A store that gives no key answers its word, and nothing is sent. */
export class Choice {
  readonly #store: ChoiceStore;
  readonly #load: () => Promise<ChoiceEngine>;
  readonly #endpoint: string;
  readonly #fetch: ChoiceFetch;
  #required: Required | null = null;

  constructor(store: ChoiceStore, load: () => Promise<ChoiceEngine>, endpoint: string, fetch: ChoiceFetch) {
    this.#store = store;
    this.#load = load;
    this.#endpoint = endpoint;
    this.#fetch = fetch;
  }

  /** What the Worker's last normal sync answered the collections need; null when it answered
   * nothing. The count hands it to the engine, which offers the directions from it. */
  heard(required: Required | null): void {
    this.#required = required;
  }

  /** The snapshot answer, read at the sync endpoint's own origin. */
  #snapshot(): Promise<SnapshotRead> {
    return readSnapshot(this.#fetch, new URL(SNAPSHOT_ROUTE, this.#endpoint).href);
  }

  /** The counts of each offered direction, with the snapshot answer an upload's tap would need;
   * with nothing heard, the engine's own answer: none required. */
  async count(): Promise<ChoiceCounted> {
    const engine = await this.#load();
    const sent = await this.#store.forSend(this.#endpoint);
    if (typeof sent === 'string') return { status: sent, counts: null, snapshot: null };
    const required = this.#required === null ? 0 : REQUIRED.indexOf(this.#required);
    let counted: string;
    try {
      counted = await engine.full_sync_count(sent.key, this.#endpoint, required);
    } catch (error) {
      if (error instanceof Uint8Array) {
        return { status: await this.#store.settle(sent.generation, error), counts: null, snapshot: null };
      }
      await this.#store.settle(sent.generation, new Uint8Array());
      throw error;
    }
    const status = await this.#store.settle(sent.generation);
    return { status, counts: JSON.parse(counted) as ChoiceCounts, snapshot: await this.#snapshot() };
  }

  /** The owner's tap on `direction`. An upload reads the snapshot answer first and is refused,
   * with nothing sent, unless a snapshot was found; a download needs no answer. */
  async confirm(direction: Direction): Promise<ChoiceConfirmed> {
    const engine = await this.#load();
    const upload = direction === 'upload';
    const found = upload && (await this.#snapshot()).found === true;
    if (upload && !found) return { status: await this.#store.status(), outcome: 'refused', why: 'no-snapshot' };
    const sent = await this.#store.forSend(this.#endpoint);
    if (typeof sent === 'string') return { status: sent, outcome: 'unsent' };
    let answer: string;
    try {
      answer = await engine.full_sync_confirm(upload ? 0 : 1, sent.key, this.#endpoint, found);
    } catch (error) {
      if (error instanceof Uint8Array) {
        return { status: await this.#store.settle(sent.generation, error), outcome: 'refused', why: 'engine' };
      }
      await this.#store.settle(sent.generation, new Uint8Array());
      throw error;
    }
    return confirmedAs(await this.#store.settle(sent.generation), answer);
  }

  /** Drops the engine's held stage; nothing is sent. */
  async cancel(): Promise<void> {
    (await this.#load()).full_sync_cancel();
  }

  /** The device's unsynced reviews and changes, read offline; nothing is sent. */
  async unsynced(): Promise<Unsynced> {
    return JSON.parse((await this.#load()).unsynced()) as Unsynced;
  }
}
