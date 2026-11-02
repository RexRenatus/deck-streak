/**
 * @vitest-environment jsdom
 */
import { describe, expect, it } from 'vitest';
import { frameDocument, REFUSAL_REASONS, type FrameDocument, type RefusalReason } from './frame-document';
import { frameCard } from './frame-host';

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
      expect(frameDocument(mutation + element, CSS), element).toStrictEqual({ refused: 'reparse-srcset' });
    }
    examined('srcsets the re-parse brings back', returns);
  });

  // SPEC-402 R7, A5; ADR-498 D2. A link's target is what lets an engine's click handler start a
  // connection the frame's own navigation does not, so the strip removes every target, on an a,
  // an area, an SVG a, a form or any other element, whatever its value, and keeps each element
  // with its href
  it('the frame document removes every target and keeps each element and its href', () => {
    const planted = [
      `<a target="_blank" href="${ELSEWHERE}1">a</a>`,
      `<map name="m"><area target="_blank" href="${ELSEWHERE}2" alt="b"></map>`,
      `<svg><a target="_blank" href="${ELSEWHERE}3"><text>c</text></a></svg>`,
      `<form target="_blank" action="${ELSEWHERE}4"><button>d</button></form>`,
      '<span target="_blank">e</span>',
      // a target whose value holds a line feed and a `<`, removed like any other
      `<a target="_blank\n<b" href="${ELSEWHERE}6">f</a>`
    ];
    const body = read(frameDocument(planted.join(''), CSS).srcdoc).body;

    // no target reaches the frame's body
    expect([...body.querySelectorAll('[target]')].map((element) => element.outerHTML)).toEqual([]);
    // and every element is kept, each link and area with its href
    expect(body.innerHTML).toBe(
      `<a href="${ELSEWHERE}1">a</a><map name="m"><area href="${ELSEWHERE}2" alt="b"></map>` +
        `<svg><a href="${ELSEWHERE}3"><text>c</text></a></svg>` +
        `<form action="${ELSEWHERE}4"><button>d</button></form><span>e</span><a href="${ELSEWHERE}6">f</a>`
    );
    // each plant carries a target the frame would read, one per plant
    const inert = new DOMParser().parseFromString(`<!doctype html><body>${planted.join('')}`, 'text/html');
    expect(examined('planted target carriers', [...inert.body.querySelectorAll('[target]')])).toHaveLength(planted.length);
  });

  // SPEC-402 R8, A6; ADR-498 D2. Markup the first parse reads as a style's text, and the frame's
  // parse as an element carrying a target: on an a, an area, or the root element, which an html
  // start tag inside the body gives its attributes to
  it('a card whose re-parse brings a target back is refused', () => {
    const mutation = '<form><math><mtext></form><form><mglyph><style></math>';
    const returns = [
      `<a target="_blank" href="${ELSEWHERE}a">a</a>`,
      `<area target="_blank" href="${ELSEWHERE}b" alt="">`,
      '<html target="_blank">'
    ];
    for (const element of returns) {
      expect(frameDocument(mutation + element, CSS), element).toStrictEqual({ refused: 'reparse-target' });
    }
    examined('targets the re-parse brings back', returns);
  });

  it('a card whose markup escapes the frame document is refused', () => {
    const escapes: [html: string, css: string, reason: RefusalReason][] = [
      // CSS that ends the style element and opens a link, then a refresh, in either case
      [CARD, `</style><link rel="preconnect" href="${ELSEWHERE}">`, 'style-closed'],
      [CARD, `</STYLE ><meta http-equiv="refresh" content="0; url=${ELSEWHERE}">`, 'style-closed'],
      // markup the first parse reads as a style's text, and the frame's parse as a link: a mutation
      [
        `<form><math><mtext></form><form><mglyph><style></math><link rel="preconnect" href="${ELSEWHERE}">`,
        CSS,
        'reparse-element'
      ]
    ];
    for (const [html, css, reason] of escapes) {
      expect(frameDocument(html, css), `${html} | ${css}`).toStrictEqual({ refused: reason });
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
      expect(frameDocument(CARD, CSS, classes), `"${classes}"`).toStrictEqual({ refused: 'class-list' });
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
    expect(frameDocument(html, '.a{}')).toStrictEqual({ refused: 'body-attributes' });
  });

  // SPEC-505 R1, R2, A1; ADR-523 D1, D2. Each check refuses with its own reason, and when a card
  // breaks several, the first check that fails names it, so the reason is a function of the card.
  // Each plant is the clean card with one fault, and the clean card itself is kept, so a refusal
  // is the fault's and not the card's.
  it('each frame check refuses with its own reason, the first failing check naming it', () => {
    const mutation = '<form><math><mtext></form><form><mglyph><style></math>';
    const clean = { html: CARD, css: CSS, title: 'The card', classes: 'card card1' };
    const plants: [check: string, card: typeof clean][] = [
      ['a class list outside the admitted set', { ...clean, classes: 'card card1 x' }],
      ['CSS that ends its own style element', { ...clean, css: `${CSS}</style><base href="${ELSEWHERE}">` }],
      ['a template the re-parse brings back', { ...clean, html: `${CARD}${mutation}<template></template>` }],
      ['a srcset the re-parse brings back', { ...clean, html: `${CARD}${mutation}<img alt="" srcset="${ELSEWHERE}a.png 1x">` }],
      ['a target the re-parse brings back', { ...clean, html: `${CARD}${mutation}<a target="_blank" href="${ELSEWHERE}a">a</a>` }],
      ['a body attribute the re-parse brings', { ...clean, html: `${CARD}${mutation}<body data-plant="1">` }],
      ['a host whose title does not read back', { ...clean, title: 'The\rcard' }]
    ];

    // the identity control: the clean card, with every field each plant starts from, is kept
    const kept = frameCard(clean.html, clean.css, clean.title, clean.classes);
    expect(kept.refused, 'the clean card').toBeUndefined();
    expect(kept.srcdoc).toBeTypeOf('string');

    const reasons = examined('frame checks', plants).map(([check, card]) => {
      const framed = frameCard(card.html, card.css, card.title, card.classes);
      expect(framed.srcdoc, check).toBeUndefined();
      return framed.refused;
    });
    // each plant has its own reason, in the order the checks run, and they are the closed list
    expect(reasons).toEqual([...REFUSAL_REASONS]);
    expect(new Set(reasons).size).toBe(7);
  });

  // SPEC-505 R2; ADR-523 D2. A card that breaks two checks next to each other in the order is
  // refused by the earlier one. Each pair's card holds both faults, and each fault alone is refused
  // by its own check, so the card breaks both checks and only the order picks its reason.
  const MUTATION = '<form><math><mtext></form><form><mglyph><style></math>';
  const CLEAN = { html: CARD, css: CSS, title: 'The card', classes: 'card card1' };
  /** Each check's fault: a field of the clean card it replaces, or markup after the mutation. */
  const FAULTS: Record<RefusalReason, Partial<typeof CLEAN> & { markup?: string }> = {
    'class-list': { classes: 'card card1 x' },
    'style-closed': { css: `${CSS}</style><base href="${ELSEWHERE}">` },
    'reparse-element': { markup: '<template></template>' },
    'reparse-srcset': { markup: `<img alt="" srcset="${ELSEWHERE}a.png 1x">` },
    'reparse-target': { markup: `<a target="_blank" href="${ELSEWHERE}a">a</a>` },
    'body-attributes': { markup: '<body data-plant="1">' },
    'host-readback': { title: 'The\rcard' }
  };

  /** The reason the frame gives the clean card with each named check's fault planted in it. */
  function refusedWith(...faults: RefusalReason[]): RefusalReason | undefined {
    const card = { ...CLEAN };
    const markup: string[] = [];
    for (const fault of faults) {
      const { markup: planted, ...fields } = FAULTS[fault];
      Object.assign(card, fields);
      if (planted !== undefined) markup.push(planted);
    }
    if (markup.length > 0) card.html = `${CARD}${MUTATION}${markup.join('')}`;
    return frameCard(card.html, card.css, card.title, card.classes).refused;
  }

  const PAIRS = REFUSAL_REASONS.slice(1).map((later, index) => [REFUSAL_REASONS[index], later] as const);
  for (const [earlier, later] of PAIRS) {
    it(`a card that breaks both ${earlier} and ${later} is refused by the earlier check, ${earlier}`, () => {
      // each fault alone is refused by its own check, so the card below breaks both
      expect([refusedWith(earlier), refusedWith(later)], 'each fault alone').toEqual([earlier, later]);
      expect(refusedWith(earlier, later), 'both faults').toBe(earlier);
    });
  }

  // SPEC-505 R1, R5; ADR-523 D1, D4. The types close the set: no word outside the seven is a
  // reason, and a refusal is its reason alone, with no document to show. The app's check reads each
  // directive below, and a type that opened would leave a directive unused, which it reports.
  it('no other word is a refusal reason, and a refusal carries no document', () => {
    // @ts-expect-error the one word every check used to answer is no reason (SPEC-505 R1)
    const word: RefusalReason = 'escaped';
    // @ts-expect-error a refusal carries no document, so no part of the card (ADR-523 D4)
    const carrying: FrameDocument = { refused: 'class-list', srcdoc: CARD };
    expect(examined('reasons', [...REFUSAL_REASONS])).not.toContain(word);
    // the frame's own refusal holds its reason alone, where the planted one holds a document too
    expect(Object.keys(frameDocument(CARD, CSS, 'card card1 x'))).toEqual(['refused']);
    expect(Object.keys(carrying)).toEqual(['refused', 'srcdoc']);
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
