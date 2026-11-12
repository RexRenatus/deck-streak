import { readdirSync, readFileSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { THEMES } from '../../tests/telegram-palettes';
import { ROUTES } from './routes';

// SPEC-028 R10, A4. The rendered audit (tests/a11y.spec.ts) runs axe-core over every route in both
// of Telegram's colour schemes. Playwright's spec is not something the tdd probe resolves, so this
// test proves its coverage instead: the route table is every screen under src/routes, and the
// audit takes its routes from that table and its palettes from the shared palette module.
const APP = fileURLToPath(new URL('../..', import.meta.url));
const SCREENS = join(APP, 'src', 'routes');
const AUDIT = join(APP, 'tests', 'a11y.spec.ts');

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** The path each `+page.svelte` under `dir` answers: its directory, with route groups dropped. */
function screens(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    if (entry.isDirectory()) return screens(join(dir, entry.name));
    if (entry.name !== '+page.svelte') return [];
    const segments = relative(SCREENS, dir)
      .split(sep)
      .filter((segment) => segment !== '' && !/^\(.+\)$/.test(segment));
    // A parameterised route has no one URL to audit: its SPEC names the sample the audit visits.
    expect(segments.filter((segment) => segment.startsWith('['))).toEqual([]);
    return [`/${segments.join('/')}`];
  });
}

describe('the rendered accessibility audit', () => {
  it('the accessibility audit covers every route in both colour schemes', () => {
    const found = examined('screens under src/routes', screens(SCREENS)).sort();

    // the route table is every screen, and nothing else
    expect([...ROUTES].sort()).toEqual(found);

    // the audit visits the route table's paths, in each palette of the palette module, and keeps no
    // list of its own
    const audit = readFileSync(AUDIT, 'utf8');
    expect(audit).toMatch(/import\s*\{\s*ROUTES\s*\}\s*from\s*'\.\.\/src\/lib\/routes'/);
    expect(audit).toMatch(/import\s*\{\s*THEMES\s*\}\s*from\s*'\.\/telegram-palettes'/);
    expect(audit).toMatch(/for\s*\(\s*const\s+\[\s*scheme\s*,\s*themeParams\s*\]\s+of\s+Object\.entries\(\s*THEMES\s*\)\s*\)/);
    expect(audit).toMatch(/for\s*\(\s*const\s+route\s+of\s+ROUTES\s*\)/);
    expect(audit).toMatch(/page\.goto\(\s*route\s*\)/);
    expect(audit.match(/page\.goto\(/g) ?? []).toHaveLength(1);

    // both of Telegram's schemes, each a complete palette
    expect(Object.keys(THEMES).sort()).toEqual(['dark', 'light']);
    expect(Object.keys(THEMES.dark).sort()).toEqual(Object.keys(THEMES.light).sort());
  });
});
