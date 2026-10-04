// The Worker's session (SPEC-338 R3 to R6, ADR-348): the protocol's operations only, the tab lock
// first, then the storage, then the engine.
import type { Reply } from './protocol';

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

export class Session {
  constructor(deps: SessionDeps) {
    void deps;
  }

  handle(data: unknown): Promise<Reply> {
    void data;
    return Promise.resolve({ id: 0, ok: true, value: null });
  }
}
