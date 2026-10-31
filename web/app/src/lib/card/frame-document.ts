import { FRAME_POLICY } from './policy.js';

// SPEC-341 R2 to R4; ADR-352 D3, D4 (SEC01-F15); SPEC-402 R1, R2, R7, R8; ADR-416 D2, D3;
// ADR-498 D2. The card frame's document: the card's markup and CSS composed under the frame
// policy. The card is parsed into an inert document (a DOMParser document runs no script and loads
// nothing), every link, meta, base and template element is removed, and so are the `srcset` and
// the `target` attribute of every element that carries one, and the rest is composed after the
// policy, the DNS-prefetch switch and the card's style. The composed string is then parsed again,
// as the frame will parse it, and a card whose markup reads differently the second time (CSS that
// ends its style element, markup that mutates on a re-parse) is refused rather than rendered, and
// so is a card whose re-parsed document holds a `srcset` or a `target` on any element, the root
// included.

/** Why the frame refuses a card: each check's own reason, in the order the checks run. */
export const REFUSAL_REASONS = [
  'class-list',
  'style-closed',
  'reparse-element',
  'reparse-srcset',
  'reparse-target',
  'body-attributes',
  'host-readback'
] as const;

/** One check's reason for refusing a card. */
export type RefusalReason = (typeof REFUSAL_REASONS)[number];

/** The frame document `CardFrame` sets as its `srcdoc`, or the refusal of a card that escaped it,
 * named by the first check that failed (SPEC-505 R1, R2; ADR-523 D1, D2). */
export type FrameDocument =
  | { srcdoc: string; refused?: undefined }
  | { srcdoc?: undefined; refused: RefusalReason };

/** What the frame keeps out of the card: resource hints, refreshes, policies, bases, shadow roots. */
const STRIPPED = 'link, meta, base, template';

/** What the frame takes off every element it keeps: image-set candidates, each a fetch of its own. */
const SRCSET = '[srcset]';

/**
 * What the frame takes off every element it keeps: a link's target, which lets an engine's click
 * handler start a connection the frame's own navigation does not.
 */
const TARGET = '[target]';

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
  if (classes !== undefined && !CLASSES.test(classes)) return { refused: 'class-list' };
  const card = parse(`<!doctype html><body>${html}`);
  for (const element of card.querySelectorAll(STRIPPED)) element.remove();
  // SPEC-402 R1; ADR-416 D2: the element stays, and its src still shows its image
  for (const element of card.querySelectorAll(SRCSET)) element.removeAttribute('srcset');
  // SPEC-402 R7; ADR-498 D2: the element stays, with its href and its other attributes
  for (const element of card.querySelectorAll(TARGET)) element.removeAttribute('target');
  // the parser reads a carriage return, alone or before a line feed, as a line feed
  const style = `<style>${css.replace(/\r\n?/g, '\n')}</style>`;
  const body = classes === undefined ? '<body>' : `<body class="${classes}">`;
  // the body carries exactly the class written, or nothing when no class was given
  const expected = classes === undefined ? [] : [['class', classes]];
  const composed = `<!doctype html><html><head>${HEAD}${style}</head>${body}${card.body.innerHTML}</body></html>`;
  const frame = parse(composed);
  // SPEC-505 R2; ADR-523 D2: each check its own reason, the first that fails naming the refusal
  if (frame.head.innerHTML !== HEAD + style) return { refused: 'style-closed' };
  if (frame.body.querySelector(STRIPPED) !== null) return { refused: 'reparse-element' };
  if (frame.querySelector(SRCSET) !== null) return { refused: 'reparse-srcset' };
  if (frame.querySelector(TARGET) !== null) return { refused: 'reparse-target' };
  if (JSON.stringify(attributes(frame.body)) !== JSON.stringify(expected)) return { refused: 'body-attributes' };
  return { srcdoc: composed };
}
