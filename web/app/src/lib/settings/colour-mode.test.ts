import { readFileSync } from 'node:fs';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { compileTokens } from '../../../scripts/build-tokens';
import {
  applyColourMode,
  applyStoredColourMode,
  COLOUR_MODE_KEY,
  deviceStorage,
  storeColourMode,
  type ModeRoot
} from './colour-mode';

// SPEC-593 A7, R8; ADR-616 D1. Until the app mounts, the stylesheet alone paints the system mode: a
// rule in layout.css gives every colour role the dark fallback the compile emits under .dark, unless
// the browser reports light or the root element carries the light class. After mount the root
// layout applies the stored choice as a class on the root element. These tests read layout.css, the
// compiled stylesheet and the root layout as they ship, and call the colour-mode module itself.
const LAYOUT_CSS = readFileSync(new URL('../../routes/layout.css', import.meta.url), 'utf8');
const ROOT_LAYOUT = readFileSync(new URL('../../routes/+layout.svelte', import.meta.url), 'utf8');
const COMPILED = compileTokens(JSON.parse(readFileSync(new URL('../design/tokens.json', import.meta.url), 'utf8')));
const SYSTEM_RULE = /@media\s+not\s+all\s+and\s+\(prefers-color-scheme:\s*light\)\s*\{/;

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** The custom properties a declaration block sets. */
function declarations(block: string): Map<string, string> {
  const found = new Map<string, string>();
  for (const [, name, value] of block.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) found.set(name, value.trim());
  return found;
}

/** The custom properties the compiled stylesheet's `.dark` blocks declare. */
function compiledDark(css: string): Map<string, string> {
  const found = new Map<string, string>();
  for (const [, block] of css.matchAll(/(?:^|\n)\.dark\s*\{([^}]*)\}/g)) {
    for (const [name, value] of declarations(block)) found.set(name, value);
  }
  return found;
}

/** The body of the brace block whose `{` sits at `open`. */
function blockAt(text: string, open: number): string {
  let depth = 0;
  for (let at = open; at < text.length; at += 1) {
    if (text[at] === '{') depth += 1;
    if (text[at] === '}') {
      depth -= 1;
      if (depth === 0) return text.slice(open + 1, at);
    }
  }
  return '';
}

/** Each way the stylesheet's system rule differs from the compile's `.dark` block, by property. */
function systemRuleFindings(stylesheet: string, compiled: string): string[] {
  const want = compiledDark(compiled);
  if (want.size === 0) return ['the compile emits no .dark block'];
  const media = SYSTEM_RULE.exec(stylesheet);
  if (media === null) return ['layout.css holds no rule for a system that does not report light'];
  const rule = /:root:not\(\.light\)\s*\{([^}]*)\}/.exec(blockAt(stylesheet, media.index + media[0].length - 1));
  if (rule === null) return ['the system rule has no :root:not(.light) block'];
  const got = declarations(rule[1]);
  const findings: string[] = [];
  for (const [name, value] of want) {
    if (!got.has(name)) findings.push(`${name} is missing from the system rule`);
    else if (got.get(name) !== value) findings.push(`${name} is ${got.get(name)}, not ${value}`);
  }
  for (const name of got.keys()) {
    if (!want.has(name)) findings.push(`${name} is no role the compile emits under .dark`);
  }
  return findings;
}

/** A root element that keeps its classes in a set. */
function fakeRoot(...classes: string[]): ModeRoot & { classes(): string[] } {
  const held = new Set(classes);
  return {
    classList: {
      add: (...names: string[]) => names.forEach((name) => held.add(name)),
      remove: (...names: string[]) => names.forEach((name) => held.delete(name))
    },
    classes: () => [...held].sort()
  };
}

// What each stored value leaves on a root element that carried both classes and one of the page's.
const STORED: readonly (readonly [string | null, readonly string[]])[] = [
  ['light', ['light']],
  ['dark', ['dark']],
  ['system', []],
  [null, []],
  ['sepia', []]
];

/** Each stored value an apply leaves wrong on the root element, as `stored <value>: <what>`. */
function applyFindings(apply: (root: ModeRoot, storage: Pick<Storage, 'getItem'>) => unknown): string[] {
  return examined('stored values', [...STORED]).flatMap(([stored, want]) => {
    const root = fakeRoot('dark', 'light', 'page');
    apply(root, { getItem: (key: string) => (key === COLOUR_MODE_KEY ? stored : null) });
    const [got, expected] = [root.classes(), [...want, 'page'].sort()];
    return got.join(' ') === expected.join(' ')
      ? []
      : [`stored ${stored}: the root carries ${got.join(' ')}, not ${expected.join(' ')}`];
  });
}

describe('the web colour mode', () => {
  it('the stylesheet paints the system mode first, dark unless the browser reports light', () => {
    examined('colour roles the compile emits under .dark', [...compiledDark(COMPILED).keys()]);
    expect(systemRuleFindings(LAYOUT_CSS, COMPILED)).toEqual([]);
  });

  it('a planted system rule that misses a colour role is refused', () => {
    const planted = LAYOUT_CSS.replace(/^[ \t]*--border\s*:[^;]*;[ \t]*\n/m, '');
    expect(planted, 'the plant removed the border role from the system rule').not.toBe(LAYOUT_CSS);
    expect(systemRuleFindings(planted, COMPILED)).toEqual(['--border is missing from the system rule']);
    expect(systemRuleFindings(LAYOUT_CSS, COMPILED), 'the control').toEqual([]);
  });

  it('the stored choice is applied to the root element after mount', () => {
    expect(applyFindings(applyStoredColourMode)).toEqual([]);

    // storage the browser refuses, or none at all, reads as the system choice
    const refused = fakeRoot('dark', 'light', 'page');
    const denied = {
      getItem: (): string | null => {
        throw new Error('storage is denied');
      }
    };
    expect(applyStoredColourMode(refused, denied)).toBe('system');
    expect(refused.classes()).toEqual(['page']);
    const bare = fakeRoot('dark', 'page');
    expect(applyStoredColourMode(bare, undefined)).toBe('system');
    expect(bare.classes()).toEqual(['page']);

    // the root layout calls it once it has mounted, on the document's root element
    expect(ROOT_LAYOUT).toMatch(/import\s*\{[^}]*\bapplyStoredColourMode\b[^}]*\}\s*from\s*'#lib\/settings\/colour-mode\.js'/);
    expect(ROOT_LAYOUT).toMatch(/onMount\(\s*\(\)\s*=>[\s\S]*?applyStoredColourMode\(\s*document\.documentElement\b/);
  });

  it('a planted apply that ignores the stored choice is refused', () => {
    const ignoring = (root: ModeRoot) => applyColourMode(root, 'system');
    expect(applyFindings(ignoring).map((finding) => finding.replace(/:.*$/, ''))).toEqual([
      'stored light',
      'stored dark'
    ]);
    expect(applyFindings(applyStoredColourMode), 'the control').toEqual([]);
  });
});

// SPEC-593 R8; ADR-616 D1. Mutation coverage: how the choice reaches the device's storage and back.
// The key is spelled out here, never read from the module, so a changed key fails these tests.
describe('the web colour mode on the device', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('the choice is kept under the key deckstreak.colour-mode', () => {
    expect(COLOUR_MODE_KEY).toBe('deckstreak.colour-mode');
  });

  it("the device storage is the browser's local storage, or none where the browser has none", () => {
    const kept = { getItem: (): string | null => null, setItem: (): void => undefined };
    vi.stubGlobal('localStorage', kept);
    expect(deviceStorage()).toBe(kept);
    vi.stubGlobal('localStorage', undefined);
    expect(deviceStorage()).toBeUndefined();
  });

  it('storage the browser refuses to open reads as none', () => {
    let opened = 0;
    vi.stubGlobal('localStorage', undefined);
    Object.defineProperty(globalThis, 'localStorage', {
      configurable: true,
      get: () => {
        opened += 1;
        throw new Error('storage is denied');
      }
    });
    expect(deviceStorage()).toBeUndefined();
    expect(opened).toBe(1);
  });

  it('a chosen mode is written under its key and applied to the root element', () => {
    const writes: string[][] = [];
    const root = fakeRoot('light', 'page');
    storeColourMode({ setItem: (key: string, value: string) => void writes.push([key, value]) }, root, 'dark');
    expect(writes).toEqual([['deckstreak.colour-mode', 'dark']]);
    expect(root.classes()).toEqual(['dark', 'page']);
  });

  it('a chosen mode that storage refuses, or that no storage keeps, still applies to the page', () => {
    let tried = 0;
    const refusing = {
      setItem: (): void => {
        tried += 1;
        throw new Error('the storage quota is full');
      }
    };
    const refused = fakeRoot('dark', 'page');
    expect(() => storeColourMode(refusing, refused, 'light')).not.toThrow();
    expect([tried, refused.classes()]).toEqual([1, ['light', 'page']]);
    const bare = fakeRoot('dark', 'light', 'page');
    expect(() => storeColourMode(undefined, bare, 'system')).not.toThrow();
    expect(bare.classes()).toEqual(['page']);
  });
});
