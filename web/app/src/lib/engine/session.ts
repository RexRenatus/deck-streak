// The Worker's session (SPEC-338 R3 to R6, ADR-348): the protocol's operations only, the tab lock
// first, then the storage, then the engine.
import { parseRequest } from './protocol';
import type { ErrorCode, Opened, Reply, Request, Snapshot } from './protocol';

/** The Web Lock that holds one collection per origin. */
export const LOCK = 'deck-streak-collection';

/** The Web Lock's answer: held for the Worker's life, held by another tab, or no Web Locks API. */
export type LockAnswer = 'held' | 'busy' | 'unsupported';

/** The module's exports, as `wasm-bindgen` writes them for `crates/web-engine/src/wasm.rs`. */
export interface EngineModule {
  install_storage(): Promise<number>;
  init(): void;
  open(): string;
  close(): void;
  seed(count: number): number;
  next_card(): bigint | undefined;
  answer(rating: number, ms: number): bigint;
  undo(): void;
  snapshot(card: bigint): string;
  last_panic(): string | undefined;
}

/** What the session needs from the Worker's browser. */
export interface SessionDeps {
  lock(name: string): Promise<LockAnswer>;
  /** Null when the origin private file system is there to use, else why it is refused. */
  storage(): Promise<string | null>;
  load(): Promise<EngineModule>;
}

/** The states a session ends in, each with the code it answers from then on. */
const ENDED = {
  busy: 'collection-busy',
  refused: 'storage-refused',
  failed: 'engine-failed'
} as const satisfies Record<string, ErrorCode>;
type State = 'idle' | 'open' | 'closed' | keyof typeof ENDED;

const describe = (error: unknown) => (error instanceof Error ? error.message : String(error));

/** The scheduling fields `snapshot` reads, in its query's column order. */
function toSnapshot(row: string): Snapshot | null {
  const fields = JSON.parse(row) as number[] | null;
  if (fields === null) return null;
  const [id, queue, type, due, interval, reps, lapses] = fields;
  return { id: BigInt(id), queue, type, due, interval, reps, lapses };
}

/** One Worker's session over one collection. It answers one request at a time, in order; it
 * takes the Web Lock before the storage and the storage before the engine, so a second tab and
 * a refused storage each answer by name with no engine loaded. */
export class Session {
  readonly #deps: SessionDeps;
  #state: State = 'idle';
  #why = '';
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
    if (this.#state in ENDED) {
      return refuse(request.id, ENDED[this.#state as keyof typeof ENDED], this.#why);
    }
    if (request.op === 'open') return this.#open(request.id);
    if (this.#state !== 'open') return refuse(request.id, 'not-open', `${request.op} before open`);
    return this.#run(request.id, (engine) => this.#call(engine, request));
  }

  async #open(id: number): Promise<Reply> {
    if (this.#state === 'open') return refuse(id, 'bad-request', 'the collection is already open');
    if (this.#engine === null) {
      const ended = await this.#start(id);
      if (ended !== null) return ended;
    }
    return this.#run(id, (engine) => {
      const opened = JSON.parse(engine.open()) as Opened;
      this.#state = 'open';
      return opened;
    });
  }

  /** The lock, then the storage, then the module, its pool and its engine. */
  async #start(id: number): Promise<Reply | null> {
    const lock = await this.#deps.lock(LOCK);
    if (lock === 'busy') return this.#end(id, 'busy', 'another tab holds the collection');
    if (lock === 'unsupported') return this.#end(id, 'refused', 'this browser has no Web Locks API');
    const refused = await this.#deps.storage();
    if (refused !== null) return this.#end(id, 'refused', refused);
    let engine: EngineModule;
    try {
      engine = await this.#deps.load();
    } catch (error) {
      return this.#end(id, 'failed', `the engine did not load: ${describe(error)}`);
    }
    try {
      await engine.install_storage();
    } catch (error) {
      return this.#end(id, 'refused', describe(error));
    }
    try {
      engine.init();
    } catch (error) {
      return this.#end(id, 'failed', this.#explain(engine, error));
    }
    this.#engine = engine;
    return null;
  }

  #end(id: number, state: keyof typeof ENDED, why: string): Reply {
    this.#state = state;
    this.#why = why;
    return refuse(id, ENDED[state], why);
  }

  /** A trap's message is the panic the module recorded, when it recorded one. */
  #explain(engine: EngineModule, error: unknown): string {
    return (error instanceof WebAssembly.RuntimeError && engine.last_panic()) || describe(error);
  }

  /** Runs an engine call. An error the engine returns leaves the session as it was; a trap spends
   * the module, so the session ends. */
  #run(id: number, call: (engine: EngineModule) => unknown): Reply {
    const engine = this.#engine as EngineModule;
    try {
      return { id, ok: true, value: call(engine) };
    } catch (error) {
      const why = this.#explain(engine, error);
      if (error instanceof WebAssembly.RuntimeError) return this.#end(id, 'failed', why);
      return refuse(id, 'engine-failed', why);
    }
  }

  #call(engine: EngineModule, request: Request): unknown {
    switch (request.op) {
      case 'seed':
        return engine.seed(request.count);
      case 'next':
        return engine.next_card() ?? null;
      case 'answer':
        return engine.answer(request.rating, request.ms);
      case 'snapshot':
        return toSnapshot(engine.snapshot(request.card));
      case 'undo':
        engine.undo();
        return null;
      default:
        engine.close();
        this.#state = 'closed';
        return null;
    }
  }
}

function refuse(id: number | null, code: ErrorCode, message: string): Reply {
  return { id, ok: false, code, message };
}
