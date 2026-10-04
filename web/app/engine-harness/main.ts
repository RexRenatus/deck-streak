// The harness page's script (SPEC-338 R9): it starts the engine's Worker as the app will, a
// dedicated module Worker reached only through EngineClient, and hands the browser tests one
// client at a time on `window.harness`.
import { EngineClient, EngineError } from '../src/lib/engine/client';
import { requestPersistence } from '../src/lib/engine/persistence';

/** A refusal as the tests read it: the Worker's code and message. */
export interface Refusal {
  code: string;
  message: string;
}

export interface Harness {
  /** The page's Worker client; null until `start`. */
  client: EngineClient | null;
  /** Starts the engine's Worker and its client. */
  start(): EngineClient;
  /** The page's persistent-storage request (SPEC-338 R4). */
  requestPersistence: typeof requestPersistence;
  /** The client's refusal as plain data, so a test can read it across the page boundary. */
  refusal(error: unknown): Refusal;
  /** Each policy violation the page reported, by its directive and blocked URI. */
  violations: string[];
}

declare global {
  interface Window {
    harness: Harness;
  }
}

const violations: string[] = [];
document.addEventListener('securitypolicyviolation', (event) => {
  violations.push(`${event.effectiveDirective} ${event.blockedURI}`);
});

window.harness = {
  client: null,
  start() {
    const worker = new Worker(new URL('../src/lib/engine/worker.ts', import.meta.url), {
      type: 'module'
    });
    this.client = new EngineClient(worker, location.origin);
    return this.client;
  },
  requestPersistence,
  refusal(error) {
    if (error instanceof EngineError) return { code: error.code, message: error.message };
    return { code: 'not-a-refusal', message: String(error) };
  },
  violations
};
