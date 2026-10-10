/**
 * @vitest-environment jsdom
 */
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { frameHost, hosted } from './frame-host';

// SPEC-407 R1 to R3, R7; ADR-421 D2 to D4. The card frame's host: a head carrying the host policy
// and one frame whose `srcdoc` is the card document, read back byte for byte. These tests parse the
// host with `DOMParser` themselves and never import the code's own reader, so the oracle is not the
// code under test.

const POLICY = "img-src data:; script-src 'none'; object-src 'none'; base-uri 'none'";
const STYLE =
  '<style>html,body{margin:0;height:100%;overflow:hidden}iframe{display:block;border:0;width:100%;height:100%}</style>';
const APP = join(import.meta.dirname, '..', '..', '..');

/** The host's document, parsed as the frame will parse it. */
function parsed(srcdoc: string | undefined): Document {
  return new DOMParser().parseFromString(srcdoc ?? '', 'text/html');
}

describe('the card frame host', () => {
  it('the host holds the card document as its one frame, under the host policy', () => {
    const card = '<!doctype html><html><head></head><body><p title="a&amp;b">&amp; "q"</p></body></html>';
    const title = 'The "card" & more';
    const host = parsed(frameHost(card, title).srcdoc);

    const metas = host.head.querySelectorAll('meta');
    expect(metas).toHaveLength(1);
    expect(metas[0].getAttribute('http-equiv')).toBe('Content-Security-Policy');
    expect(metas[0].getAttribute('content')).toBe(POLICY);
    expect(host.head.firstElementChild).toBe(metas[0]);
    expect(host.body.children).toHaveLength(1);
    const frame = host.body.children[0];
    expect(frame.tagName).toBe('IFRAME');
    expect(frame.getAttributeNames().sort()).toEqual(['srcdoc', 'title']);
    expect(frame.getAttribute('srcdoc')).toBe(card);
    expect(frame.getAttribute('title')).toBe(title);

    // an empty policy writes no meta element at all
    const bare = parsed(frameHost(card, title, '').srcdoc);
    expect(bare.head.querySelectorAll('meta')).toHaveLength(0);
    expect(bare.body.children[0].getAttribute('srcdoc')).toBe(card);
  });

  it('a host that cannot carry the card document or its title byte for byte is refused', () => {
    expect(frameHost('<p>a\rb</p>', 'The card')).toEqual({ refused: 'escaped' });
    expect(frameHost('<p>ab</p>', 'The\rcard')).toEqual({ refused: 'escaped' });
    expect(frameHost('<p>ab</p>', 'The card', 'img-src "data:"')).toEqual({ refused: 'escaped' });
    expect(frameHost('<p>ab</p>', 'The card').refused).toBeUndefined();
  });

  // The killers of the host's own code, one behaviour each (SPEC-407 R1, R3; ADR-421 D4).
  it('the head holds exactly the policy meta and the style, and no meta for an empty policy', () => {
    const meta = `<meta http-equiv="Content-Security-Policy" content="${POLICY}">`;
    expect(parsed(frameHost('<p>a</p>', 'The card').srcdoc).head.innerHTML).toBe(meta + STYLE);
    expect(parsed(frameHost('<p>a</p>', 'The card', '').srcdoc).head.innerHTML).toBe(STYLE);
  });

  it('a card holding an ampersand, a quote, a reference and a line feed is read back byte for byte', () => {
    for (const card of ['a &amp; b', 'a & b', 'say "hi"', '&quot;&#13;&lt;', 'one\ntwo', '&"']) {
      const kept = frameHost(card, 'The card');
      expect(hosted(kept.srcdoc ?? '')).toBe(card);
    }
    const title = parsed(frameHost('<p>a</p>', 'a & "b" &amp;').srcdoc).body.children[0].getAttribute('title');
    expect(title).toBe('a & "b" &amp;');
  });

  it('a policy, a card or a title the parser changes is refused, each alone', () => {
    expect(frameHost('<p>a</p>', 'The card', 'img-src\u00a0data:')).toEqual({ refused: 'escaped' });
    expect(frameHost('<p>a\u0000b</p>', 'The card')).toEqual({ refused: 'escaped' });
    expect(frameHost('<p>a</p>', 'The\u0000card')).toEqual({ refused: 'escaped' });
  });

  it('a document that is not a host carries no frame document', () => {
    expect(hosted('')).toBeNull();
    expect(hosted('<p srcdoc="a"></p>')).toBeNull();
    expect(hosted('<iframe srcdoc="a"></iframe><iframe srcdoc="b"></iframe>')).toBeNull();
    expect(hosted('<iframe srcdoc="a"></iframe>')).toBe('a');
    expect(hosted('<iframe></iframe>')).toBeNull();
  });

  // SPEC-407 A6. The Playwright case runs in `card-sandbox`; the tdd probe resolves no Playwright
  // command, so this reads the case and its harness variant as text, as planted-coverage does.
  it('the planted suite holds the host policy case, and the harness builds it through the host', () => {
    const spec = readFileSync(join(APP, 'tests-card', 'card.spec.ts'), 'utf8');
    const harness = readFileSync(join(APP, 'tests-card', 'harness', 'main.ts'), 'utf8');

    expect(spec).toContain("test('the host policy alone holds the image-set forms in every engine'");
    expect(spec).toContain("'variant.html?off=W3meta'");
    expect(spec).toContain("planted.id === 'srcset'");
    expect(harness).toContain("import { frameHost } from '../../src/lib/card/frame-host'");
    expect(harness).toContain("off === 'W3meta'");
    expect(harness).toContain('hostOf(composed(');
  });
});
