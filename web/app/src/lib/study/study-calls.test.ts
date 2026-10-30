import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

// SPEC-350 R2, A17; ADR-361. The study screens call only the study operations: the engine harness's
// `answer`, `next`, `seed` and `snapshot`, which SPEC-338 keeps for its own suite, are called
// nowhere under the study modules or the study routes. The census reads text, so a comment counts,
// and reading a field such as a card view's `answer` is not a call.
const APP = fileURLToPath(new URL('../../..', import.meta.url));
const SCREENS = ['src/lib/study', 'src/routes/study'];

/** A call of one of the harness's operations, by the name the census reports it under. */
const HARNESS = /\.(answer|next|seed|snapshot)\s*\(/g;

/** The harness calls one text holds, by name, with how many of each. */
function callsIn(text: string): Record<string, number> {
  const found: Record<string, number> = {};
  for (const [, name] of text.matchAll(HARNESS)) found[name] = (found[name] ?? 0) + 1;
  return found;
}

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** Every shipped source file under `dir`, relative to the app, never a test; a directory the tree
 * does not hold contributes none. */
function sources(dir: string): string[] {
  const path = join(APP, dir);
  if (!existsSync(path)) return [];
  return readdirSync(path, { withFileTypes: true, recursive: true })
    .filter((entry) => entry.isFile() && /\.(?:ts|js|svelte)$/.test(entry.name))
    .filter((entry) => !/\.(?:test|spec)\./.test(entry.name))
    .map((entry) => relative(APP, join(entry.parentPath, entry.name)).split('\\').join('/'))
    .sort();
}

describe('the study screens call only the study operations', () => {
  it('the study screens call only the study operations', () => {
    const files = SCREENS.flatMap(sources);
    const census: Record<string, Record<string, number>> = {};
    for (const file of files) {
      const found = callsIn(readFileSync(join(APP, file), 'utf8'));
      if (Object.keys(found).length > 0) census[file] = found;
    }

    // no study screen calls the harness
    expect(census).toEqual({});
    examined('shipped source files under the study modules and routes', files);
    expect(files).toContain('src/lib/study/ReviewScreen.svelte');
    // ruling 317 item 6: both study route pages are read too
    expect(files).toEqual(
      expect.arrayContaining(['src/routes/study/+page.svelte', 'src/routes/study/review/+page.svelte'])
    );

    // and the matcher refuses each harness call by name, where reading a field is no call
    const planted: Record<string, string> = {
      answer: 'await engine.answer(rating, ms);',
      next: 'const card = await client.next ();',
      seed: 'void client.seed(10);',
      snapshot: 'client\n  .snapshot(card);'
    };
    for (const [name, text] of examined('planted harness calls', Object.entries(planted))) {
      expect(callsIn(text), text).toEqual({ [name]: 1 });
    }
    expect(callsIn('const html = view.answer; const next = queue.next; rate(card, answer(3));')).toEqual({});
  });
});
