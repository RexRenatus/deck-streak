import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

// SPEC-350 R13, A20; ADR-361. The study suite (tests-study/study.spec.ts) runs in Playwright, which
// the tdd probe does not resolve, so this test proves its coverage instead, as
// planted-coverage.test.ts does for the card suite: the suite names a test for each step of the
// review loop, each test in a persistent profile of its own, seeded through the built Worker's own
// `seed` operation; it audits both study screens; it runs in Chromium and WebKit over the staged
// build; and `test:study` runs it.
const APP = fileURLToPath(new URL('../../..', import.meta.url));
const SUITE = join(APP, 'tests-study');
const SPEC = join(SUITE, 'study.spec.ts');
const SEED = join(SUITE, 'seed.ts');
const CONFIG = join(APP, 'playwright.study.config.ts');
const SERVER = join(APP, 'vite.study.config.ts');

/** The review loop's steps (R13), each a test the suite names. */
const STEPS = [
  'the deck list shows the seeded deck and opens the review',
  'show answer reveals the answer and the buttons show the intervals',
  'a rating by key and one by button each show the next card',
  'undo returns the rated card',
  'bury and flag act on the shown card',
  'a key after a tap on the card still rates',
  'axe passes on both screens'
];

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** The title of each test a spec declares. */
function titles(text: string): string[] {
  return [...text.matchAll(/\btest\(\s*'([^']+)'/g)].map((match) => match[1]);
}

function read(path: string): string {
  expect(existsSync(path), `${path.slice(APP.length)} does not exist`).toBe(true);
  return readFileSync(path, 'utf8');
}

describe('the study suite', () => {
  it('the study suite covers the review loop in both engines', () => {
    // the suite names a test for each step of the review loop, and nothing else
    const text = read(SPEC);
    expect(titles(text).sort()).toEqual([...examined('steps of the review loop', STEPS)].sort());
    const planted = "test('undo returns the rated card', async ({ playwright }) => {";
    expect(titles(planted)).toEqual(['undo returns the rated card']);

    // each test opens a persistent profile of its own and seeds it through the built Worker's own
    // `seed` operation, the Worker found in the build's manifest
    expect(text).toMatch(/\.launchPersistentContext\(\s*''/);
    expect(text).toMatch(/import\s*\{[^}]*\bseed\b[^}]*\}\s*from\s*'\.\/seed'/);
    const seed = read(SEED);
    expect(seed).toMatch(/\.vite\/manifest\.json/);
    expect(seed).toMatch(/new Worker\(/);
    expect(seed).toMatch(/op:\s*'seed'/);

    // axe audits both study screens
    expect(text).toMatch(/import\s+AxeBuilder\s+from\s+'@axe-core\/playwright'/);
    const audit = text.slice(text.indexOf("test('axe passes on both screens'"));
    expect(audit.match(/new AxeBuilder\(/g) ?? []).toHaveLength(2);
    expect(audit).toMatch(/toHaveURL\(\/\\\/study\\\/review\$\/\)/);

    // Chromium and WebKit, one worker, over the staged build on the study port
    const config = read(CONFIG);
    expect(config).toMatch(/testDir:\s*'tests-study'/);
    expect([...config.matchAll(/devices\['(Desktop \w+)'\]/g)].map((match) => match[1])).toEqual([
      'Desktop Chrome',
      'Desktop Safari'
    ]);
    expect(config).toMatch(/workers:\s*1,/);
    expect(config).toMatch(/process\.env\.STUDY_PORT\s*\?\?\s*4176/);
    expect(config).toMatch(/command:\s*'vite preview --config vite\.study\.config\.ts'/);
    const server = read(SERVER);
    expect(server).toMatch(/outDir:\s*'build'/);
    expect(server).toMatch(/appType:\s*'spa'/);
    expect(server).toMatch(/strictPort:\s*true/);

    // and `test:study` runs it
    const scripts = (JSON.parse(read(join(APP, 'package.json'))) as { scripts: Record<string, string> }).scripts;
    expect(scripts['test:study']).toBe('playwright test --config playwright.study.config.ts');
  });
});
