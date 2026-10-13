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

/** The sinks one text holds, by name, with how many of each; a sink it does not hold is absent. */
function sinksIn(text: string): Record<string, number> {
  const found: Record<string, number> = {};
  for (const [name, pattern] of Object.entries(SINKS)) {
    const count = text.match(pattern)?.length ?? 0;
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
