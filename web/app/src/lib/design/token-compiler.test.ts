import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it, vi } from 'vitest';
import { compileTokens, writeTokens } from '../../../scripts/build-tokens';

// SPEC-028 R5; ADR-028; SPEC-636 R1. The token compiler's own contract, beside the token file's design
// rules in tokens.test.ts: each refusal by its exact message, a small planted document's whole
// stylesheet, every Telegram theme parameter and every CSS generic family by name, and the one read
// and one write of writeTokens. The file system calls are stand-ins here, so no test writes the
// stylesheet; reads pass through to the real files.
vi.mock('node:fs', async (importOriginal) => {
  const actual = await importOriginal<typeof import('node:fs')>();
  return { ...actual, readFileSync: vi.fn(actual.readFileSync), writeFileSync: vi.fn() };
});

const SOURCE = fileURLToPath(new URL('./tokens.json', import.meta.url));
const OUTPUT = fileURLToPath(new URL('./tokens.css', import.meta.url));
const TOKENS: unknown = JSON.parse(readFileSync(SOURCE, 'utf8'));

// Telegram's theme parameters (core.telegram.org/bots/webapps, ThemeParams), spelt here.
const THEME_PARAMETERS = [
  'bg_color',
  'text_color',
  'hint_color',
  'link_color',
  'button_color',
  'button_text_color',
  'secondary_bg_color',
  'header_bg_color',
  'bottom_bar_bg_color',
  'accent_text_color',
  'section_bg_color',
  'section_header_text_color',
  'section_separator_color',
  'subtitle_text_color',
  'destructive_text_color'
];

// CSS's generic family keywords (CSS Fonts 4, generic-family), spelt here.
const GENERIC_FAMILIES = [
  'serif',
  'sans-serif',
  'monospace',
  'cursive',
  'fantasy',
  'system-ui',
  'ui-serif',
  'ui-sans-serif',
  'ui-monospace',
  'ui-rounded',
  'emoji',
  'math',
  'fangsong'
];

type Tree = Record<string, unknown>;

/** A small token document, fresh on each call: two colours (one an alias), a radius and a font. */
function minimal(): Tree {
  return {
    ref: { black: { $type: 'color', $value: { colorSpace: 'srgb', components: [0, 0, 0], hex: '#000000' } } },
    color: {
      $type: 'color',
      paper: {
        $value: { colorSpace: 'srgb', components: [1, 1, 1], hex: '#ffffff' },
        $extensions: { 'deck-streak': { telegram: 'bg_color', tide: 'background' } }
      },
      ink: {
        $value: '{ref.black}',
        $extensions: { 'deck-streak': { telegram: 'section_header_text_color', tide: 'ink' } }
      }
    },
    radius: { $type: 'dimension', $value: { value: 0.5, unit: 'rem' } },
    font: { $type: 'fontFamily', sans: { $value: ["Rock 'n' Roll", 'Inter Variable', 'sans-serif'] } }
  };
}

/** The group or token at `path` in a planted document. */
function at(document: Tree, ...path: string[]): Tree {
  return path.reduce<Tree>((node, key) => node[key] as Tree, document);
}

/** The compile's message for a document it refuses, or `compiled`. */
function messageOf(document: unknown): string {
  try {
    compileTokens(document);
  } catch (error) {
    expect(error).toBeInstanceOf(Error);
    return (error as Error).message;
  }
  return 'compiled';
}

const NO_PARAMETER =
  "names no Telegram theme parameter (its deck-streak extension's telegram), so Telegram's theme cannot paint it";
const NOT_SRGB = 'tokens: color.paper is not an sRGB colour with a six-digit lowercase hex';
const NOT_MADE = "tokens: color.paper's components do not make #ffffff";
const NOT_DIMENSION = 'tokens: radius is not a dimension in rem or px';
const NOT_FAMILY = 'tokens: font.sans is not a font family';

/** A planted document: one change to the small document, and the exact message the compile owes it. */
const REFUSALS: readonly { readonly name: string; readonly plant: (document: Tree) => void; readonly message: string }[] = [
  { name: 'no colour tier', plant: (d) => delete d.color, message: 'tokens: color is not a token or a group' },
  {
    name: 'a colour whose own type is another',
    plant: (d) => (at(d, 'color', 'paper').$type = 'dimension'),
    message: 'tokens: color.paper is not a color token'
  },
  {
    name: 'a font group of another type',
    plant: (d) => (at(d, 'font').$type = 'color'),
    message: 'tokens: font.sans is not a fontFamily token'
  },
  {
    name: 'a colour with no extensions',
    plant: (d) => delete at(d, 'color', 'paper').$extensions,
    message: `tokens: color.paper ${NO_PARAMETER}`
  },
  {
    name: 'a null deck-streak extension',
    plant: (d) => (at(d, 'color', 'paper', '$extensions')['deck-streak'] = null),
    message: `tokens: color.paper ${NO_PARAMETER}`
  },
  {
    name: 'a parameter spelt with dashes',
    plant: (d) => (at(d, 'color', 'paper', '$extensions', 'deck-streak').telegram = 'bg-color'),
    message: `tokens: color.paper ${NO_PARAMETER}`
  },
  {
    name: 'a string value that is no alias',
    plant: (d) => (at(d, 'color', 'paper').$value = 'red'),
    message: 'tokens: color.paper holds red, which is no alias'
  },
  {
    name: 'an alias with text before it',
    plant: (d) => (at(d, 'color', 'paper').$value = 'x{ref.black}'),
    message: 'tokens: color.paper holds x{ref.black}, which is no alias'
  },
  {
    name: 'an alias with text after it',
    plant: (d) => (at(d, 'color', 'paper').$value = '{ref.black}x'),
    message: 'tokens: color.paper holds {ref.black}x, which is no alias'
  },
  {
    name: 'an alias of itself',
    plant: (d) => {
      at(d, 'ref').loop = { $value: '{ref.loop}' };
      at(d, 'color', 'paper').$value = '{ref.loop}';
    },
    message: 'tokens: ref.loop aliases itself through {ref.loop}'
  },
  {
    name: 'an alias through a missing group',
    plant: (d) => (at(d, 'color', 'paper').$value = '{ref.missing.black}'),
    message: 'tokens: ref.missing.black is not a token or a group'
  },
  {
    name: 'an alias through a null',
    plant: (d) => {
      at(d, 'ref').void = null;
      at(d, 'color', 'paper').$value = '{ref.void.black}';
    },
    message: 'tokens: ref.void.black is not a token or a group'
  },
  {
    name: 'a colour space other than sRGB',
    plant: (d) => (at(d, 'color', 'paper', '$value').colorSpace = 'display-p3'),
    message: NOT_SRGB
  },
  {
    name: 'a hex that is no string',
    plant: (d) => (at(d, 'color', 'paper', '$value').hex = ['#ffffff']),
    message: NOT_SRGB
  },
  { name: 'a hex with a prefix', plant: (d) => (at(d, 'color', 'paper', '$value').hex = 'x#ffffff'), message: NOT_SRGB },
  { name: 'a hex of seven digits', plant: (d) => (at(d, 'color', 'paper', '$value').hex = '#ffffff0'), message: NOT_SRGB },
  {
    name: 'components of another colour',
    plant: (d) => (at(d, 'color', 'paper', '$value').components = [0, 0, 0]),
    message: NOT_MADE
  },
  {
    name: 'components with one channel right',
    plant: (d) => (at(d, 'color', 'paper', '$value').components = [1, 0, 0]),
    message: NOT_MADE
  },
  { name: 'two components', plant: (d) => (at(d, 'color', 'paper', '$value').components = [1, 1]), message: NOT_MADE },
  { name: 'components as text', plant: (d) => (at(d, 'color', 'paper', '$value').components = 'abc'), message: NOT_MADE },
  {
    name: 'components as numerals',
    plant: (d) => (at(d, 'color', 'paper', '$value').components = ['1', '1', '1']),
    message: NOT_MADE
  },
  { name: 'no radius', plant: (d) => delete d.radius, message: 'tokens: radius is not a token or a group' },
  {
    name: 'a radius of another type',
    plant: (d) => (at(d, 'radius').$type = 'number'),
    message: 'tokens: radius is not a dimension token'
  },
  {
    name: 'a radius in em',
    plant: (d) => (at(d, 'radius').$value = { value: 2, unit: 'em' }),
    message: NOT_DIMENSION
  },
  {
    name: 'a radius whose amount is text',
    plant: (d) => (at(d, 'radius').$value = { value: '2', unit: 'px' }),
    message: NOT_DIMENSION
  },
  { name: 'no font tier', plant: (d) => delete d.font, message: 'tokens: font is not a token or a group' },
  { name: 'a font that is a number', plant: (d) => (at(d, 'font', 'sans').$value = 5), message: NOT_FAMILY },
  { name: 'a font with no family', plant: (d) => (at(d, 'font', 'sans').$value = []), message: NOT_FAMILY },
  {
    name: 'a font with a name that is no string',
    plant: (d) => (at(d, 'font', 'sans').$value = ['Inter Variable', 5]),
    message: NOT_FAMILY
  }
];

describe('the token compiler', () => {
  it('compiles a small document to the whole stylesheet, line by line', () => {
    expect(compileTokens(minimal())).toBe(
      [
        '/* Compiled from tokens.json by scripts/build-tokens.ts (ADR-028). Edit the tokens, not this file. */',
        ':root {',
        '  --paper: var(--tg-theme-bg-color, #ffffff);',
        '  --ink: var(--tg-theme-section-header-text-color, #000000);',
        '  --radius: 0.5rem;',
        '}',
        '',
        '@theme inline {',
        '  --color-paper: var(--paper);',
        '  --color-ink: var(--ink);',
        "  --font-sans: 'Rock \\'n\\' Roll', 'Inter Variable', sans-serif;",
        '}',
        ''
      ].join('\n')
    );
  });

  it('refuses each malformed document by its exact message', () => {
    expect(messageOf(null)).toBe('tokens: the document is not a token or a group');
    let reached = 0;
    for (const { name, plant, message } of REFUSALS) {
      const document = minimal();
      plant(document);
      expect(messageOf(document), name).toBe(message);
      reached += 1;
    }
    console.log(`examined ${reached} of ${REFUSALS.length} planted documents`);
    expect(reached).toBe(29);
    // the control: the unplanted document compiles
    expect(messageOf(minimal())).toBe('compiled');
  });

  it('compiles a colour naming each Telegram theme parameter', () => {
    let reached = 0;
    for (const parameter of THEME_PARAMETERS) {
      const document = minimal();
      at(document, 'color', 'paper', '$extensions', 'deck-streak').telegram = parameter;
      const variable = `--tg-theme-${parameter.replaceAll('_', '-')}`;
      expect(compileTokens(document).split('\n'), parameter).toContain(`  --paper: var(${variable}, #ffffff);`);
      reached += 1;
    }
    console.log(`examined ${reached} of ${THEME_PARAMETERS.length} theme parameters`);
    expect(reached).toBe(15);
  });

  it('writes each CSS generic family unquoted and every other family quoted', () => {
    const document = minimal();
    at(document, 'font', 'sans').$value = ['Inter Variable', ...GENERIC_FAMILIES];
    console.log(`examined ${GENERIC_FAMILIES.length} generic families`);
    expect(GENERIC_FAMILIES).toHaveLength(13);
    expect(compileTokens(document).split('\n')).toContain(
      `  --font-sans: 'Inter Variable', ${GENERIC_FAMILIES.join(', ')};`
    );
  });

  it('compiles a radius in px and a font given as one name', () => {
    const document = minimal();
    at(document, 'radius').$value = { value: 4, unit: 'px' };
    at(document, 'font', 'sans').$value = 'Inter Variable';
    const lines = compileTokens(document).split('\n');
    expect(lines).toContain('  --radius: 4px;');
    expect(lines).toContain("  --font-sans: 'Inter Variable';");
  });

  it('writeTokens reads tokens.json as text and writes its stylesheet to tokens.css', () => {
    const read = vi.mocked(readFileSync);
    const write = vi.mocked(writeFileSync);
    read.mockClear();
    write.mockClear();
    writeTokens();
    expect(read.mock.calls).toEqual([[SOURCE, 'utf8']]);
    expect(write.mock.calls).toEqual([[OUTPUT, compileTokens(TOKENS)]]);
  });
});
