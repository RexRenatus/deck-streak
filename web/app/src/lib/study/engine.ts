// SPEC-350 R5; ADR-361. The app's engine: the one place the app starts the engine's Worker. It
// starts on the first study screen's first call, never at import; it opens the collection in the
// app's languages; and when the page is hidden it closes the collection, then ends the Worker, so
// the next call starts again. A refused open ends its Worker too, so a retry starts afresh.
import { EngineClient, type EnginePort } from '$lib/engine/client';
import { requestPersistence } from '$lib/engine/persistence';
import { engineLanguages } from './locale';

/** The Worker as the study engine sees it: a port it can also end. */
export interface WorkerLike extends EnginePort {
  terminate(): void;
}

/** What the engine hears from the page: that it is hidden. */
export interface PageLike {
  addEventListener(type: 'pagehide', listener: () => void): void;
}

/** A started engine: its Worker, its client, and the open the client sent first. */
interface Started {
  worker: WorkerLike;
  client: EngineClient;
  opened: Promise<unknown>;
}

export class StudyEngine {
  readonly #make: () => WorkerLike;
  readonly #origin: string;
  readonly #languages: () => string[];
  readonly #persist: () => Promise<unknown>;
  #started: Started | null = null;
  #closed: Promise<void> = Promise.resolve();

  /** `make` starts the Worker; `origin` is the page's own, the only one whose replies are heard;
   * `persist` asks the browser to keep the origin's storage (SPEC-364 R19). */
  constructor(
    make: () => WorkerLike,
    page: PageLike,
    origin: string,
    languages: () => string[],
    persist: () => Promise<unknown> = async () => undefined
  ) {
    this.#make = make;
    this.#origin = origin;
    this.#languages = languages;
    this.#persist = persist;
    page.addEventListener('pagehide', () => this.#close());
  }

  /** The engine's client, its Worker started and the collection opened on the first call. */
  async client(): Promise<EngineClient> {
    if (this.#started === null) {
      const worker = this.#make();
      // every session start asks, whatever the browser answered before (SPEC-364 R19); an ask
      // that fails changes nothing the session needs
      void this.#persist().catch(() => undefined);
      const client = new EngineClient(worker, this.#origin);
      const opened = client.open(this.#languages()).catch((error: unknown) => {
        if (this.#started?.client === client) this.#started = null;
        worker.terminate();
        throw error;
      });
      this.#started = { worker, client, opened };
    }
    const { client, opened } = this.#started;
    await opened;
    return client;
  }

  /** Settles when the last close has closed the collection and ended its Worker. */
  closed(): Promise<void> {
    return this.#closed;
  }

  #close(): void {
    const started = this.#started;
    if (started === null) return;
    this.#started = null;
    const end = () => started.worker.terminate();
    this.#closed = started.client.close().then(end, end);
  }
}

/** The engine's module Worker, which the build bundles from this one expression. */
function browserWorker(): WorkerLike {
  return new Worker(new URL('../engine/worker.ts', import.meta.url), { type: 'module' });
}

let shared: StudyEngine | null = null;

/** The app's one engine, made on the first study screen and kept for the page's life. */
export function studyEngine(): StudyEngine {
  shared ??= new StudyEngine(browserWorker, window, location.origin, () => engineLanguages(), requestPersistence);
  return shared;
}
