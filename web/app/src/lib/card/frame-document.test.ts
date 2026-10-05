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
});
