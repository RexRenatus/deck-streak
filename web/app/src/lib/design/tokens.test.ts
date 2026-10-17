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
});
