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

/** The roles every Ember mode holds, and the two modes (SPEC-578 R2). */
const EMBER_MODES = ['dark', 'light'] as const;
const EMBER_ROLES = ['background', 'ink', 'accent'] as const;

/** An Ember mode of a token document, or undefined when the document holds none. */
function emberMode(document: unknown, mode: string): Record<string, unknown> | undefined {
  return member(member(member(document, 'modes'), 'ember'), mode) as Record<string, unknown> | undefined;
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

  it('the Ember palette compiles to six custom properties in both modes', () => {
    // SPEC-578 A1: the expected hex comes from the token file's modes.ember, never from the compile
    const roles = examined(
      'Ember roles',
      EMBER_MODES.flatMap((mode) =>
        EMBER_ROLES.map((role) => ({ mode, role, hex: hexOf(member(emberMode(TOKENS, mode)?.[role], '$value')) }))
      )
    );
    const css = compileTokens(TOKENS);
    const properties = rootProperties(css);
    const theme = /@theme inline\s*\{([^}]*)\}/.exec(css)?.[1] ?? '';

    for (const { mode, role, hex } of roles) {
      const name = `--ember-${mode}-${role}`;
      expect(properties.has(name), `${name} is not declared`).toBe(true);
      expect(properties.get(name), name).toBe(hex);
      expect(hex, `${name}'s token`).toMatch(/^#[0-9a-f]{6}$/);
      expect(theme, `${name} is not mapped into the theme`).toContain(
        `  --color-ember-${mode}-${role}: var(${name});`
      );
    }
    expect([...properties.keys()].filter((name) => name.startsWith('--ember-')).sort()).toEqual(
      roles.map(({ mode, role }) => `--ember-${mode}-${role}`).sort()
    );
  });

  it('a role missing from either Ember mode fails the compile by name', () => {
    // SPEC-578 A2
    const planted = () => structuredClone(TOKENS);

    const withoutLightAccent = planted();
    delete emberMode(withoutLightAccent, 'light')?.accent;
    expect(() => compileTokens(withoutLightAccent)).toThrow(/modes\.ember\.light\.accent/);

    const withoutDarkInk = planted();
    delete emberMode(withoutDarkInk, 'dark')?.ink;
    expect(() => compileTokens(withoutDarkInk)).toThrow(/modes\.ember\.dark\.ink/);

    const withFourthRole = planted();
    const dark = emberMode(withFourthRole, 'dark');
    if (dark !== undefined) dark.muted = structuredClone(dark.ink);
    expect(() => compileTokens(withFourthRole)).toThrow(/modes\.ember\.dark\.muted/);

    const withoutDark = planted();
    delete (member(member(withoutDark, 'modes'), 'ember') as Record<string, unknown> | undefined)?.dark;
    expect(() => compileTokens(withoutDark)).toThrow(/modes\.ember\.dark/);

    // the control: the unmodified file compiles, and holds both modes
    expect(() => compileTokens(TOKENS)).not.toThrow();
    examined(
      'Ember modes',
      EMBER_MODES.filter((mode) => emberMode(TOKENS, mode) !== undefined)
    );
  });

  it('ink and accent on the background meet AA in both Ember modes', () => {
    // SPEC-578 A3: the formula refuses a pair it should refuse, #767676 on #808080
    expect(contrast('#767676', '#808080')).toBeLessThan(AA_NORMAL_TEXT);

    const modes = examined(
      'Ember modes',
      EMBER_MODES.filter((mode) => emberMode(TOKENS, mode) !== undefined)
    );
    const hexFor = (mode: string, role: string) => hexOf(member(emberMode(TOKENS, mode)?.[role], '$value'));
    const judged = modes.flatMap((mode) =>
      (['ink', 'accent'] as const).map((role) => {
        const ratio = contrast(hexFor(mode, role), hexFor(mode, 'background'));
        return `${role} on background (${mode}): ${ratio >= AA_NORMAL_TEXT ? 'AA' : ratio.toFixed(2)}`;
      })
    );
    examined('Ember text pairs', judged);

    expect(judged).toEqual(
      modes.flatMap((mode) => [`ink on background (${mode}): AA`, `accent on background (${mode}): AA`])
    );
  });

  it('the modes tier refuses what it cannot compile by the exact message naming it', () => {
    // SPEC-578 A2, the compile's own messages: each planted document breaks one thing
    const planted = () => structuredClone(TOKENS) as Record<string, Record<string, Record<string, unknown>>>;
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
    withDusk.modes.ember.dusk = structuredClone(withDusk.modes.ember.dark);
    expect(messageOf(withDusk)).toBe('tokens: modes.ember.dusk is not a mode (dark or light)');

    // a role outside the three, in the exact words
    const withMuted = planted();
    withMuted.modes.ember.dark.muted = structuredClone(withMuted.modes.ember.dark.ink);
    expect(messageOf(withMuted)).toBe(
      'tokens: modes.ember.dark.muted is not a role (background, ink or accent)'
    );

    // a role absent, in the exact words (not the message of a role that is no token)
    const withoutInk = planted();
    delete withoutInk.modes.ember.dark.ink;
    expect(messageOf(withoutInk)).toBe('tokens: modes.ember.dark.ink is missing');

    // a role that is no token (the group reader names it), and a role token whose value is no colour
    const inkNotAToken = planted();
    inkNotAToken.modes.ember.light.ink = 5;
    expect(messageOf(inkNotAToken)).toBe('tokens: ink is not a token or a group');
    const accentNotAColour = planted();
    accentNotAColour.modes.ember.dark.accent = { $type: 'color', $value: { colorSpace: 'srgb', hex: 'red' } };
    expect(messageOf(accentNotAColour)).toBe(
      'tokens: modes.ember.dark.accent is not an sRGB colour with a six-digit lowercase hex'
    );

    // a modes tier that is no group
    const modesNotAGroup = planted() as Record<string, unknown>;
    modesNotAGroup.modes = 5;
    expect(messageOf(modesNotAGroup)).toBe('tokens: modes is not a token or a group');

    // the control: no modes tier at all compiles, and emits no Ember property
    const withoutModes = planted() as Record<string, unknown>;
    delete withoutModes.modes;
    expect(messageOf(withoutModes)).toBe('compiled');
    const css = compileTokens(withoutModes);
    expect(css).not.toContain('--ember-');
    expect(css).toContain('--radius:');
  });
});
