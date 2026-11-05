import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { compileTokens } from '../../../scripts/build-tokens';
import { THEMES } from '../../../tests/telegram-palettes';

// SPEC-028 R5, A8, A9; ADR-028. The tokens compile into the CSS custom properties the page ships
// with. Inside Telegram every colour must come from the chat's theme, and every text colour must
// stay readable on each background it is drawn on, in both of Telegram's default palettes and in
// the fallbacks a plain browser shows. These tests judge the compiled CSS, the exact variables that
// ship, against the token file they read for themselves.
const TOKENS: unknown = JSON.parse(readFileSync(new URL('./tokens.json', import.meta.url), 'utf8'));

// Telegram's theme parameters (core.telegram.org/bots/webapps, ThemeParams). The client sets each
// as the CSS variable --tg-theme-<name>, its underscores written as dashes.
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

const TELEGRAM_VARIABLE = /^var\(\s*--tg-theme-([a-z]+(?:-[a-z]+)*)\s*,\s*(#[0-9a-f]{6})\s*\)$/;
const AA_NORMAL_TEXT = 4.5;

interface ColourToken {
  readonly name: string;
  readonly hex: string;
  readonly telegram: unknown;
  readonly textOn: readonly string[];
}

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

function member(node: unknown, key: string): unknown {
  return typeof node === 'object' && node !== null ? (node as Record<string, unknown>)[key] : undefined;
}

/** A colour value's hex: the DTCG colour's own `hex`, or the hex of the token an alias names. */
function hexOf(value: unknown): string {
  if (typeof value === 'string') {
    const path = /^\{(.+)\}$/.exec(value)?.[1] ?? '';
    const target = path.split('.').reduce<unknown>((node, key) => member(node, key), TOKENS);
    return hexOf(member(target, '$value'));
  }
  return String(member(value, 'hex'));
}

/** The tokens of the `color` tier, as the token file states them. */
function colourTokens(document: unknown = TOKENS): ColourToken[] {
  const group = member(document, 'color') as Record<string, unknown>;
  return Object.entries(group)
    .filter(([name]) => !name.startsWith('$'))
    .map(([name, token]) => {
      const extension = member(member(token, '$extensions'), 'deck-streak');
      return {
        name,
        hex: hexOf(member(token, '$value')),
        telegram: member(extension, 'telegram'),
        textOn: (member(extension, 'text-on') as string[] | undefined) ?? []
      };
    });
}

/** The custom properties the compiled stylesheet's `:root` blocks declare. */
function rootProperties(css: string): Map<string, string> {
  const properties = new Map<string, string>();
  for (const [, block] of css.matchAll(/:root\s*\{([^}]*)\}/g)) {
    for (const [, name, value] of block.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) {
      properties.set(name, value.trim());
    }
  }
  return properties;
}

/** The relative luminance of an sRGB colour (WCAG 2.2, "relative luminance"). */
function luminance(hex: string): number {
  const channel = (at: number) => {
    const c = parseInt(hex.slice(at, at + 2), 16) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}

/** The WCAG 2.2 contrast ratio of two colours, the lighter over the darker. */
function contrast(a: string, b: string): number {
  const [lighter, darker] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (lighter + 0.05) / (darker + 0.05);
}

/** The two Tide modes, and the three roles SPEC-578 R2 gave each of them. */
const TIDE_MODES = ['dark', 'light'] as const;
const TIDE_ROLES = ['background', 'ink', 'accent'] as const;

// SPEC-593 R1 to R3, ADR-615 D1 and D2, spelt here and never read from the token file or the
// compile: the six roles of each Tide mode and their values, the Tide role each colour role falls
// back to, and the declared pairs with their WCAG 2.2 bars (1.4.3 for text, 1.4.11 for the rest).
const SIX_ROLES = ['background', 'ink', 'accent', 'card', 'muted', 'border'] as const;
type SixRole = (typeof SIX_ROLES)[number];
const R1: Record<(typeof TIDE_MODES)[number], Record<SixRole, string>> = {
  dark: {
    background: '#0f161d',
    ink: '#f4f7fa',
    accent: '#34c6c0',
    card: '#21282f',
    muted: '#8f9499',
    border: '#74797e'
  },
  light: {
    background: '#f4f7fa',
    ink: '#0f161d',
    accent: '#0b6a67',
    card: '#ffffff',
    muted: '#5f656a',
    border: '#82868c'
  }
};
const R2_TIDE_ROLE: Record<string, SixRole> = {
  background: 'background',
  foreground: 'ink',
  'card-foreground': 'ink',
  card: 'card',
  'muted-foreground': 'muted',
  border: 'border',
  link: 'accent',
  ring: 'accent'
};
const R3_PAIRS: readonly { readonly pair: string; readonly on: SixRole; readonly of: SixRole; readonly bar: number }[] = [
  { pair: 'ink on background', of: 'ink', on: 'background', bar: 4.5 },
  { pair: 'ink on card', of: 'ink', on: 'card', bar: 4.5 },
  { pair: 'muted on background', of: 'muted', on: 'background', bar: 4.5 },
  { pair: 'muted on card', of: 'muted', on: 'card', bar: 4.5 },
  { pair: 'accent on background', of: 'accent', on: 'background', bar: 4.5 },
  { pair: 'accent on card', of: 'accent', on: 'card', bar: 4.5 },
  { pair: 'background on ink', of: 'background', on: 'ink', bar: 4.5 },
  { pair: 'border on background', of: 'border', on: 'background', bar: 3 },
  { pair: 'border on card', of: 'border', on: 'card', bar: 3 },
  { pair: 'focus ring on background', of: 'accent', on: 'background', bar: 3 },
  { pair: 'focus ring on card', of: 'accent', on: 'card', bar: 3 }
];
const SIX_HEX = /^#[0-9a-f]{6}$/;

/** The custom properties the compiled stylesheet's `.dark` blocks declare (SPEC-593 R2). */
function darkProperties(css: string): Map<string, string> {
  const properties = new Map<string, string>();
  for (const [, block] of css.matchAll(/(?:^|\n)\.dark\s*\{([^}]*)\}/g)) {
    for (const [, name, value] of block.matchAll(/(--[\w-]+)\s*:\s*([^;]+);/g)) {
      properties.set(name, value.trim());
    }
  }
  return properties;
}

/** Each declared Tide pair under its bar in either mode of `document`, as `<pair> (<mode>): <why>`. */
function tidePairFailures(document: unknown): string[] {
  return TIDE_MODES.flatMap((mode) =>
    R3_PAIRS.flatMap(({ pair, of, on, bar }) => {
      const [text, back] = [of, on].map((role) => hexOf(member(tideMode(document, mode)?.[role], '$value')));
      if (!SIX_HEX.test(text) || !SIX_HEX.test(back)) return [`${pair} (${mode}): a role holds no colour`];
      const ratio = contrast(text, back);
      return ratio >= bar ? [] : [`${pair} (${mode}): ${ratio.toFixed(2)} is under ${bar}`];
    })
  );
}

/** An Tide mode of a token document, or undefined when the document holds none. */
function tideMode(document: unknown, mode: string): Record<string, unknown> | undefined {
  return member(member(member(document, 'modes'), 'tide'), mode) as Record<string, unknown> | undefined;
}

describe('the design tokens', () => {
  it('every colour token resolves to a Telegram theme variable with a fallback', () => {
    const tokens = examined('colour tokens', colourTokens());
    const properties = rootProperties(compileTokens(TOKENS));

    for (const token of tokens) {
      const value = properties.get(`--${token.name}`) ?? '(not declared)';
      expect(value, `--${token.name}`).toMatch(TELEGRAM_VARIABLE);
      const [, variable, fallback] = TELEGRAM_VARIABLE.exec(value) ?? [];
      const parameter = variable.replaceAll('-', '_');
      expect(THEME_PARAMETERS, `--${token.name}`).toContain(parameter);
      expect([parameter, fallback], `--${token.name}`).toEqual([token.telegram, token.hex]);
    }

    // a colour that names no Telegram parameter fails the compile, so no build can ship one
    const planted = structuredClone(TOKENS) as { color: { background: Record<string, unknown> } };
    delete planted.color.background.$extensions;
    expect(() => compileTokens(planted)).toThrow(/color\.background/);
  });

  it('every text and background pair meets AA contrast in both Telegram palettes', () => {
    // the formula, on values WCAG's own documents give: black on white is 21:1, and #767676 is
    // the lightest grey that reaches 4.5:1 on white
    expect(contrast('#000000', '#ffffff')).toBeCloseTo(21, 5);
    expect(contrast('#767676', '#ffffff')).toBeCloseTo(4.54, 2);

    const properties = rootProperties(compileTokens(TOKENS));
    // The colour a declared variable paints in a scheme: the palette's value for its Telegram
    // parameter, or, outside Telegram, its fallback.
    const paint = (name: string, scheme: 'light' | 'dark' | 'outside') => {
      const value = properties.get(`--${name}`) ?? '(not declared)';
      expect(value, `--${name}`).toMatch(TELEGRAM_VARIABLE);
      const [, variable, fallback] = TELEGRAM_VARIABLE.exec(value) ?? [];
      if (scheme === 'outside') return fallback;
      const palette: Record<string, string> = THEMES[scheme];
      const parameter = variable.replaceAll('-', '_');
      expect(Object.keys(palette), `the ${scheme} palette sets no ${parameter}`).toContain(parameter);
      return palette[parameter];
    };

    const pairs = examined(
      'declared text and background pairs',
      colourTokens().flatMap((token) =>
        token.textOn.map((background) => [token.name, background.replace(/^color\./, '')] as const)
      )
    );
    const judged = pairs.flatMap(([text, background]) =>
      (['light', 'dark', 'outside'] as const).map((scheme) => {
        const ratio = contrast(paint(text, scheme), paint(background, scheme));
        return `${text} on ${background} (${scheme}): ${ratio >= AA_NORMAL_TEXT ? 'AA' : ratio.toFixed(2)}`;
      })
    );

    expect(judged).toEqual(
      pairs.flatMap(([text, background]) =>
        ['light', 'dark', 'outside'].map((scheme) => `${text} on ${background} (${scheme}): AA`)
      )
    );
    // and the judgement refuses what Telegram's own palettes get wrong: the link colour as text
    expect(contrast(THEMES.light.link_color, THEMES.light.bg_color)).toBeLessThan(AA_NORMAL_TEXT);
    expect(contrast(THEMES.dark.link_color, THEMES.dark.bg_color)).toBeLessThan(AA_NORMAL_TEXT);
  });

  it('the Tide palette compiles to six custom properties in both modes', () => {
    // SPEC-578 A1: the expected hex comes from the token file's modes.tide, never from the compile
    const roles = examined(
      'Tide roles',
      TIDE_MODES.flatMap((mode) =>
        TIDE_ROLES.map((role) => ({ mode, role, hex: hexOf(member(tideMode(TOKENS, mode)?.[role], '$value')) }))
      )
    );
    const css = compileTokens(TOKENS);
    const properties = rootProperties(css);
    const theme = /@theme inline\s*\{([^}]*)\}/.exec(css)?.[1] ?? '';

    for (const { mode, role, hex } of roles) {
      const name = `--tide-${mode}-${role}`;
      expect(properties.has(name), `${name} is not declared`).toBe(true);
      expect(properties.get(name), name).toBe(hex);
      expect(hex, `${name}'s token`).toMatch(/^#[0-9a-f]{6}$/);
      expect(theme, `${name} is not mapped into the theme`).toContain(
        `  --color-tide-${mode}-${role}: var(${name});`
      );
    }
    // the closure, as SPEC-593 R1 and R2 widen it: the twelve properties of the six roles, a literal
    expect([...properties.keys()].filter((name) => name.startsWith('--tide-')).sort()).toEqual([
      '--tide-dark-accent',
      '--tide-dark-background',
      '--tide-dark-border',
      '--tide-dark-card',
      '--tide-dark-ink',
      '--tide-dark-muted',
      '--tide-light-accent',
      '--tide-light-background',
      '--tide-light-border',
      '--tide-light-card',
      '--tide-light-ink',
      '--tide-light-muted'
    ]);
  });

  it('a role missing from either Tide mode fails the compile by name', () => {
    // SPEC-578 A2
    const planted = () => structuredClone(TOKENS);

    const withoutLightAccent = planted();
    delete tideMode(withoutLightAccent, 'light')?.accent;
    expect(() => compileTokens(withoutLightAccent)).toThrow(/modes\.tide\.light\.accent/);

    const withoutDarkInk = planted();
    delete tideMode(withoutDarkInk, 'dark')?.ink;
    expect(() => compileTokens(withoutDarkInk)).toThrow(/modes\.tide\.dark\.ink/);

    const withUnknownRole = planted();
    const dark = tideMode(withUnknownRole, 'dark');
    if (dark !== undefined) dark.glow = structuredClone(dark.ink);
    expect(() => compileTokens(withUnknownRole)).toThrow(/modes\.tide\.dark\.glow/);

    const withoutDark = planted();
    delete (member(member(withoutDark, 'modes'), 'tide') as Record<string, unknown> | undefined)?.dark;
    expect(() => compileTokens(withoutDark)).toThrow(/modes\.tide\.dark/);

    // the control: the unmodified file compiles, and holds both modes
    expect(() => compileTokens(TOKENS)).not.toThrow();
    examined(
      'Tide modes',
      TIDE_MODES.filter((mode) => tideMode(TOKENS, mode) !== undefined)
    );
  });

  it('ink and accent on the background meet AA in both Tide modes', () => {
    // SPEC-578 A3: the formula refuses a pair it should refuse, #767676 on #808080
    expect(contrast('#767676', '#808080')).toBeLessThan(AA_NORMAL_TEXT);

    const modes = examined(
      'Tide modes',
      TIDE_MODES.filter((mode) => tideMode(TOKENS, mode) !== undefined)
    );
    const hexFor = (mode: string, role: string) => hexOf(member(tideMode(TOKENS, mode)?.[role], '$value'));
    const judged = modes.flatMap((mode) =>
      (['ink', 'accent'] as const).map((role) => {
        const ratio = contrast(hexFor(mode, role), hexFor(mode, 'background'));
        return `${role} on background (${mode}): ${ratio >= AA_NORMAL_TEXT ? 'AA' : ratio.toFixed(2)}`;
      })
    );
    examined('Tide text pairs', judged);

    expect(judged).toEqual(
      modes.flatMap((mode) => [`ink on background (${mode}): AA`, `accent on background (${mode}): AA`])
    );
  });

  it('the modes tier refuses what it cannot compile by the exact message naming it', () => {
    // SPEC-578 A2, the compile's own messages: each planted document breaks one thing
    // the planted tree, typed to the depth the cases reach: tier, group, mode, role
    type PlantedTree = Record<string, Record<string, Record<string, Record<string, unknown>>>>;
    const planted = () => structuredClone(TOKENS) as PlantedTree;
    const messageOf = (document: unknown): string => {
      try {
        compileTokens(document);
      } catch (error) {
        return (error as Error).message;
      }
      return 'compiled';
    };

    // a mode outside dark and light, beside the two known ones
    const withDusk = planted();
    withDusk.modes.tide.dusk = structuredClone(withDusk.modes.tide.dark);
    expect(messageOf(withDusk)).toBe('tokens: modes.tide.dusk is not a mode (dark or light)');

    // a role outside the six, in the exact words
    const withGlow = planted();
    withGlow.modes.tide.dark.glow = structuredClone(withGlow.modes.tide.dark.ink);
    expect(messageOf(withGlow)).toBe(
      'tokens: modes.tide.dark.glow is not a role (background, ink, accent, card, muted or border)'
    );

    // a role absent, in the exact words (not the message of a role that is no token)
    const withoutInk = planted();
    delete withoutInk.modes.tide.dark.ink;
    expect(messageOf(withoutInk)).toBe('tokens: modes.tide.dark.ink is missing');

    // a role that is no token (the group reader names it), and a role token whose value is no colour
    const inkNotAToken = planted();
    inkNotAToken.modes.tide.light.ink = 5;
    expect(messageOf(inkNotAToken)).toBe('tokens: ink is not a token or a group');
    const accentNotAColour = planted();
    accentNotAColour.modes.tide.dark.accent = { $type: 'color', $value: { colorSpace: 'srgb', hex: 'red' } };
    expect(messageOf(accentNotAColour)).toBe(
      'tokens: modes.tide.dark.accent is not an sRGB colour with a six-digit lowercase hex'
    );

    // a modes tier that is no group
    const modesNotAGroup = planted() as Record<string, unknown>;
    modesNotAGroup.modes = 5;
    expect(messageOf(modesNotAGroup)).toBe('tokens: modes is not a token or a group');

    // the control: no modes tier at all compiles, and emits no Tide property
    const withoutModes = planted() as Record<string, unknown>;
    delete withoutModes.modes;
    expect(messageOf(withoutModes)).toBe('compiled');
    const css = compileTokens(withoutModes);
    expect(css).not.toContain('--tide-');
    expect(css).toContain('--radius:');
  });

  it('the Tide palette compiles to twelve custom properties in both modes', () => {
    // SPEC-593 A1: the token file holds the R1 values, and the compile carries each into its property
    const expected = examined(
      'Tide properties',
      TIDE_MODES.flatMap((mode) => SIX_ROLES.map((role) => ({ mode, role, hex: R1[mode][role] })))
    );
    const css = compileTokens(TOKENS);
    const properties = rootProperties(css);
    const theme = /@theme inline\s*\{([^}]*)\}/.exec(css)?.[1] ?? '';

    for (const { mode, role, hex } of expected) {
      const name = `--tide-${mode}-${role}`;
      expect(hexOf(member(tideMode(TOKENS, mode)?.[role], '$value')), `${name}'s token`).toBe(hex);
      expect(properties.get(name), name).toBe(hex);
      expect(theme, `${name} is not mapped into the theme`).toContain(`  --color-tide-${mode}-${role}: var(${name});`);
    }
    expect(expected).toHaveLength(12);
  });

  it('a role outside the six or missing from either Tide mode fails the compile by name', () => {
    // SPEC-593 A2: each planted document breaks one thing, and the compile names it
    const planted = () => structuredClone(TOKENS) as Record<string, Record<string, Record<string, unknown>>>;

    const withGlow = planted();
    const dark = tideMode(withGlow, 'dark');
    if (dark !== undefined) dark.glow = structuredClone(dark.ink);
    expect(() => compileTokens(withGlow)).toThrow(/modes\.tide\.dark\.glow/);

    const cases = examined(
      'roles SPEC-593 adds, by mode',
      TIDE_MODES.flatMap((mode) => (['card', 'muted', 'border'] as const).map((role) => ({ mode, role })))
    );
    for (const { mode, role } of cases) {
      const without = planted();
      delete tideMode(without, mode)?.[role];
      expect(() => compileTokens(without), `${mode} ${role}`).toThrow(new RegExp(`modes\\.tide\\.${mode}\\.${role}\\b`));
    }

    // a colour role whose deck-streak extension names no Tide role
    const withoutTide = planted();
    const own = member(member(member(withoutTide, 'color'), 'background'), '$extensions');
    delete (member(own, 'deck-streak') as Record<string, unknown> | undefined)?.tide;
    expect(() => compileTokens(withoutTide)).toThrow(/color\.background/);

    // the control: the unmodified file compiles
    expect(() => compileTokens(TOKENS)).not.toThrow();
  });

  it('every declared Tide pair meets its contrast bar in both modes', () => {
    // SPEC-593 A3: the formula on WCAG's own values, then every R3 pair over the token file
    expect(contrast('#767676', '#ffffff')).toBeCloseTo(4.54, 2);
    examined(
      'declared Tide pairs, by mode',
      TIDE_MODES.flatMap((mode) => R3_PAIRS.map(({ pair }) => `${pair} (${mode})`))
    );
    expect(tidePairFailures(TOKENS)).toEqual([]);
  });

  it('a planted low-contrast Tide value is refused', () => {
    // SPEC-593 A3: the light muted text planted as the light background fails both of its pairs,
    // and nothing else fails
    const planted = structuredClone(TOKENS);
    const light = tideMode(planted, 'light');
    if (light !== undefined) light.muted = { $value: '{ref.tide.neutral.50}' };
    expect(tidePairFailures(planted).map((failure) => failure.replace(/:.*$/, ''))).toEqual([
      'muted on background (light)',
      'muted on card (light)'
    ]);
  });

  it('every colour role falls back to its Tide role in each mode', () => {
    // SPEC-593 A4: light in :root and dark under .dark, each inside its role's Telegram variable
    const css = compileTokens(TOKENS);
    const root = rootProperties(css);
    const dark = darkProperties(css);
    const tokens = examined('colour roles', colourTokens());
    expect(tokens.map(({ name }) => name).sort()).toEqual(Object.keys(R2_TIDE_ROLE).sort());

    for (const token of tokens) {
      const tide = R2_TIDE_ROLE[token.name];
      const own = member(member(member(member(TOKENS, 'color'), token.name), '$extensions'), 'deck-streak');
      expect(member(own, 'tide'), `color.${token.name}'s Tide role`).toBe(tide);
      const variable = `--tg-theme-${String(token.telegram).replaceAll('_', '-')}`;
      expect(root.get(`--${token.name}`), `--${token.name} in :root`).toBe(`var(${variable}, ${R1.light[tide]})`);
      expect(dark.get(`--${token.name}`), `--${token.name} under .dark`).toBe(
        `var(${variable}, var(--tide-dark-${tide}))`
      );
      expect(root.get(`--tide-dark-${tide}`), `--tide-dark-${tide}`).toBe(R1.dark[tide]);
    }
    expect([...dark.keys()].sort()).toEqual(Object.keys(R2_TIDE_ROLE).map((name) => `--${name}`).sort());
  });
});
