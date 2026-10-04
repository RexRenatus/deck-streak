// The page's side of the web engine: EngineClient, which numbers each request and settles it by
// the reply that carries its id (SPEC-338 R3, ADR-348).
import type { Body, ErrorCode, Opened, Rating, Reply, Snapshot } from './protocol';

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

  constructor(port: EnginePort) {
    this.#port = port;
    port.addEventListener('message', (event) => this.#settle(event.data));
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

  /** The module's linear memory in bytes, its high-water so far. */
  memory(): Promise<number> {
    return this.#send({ op: 'memory' }) as Promise<number>;
  }

  close(): Promise<null> {
    return this.#send({ op: 'close' }) as Promise<null>;
  }
}
