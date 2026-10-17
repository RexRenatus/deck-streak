/**
 * @vitest-environment jsdom
 */
import { describe, expect, it } from 'vitest';
import { frameDocument } from './frame-document';

// SPEC-341 R2 to R4, A3 to A5; ADR-352 D3, D4 (SEC01-F15). The card frame's document opens with
// the frame policy, turns DNS prefetching off and holds the card's CSS in one style element; it
// drops every link, meta, base and template the card carries; and a card whose markup would escape
// that document, read the way the frame will read it, is refused rather than rendered.
const POLICY =
  "default-src 'none'; img-src data:; media-src data:; font-src data:; style-src 'unsafe-inline'; form-action 'none'; base-uri 'none'";
const IMAGE = 'data:image/gif;base64,R0lGODlhAQABAAAAACw=';
// A card's body: text, a data: image and a table, as the parser serializes them.
const CARD = `<p>Der Hund</p><img alt="a dog" src="${IMAGE}"><table><tbody><tr><td>der</td><td>Hund</td></tr></tbody></table>`;
const CSS = '.card { font-family: serif; } td { padding: 0.25em; }';
const ELSEWHERE = 'https://cards.example/';
const STRIPPED = 'link, meta, base, template';

function examined<T>(what: string, items: T[]): T[] {
  console.log(`examined ${items.length} ${what}`);
  expect(items.length, `examined 0 ${what}: the population is empty, so nothing was judged`).toBeGreaterThan(0);
  return items;
}

/** The frame document as the frame reads it; a refused card has none. */
function read(srcdoc: string | undefined): Document {
  expect(srcdoc, 'the card was refused').toBeTypeOf('string');
  return new DOMParser().parseFromString(srcdoc ?? '', 'text/html');
}

describe('the frame document', () => {
  it("the frame document opens with the frame policy and keeps the card's body", () => {
    const frame = read(frameDocument(CARD, CSS).srcdoc);

    expect([...frame.head.children].map((element) => element.outerHTML)).toEqual([
      `<meta http-equiv="Content-Security-Policy" content="${POLICY}">`,
      '<meta http-equiv="x-dns-prefetch-control" content="off">',
      `<style>${CSS}</style>`
    ]);
    expect(frame.body.innerHTML).toBe(CARD);

    // CSS whose lines end the Windows way, or in a lone carriage return, is the same CSS: the
    // frame's parser reads both as line feeds, so the document holds them as line feeds
    const windows = read(frameDocument(CARD, 'p {\r\n  color: navy;\r}').srcdoc);
    expect(windows.head.querySelector('style')?.textContent).toBe('p {\n  color: navy;\n}');
  });

  it('the frame document drops every link, meta, base and template the card carries', () => {
    const planted = [
      `<link rel="preconnect" href="${ELSEWHERE}">`,
      `<link rel="dns-prefetch" href="${ELSEWHERE}">`,
      `<link rel="stylesheet" href="${ELSEWHERE}card.css">`,
      `<link rel="preload" as="image" href="${ELSEWHERE}a.png">`,
      `<link rel="prefetch" href="${ELSEWHERE}b">`,
      `<link rel="modulepreload" href="${ELSEWHERE}c.js">`,
      `<meta http-equiv="refresh" content="0; url=${ELSEWHERE}">`,
      '<meta http-equiv="Content-Security-Policy" content="default-src *">',
      `<base href="${ELSEWHERE}">`,
      `<template shadowrootmode="open"><link rel="preconnect" href="${ELSEWHERE}"></template>`
    ];
    const markers = [...planted.map((_, at) => `<p>marker ${at}</p>`), '<p>marker end</p>'];
    const card = planted.map((element, at) => markers[at] + element).join('') + markers[planted.length];
    const body = read(frameDocument(card, CSS).srcdoc).body;

    // none of them reaches the frame's body
    expect([...body.querySelectorAll(STRIPPED)].map((element) => element.outerHTML)).toEqual([]);
    // and every marker beside them is kept, in order
    expect([...body.children].map((element) => element.outerHTML)).toEqual(markers);
    // each planted element is an element the frame would read, one per plant
    const inert = new DOMParser().parseFromString(`<!doctype html><body>${card}`, 'text/html');
    expect(examined('planted elements', [...inert.body.querySelectorAll(STRIPPED)])).toHaveLength(planted.length);
  });

  // SPEC-402 R1, A1; ADR-416 D2. An image-set candidate is a fetch the element's src does not
  // make, so the strip removes every srcset, on an img, a picture's source or any other element,
  // and keeps each element with its src
  it('the frame document removes every srcset and keeps each element and its src', () => {
    const planted = [
      `<img alt="a dog" src="${IMAGE}" srcset="${ELSEWHERE}a.png 1x">`,
      `<picture><source srcset="${ELSEWHERE}b.png"><img alt="a cat" src="${IMAGE}"></picture>`,
      `<span srcset="${ELSEWHERE}c.png">any element</span>`
    ];
    const body = read(frameDocument(planted.join(''), CSS).srcdoc).body;

    // no srcset reaches the frame's body
    expect([...body.querySelectorAll('[srcset]')].map((element) => element.outerHTML)).toEqual([]);
    // and every element is kept, each image with its src
    expect(body.innerHTML).toBe(
      `<img alt="a dog" src="${IMAGE}"><picture><source><img alt="a cat" src="${IMAGE}"></picture><span>any element</span>`
    );
    // each plant carries a srcset the frame would read, one per plant
    const inert = new DOMParser().parseFromString(`<!doctype html><body>${planted.join('')}`, 'text/html');
    expect(examined('planted srcset carriers', [...inert.body.querySelectorAll('[srcset]')])).toHaveLength(planted.length);
  });

  // SPEC-402 R2, A2; ADR-416 D3. Markup the first parse reads as a style's text, and the frame's
  // parse as an element carrying a srcset: on an img, a picture's source, or the root element,
  // which an html start tag inside the body gives its attributes to
  it('a card whose re-parse brings a srcset back is refused', () => {
    const mutation = '<form><math><mtext></form><form><mglyph><style></math>';
    const returns = [
      `<img alt="" srcset="${ELSEWHERE}a.png 1x">`,
      `<picture><source srcset="${ELSEWHERE}b.png"><img alt=""></picture>`,
      `<html srcset="${ELSEWHERE}c.png">`
    ];
    for (const element of returns) {
      expect(frameDocument(mutation + element, CSS), element).toStrictEqual({ refused: 'escaped' });
    }
    examined('srcsets the re-parse brings back', returns);
  });

  it('a card whose markup escapes the frame document is refused', () => {
    const escapes: [html: string, css: string][] = [
      // CSS that ends the style element and opens a link, then a refresh, in either case
      [CARD, `</style><link rel="preconnect" href="${ELSEWHERE}">`],
      [CARD, `</STYLE ><meta http-equiv="refresh" content="0; url=${ELSEWHERE}">`],
      // markup the first parse reads as a style's text, and the frame's parse as a link: a mutation
      [`<form><math><mtext></form><form><mglyph><style></math><link rel="preconnect" href="${ELSEWHERE}">`, CSS]
    ];
    for (const [html, css] of escapes) {
      expect(frameDocument(html, css), `${html} | ${css}`).toStrictEqual({ refused: 'escaped' });
    }
    examined('escaping cards', escapes);

    // a benign card is not refused, nor is CSS holding a tag that ends no style element
    expect(frameDocument(CARD, CSS).refused).toBeUndefined();
    expect(frameDocument(CARD, 'p::after { content: "</p>"; }').refused).toBeUndefined();
  });

  // SPEC-350 R7, A18; ADR-361. The frame's body carries the card's classes, `card card<ordinal + 1>`,
  // and the night-mode pair in a dark palette, and nothing else: no other attribute, and no class
  // outside that set, so a class can neither open an attribute nor name a style the card did not.
  it("the frame body carries the card's classes and nothing else", () => {
    const admitted = ['card card1', 'card card3 nightMode night_mode', 'card card12'];
    for (const classes of admitted) {
      const body = read(frameDocument(CARD, CSS, classes).srcdoc).body;
      expect(body.getAttributeNames(), classes).toEqual(['class']);
      expect(body.getAttribute('class'), classes).toBe(classes);
      expect(body.innerHTML, classes).toBe(CARD);
    }

    // with no classes the body carries no attribute at all
    expect(read(frameDocument(CARD, CSS).srcdoc).body.getAttributeNames()).toEqual([]);

    const outside = [
      '',
      'card',
      'card card0',
      'card card01',
      'card1',
      'card card1 nightMode',
      'card card1 night_mode nightMode',
      'card card1 x',
      'card card1" onload="x',
      'card  card1',
      ' card card1',
      'card card1 ',
      'CARD card1'
    ];
    for (const classes of outside) {
      expect(frameDocument(CARD, CSS, classes), `"${classes}"`).toStrictEqual({ refused: 'escaped' });
    }
    examined('admitted class lists', admitted);
    examined('refused class lists', outside);
  });

  // Mutation coverage: a card with no classes opens a bare body, and a body tag the second parse
  // reads, which the first read as a style's text, is refused on its attribute alone.
  it('a card with no classes has a bare body, and a body the frame would give attributes is refused', () => {
    expect(frameDocument('<p>x</p>', '.a{}')).toStrictEqual({
      srcdoc: `<!doctype html><html><head><meta http-equiv="Content-Security-Policy" content="${POLICY}"><meta http-equiv="x-dns-prefetch-control" content="off"><style>.a{}</style></head><body><p>x</p></body></html>`
    });
    const html = '<form><math><mtext></form><form><mglyph><style></math><body class="x">';
    expect(frameDocument(html, '.a{}')).toStrictEqual({ refused: 'escaped' });
  });

  it('a data: media source reaches the frame unchanged', () => {
    // SPEC-350 A24, ADR-361 D12: the core writes each media file a face holds as a data: URL in
    // its text, and the frame keeps every one as written, so the frame policy's data: sources are
    // the only way media reaches the card, and nothing is rewritten on the page
    const media = [
      `<img alt="a cat" src="${IMAGE}">`,
      '<audio controls="" src="data:audio/ogg;base64,T2dnUw=="></audio>',
      '<video><source src="data:video/webm;base64,GkXfow==" type="video/webm"></video>'
    ];
    const card = `<p>die Katze</p>${media.join('')}`;
    const frame = read(frameDocument(card, CSS, 'card card1').srcdoc);
    expect(frame.body.innerHTML).toBe(card);
    const sources = examined(
      'media source(s)',
      [...frame.body.querySelectorAll('[src]')].map((element) => element.getAttribute('src'))
    );
    expect(sources).toEqual([IMAGE, 'data:audio/ogg;base64,T2dnUw==', 'data:video/webm;base64,GkXfow==']);
  });
});
