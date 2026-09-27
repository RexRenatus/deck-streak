/**
 * Compiles the design tokens into the CSS custom properties Tailwind 4's `@theme` reads (SPEC-028
 * R5; ADR-028).
 *
 * `src/lib/design/tokens.json` is the one source of DeckStreak's design tokens, in the DTCG format
 * 2025.10. A Vite plugin in `vite.config.ts` calls `writeTokens` when a build or the dev server
 * starts, and `src/routes/layout.css` imports what it writes, `src/lib/design/tokens.css`, which is
 * not tracked. `tokens.test.ts` judges `compileTokens` over the same file, so the proof and the
 * page read the same variables.
 *
 * Every colour of the `color` tier becomes `--<name>: var(--tg-theme-<parameter>, <hex>)`: inside
 * Telegram the chat's theme paints it, and outside Telegram its own colour does. A colour that
 * names no Telegram theme parameter fails the compile, so no build can ship one. Each colour is
 * also mapped into Tailwind's theme as `--color-<name>`; the radius becomes `--radius`, and each
 * font family `--font-<name>`. The `ref` tier is never emitted: only the tokens that alias it are.
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const SOURCE = fileURLToPath(new URL('../src/lib/design/tokens.json', import.meta.url));
const OUTPUT = fileURLToPath(new URL('../src/lib/design/tokens.css', import.meta.url));

// Telegram's theme parameters (core.telegram.org/bots/webapps, ThemeParams). The client sets each
// as the CSS variable --tg-theme-<name>, its underscores written as dashes.
const THEME_PARAMETERS = new Set([
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
]);

// CSS's generic family keywords, which a font stack writes unquoted.
const GENERIC_FAMILIES = new Set([
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
]);

type Node = Readonly<Record<string, unknown>>;

/** `value` as a token or a group, or a compile error naming `where`. */
function node(value: unknown, where: string): Node {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new Error(`tokens: ${where} is not a token or a group`);
  }
  return value as Node;
}

/** A group's tokens and subgroups, without its `$` properties. */
function members(group: Node): Array<[string, Node]> {
  return Object.entries(group)
    .filter(([name]) => !name.startsWith('$'))
    .map(([name, value]) => [name, node(value, name)]);
}

/** Refuses a group whose tokens are not all of `type`: a token's own `$type` overrides its group's. */
function typed(group: Node, type: string, where: string): void {
  for (const [name, token] of members(group)) {
    if ((token.$type ?? group.$type) !== type) {
      throw new Error(`tokens: ${where}.${name} is not a ${type} token`);
    }
  }
}

/**
 * A colour's hex: the DTCG colour's own `hex`, which must agree with its sRGB components, or the
 * hex of the token an alias such as `{ref.white}` names.
 */
function hexOf(document: Node, value: unknown, where: string, seen: readonly string[] = []): string {
  if (typeof value === 'string') {
    const path = /^\{([\w.-]+)\}$/.exec(value)?.[1];
    if (path === undefined) throw new Error(`tokens: ${where} holds ${value}, which is no alias`);
    if (seen.includes(path)) throw new Error(`tokens: ${where} aliases itself through {${path}}`);
    const target = path
      .split('.')
      .reduce<unknown>((at, key) => (typeof at === 'object' && at !== null ? (at as Node)[key] : undefined), document);
    return hexOf(document, node(target, path).$value, path, [...seen, path]);
  }
  const colour = node(value, where);
  const { colorSpace, components, hex } = colour;
  if (colorSpace !== 'srgb' || typeof hex !== 'string' || !/^#[0-9a-f]{6}$/.test(hex)) {
    throw new Error(`tokens: ${where} is not an sRGB colour with a six-digit lowercase hex`);
  }
  const bytes = [1, 3, 5].map((at) => parseInt(hex.slice(at, at + 2), 16));
  const agrees =
    Array.isArray(components) &&
    components.length === 3 &&
    components.every((c, i) => typeof c === 'number' && Math.round(c * 255) === bytes[i]);
  if (!agrees) throw new Error(`tokens: ${where}'s components do not make ${hex}`);
  return hex;
}

function dimension(value: unknown, where: string): string {
  const { value: amount, unit } = node(value, where);
  if (typeof amount !== 'number' || (unit !== 'rem' && unit !== 'px')) {
    throw new Error(`tokens: ${where} is not a dimension in rem or px`);
  }
  return `${amount}${unit}`;
}

function fontFamily(value: unknown, where: string): string {
  const names: unknown = typeof value === 'string' ? [value] : value;
  if (!Array.isArray(names) || names.length === 0 || !names.every((name) => typeof name === 'string')) {
    throw new Error(`tokens: ${where} is not a font family`);
  }
  return names
    .map((name: string) => (GENERIC_FAMILIES.has(name) ? name : `'${name.replaceAll("'", "\\'")}'`))
    .join(', ');
}

/** The stylesheet a DTCG token document compiles to; a malformed document throws, naming the token. */
export function compileTokens(input: unknown): string {
  const document = node(input, 'the document');
  const root: string[] = [];
  const theme: string[] = [];

  const colours = node(document.color, 'color');
  typed(colours, 'color', 'color');
  for (const [name, token] of members(colours)) {
    const where = `color.${name}`;
    const extension = token.$extensions === undefined ? {} : node(token.$extensions, where);
    const own = extension['deck-streak'];
    const parameter = typeof own === 'object' && own !== null ? (own as Node).telegram : undefined;
    if (typeof parameter !== 'string' || !THEME_PARAMETERS.has(parameter)) {
      throw new Error(
        `tokens: ${where} names no Telegram theme parameter (its deck-streak extension's telegram), so Telegram's theme cannot paint it`
      );
    }
    const fallback = hexOf(document, token.$value, where);
    root.push(`  --${name}: var(--tg-theme-${parameter.replaceAll('_', '-')}, ${fallback});`);
    theme.push(`  --color-${name}: var(--${name});`);
  }

  const radius = node(document.radius, 'radius');
  if (radius.$type !== 'dimension') throw new Error('tokens: radius is not a dimension token');
  root.push(`  --radius: ${dimension(radius.$value, 'radius')};`);

  const fonts = node(document.font, 'font');
  typed(fonts, 'fontFamily', 'font');
  for (const [name, token] of members(fonts)) {
    theme.push(`  --font-${name}: ${fontFamily(token.$value, `font.${name}`)};`);
  }

  return [
    '/* Compiled from tokens.json by scripts/build-tokens.ts (ADR-028). Edit the tokens, not this file. */',
    ':root {',
    ...root,
    '}',
    '',
    '@theme inline {',
    ...theme,
    '}',
    ''
  ].join('\n');
}

/** Compiles `tokens.json` into the stylesheet `layout.css` imports. */
export function writeTokens(): void {
  writeFileSync(OUTPUT, compileTokens(JSON.parse(readFileSync(SOURCE, 'utf8'))));
}
