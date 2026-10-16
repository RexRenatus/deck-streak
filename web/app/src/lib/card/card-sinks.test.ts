import { readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

// SPEC-341 R7, A7 (SEC01-F15). A card's HTML reaches the page in one place only: the card frame's
// `srcdoc`, written by `CardFrame.svelte`. Every other way markup can enter the page's own document
// (`{@html}`, an `innerHTML` or `outerHTML` assignment, `insertAdjacentHTML`, `document.write`, a
// second `srcdoc`) and every window message listener, through which a card frame could talk to the
// page, is held at none. The census reads text, so a comment counts, and a read of `innerHTML` is
// not an assignment.
const APP = fileURLToPath(new URL('../../..', import.meta.url));
const SRC = join(APP, 'src');

/** Each sink, by the name the census reports it under, and the pattern that finds one. */
const SINKS: Readonly<Record<string, RegExp>> = {
  '{@html': /\{@html\b/g,
  'innerHTML=': /\.innerHTML\s*\+?=(?!=)/g,
  'outerHTML=': /\.outerHTML\s*\+?=(?!=)/g,
  'insertAdjacentHTML(': /\binsertAdjacentHTML\s*\(/g,
  'document.write(': /\bdocument\s*\.\s*write(?:ln)?\s*\(/g,
  // an attribute or property assignment, or the attribute set by name; reading a result's
  // `srcdoc` field is neither
  srcdoc: /\bsrcdoc\s*=(?!=)|\bsetAttribute\(\s*['"`]srcdoc['"`]/g,
  'message listener':
    /\b(?:window|globalThis|self)\s*\.\s*addEventListener\(\s*['"`]message['"`]|\b(?:window|globalThis|self)\s*\.\s*onmessage\s*=(?!=)|<svelte:window[^>]*\bonmessage\b/g
};

/** An arm is a pattern whose every match is a hit, or a counter for an arm a pattern cannot state. */
type Arm = RegExp | ((text: string) => number);

/** SPEC-425 R1: six shapes the page holds at none, each read from raw text, so a comment counts. */
const ZERO_SHAPES: Readonly<Record<string, Arm>> = {};

/** The arms of one table that one text holds, by name, with how many of each; an arm it does not
 * hold is absent. The table is SPEC-341's sinks unless a caller names another. */
function sinksIn(
  text: string,
  arms: Readonly<Record<string, Arm>> = SINKS
): Record<string, number> {
  const found: Record<string, number> = {};
  for (const [name, arm] of Object.entries(arms)) {
    const count = typeof arm === 'function' ? arm(text) : (text.match(arm)?.length ?? 0);
    if (count > 0) found[name] = count;
  }
  return found;
}

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** Every shipped source file under `dir`, relative to the app: never a test or the generated messages. */
function sources(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true })
    .flatMap((entry) => {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) return entry.name === 'paraglide' ? [] : sources(path);
      if (!/\.(?:ts|js|svelte)$/.test(entry.name)) return [];
      if (/\.(?:test|spec)\./.test(entry.name)) return [];
      return [relative(APP, path).split('\\').join('/')];
    })
    .sort();
}

describe('the card frame is the only way card HTML reaches the page', () => {
  it('card HTML reaches the page only through the card frame', () => {
    const files = sources(SRC);
    const census: Record<string, Record<string, number>> = {};
    for (const file of files) {
      const found = sinksIn(readFileSync(join(APP, file), 'utf8'));
      if (Object.keys(found).length > 0) census[file] = found;
    }

    // two sinks in the whole Mini App, both the card frame's: its component and its host, once each
    expect(census).toEqual({
      'src/lib/card/CardFrame.svelte': { srcdoc: 1 },
      'src/lib/card/frame-host.ts': { srcdoc: 1 }
    });
    examined('shipped source files under src', files);

    // and the matcher refuses each sink by name, where a read or a comparison is no sink
    const planted: Record<string, string> = {
      '{@html': '<p>{@html card}</p>',
      'innerHTML=': 'node.innerHTML = card;',
      'outerHTML=': 'node.outerHTML += card;',
      'insertAdjacentHTML(': "node.insertAdjacentHTML('beforeend', card);",
      'document.write(': 'document.writeln(card);',
      srcdoc: "frame.setAttribute('srcdoc', card);",
      'message listener': "window.addEventListener('message', onCard);"
    };
    for (const [name, text] of examined('planted sinks', Object.entries(planted))) {
      expect(sinksIn(text), text).toEqual({ [name]: 1 });
    }
    expect(Object.keys(planted).sort()).toEqual(Object.keys(SINKS).sort());
    expect(sinksIn('<iframe sandbox="" srcdoc={doc.srcdoc}></iframe>')).toEqual({ srcdoc: 1 });
    expect(sinksIn('frame.srcdoc = card;')).toEqual({ srcdoc: 1 });
    expect(sinksIn('globalThis.onmessage = onCard;')).toEqual({ 'message listener': 1 });
    expect(sinksIn('<svelte:window onmessage={onCard} />')).toEqual({ 'message listener': 1 });
    expect(
      sinksIn('const text = node.innerHTML; if (a.outerHTML === b) {} return { srcdoc: html };')
    ).toEqual({});
  });
});

// SPEC-425 R1 to R4, A1 and A2 (ADR-439). Six more shapes the page holds at none, in a table of
// their own read by the same matcher over the same walk, and the walk itself held by its kinds and
// one anchor per kind, so a walk that stops reading one kind fails rather than reading less.
describe('no shipped code runs text as code, leaks the opener, posts to any origin or links a script URL', () => {
  it('shipped code holds none of the six refused shapes', () => {
    const files = sources(SRC);
    const census: Record<string, Record<string, number>> = {};
    for (const file of files) {
      const found = sinksIn(readFileSync(join(APP, file), 'utf8'), ZERO_SHAPES);
      if (Object.keys(found).length > 0) census[file] = found;
    }

    // SPEC-425 R2: each plant reads its own arm once, and each near miss reads none
    const planted: Record<string, string[]> = {
      'eval(': [
        'const run = eval(card);',
        'window.eval(card);',
        '// a comment naming eval(card) counts too'
      ],
      'new Function(': ["const run = new Function('return ' + card);", "Function('return 1')();"],
      'string timer': ["setTimeout('render()', 0);", 'setInterval(`tick()`, 10);'],
      'blank target without rel': [
        '<a href={url} target="_blank">open</a>',
        `<a target='_blank' rel="noopener" href={url}>`,
        "<a href={url} target={'_blank'}>",
        "link.target = '_blank';",
        '<a rel="noopener noreferrer" onclick={() => go()} target="_blank">'
      ],
      "postMessage to '*'": [
        "parent.postMessage('card', '*');",
        "frame.contentWindow.postMessage(card, { targetOrigin: '*' });"
      ],
      'javascript: URL': [
        '<a href="javascript:void 0">run</a>',
        "location.href = 'JavaScript:void 0';",
        '<a href=javascript:void(0)>',
        "if (url.protocol === 'javascript:') refuse();"
      ]
    };
    const nearMisses: Record<string, string[]> = {
      'eval(': [`"script-src 'self' 'wasm-unsafe-eval'"`, 'page.evaluate(fn);', 'page.$eval(sel, fn);'],
      'new Function(': ['isFunction(value);', "typeof x === 'function';"],
      'string timer': ['setTimeout(() => input.blur(), 0);', 'window.setTimeout(next, 0);'],
      'blank target without rel': [
        '<a href={url} target="_blank" rel="noopener noreferrer">',
        '<a rel="noreferrer noopener" href={url} target="_blank">',
        '<a onclick={() => go()} target="_blank" rel="noopener noreferrer">'
      ],
      "postMessage to '*'": ['scope.postMessage(reply, transferred(reply));', 'channel.postMessage(null);'],
      'javascript: URL': ["// the page's own JavaScript runs from its own files"]
    };
    for (const [arm, texts] of Object.entries(planted)) {
      for (const text of texts) expect(sinksIn(text, ZERO_SHAPES), text).toEqual({ [arm]: 1 });
    }
    for (const text of Object.values(nearMisses).flat()) {
      expect(sinksIn(text, ZERO_SHAPES), text).toEqual({});
    }
    expect(Object.keys(planted).sort()).toEqual(Object.keys(ZERO_SHAPES).sort());

    expect(census).toEqual({});
    examined('shipped source files under src', files);
    examined('planted zero shapes', Object.values(planted).flat());
    examined('near misses', Object.values(nearMisses).flat());
  });

  it('the census reads every shipped kind, the page shell included', () => {
    const files = sources(SRC);
    const kinds = [...new Set(files.map((file) => file.slice(file.lastIndexOf('.'))))].sort();
    expect(kinds).toEqual(['.html', '.js', '.svelte', '.ts']);
    for (const anchor of [
      'src/app.html',
      'src/lib/card/CardFrame.svelte',
      'src/lib/card/frame-host.ts',
      'src/lib/card/policy.js'
    ]) {
      expect(files, anchor).toContain(anchor);
    }
    expect(
      files.filter((file) => /\.(?:test|spec)\./.test(file) || file.includes('/paraglide/'))
    ).toEqual([]);
    examined('shipped source files under src', files);
  });
});
