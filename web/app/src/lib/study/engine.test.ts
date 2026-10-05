import { readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { StudyEngine, type WorkerLike } from './engine';

// SPEC-350 R5, A10; ADR-361. The app starts the engine's Worker in one place, `engine.ts`: on the
// first study screen's first call, never at import, opening the collection in the app's languages,
// and closing it when the page is hidden. The census reads text, so a comment counts.
const APP = fileURLToPath(new URL('../../..', import.meta.url));
const SRC = join(APP, 'src');
const ORIGIN = 'https://app.example';

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
  #listeners: ((event: MessageEvent) => void)[] = [];

  postMessage(message: unknown) {
    this.sent.push(message);
    const { id } = message as { id: number };
    queueMicrotask(() => {
      for (const listener of this.#listeners) {
        listener(new MessageEvent('message', { data: { id, ok: true, value: null } }));
      }
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
});
