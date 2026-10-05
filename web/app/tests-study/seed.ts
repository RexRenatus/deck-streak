// The study suite's seed (SPEC-350 R13; ADR-361): a collection made through the shipped Worker's own
// `seed` operation, never a route, flag or query the app ships for the suite. The Worker is the one
// the app's build bundled, found in the build's manifest; a page of the app starts it, opens the
// collection, seeds it, closes it and ends it, then waits until the collection's Web Lock is free,
// so the app's own Worker opens what the seed wrote.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';

/** The manifest `vite build` writes for the app's client, which names each asset it emitted. */
const MANIFEST = fileURLToPath(new URL('../.svelte-kit/output/client/.vite/manifest.json', import.meta.url));

/** Where the build puts a Worker's bundle. */
const WORKERS = '_app/immutable/workers/';

/** One entry of the manifest: its chunk, and the assets it emitted beside it. */
interface Chunk {
  file?: string;
  assets?: string[];
}

/** The path of the build's one Worker, from the site's root. A build with none, or more than one,
 * is refused: the suite would seed through a Worker the app does not start. */
export function workerAsset(): string {
  const chunks = Object.values(JSON.parse(readFileSync(MANIFEST, 'utf8')) as Record<string, Chunk>);
  const named = chunks.flatMap((chunk) => [chunk.file ?? '', ...(chunk.assets ?? [])]);
  const workers = [...new Set(named.filter((path) => path.startsWith(WORKERS)))];
  if (workers.length !== 1) {
    throw new Error(`the build's manifest names ${workers.length} Workers, not one: ${workers.join(', ')}`);
  }
  return `/${workers[0]}`;
}

/** Seeds `count` notes into the empty collection of `page`'s origin through the app's own Worker,
 * and returns the number the Worker added. `page` must already show a page of the app. */
export async function seed(page: Page, count: number): Promise<number> {
  return page.evaluate(
    async ({ url, count }) => {
      const worker = new Worker(url, { type: 'module' });
      let next = 0;
      const ask = (body: Record<string, unknown>) =>
        new Promise<unknown>((resolve, reject) => {
          const id = (next += 1);
          const heard = (event: MessageEvent) => {
            // a dedicated Worker's channel delivers an empty origin; any other than the page's own is not heard
            if (event.origin !== '' && event.origin !== location.origin) return;
            const reply = event.data as { id: number; ok: boolean; value?: unknown; code?: string; message?: string };
            if (reply.id !== id) return;
            worker.removeEventListener('message', heard);
            if (reply.ok) resolve(reply.value);
            else reject(new Error(`${String(body.op)} refused: ${reply.code}: ${reply.message}`));
          };
          worker.addEventListener('message', heard);
          worker.postMessage({ id, ...body });
        });
      let added: number;
      try {
        await ask({ op: 'open' });
        added = (await ask({ op: 'seed', count })) as number;
        await ask({ op: 'close' });
      } finally {
        worker.terminate();
      }
      // the browser lets the lock go when it ends the Worker, not before the next page loads
      for (let attempt = 0; attempt < 200; attempt += 1) {
        const { held = [] } = await navigator.locks.query();
        if (!held.some((lock) => lock.name === 'deck-streak-collection')) return added;
        await new Promise((resolve) => setTimeout(resolve, 50));
      }
      throw new Error('the collection lock was never released');
    },
    { url: workerAsset(), count }
  );
}
