// The page's side of the web engine: EngineClient, which numbers each request and settles it by
// the reply that carries its id (SPEC-338 R3, ADR-348).
import type { Body, ErrorCode, Opened, Rating, Snapshot } from './protocol';

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
  constructor(port: EnginePort) {
    void port;
  }

  /** How many requests await their reply. */
  get waiting(): number {
    return 0;
  }

  #send(body: Body): Promise<unknown> {
    void body;
    return Promise.resolve(undefined);
  }

  open(): Promise<Opened> {
    return this.#send({ op: 'open' }) as Promise<Opened>;
  }

  seed(count: number): Promise<number> {
    return this.#send({ op: 'seed', count }) as Promise<number>;
  }

  next(): Promise<bigint | null> {
    return this.#send({ op: 'next' }) as Promise<bigint | null>;
  }

  answer(rating: Rating, ms: number): Promise<bigint> {
    return this.#send({ op: 'answer', rating, ms }) as Promise<bigint>;
  }

  undo(): Promise<null> {
    return this.#send({ op: 'undo' }) as Promise<null>;
  }

  snapshot(card: bigint): Promise<Snapshot | null> {
    return this.#send({ op: 'snapshot', card }) as Promise<Snapshot | null>;
  }

  close(): Promise<null> {
    return this.#send({ op: 'close' }) as Promise<null>;
  }
}
