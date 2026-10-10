import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { LAYERS, PLANTED, RENDER, UNOBSERVABLE, type Engine } from '../../../tests-card/planted';

// SPEC-341 R8 to R12, A8 to A12. The planted suite (tests-card/card.spec.ts) runs in Playwright,
// which the tdd probe does not resolve, so this test proves its coverage instead, as
// a11y-coverage.test.ts does for the rendered audit: the planted cards are the schematic's web
// channels, each with the schematic's layer alone; the declared UNOBSERVABLE pairs hold every
// channel the schematic calls unobservable; and the spec opens every card in the reference frame
// and the card frame, every layer's variant, the scripts-on measurement and the render proof,
// reading arrivals only through its listeners.
const APP = fileURLToPath(new URL('../../..', import.meta.url));
const SUITE = join(APP, 'tests-card');
const SPEC = join(SUITE, 'card.spec.ts');
const CARD_CONFIG = join(APP, 'vite.card.config.ts');
const ENGINES: readonly Engine[] = ['chromium', 'webkit'];

/** The repository root, found by walking up to the workspace file, so a StrykerJS sandbox below
 * `web/app/.stryker-tmp` still finds the schematic. */
function repositoryRoot(from: string): string {
  for (let dir = from; ; dir = dirname(dir)) {
    if (existsSync(join(dir, 'pnpm-workspace.yaml'))) return dir;
    if (dirname(dir) === dir) throw new Error(`no pnpm-workspace.yaml above ${from}`);
  }
}

const SCHEMATIC = join(repositoryRoot(APP), 'docs', 'schematics', 'card-frame-channels.md');

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** The schematic's section 3, the web channels, from its heading to the next. */
function webSection(): string {
  const section = readFileSync(SCHEMATIC, 'utf8')
    .split(/^## /m)
    .find((part) => part.startsWith('3. Web channels'));
  expect(section, 'the schematic has no section 3, Web channels').toBeDefined();
  return section ?? '';
}

/** Section 3's channel rows, those whose first cell is a backticked id, with the layer-alone cell. */
function webChannels(): { id: string; alone: string }[] {
  return webSection()
    .split('\n')
    .flatMap((line) => {
      const row = /^\|\s*`([a-z0-9-]+)`\s*\|(.*)\|\s*$/.exec(line);
      if (row === null) return [];
      const cells = row[2].split('|').map((cell) => cell.trim());
      return [{ id: row[1], alone: cells[cells.length - 1] }];
    });
}

/** Section 3's layer rows: W1 to W4, in the table's order (the page policy P is no layer). */
function webLayers(): string[] {
  return webSection()
    .split('\n')
    .flatMap((line) => /^\|\s*(W[0-9]+)\s*\|/.exec(line)?.[1] ?? []);
}

/** The body of every block that follows a match of `header`, braces balanced. */
function blocksAfter(text: string, header: RegExp): string[] {
  const bodies: string[] = [];
  for (const match of text.matchAll(new RegExp(header.source, 'g'))) {
    const open = text.indexOf('{', (match.index ?? 0) + match[0].length);
    let depth = 0;
    for (let at = open; open >= 0 && at < text.length; at += 1) {
      if (text[at] === '{') depth += 1;
      if (text[at] === '}') depth -= 1;
      if (depth === 0) {
        bodies.push(text.slice(open, at + 1));
        break;
      }
    }
  }
  return bodies;
}

/** Each way a spec could count the browser's own request events instead of a listener's arrivals. */
const BROWSER_EVENTS: readonly RegExp[] = [
  /\.on\(\s*['"`](?:request|requestfinished|requestfailed|response)['"`]/,
  /\bwaitFor(?:Request|Response)\s*\(/,
  /\.route(?:FromHAR)?\s*\(/
];

function browserEvents(text: string): string[] {
  return BROWSER_EVENTS.filter((pattern) => pattern.test(text)).map((pattern) => pattern.source);
}

function spec(): string {
  expect(existsSync(SPEC), 'tests-card/card.spec.ts does not exist').toBe(true);
  return readFileSync(SPEC, 'utf8');
}

describe('the planted card suite', () => {
  it('every channel in the schematic has a planted card', () => {
    const channels = webChannels();
    const planted = PLANTED.map((card) => card.id);

    // the suite plants exactly the schematic's web channels, each once
    expect([...planted].sort()).toEqual(channels.map((channel) => channel.id).sort());
    examined('web channels in the schematic', channels);
    expect(new Set(planted).size).toBe(planted.length);

    // each card's layer alone is the first token of the schematic's cell, none for a dash
    const alone = new Map(PLANTED.map((card) => [card.id, card.alone]));
    for (const channel of channels) {
      const token = channel.alone.split(/\s+/)[0];
      expect(['-', 'W1', 'W2', 'W3', 'W4'], channel.id).toContain(token);
      expect(alone.get(channel.id), channel.id).toBe(token === '-' ? null : token);
    }
    for (const card of PLANTED) expect(card.paths.length, card.id).toBeGreaterThan(0);

    // SPEC-402 R4, A3; ADR-416 D5: the image-set forms are planted on the srcset card alone, so no
    // other card's variants can read them through its own prefix
    const listener = { http: 'http://listener.invalid', tcp: 'http://listener.invalid:2', udp: 3 };
    const carriers = PLANTED.filter((card) => /\bsrcset=/i.test(card.html(listener))).map((card) => card.id);
    expect(carriers).toEqual(['srcset']);

    // a channel the schematic calls unobservable is declared so in both engines
    const blind = examined(
      'channels the schematic calls unobservable',
      channels.filter((channel) => channel.alone.includes('UNOBSERVABLE:')).map((channel) => channel.id)
    );
    for (const engine of ENGINES) {
      const declared = UNOBSERVABLE.filter((entry) => entry.engine === engine).map((entry) => entry.id);
      for (const id of blind) expect(declared, `${engine} ${id}`).toContain(id);
    }
    // and every declared pair names an engine, a planted card and its reason, once
    for (const entry of UNOBSERVABLE) {
      expect(ENGINES).toContain(entry.engine);
      expect(planted).toContain(entry.id);
      expect(entry.why.trim(), `${entry.engine} ${entry.id}`).not.toBe('');
    }
    const pairs = UNOBSERVABLE.map((entry) => `${entry.engine} ${entry.id}`);
    expect(new Set(pairs).size).toBe(pairs.length);

    // the harness serves the page policy the app ships: svelte.config.js's directives, written by
    // the shared header writer, and the edge's header from the Caddy file
    expect(existsSync(CARD_CONFIG), 'vite.card.config.ts does not exist').toBe(true);
    const config = readFileSync(CARD_CONFIG, 'utf8');
    expect(config).toMatch(/import\s+\w+\s+from\s+'\.\/svelte\.config\.js'/);
    expect(config).toMatch(/import\s*\{\s*policyHeader\s*\}\s*from\s*'\.\/policy-header(?:\.ts)?'/);
    expect(config).toMatch(/deploy\/caddy\/deck-streak\.caddy/);
  });

  it('the planted suite runs every card in the reference frame and the card frame', () => {
    const text = spec();

    // one pair per planted card: the reference frame once and the card frame once
    expect(text).toMatch(/import\s*\{[^}]*\bPLANTED\b[^}]*\}\s*from\s*'\.\/planted'/);
    const loops = examined(
      'loops over the planted cards',
      blocksAfter(text, /for\s*\(\s*const\s+card\s+of\s+PLANTED\s*\)\s*/)
    );
    const pairs = loops.filter(
      (body) =>
        (body.match(/'open\.html'/g) ?? []).length === 1 &&
        (body.match(/'index\.html'/g) ?? []).length === 1
    );
    expect(pairs, 'no loop opens each card once in open.html and once in index.html').toHaveLength(1);

    // arrivals are read through the listeners, never from the browser's own request events
    expect(text).toMatch(/from\s*'\.\/listeners'/);
    expect(browserEvents(text)).toEqual([]);
    const planted = [
      "page.on('request', count);",
      'await page.waitForRequest(url);',
      "await context.route('**/*', handle);"
    ];
    for (const line of examined('planted request-event reads', planted)) {
      expect(browserEvents(line), line).toHaveLength(1);
    }
  });

  it('every layer has a single-layer variant and a channel of its own', () => {
    // the layers are the schematic's, in its order
    expect([...LAYERS]).toEqual(examined('layers in the schematic', webLayers()));

    // each layer alone holds a channel that some engine can observe
    for (const layer of LAYERS) {
      const own = PLANTED.filter((card) => card.alone === layer).map((card) => card.id);
      const observable = own.filter(
        (id) => !ENGINES.every((engine) => UNOBSERVABLE.some((entry) => entry.engine === engine && entry.id === id))
      );
      expect(observable, `${layer} holds no observable channel alone`).not.toEqual([]);
    }

    // and the spec opens every card with each layer off
    const text = spec();
    const variants = blocksAfter(text, /for\s*\(\s*const\s+layer\s+of\s+LAYERS\s*\)\s*/);
    expect(variants.join('\n')).toMatch(/variant\.html\?off=\$\{layer\}/);
  });

  it('the scripts-on measurement plants a peer connection and a parent message', () => {
    // the two cards whose reach with scripts on the ADR records
    expect(PLANTED.map((card) => card.id)).toEqual(expect.arrayContaining(['webrtc', 'bridge']));
    for (const script of ['webrtc.js', 'bridge.js']) {
      expect(existsSync(join(SUITE, 'harness', 'planted', script)), script).toBe(true);
    }
    // the peer connection is measured in at least one engine
    const blind = ENGINES.filter((engine) =>
      UNOBSERVABLE.some((entry) => entry.engine === engine && entry.id === 'webrtc')
    );
    expect(blind.length, 'webrtc is unobservable in every engine').toBeLessThan(ENGINES.length);
    // and the spec runs the scripts-on variant
    expect(spec()).toMatch(/variant\.html\?off=scripts/);
  });

  it('the render proof compares the card frame with the reference and a blank frame', () => {
    // a card that is not a channel: text, a data: image and a table, pointing nowhere
    expect(RENDER, 'planted.ts exports no render-proof card').not.toBeNull();
    const card = RENDER ?? { html: '', css: '' };
    expect(card.html).toMatch(/<img\b[^>]*\bsrc="data:image\//);
    expect(card.html).toMatch(/<table\b/);
    expect(card.html.replace(/<[^>]*>/g, '').trim()).not.toBe('');
    expect(card.html + card.css).not.toMatch(/\b(?:https?|wss?|ftp|mailto|javascript):/);

    // the spec screenshots it in the card frame, the reference frame and a blank frame
    const text = spec();
    expect(text).toMatch(/import\s*\{[^}]*\bRENDER\b[^}]*\}\s*from\s*'\.\/planted'/);
    expect(text).toMatch(/\.screenshot\(/);
    const proof = text.slice(text.indexOf('render-proof'));
    for (const frame of ["'index.html'", "'open.html'", "'blank'"]) expect(proof).toContain(frame);
  });
});
