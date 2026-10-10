/**
 * @vitest-environment jsdom
 */
import { render } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import CardFrame from './CardFrame.svelte';

// SPEC-341 R1, R4, R6, A6; ADR-352 D1, D2. A card face renders in one sandboxed iframe with no
// token, whose document is the frame document, and with nothing that would widen it: no `src`,
// `allow` or `allowfullscreen`. A card the frame document refuses renders a frame with no document,
// marked as refused.
const IMAGE = 'data:image/gif;base64,R0lGODlhAQABAAAAACw=';
const CARD = `<p>Der Hund</p><img alt="a dog" src="${IMAGE}">`;
const CSS = '.card { font-family: serif; }';
const OPENING =
  "<!doctype html><html><head><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none';";

describe('the card frame', () => {
  it('the card frame is a sandboxed srcdoc frame with no token', () => {
    const shown = render(CardFrame, { html: CARD, css: CSS, title: 'The card' });
    const frames = [...shown.container.querySelectorAll('iframe')];

    expect(frames).toHaveLength(1);
    const [frame] = frames;
    expect(frame.getAttribute('sandbox')).toBe('');
    expect(frame.getAttributeNames().sort()).toEqual(['sandbox', 'srcdoc', 'title']);
    expect(frame.getAttribute('srcdoc')?.startsWith(OPENING)).toBe(true);
    expect(frame.getAttribute('title')).toBe('The card');

    // a refused card renders a frame with no document, still sandboxed, marked as refused
    const refused = render(CardFrame, { html: CARD, css: '</style><base href="https://cards.example/">', title: 'The card' });
    const marked = [...refused.container.querySelectorAll('iframe')];
    expect(marked).toHaveLength(1);
    expect(marked[0].getAttributeNames().sort()).toEqual(['data-card-refused', 'sandbox', 'title']);
    expect(marked[0].getAttribute('data-card-refused')).toBe('escaped');
    expect(marked[0].getAttribute('sandbox')).toBe('');
  });

  // SPEC-350 R7, A18; ADR-361. The card's classes reach the frame's body through its document, and
  // the frame itself gains nothing: the same three attributes, the same empty sandbox.
  it("the card's classes reach the frame body and the frame gains no attribute", () => {
    const shown = render(CardFrame, { html: CARD, css: CSS, title: 'The card', classes: 'card card2 nightMode night_mode' });
    const frames = [...shown.container.querySelectorAll('iframe')];

    expect(frames).toHaveLength(1);
    const [frame] = frames;
    const body = new DOMParser().parseFromString(frame.getAttribute('srcdoc') ?? '', 'text/html').body;
    expect(body.getAttribute('class')).toBe('card card2 nightMode night_mode');
    expect(frame.getAttributeNames().sort()).toEqual(['sandbox', 'srcdoc', 'title']);
    expect(frame.getAttribute('sandbox')).toBe('');
  });

  // SPEC-407 R1; ADR-421 D2. The frame's document is a host: its one frame, with no sandbox of its
  // own, holds the frame document, so the frame document inherits the host's policy at creation.
  it("the card frame's document is the host, whose one frame holds the frame document", () => {
    const shown = render(CardFrame, { html: CARD, css: CSS, title: 'The card' });
    const [frame] = [...shown.container.querySelectorAll('iframe')];
    const host = new DOMParser().parseFromString(frame.getAttribute('srcdoc') ?? '', 'text/html');
    const inner = host.body.children;

    expect(host.head.querySelector('meta[http-equiv="Content-Security-Policy"]')?.getAttribute('content')).toBe(
      "img-src data:; script-src 'none'; object-src 'none'; base-uri 'none'"
    );
    expect(inner).toHaveLength(1);
    expect(inner[0].tagName).toBe('IFRAME');
    expect(inner[0].getAttributeNames().sort()).toEqual(['srcdoc', 'title']);
    expect(inner[0].getAttribute('title')).toBe('The card');
    expect(inner[0].getAttribute('srcdoc')?.startsWith(OPENING)).toBe(true);
    expect(cardDocument(frame)).toBe(inner[0].getAttribute('srcdoc'));
  });
});

/** The card document a card frame carries: the host's one frame's `srcdoc`, or null. */
function cardDocument(frame: Element): string | null {
  const host = new DOMParser().parseFromString(frame.getAttribute('srcdoc') ?? '', 'text/html');
  return host.body.querySelector('iframe')?.getAttribute('srcdoc') ?? null;
}
