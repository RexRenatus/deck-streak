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
});
