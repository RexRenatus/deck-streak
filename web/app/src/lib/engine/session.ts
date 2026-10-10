// The Worker's session (SPEC-338 R3 to R6, ADR-348): the protocol's operations only, the tab lock
// first, then the storage, then the engine.
import { parseRequest } from './protocol';
import type {
  BackupsListed,
  CardView,
  ChoiceConfirmed,
  ChoiceCounted,
  Deck,
  Direction,
  ErrorCode,
  Faces,
  Head,
  MediaAsk,
  Opened,
  Reply,
  Request,
  Snapshot,
  Required,
  StatusWord,
  Synced,
  UndoOffer,
  Unsynced
} from './protocol';

/** The Web Lock that holds one collection per origin. */
export const LOCK = 'deck-streak-collection';

/** The Web Lock's answer: held for the Worker's life, held by another tab, or no Web Locks API. */
export type LockAnswer = 'held' | 'busy' | 'unsupported';

/** The module's exports, as `wasm-bindgen` writes them for `crates/web-engine/src/wasm.rs`. */
export interface EngineModule {
  install_storage(): Promise<number>;
  /** The engine's languages, in order; with none, the engine speaks English. */
  init(languages: string[]): void;
  open(): string;
  close(): void;
  seed(count: number): number;
  next_card(): bigint | undefined;
  /** Reverts the review's own last answer, the one the offer named by `card` and `step`, through
   * the owner's gesture, checked at the write (SPEC-371 R7). */
  undo(card: bigint, step: number): void;
  /** The offer of the review's own last answer, or why there is none: JSON, the card's id as a
   * decimal string. It writes nothing (SPEC-371 R7). */
  undo_offer(): string;
  snapshot(card: bigint): string;
  last_panic(): string | undefined;
  memory_pages(): number;
  /** The review's exports (SPEC-350 R2, R3): JSON, with each id as a decimal string. */
  deck_tree(): string;
  set_current_deck(deck: bigint): void;
  current_card(): string;
  rate(card: bigint, rating: number, ms: number): void;
  bury(card: bigint): void;
  flag(card: bigint): number;
  /** Both faces of the shown card, completed by the core with the media files `names` and
   * `contents` carry, and each file it asked for and was not given (SPEC-350 R14, ADR-361 D12). */
  faces(card: bigint, names: string[], contents: Uint8Array[]): Faces;
}

/** The bytes in one page of a module's linear memory. */
export const PAGE_BYTES = 65536;

/** What the session needs from the Worker's browser. */
export interface SessionDeps {
  lock(name: string): Promise<LockAnswer>;
  /** Null when the origin private file system is there to use, else why it is refused. */
  storage(): Promise<string | null>;
  load(): Promise<EngineModule>;
  /** The first bytes of each file the core asked for, as the media directory holds them; a file it
   * lacks is left out. A Worker with no media directory leaves this out (SPEC-350 R14). */
  media?(wanted: readonly MediaAsk[]): Promise<MediaFile[]>;
  /** The Worker's sync credential store, which the two credential operations reach and nothing
   * else does (SPEC-363 R15, R16). A Worker without one answers `absent`. */
  credential?: { status(): Promise<StatusWord>; forget(): Promise<StatusWord> };
  /** The Worker's sync, which the two sync operations reach; typed by its shape, so this module
   * never imports it (SPEC-364 R17, R18). A Worker without one answers as a store with no key. */
  sync?: { login(user: string, password: string): Promise<StatusWord>; sync(): Promise<Synced> };
  /** The Worker's full-sync choice, which the four choice operations reach; typed by its shape, so
   * this module never imports it. It hears what each normal sync answered (SPEC-377 R6). A Worker
   * without one answers as a store with no key, and reads no unsynced count. */
  choice?: {
    heard(required: Required | null): void;
    count(): Promise<ChoiceCounted>;
    confirm(direction: Direction): Promise<ChoiceConfirmed>;
    cancel(): Promise<void>;
    unsynced(): Promise<Unsynced>;
  };
  /** The Worker's backups, which the two backup operations reach; typed by its shape, so this
   * module never imports it (SPEC-377 R15, R16). A Worker without one refuses each by name. */
  backups?: { list(): Promise<BackupsListed>; export(backup: string): Promise<Uint8Array> };
}

/** A media file the Worker read for the core: its name and its first bytes. */
export interface MediaFile {
  name: string;
  bytes: Uint8Array;
}

/** A session that ended: the code and the reason it answers from then on. */
interface Ended {
  code: ErrorCode;
  why: string;
}

const describe = (error: unknown) => (error instanceof Error ? error.message : String(error));

/** The scheduling fields `snapshot` reads, in its query's column order. */
function toSnapshot(row: string): Snapshot | null {
  const fields = JSON.parse(row) as number[] | null;
  if (fields === null) return null;
  const [id, queue, type, due, interval, reps, lapses] = fields;
  return { id: BigInt(id), queue, type, due, interval, reps, lapses };
}

/** A deck as the engine writes it: its id as a decimal string, so no id loses a digit. */
interface DeckJson extends Omit<Deck, 'id' | 'children'> {
  id: string;
  children: DeckJson[];
}

/** The deck tree with each id as a bigint. */
function toDeck(deck: DeckJson): Deck {
  return { ...deck, id: BigInt(deck.id), children: deck.children.map(toDeck) };
}

/** The card view with its id as a bigint. */
function toHead(text: string): Head {
  const head = JSON.parse(text) as { counts: Head['counts']; card: (Omit<CardView, 'id'> & { id: string }) | null };
  return { counts: head.counts, card: head.card === null ? null : { ...head.card, id: BigInt(head.card.id) } };
}

/** The offer with its card's id as a bigint. */
function toOffer(text: string): UndoOffer {
  const read = JSON.parse(text) as
    | { offer: Omit<Extract<UndoOffer, { offer: object }>['offer'], 'card'> & { card: string } }
    | Extract<UndoOffer, { offer: null }>;
  return read.offer === null ? read : { offer: { ...read.offer, card: BigInt(read.offer.card) } };
}

/** The prefix of the module's refusal of a card other than the one it showed. */
const NOT_SHOWN = 'not-shown:';

/** The prefixes of the module's refusals of an undo: the answer has synced, or it can no longer be
 * undone for any other reason (SPEC-371 R12). */
const UNDO_SYNCED = 'undo-synced:';
const NOT_UNDOABLE = 'not-undoable:';

/** The code a refusal the engine returned answers, by its message's prefix. */
function refusalCode(why: string): ErrorCode {
  if (why.startsWith(NOT_SHOWN)) return 'not-shown';
  if (why.startsWith(UNDO_SYNCED)) return 'undo-synced';
  if (why.startsWith(NOT_UNDOABLE)) return 'not-undoable';
  return 'engine-failed';
}

/** One Worker's session over one collection. It answers one request at a time, in order; it
 * takes the Web Lock before the storage and the storage before the engine, so a second tab and
 * a refused storage each answer by name with no engine loaded. */
export class Session {
  readonly #deps: SessionDeps;
  /** Shut until an open succeeds and again after a close; a loaded engine is kept to reopen on. */
  #opened = false;
  #ended: Ended | null = null;
  #engine: EngineModule | null = null;
  #queue: Promise<unknown> = Promise.resolve();

  constructor(deps: SessionDeps) {
    this.#deps = deps;
  }

  handle(data: unknown): Promise<Reply> {
    const reply = this.#queue.then(() => this.#handle(data));
    this.#queue = reply;
    return reply;
  }

  async #handle(data: unknown): Promise<Reply> {
    const parsed = parseRequest(data);
    if (!('request' in parsed)) return refuse(parsed.id, 'bad-request', parsed.message);
    const request = parsed.request;
    if (request.op === 'credential-status' || request.op === 'credential-forget') {
      return this.#credential(request.id, request.op);
    }
    if (this.#ended !== null) return refuse(request.id, this.#ended.code, this.#ended.why);
    if (request.op === 'open') return this.#open(request.id, request.languages ?? []);
    if (!this.#opened) return refuse(request.id, 'not-open', `${request.op} before open`);
    if (request.op === 'sync-login' || request.op === 'sync') return this.#sync(request);
    if (
      request.op === 'choice-count' ||
      request.op === 'choice-confirm' ||
      request.op === 'choice-cancel' ||
      request.op === 'unsynced'
    ) {
      return this.#choice(request);
    }
    if (request.op === 'backups' || request.op === 'backup-export') return this.#backups(request);
    if (request.op === 'faces') return this.#faces(request.id, request.card);
    return this.#run(request.id, (engine) => this.#call(engine, request));
  }

  async #open(id: number, languages: string[]): Promise<Reply> {
    if (this.#opened) return refuse(id, 'bad-request', 'the collection is already open');
    if (this.#engine === null) {
      const ended = await this.#start(id, languages);
      if (ended !== null) return ended;
    }
    return this.#run(id, (engine) => {
      const opened = JSON.parse(engine.open()) as Opened;
      this.#opened = true;
      return opened;
    });
  }

  /** A credential operation's status word. It is served before the session's own state is read, so
   * a tab whose session ended, or never opened, still forgets the sync key (SPEC-363 R14); a store
   * that fails answers `storage-refused`. */
  async #credential(id: number, op: 'credential-status' | 'credential-forget'): Promise<Reply> {
    const store = this.#deps.credential;
    if (store === undefined) return { id, ok: true, value: 'absent' };
    try {
      return { id, ok: true, value: op === 'credential-status' ? await store.status() : await store.forget() };
    } catch (error) {
      return refuse(id, 'storage-refused', describe(error));
    }
  }

  /** The lock, then the storage, then the module, its pool and its engine, in `languages`. */
  async #start(id: number, languages: string[]): Promise<Reply | null> {
    const lock = await this.#deps.lock(LOCK);
    if (lock === 'busy') return this.#end(id, 'collection-busy', 'another tab holds the collection');
    if (lock === 'unsupported') {
      return this.#end(id, 'storage-refused', 'this browser has no Web Locks API');
    }
    const refused = await this.#deps.storage();
    if (refused !== null) return this.#end(id, 'storage-refused', refused);
    let engine: EngineModule;
    try {
      engine = await this.#deps.load();
    } catch (error) {
      return this.#end(id, 'engine-failed', `the engine did not load: ${describe(error)}`);
    }
    try {
      await engine.install_storage();
    } catch (error) {
      return this.#end(id, 'storage-refused', describe(error));
    }
    try {
      engine.init(languages);
    } catch (error) {
      return this.#end(id, 'engine-failed', this.#explain(engine, error));
    }
    this.#engine = engine;
    return null;
  }

  /** Ends the session: every request from then on answers `code` and `why`. */
  #end(id: number, code: ErrorCode, why: string): Reply {
    this.#ended = { code, why };
    return refuse(id, code, why);
  }

  /** A trap's message is the panic the module recorded, when it recorded one. */
  #explain(engine: EngineModule, error: unknown): string {
    return (error instanceof WebAssembly.RuntimeError && engine.last_panic()) || describe(error);
  }

  /** A sync operation's answer (SPEC-364 R17, R18). It runs on the session's queue like any other
   * request, so a study request waits for it and is then answered (ADR-375 D18). A Worker with no
   * sync answers as a store with no key; a trap ends the session, and any other throw answers
   * `engine-failed`. */
  async #sync(request: Extract<Request, { op: 'sync-login' | 'sync' }>): Promise<Reply> {
    const sync = this.#deps.sync;
    if (sync === undefined) {
      const absent: Synced = { status: 'absent', required: null };
      return { id: request.id, ok: true, value: request.op === 'sync' ? absent : absent.status };
    }
    try {
      if (request.op === 'sync-login') return { id: request.id, ok: true, value: await sync.login(request.user, request.password) };
      const value = await sync.sync();
      this.#deps.choice?.heard(value.required);
      return { id: request.id, ok: true, value };
    } catch (error) {
      return this.#failed(request.id, error);
    }
  }

  /** A choice operation's answer (SPEC-377 R6), on the session's queue like a sync. A Worker with no
   * choice refuses each by name; a trap ends the session, and any other throw answers
   * `engine-failed`. */
  async #choice(request: Extract<Request, { op: 'choice-count' | 'choice-confirm' | 'choice-cancel' | 'unsynced' }>): Promise<Reply> {
    const choice = this.#deps.choice;
    if (choice === undefined) return refuse(request.id, 'engine-failed', `this Worker has no full-sync choice for ${request.op}`);
    try {
      switch (request.op) {
        case 'choice-count':
          return { id: request.id, ok: true, value: await choice.count() };
        case 'choice-confirm':
          return { id: request.id, ok: true, value: await choice.confirm(request.direction) };
        case 'choice-cancel':
          await choice.cancel();
          return { id: request.id, ok: true, value: null };
        default:
          return { id: request.id, ok: true, value: await choice.unsynced() };
      }
    } catch (error) {
      return this.#failed(request.id, error);
    }
  }

  /** A backup operation's answer (SPEC-377 R15, R16), on the session's queue like a choice: the
   * list after retention, or one listed backup's bytes. A Worker with no backups refuses each by
   * name; a trap ends the session, and any other throw answers `engine-failed`. */
  async #backups(request: Extract<Request, { op: 'backups' | 'backup-export' }>): Promise<Reply> {
    const backups = this.#deps.backups;
    if (backups === undefined) return refuse(request.id, 'engine-failed', `this Worker keeps no backups for ${request.op}`);
    try {
      if (request.op === 'backups') return { id: request.id, ok: true, value: await backups.list() };
      return { id: request.id, ok: true, value: await backups.export(request.backup) };
    } catch (error) {
      return this.#failed(request.id, error);
    }
  }

  /** A sync or choice operation that threw: a trap ends the session, and any other throw answers
   * `engine-failed`. */
  #failed(id: number, error: unknown): Reply {
    const why = this.#explain(this.#engine as EngineModule, error);
    if (error instanceof WebAssembly.RuntimeError) return this.#end(id, 'engine-failed', why);
    return refuse(id, 'engine-failed', why);
  }

  /** Both faces of `card`. The core reads media synchronously and the media directory does not, so
   * the first ask carries no file and names the files the core wants; when it wants any, the
   * Worker reads them and asks again, and that answer is the reply. Both asks and the read run
   * inside this one request, so no other request comes between them (SPEC-350 R14, ADR-361 D12). */
  async #faces(id: number, card: bigint): Promise<Reply> {
    const first = this.#run(id, (engine) => engine.faces(card, [], []));
    const wanted = first.ok ? (first.value as Faces).wanted : [];
    if (wanted.length === 0) return first;
    const files = (await this.#deps.media?.(wanted)) ?? [];
    return this.#run(id, (engine) =>
      engine.faces(
        card,
        files.map((file) => file.name),
        files.map((file) => file.bytes)
      )
    );
  }

  /** Runs an engine call. An error the engine returns leaves the session as it was; a trap spends
   * the module, so the session ends. The module's refusal of a card it did not show answers
   * `not-shown` (SPEC-350 R2), and its refusal of an undo `undo-synced` or `not-undoable`
   * (SPEC-371 R12). */
  #run(id: number, call: (engine: EngineModule) => unknown): Reply {
    const engine = this.#engine as EngineModule;
    try {
      return { id, ok: true, value: call(engine) };
    } catch (error) {
      const why = this.#explain(engine, error);
      if (error instanceof WebAssembly.RuntimeError) return this.#end(id, 'engine-failed', why);
      return refuse(id, refusalCode(why), why);
    }
  }

  #call(engine: EngineModule, request: Request): unknown {
    switch (request.op) {
      case 'seed':
        return engine.seed(request.count);
      case 'next':
        return engine.next_card() ?? null;
      case 'snapshot':
        return toSnapshot(engine.snapshot(request.card));
      case 'undo':
        engine.undo(request.card, request.step);
        return null;
      case 'undo-offer':
        return toOffer(engine.undo_offer());
      case 'memory':
        return engine.memory_pages() * PAGE_BYTES;
      case 'decks':
        return (JSON.parse(engine.deck_tree()) as DeckJson[]).map(toDeck);
      case 'study':
        engine.set_current_deck(request.deck);
        return null;
      case 'card':
        return toHead(engine.current_card());
      case 'rate':
        engine.rate(request.card, request.rating, request.ms);
        return null;
      case 'bury':
        engine.bury(request.card);
        return null;
      case 'flag':
        return engine.flag(request.card);
      default:
        engine.close();
        this.#opened = false;
        return null;
    }
  }
}

function refuse(id: number | null, code: ErrorCode, message: string): Reply {
  return { id, ok: false, code, message };
}
