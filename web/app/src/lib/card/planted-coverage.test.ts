import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { LAYERS, PLANTED, RENDER, UNOBSERVABLE } from '../../../tests-card/planted';

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
const PLAYWRIGHT_CARD = join(APP, 'playwright.card.config.ts');
const ENGINES: readonly string[] = configuredEngines(readFileSync(PLAYWRIGHT_CARD, 'utf8')).engines;

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

/** The engines the card configuration runs: the literal `name` of each project object in its
 * `projects` array, in order, read from the file's text (SPEC-399 R2). */
function configuredEngines(config: string): { engines: string[]; refused: string[] } {
  const engines: string[] = [];
  const start = /\bprojects:\s*\[/.exec(config);
  if (start === null) return { engines, refused: ['no projects array'] };
  const open = start.index + start[0].length - 1;
  let depth = 0;
  let close = -1;
  for (let at = open; at < config.length; at += 1) {
    if (config[at] === '[') depth += 1;
    if (config[at] === ']') depth -= 1;
    if (depth === 0) {
      close = at;
      break;
    }
  }
  if (close < 0) return { engines, refused: ['no projects array'] };

  // each top-level object of the array, kept without the objects nested in it
  const projects: string[] = [];
  let braces = 0;
  let own = '';
  for (const char of config.slice(open + 1, close)) {
    if (char === '{') {
      braces += 1;
      if (braces === 1) own = '';
      continue;
    }
    if (char === '}') {
      braces -= 1;
      if (braces === 0) projects.push(own);
      continue;
    }
    if (braces === 1) own += char;
  }

  const refused: string[] = [];
  projects.forEach((project, index) => {
    const name = /(?:^|[\s,])name:\s*(['"])([^'"]*)\1/.exec(project)?.[2];
    if (name === undefined) refused.push(`project ${index + 1} has no literal name`);
    else if (engines.includes(name)) refused.push(`${name} is named twice`);
    else engines.push(name);
  });
  return { engines, refused };
}

/** Section 3's unobservable cells and the `<engine> <id>` pairs they name, over `engines`
 * (SPEC-399 R1). A token read in no form the check knows is refused with its row's id. */
function unobservableCells(
  section: string,
  engines: readonly string[]
): { cells: string[]; pairs: string[]; refused: string[] } {
  const cells: string[] = [];
  const pairs: string[] = [];
  const refused: string[] = [];
  for (const line of section.split('\n')) {
    const row = /^\|\s*`([a-z0-9-]+)`\s*\|(.*)\|\s*$/.exec(line);
    if (row === null) continue;
    const id = row[1];
    for (const cell of row[2].split('|')) {
      const seen = cell.match(/unobservable/gi)?.length ?? 0;
      if (seen === 0) continue;
      const tokens = [...cell.matchAll(/UNOBSERVABLE(?::| in ([A-Za-z]+)(?=[,)]))/g)];
      if (tokens.length !== seen) {
        refused.push(`${id}: ${seen} UNOBSERVABLE tokens in a cell, ${tokens.length} in a form the check reads`);
        continue;
      }
      cells.push(id);
      for (const token of tokens) {
        if (token[1] === undefined) {
          for (const engine of engines) pairs.push(`${engine} ${id}`);
        } else if (engines.includes(token[1].toLowerCase())) {
          pairs.push(`${token[1].toLowerCase()} ${id}`);
        } else {
          refused.push(`${id}: no configured engine is named ${token[1]}`);
        }
      }
    }
  }
  return { cells, pairs, refused };
}

/** The named pairs no declaration holds, and the declared pairs no cell names, each sorted. */
function pairDifference(
  named: readonly string[],
  declared: readonly string[]
): { missing: string[]; extra: string[] } {
  const have = new Set(declared);
  const want = new Set(named);
  return {
    missing: [...want].filter((pair) => !have.has(pair)).sort(),
    extra: [...have].filter((pair) => !want.has(pair)).sort()
  };
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

    // a channel the schematic calls unobservable is declared so in every engine
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

  it('the card config runs the planted suite in Chromium, WebKit and Firefox', () => {
    // SPEC-398 A1: the card configuration's projects are the suite's engines, in this order
    const config = readFileSync(PLAYWRIGHT_CARD, 'utf8');
    const projects = [
      ...config.matchAll(/\{\s*name:\s*'(\w+)',\s*use:\s*\{\s*\.\.\.devices\['([^']+)'\]/g)
    ].map((match) => ({ name: match[1], device: match[2] }));
    expect(projects.map((project) => project.name)).toEqual(['chromium', 'webkit', 'firefox']);
    expect(projects.map((project) => project.name)).toEqual([...ENGINES]);
    expect(projects.map((project) => project.device)).toEqual([
      'Desktop Chrome',
      'Desktop Safari',
      'Desktop Firefox'
    ]);
    // the engines run one after another: one worker, no parallelism
    expect(config).toMatch(/workers:\s*1,/);
    expect(config).toMatch(/fullyParallel:\s*false,/);
    examined('card projects', projects);
  });

  it("the schematic's unobservable cells equal the declared unobservable pairs both ways", () => {
    // SPEC-399 A1; ADR-413 D2: every cell, in either spelling, over the configured engines
    const { cells, pairs, refused } = unobservableCells(webSection(), ENGINES);
    const declared = UNOBSERVABLE.map((entry) => `${entry.engine} ${entry.id}`);

    expect(refused, 'a token section 3 spells in a form the check does not read').toEqual([]);
    expect(pairDifference(pairs, declared)).toEqual({ missing: [], extra: [] });
    expect([...pairs].sort()).toEqual([...declared].sort());

    examined('engines in the card configuration', [...ENGINES]);
    examined('unobservable cells in section 3', cells);
    examined('unobservable pairs section 3 names', pairs);
    examined('unobservable pairs planted.ts declares', declared);
  });

  it('a planted mismatch between the cells and the declared pairs is refused', () => {
    // SPEC-399 A2: section 3's seven unobservable rows, copied, and the pairs they name
    const FIXTURE = [
      '3. Web channels',
    '| `prefetch` | `link rel=prefetch` | W3, W2 | - (UNOBSERVABLE in WebKit, measured: it sends no prefetch request under the suite) |',
    '| `preconnect` | `link rel=preconnect` (a TCP connection, no request) | W3 | W3 (UNOBSERVABLE in Chromium, measured: it opens no preconnect connection under the suite; UNOBSERVABLE in Firefox, measured: it opens no preconnect connection under the suite) |',
    '| `dns-prefetch` | `link rel=dns-prefetch` | W3 | W3 (UNOBSERVABLE: no lookup reaches a listener) |',
    '| `shadow-link` | a `template shadowrootmode=open` holding `link rel=preconnect` | W3 | W3 (UNOBSERVABLE in Chromium, measured: it opens no preconnect connection under the suite, in a shadow tree or out of one; UNOBSERVABLE in Firefox, measured: it opens no preconnect connection under the suite, in a shadow tree or out of one) |',
    '| `ping` | a same-document link with `ping` at the listener, clicked | W2, P | - (UNOBSERVABLE in Firefox, measured: it sends no hyperlink audit under the suite) |',
    '| `external-scheme` | a full-frame `mailto:` link, clicked | W1, W4 | - (UNOBSERVABLE: no listener sees a handler launch) |',
    '| `webrtc` | an inline script that opens a peer connection to the UDP listener as its STUN server | W1, W2, P (none of them by policy: they stop the script, not the peer connection) | - (UNOBSERVABLE in Firefox, measured: its peer connection sends no datagram to the UDP listener under the suite) |',
      ''
    ].join('\n');
    const THREE = ['chromium', 'webkit', 'firefox'];
    const THIRTEEN = [
      'chromium dns-prefetch',
      'webkit dns-prefetch',
      'firefox dns-prefetch',
      'webkit prefetch',
      'chromium preconnect',
      'firefox preconnect',
      'chromium shadow-link',
      'firefox shadow-link',
      'firefox ping',
      'chromium external-scheme',
      'webkit external-scheme',
      'firefox external-scheme',
      'firefox webrtc'
    ];
    const once = (text: string, part: string): void => {
      expect(text.split(part).length - 1, `${part} occurs once`).toBe(1);
    };
    const read = (text: string, engines: readonly string[]) => unobservableCells(text, engines);
    const plants: string[] = [];

    // 1: a cell for one engine with no declared pair
    plants.push('a one-engine cell with no declared pair');
    expect(
      pairDifference(read(FIXTURE, THREE).pairs, THIRTEEN.filter((pair) => pair !== 'webkit prefetch'))
    ).toEqual({ missing: ['webkit prefetch'], extra: [] });

    // 2: a declared pair no cell names
    plants.push('a declared pair with no cell');
    expect(pairDifference(read(FIXTURE, THREE).pairs, [...THIRTEEN, 'webkit preconnect'])).toEqual({
      missing: [],
      extra: ['webkit preconnect']
    });

    // 3: an every-engine cell declared in one engine only
    plants.push('an every-engine cell declared in one engine');
    expect(
      pairDifference(read(FIXTURE, THREE).pairs, THIRTEEN.filter((pair) => pair !== 'webkit dns-prefetch'))
    ).toEqual({ missing: ['webkit dns-prefetch'], extra: [] });

    // 4: an engine no project carries
    plants.push('an engine no project carries');
    once(FIXTURE, 'UNOBSERVABLE in WebKit');
    const elsewhere = read(FIXTURE.replace('UNOBSERVABLE in WebKit', 'UNOBSERVABLE in Elsewhere'), THREE);
    expect(elsewhere.refused).toHaveLength(1);
    expect(elsewhere.refused[0]).toMatch(/^prefetch: /);

    // 5: two engines in one note, which neither spelling carries
    plants.push('a note for two engines in one token');
    const preconnect = FIXTURE.split('\n').filter((line) => line.startsWith('| `preconnect` |'));
    expect(preconnect).toHaveLength(1);
    once(FIXTURE, preconnect[0]);
    once(preconnect[0], 'UNOBSERVABLE in Chromium,');
    const both = read(
      FIXTURE.replace(preconnect[0], preconnect[0].replace('UNOBSERVABLE in Chromium,', 'UNOBSERVABLE in Chromium and WebKit,')),
      THREE
    );
    expect(both.refused).toHaveLength(1);
    expect(both.refused[0]).toMatch(/^preconnect: /);

    // 6: a token in another case
    plants.push('a token in lower case');
    const lower = read(FIXTURE.replace('UNOBSERVABLE in WebKit', 'unobservable in WebKit'), THREE);
    expect(lower.refused).toHaveLength(1);
    expect(lower.refused[0]).toMatch(/^prefetch: /);

    // 7: a further configured engine takes a pair from each every-engine cell
    plants.push('a further configured engine');
    expect(read(FIXTURE, ['chromium', 'firefox', 'third', 'webkit']).pairs.sort()).toEqual(
      [...THIRTEEN, 'third dns-prefetch', 'third external-scheme'].sort()
    );

    // the control: the copied rows over the three engines read clean and equal the thirteen
    const control = read(FIXTURE, THREE);
    expect(control.refused).toEqual([]);
    expect(control.cells).toHaveLength(7);
    expect([...control.pairs].sort()).toEqual([...THIRTEEN].sort());

    examined('planted mismatches', plants);
  });

  it("the engines are the card configuration's projects", () => {
    // SPEC-399 A3; ADR-413 D2: the literal name of each project, in order, from the file's text
    const configuration = (...projects: string[]): string =>
      `export default defineConfig({\n  projects: [\n    ${projects.join(',\n    ')}\n  ],\n  workers: 1\n});\n`;
    const altered = configuredEngines(
      configuration(
        "{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }",
        "{ name: 'third', use: { ...devices['Desktop Safari'] } }",
        "{ name: 'webkit', use: {} }"
      )
    );
    expect(altered.engines).toEqual(['chromium', 'third', 'webkit']);
    expect(altered.refused).toEqual([]);

    expect(
      configuredEngines(
        configuration("{ name: 'chromium', use: {} }", '{ name: projectName, use: {} }', "{ name: 'webkit', use: {} }")
      ).refused
    ).toEqual(['project 2 has no literal name']);
    expect(
      configuredEngines(configuration("{ name: 'chromium', use: {} }", "{ name: 'chromium', use: {} }")).refused
    ).toEqual(['chromium is named twice']);
    expect(configuredEngines('export default defineConfig({ workers: 1 });').refused).toEqual(['no projects array']);

    const real = configuredEngines(readFileSync(PLAYWRIGHT_CARD, 'utf8'));
    expect(real.refused).toEqual([]);
    for (const name of real.engines) expect(name).toMatch(/^[a-z0-9-]+$/);
    expect(real.engines).toEqual(ENGINES);
    examined('projects in the card configuration', real.engines);
  });
});
