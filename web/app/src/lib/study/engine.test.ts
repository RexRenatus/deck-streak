import { readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { EngineError } from '$lib/engine/client';
import { getLocale, overwriteGetLocale } from '$lib/paraglide/runtime.js';
import { StudyEngine, studyEngine, type WorkerLike } from './engine';

// SPEC-350 R5, A10; ADR-361. The app starts the engine's Worker in one place, `engine.ts`: on the
// first study screen's first call, never at import, opening the collection in the app's languages,
// and closing it when the page is hidden. The census reads text, so a comment counts.
const APP = fileURLToPath(new URL('../../..', import.meta.url));
const SRC = join(APP, 'src');
const ORIGIN = 'https://app.example';
const ORIGINAL_LOCALE = getLocale;

/** A Worker's construction: a dedicated or a shared one. */
const WORKER = /\bnew\s+(?:Shared)?Worker\b/g;

function workersIn(text: string): number {
  return text.match(WORKER)?.length ?? 0;
}

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** Every shipped source file under `dir`, relative to the app: never a test or the generated messages. */
function sources(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true })
    .flatMap((entry) => {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) return entry.name === 'paraglide' ? [] : sources(path);
      if (!/\.(?:ts|js|svelte)$/.test(entry.name)) return [];
      if (/\.(?:test|spec)\./.test(entry.name)) return [];
      return [relative(APP, path).split('\\').join('/')];
    })
    .sort();
}

/** A Worker that answers every request with success, and records what it was sent. */
class FakeWorker implements WorkerLike {
  sent: unknown[] = [];
  terminated = false;
  /** The operations this Worker refuses, each with its code. */
  refuses: Record<string, string> = {};
  #listeners: ((event: MessageEvent) => void)[] = [];

  postMessage(message: unknown) {
    this.sent.push(message);
    const { id, op } = message as { id: number; op: string };
    const code = this.refuses[op];
    const data =
      code === undefined ? { id, ok: true, value: null } : { id, ok: false, code, message: `${op} refused` };
    queueMicrotask(() => {
      for (const listener of this.#listeners) listener(new MessageEvent('message', { data }));
    });
  }

  addEventListener(_type: 'message', listener: (event: MessageEvent) => void) {
    this.#listeners.push(listener);
  }

  terminate() {
    this.terminated = true;
  }
}

describe("the app's engine", () => {
  it("the app starts the engine's Worker in one place", async () => {
    const files = sources(SRC);
    const census: Record<string, number> = {};
    for (const file of files) {
      const count = workersIn(readFileSync(join(APP, file), 'utf8'));
      if (count > 0) census[file] = count;
    }

    // one Worker in the whole Mini App, started by the study engine
    expect(census).toEqual({ 'src/lib/study/engine.ts': 1 });
    examined('shipped source files under src', files);

    // the matcher finds a Worker by either name, and nothing that only mentions one
    expect(workersIn("const worker = new Worker(new URL('./w.ts', import.meta.url));")).toBe(1);
    expect(workersIn('const shared = new  SharedWorker(url);')).toBe(1);
    expect(workersIn('const port: Worker = start(); type W = Worker;')).toBe(0);

    // it starts on the first call, not before, and opens in the app's languages
    const made: FakeWorker[] = [];
    const page = new EventTarget();
    const engine = new StudyEngine(
      () => {
        const worker = new FakeWorker();
        made.push(worker);
        return worker;
      },
      page,
      ORIGIN,
      () => ['ja']
    );
    expect(made).toHaveLength(0);
    const client = await engine.client();
    expect(made).toHaveLength(1);
    expect(made[0].sent).toEqual([{ id: 1, op: 'open', languages: ['ja'] }]);
    expect(await engine.client()).toBe(client);
    expect(made).toHaveLength(1);

    // the hidden page closes the collection, then the Worker; the next call starts again
    page.dispatchEvent(new Event('pagehide'));
    expect(made[0].sent).toEqual([
      { id: 1, op: 'open', languages: ['ja'] },
      { id: 2, op: 'close' }
    ]);
    await engine.closed();
    expect(made[0].terminated).toBe(true);
    await engine.client();
    expect(made).toHaveLength(2);
    expect(made[1].sent).toEqual([{ id: 1, op: 'open', languages: ['ja'] }]);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    overwriteGetLocale(ORIGINAL_LOCALE);
  });

  it("a refused open starts again, and the app's engine is the engine's module Worker", async () => {
    // a refused open ends its Worker; the next call starts a new one and opens again
    const made: FakeWorker[] = [];
    const page = new EventTarget();
    const engine = new StudyEngine(
      () => {
        const worker = new FakeWorker();
        if (made.length === 0) worker.refuses.open = 'collection-busy';
        made.push(worker);
        return worker;
      },
      page,
      ORIGIN,
      () => ['fr']
    );
    const refused = engine.client();
    await expect(refused).rejects.toBeInstanceOf(EngineError);
    await expect(refused).rejects.toMatchObject({ code: 'collection-busy', message: 'open refused' });
    expect(made[0].terminated).toBe(true);
    await engine.client();
    expect([made.length, made[1].terminated, made[1].sent]).toEqual([2, false, [{ id: 1, op: 'open', languages: ['fr'] }]]);
    // a page hidden before any call closes nothing
    const idle = new FakeWorker();
    const quiet = new EventTarget();
    new StudyEngine(() => idle, quiet, ORIGIN, () => ['fr']);
    quiet.dispatchEvent(new Event('pagehide'));
    expect(idle.sent).toEqual([]);

    // the app's one engine: a module Worker from the engine's worker module, on the page's origin,
    // opened in the app's locale, and the same engine on every call
    const built: { url: string; options: unknown }[] = [];
    vi.stubGlobal('window', new EventTarget());
    vi.stubGlobal('location', { origin: ORIGIN });
    vi.stubGlobal(
      'Worker',
      class extends FakeWorker {
        constructor(url: URL | string, options: unknown) {
          super();
          built.push({ url: String(url), options });
          made.push(this);
        }
      }
    );
    overwriteGetLocale(() => 'zh-Hant');
    const app = studyEngine();
    expect(studyEngine()).toBe(app);
    expect(built).toEqual([]);
    await app.client();
    expect(built).toHaveLength(1);
    expect(built[0].url.endsWith('/src/lib/engine/worker.ts')).toBe(true);
    expect(built[0].options).toEqual({ type: 'module' });
    expect(made[2].sent).toEqual([{ id: 1, op: 'open', languages: ['zh-TW'] }]);
  });

  // Mutation coverage: an open that is refused after the page was hidden, and after a newer start.
  it('a refused open after the page was hidden rejects with the engine\'s error and spares a newer start', async () => {
    class HeldWorker extends FakeWorker {
      #answer: ((event: MessageEvent) => void)[] = [];
      override postMessage(message: unknown) {
        this.sent.push(message);
      }
      override addEventListener(_type: 'message', listener: (event: MessageEvent) => void) {
        this.#answer.push(listener);
      }
      reply(data: unknown) {
        for (const listener of this.#answer) listener(new MessageEvent('message', { data }));
      }
    }
    const made: HeldWorker[] = [];
    const page = new EventTarget();
    const engine = new StudyEngine(
      () => {
        const worker = new HeldWorker();
        made.push(worker);
        return worker;
      },
      page,
      ORIGIN,
      () => ['en']
    );
    const first = engine.client();
    const firstRefusal = expect(first).rejects.toMatchObject({ code: 'collection-busy' });
    page.dispatchEvent(new Event('pagehide'));
    // a newer start, while the first open is still unanswered
    const second = engine.client();
    expect(made).toHaveLength(2);
    made[0].reply({ id: 1, ok: false, code: 'collection-busy', message: 'open refused' });
    await firstRefusal;
    await expect(first).rejects.toBeInstanceOf(EngineError);
    // the refusal of the older start leaves the newer one standing
    made[1].reply({ id: 1, ok: true, value: null });
    const client = await second;
    expect(await engine.client()).toBe(client);
    expect(made).toHaveLength(2);
    expect(made[1].terminated).toBe(false);
  });
});
