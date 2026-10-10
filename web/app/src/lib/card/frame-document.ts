import { FRAME_POLICY } from './policy.js';

// SPEC-341 R2 to R4; ADR-352 D3, D4 (SEC01-F15); SPEC-402 R1, R2; ADR-416 D2, D3. The card frame's
// document: the card's markup and CSS composed under the frame policy. The card is parsed into an
// inert document (a DOMParser document runs no script and loads nothing), every link, meta, base and
// template element is removed, and so is the `srcset` attribute of every element that carries one,
// and the rest is composed after the policy, the DNS-prefetch switch and the card's style. The
// composed string is then parsed again, as the frame will parse it, and a card whose markup reads
// differently the second time (CSS that ends its style element, markup that mutates on a re-parse)
// is refused rather than rendered.

/** The frame document `CardFrame` sets as its `srcdoc`, or the refusal of a card that escaped it. */
export type FrameDocument =
  | { srcdoc: string; refused?: undefined }
  | { srcdoc?: undefined; refused: 'escaped' };

/** What the frame keeps out of the card: resource hints, refreshes, policies, bases, shadow roots. */
const STRIPPED = 'link, meta, base, template';

/** What the frame takes off every element it keeps: image-set candidates, each a fetch of its own. */
const SRCSET = '[srcset]';

/** The frame document's head before the card's style: the frame policy, then prefetching off. */
const HEAD = `<meta http-equiv="Content-Security-Policy" content="${FRAME_POLICY}"><meta http-equiv="x-dns-prefetch-control" content="off">`;

function parse(text: string): Document {
  return new DOMParser().parseFromString(text, 'text/html');
}

// SPEC-350 R7, A18; ADR-361. The body's classes: the card's `card card<ordinal + 1>`, and the
// night-mode pair in a dark palette. Nothing else is admitted, so a class list cannot open another
// attribute or name a style the card's own CSS did not ask for.
const CLASSES = /^card card[1-9][0-9]*( nightMode night_mode)?$/;

/** The body's attributes as the frame reads them back, each a name and its value, in order. */
function attributes(body: HTMLElement): string[][] {
  return Array.from(body.attributes, (attribute) => [attribute.name, attribute.value]);
}

export function frameDocument(html: string, css: string, classes?: string): FrameDocument {
  if (classes !== undefined && !CLASSES.test(classes)) return { refused: 'escaped' };
  const card = parse(`<!doctype html><body>${html}`);
  for (const element of card.querySelectorAll(STRIPPED)) element.remove();
  // SPEC-402 R1; ADR-416 D2: the element stays, and its src still shows its image
  for (const element of card.querySelectorAll(SRCSET)) element.removeAttribute('srcset');
  // the parser reads a carriage return, alone or before a line feed, as a line feed
  const style = `<style>${css.replace(/\r\n?/g, '\n')}</style>`;
  const body = classes === undefined ? '<body>' : `<body class="${classes}">`;
  // the body carries exactly the class written, or nothing when no class was given
  const expected = classes === undefined ? [] : [['class', classes]];
  const composed = `<!doctype html><html><head>${HEAD}${style}</head>${body}${card.body.innerHTML}</body></html>`;
  const frame = parse(composed);
  const kept =
    frame.head.innerHTML === HEAD + style &&
    frame.body.querySelector(STRIPPED) === null &&
    frame.querySelector(SRCSET) === null &&
    JSON.stringify(attributes(frame.body)) === JSON.stringify(expected);
  return kept ? { srcdoc: composed } : { refused: 'escaped' };
}
