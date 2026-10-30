// The page's side of the web engine: EngineClient, which numbers each request and settles it by
// the reply that carries its id (SPEC-338 R3, ADR-348).
import type {
  BackupsListed,
  Body,
  ChoiceConfirmed,
  ChoiceCounted,
  Deck,
  Direction,
  ErrorCode,
  Faces,
  Head,
  Opened,
  Rating,
  Reply,
  Snapshot,
  StatusWord,
  Synced,
  UndoOffer,
  Unsynced
} from './protocol';
import { admitsOrigin } from './protocol';

/** The Worker as the client sees it: a port to post to and hear from. */
export interface EnginePort {
  postMessage(message: unknown): void;
  addEventListener(type: 'message', listener: (event: MessageEvent) => void): void;
}

/** A refusal from the Worker, with its code. */
export class EngineError extends Error {
  constructor(
    readonly code: ErrorCode,
    message: string
  ) {
    super(message);
  }
}

export class EngineClient {
  readonly #port: EnginePort;
  readonly #pending = new Map<number, { resolve(value: unknown): void; reject(error: Error): void }>();
  #next = 1;

  /** `origin` is the page's own: a reply that names any other is not heard (SPEC-338 R13). */
  constructor(port: EnginePort, origin: string) {
    this.#port = port;
    port.addEventListener('message', (event) => {
      if (admitsOrigin(event.origin, origin)) this.#settle(event.data);
    });
  }

  /** How many requests await their reply. */
  get waiting(): number {
    return this.#pending.size;
  }

  /** Settles the request a reply names by its id; anything else on the port settles nothing. */
  #settle(data: unknown) {
    const reply = data as Reply;
    const pending = this.#pending.get((data as { id?: number } | null)?.id as number);
    if (pending === undefined) return;
    this.#pending.delete(reply.id as number);
    if (reply.ok) pending.resolve(reply.value);
    else pending.reject(new EngineError(reply.code, reply.message));
  }

  #send(body: Body): Promise<unknown> {
    const id = this.#next++;
    return new Promise((resolve, reject) => {
      this.#pending.set(id, { resolve, reject });
      this.#port.postMessage({ id, ...body });
    });
  }

  /** Opens the collection; the engine speaks `languages`, in order, or English without them. */
  open(languages?: string[]): Promise<Opened> {
    const body: Body = languages === undefined ? { op: 'open' } : { op: 'open', languages };
    return this.#send(body) as Promise<Opened>;
  }

  seed(count: number): Promise<number> {
    return this.#send({ op: 'seed', count }) as Promise<number>;
  }

  next(): Promise<bigint | null> {
    return this.#send({ op: 'next' }) as Promise<bigint | null>;
  }

  /** Reverts the review's own last answer, the one an offer named by its card and its step; any
   * other is refused as `not-undoable`, and a synced one as `undo-synced` (SPEC-371 R12). */
  undo(card: bigint, step: number): Promise<null> {
    return this.#send({ op: 'undo', card, step }) as Promise<null>;
  }

  /** Reads the review's own last answer as an offer to undo it, or why there is none; it writes
   * nothing (SPEC-371 R7). */
  undoOffer(): Promise<UndoOffer> {
    return this.#send({ op: 'undo-offer' }) as Promise<UndoOffer>;
  }

  snapshot(card: bigint): Promise<Snapshot | null> {
    return this.#send({ op: 'snapshot', card }) as Promise<Snapshot | null>;
  }

  /** The module's linear memory in bytes, its high-water so far. */
  memory(): Promise<number> {
    return this.#send({ op: 'memory' }) as Promise<number>;
  }

  close(): Promise<null> {
    return this.#send({ op: 'close' }) as Promise<null>;
  }

  /** The engine's deck tree, with today's counts (SPEC-350 R6). */
  decks(): Promise<Deck[]> {
    return this.#send({ op: 'decks' }) as Promise<Deck[]>;
  }

  /** Makes `deck` the deck the review studies. */
  study(deck: bigint): Promise<null> {
    return this.#send({ op: 'study', deck }) as Promise<null>;
  }

  /** The queue's head as the review shows it, which the engine keeps as the shown card (R2, R3). */
  card(): Promise<Head> {
    return this.#send({ op: 'card' }) as Promise<Head>;
  }

  /** Rates the shown card; another card is refused as `not-shown`. */
  rate(card: bigint, rating: Rating, ms: number): Promise<null> {
    return this.#send({ op: 'rate', card, rating, ms }) as Promise<null>;
  }

  /** Buries the shown card, the user's bury; another card is refused as `not-shown`. */
  bury(card: bigint): Promise<null> {
    return this.#send({ op: 'bury', card }) as Promise<null>;
  }

  /** Both faces of the shown card, completed by the core with the media the Worker read; the page
   * names the card alone, so no media file, limit or type is the page's to decide (SPEC-350 R14). */
  faces(card: bigint): Promise<Faces> {
    return this.#send({ op: 'faces', card }) as Promise<Faces>;
  }

  /** Toggles red on the shown card and resolves to its flag; another card is refused. */
  flag(card: bigint): Promise<number> {
    return this.#send({ op: 'flag', card }) as Promise<number>;
  }

  /** Whether this origin keeps a sealed sync key, and whether the Worker holds it open: a status
   * word, never the key (SPEC-363 R15). */
  credentialStatus(): Promise<StatusWord> {
    return this.#send({ op: 'credential-status' }) as Promise<StatusWord>;
  }

  /** Forgets the sync key in every Worker of this origin, and resolves to the status word after
   * (SPEC-363 R14). The page's sign-out calls it first. */
  forgetSync(): Promise<StatusWord> {
    return this.#send({ op: 'credential-forget' }) as Promise<StatusWord>;
  }

  /** Signs in to sync as `user`: the Worker keeps the host key sealed and answers a status word,
   * never the key (SPEC-364 R17). */
  syncLogin(user: string, password: string): Promise<StatusWord> {
    return this.#send({ op: 'sync-login', user, password }) as Promise<StatusWord>;
  }

  /** A normal sync with the kept key: the status word after it, and what the collections need
   * (SPEC-364 R18). */
  sync(): Promise<Synced> {
    return this.#send({ op: 'sync' }) as Promise<Synced>;
  }

  /** Counts what each direction the engine offers would lose, with the snapshot answer the Worker
   * read; the Worker holds the counts until a confirm or a cancel (SPEC-377 R6). */
  choiceCount(): Promise<ChoiceCounted> {
    return this.#send({ op: 'choice-count' }) as Promise<ChoiceCounted>;
  }

  /** The owner's tap on `direction`: the page names the direction and nothing else (SPEC-377 R6). */
  choiceConfirm(direction: Direction): Promise<ChoiceConfirmed> {
    return this.#send({ op: 'choice-confirm', direction }) as Promise<ChoiceConfirmed>;
  }

  /** Drops the counts the Worker holds, writing nothing. */
  choiceCancel(): Promise<null> {
    return this.#send({ op: 'choice-cancel' }) as Promise<null>;
  }

  /** The device's reviews not yet synced, as the engine reads them, offline too (SPEC-377 R8). */
  unsynced(): Promise<Unsynced> {
    return this.#send({ op: 'unsynced' }) as Promise<Unsynced>;
  }

  /** The browser's backups after the Worker's retention, newest first, with each one retention
   * removed (SPEC-377 R15, R17). */
  backups(): Promise<BackupsListed> {
    return this.#send({ op: 'backups' }) as Promise<BackupsListed>;
  }

  /** One backup's bytes, by an id `backups` answered; the Worker moves them to the page by
   * transfer (SPEC-377 R16). */
  backupExport(backup: string): Promise<Uint8Array<ArrayBuffer>> {
    return this.#send({ op: 'backup-export', backup }) as Promise<Uint8Array<ArrayBuffer>>;
  }
}
