import type { FrameDocument } from './frame-document';
import { HOST_POLICY } from './policy.js';

// SPEC-407 R1 to R3; ADR-421 D1, D2, D4. The card frame's host: the document `CardFrame` sets on
// its frame, whose head carries the host policy and whose body is one frame holding the frame
// document. A document a frame inherits its policy from has that policy in force from its creation,
// before its first byte is parsed, so the host holds an image-set candidate that an engine fetches
// ahead of the frame document's own policy meta element. The host is parsed again as the frame will
// parse it, and a card that does not come back byte for byte is refused rather than rendered.

/** The host's style: the nested frame fills the frame it sits in, so the host adds no size of its own. */
const HOST_STYLE =
  '<style>html,body{margin:0;height:100%;overflow:hidden}iframe{display:block;border:0;width:100%;height:100%}</style>';

/**
 * Text as a double-quoted attribute value: `&` first, then the quote. A carriage return is left as
 * written, and the parser reads it back as a line feed, so the read-back refuses a card or a title
 * that holds one rather than carrying it changed.
 */
function attribute(text: string): string {
  return text.replaceAll('&', '&amp;').replaceAll('"', '&quot;');
}

function parse(text: string): Document {
  return new DOMParser().parseFromString(text, 'text/html');
}

/**
 * The frame document a host carries, read back the way the frame reads it: the one frame of the
 * host's body, when it is the body's only child, or null for a document that is not a host.
 */
export function hosted(host: string): string | null {
  const body = parse(host).body;
  const only = body.children.length === 1 ? body.children[0] : null;
  return only?.tagName === 'IFRAME' ? only.getAttribute('srcdoc') : null;
}

/**
 * The host for a frame document: the policy meta (none for an empty policy), the style, and one
 * frame. A host whose head, frame document or title does not read back as written is refused, so a
 * card that escaped its attribute shows nothing rather than itself.
 */
export function frameHost(card: string, title: string, policy: string = HOST_POLICY): FrameDocument {
  const meta = policy === '' ? '' : `<meta http-equiv="Content-Security-Policy" content="${policy}">`;
  const host = `<!doctype html><html><head>${meta}${HOST_STYLE}</head><body><iframe title="${attribute(title)}" srcdoc="${attribute(card)}"></iframe></body></html>`;
  const again = parse(host);
  const kept =
    again.head.innerHTML === meta + HOST_STYLE &&
    hosted(host) === card &&
    again.body.children[0].getAttribute('title') === title;
  return kept ? { srcdoc: host } : { refused: 'escaped' };
}
