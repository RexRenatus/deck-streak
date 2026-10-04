import { FRAME_POLICY } from './policy.js';

// SPEC-341 R2 to R4; ADR-352 D3, D4 (SEC01-F15). The card frame's document: the card's markup and
// CSS composed under the frame policy. The card is parsed into an inert document (a DOMParser
// document runs no script and loads nothing), every link, meta, base and template element is
// removed, and the rest is composed after the policy, the DNS-prefetch switch and the card's style.
// The composed string is then parsed again, as the frame will parse it, and a card whose markup
// reads differently the second time (CSS that ends its style element, markup that mutates on a
// re-parse) is refused rather than rendered.

/** The frame document `CardFrame` sets as its `srcdoc`, or the refusal of a card that escaped it. */
export type FrameDocument =
  | { srcdoc: string; refused?: undefined }
  | { srcdoc?: undefined; refused: 'escaped' };

/** What the frame keeps out of the card: resource hints, refreshes, policies, bases, shadow roots. */
const STRIPPED = 'link, meta, base, template';

/** The frame document's head before the card's style: the frame policy, then prefetching off. */
const HEAD = `<meta http-equiv="Content-Security-Policy" content="${FRAME_POLICY}"><meta http-equiv="x-dns-prefetch-control" content="off">`;

function parse(text: string): Document {
  return new DOMParser().parseFromString(text, 'text/html');
}

export function frameDocument(html: string, css: string): FrameDocument {
  const card = parse(`<!doctype html><body>${html}`);
  for (const element of card.querySelectorAll(STRIPPED)) element.remove();
  // the parser reads a carriage return, alone or before a line feed, as a line feed
  const style = `<style>${css.replace(/\r\n?/g, '\n')}</style>`;
  const composed = `<!doctype html><html><head>${HEAD}${style}</head><body>${card.body.innerHTML}</body></html>`;
  const frame = parse(composed);
  const kept = frame.head.innerHTML === HEAD + style && frame.body.querySelector(STRIPPED) === null;
  return kept ? { srcdoc: composed } : { refused: 'escaped' };
}
