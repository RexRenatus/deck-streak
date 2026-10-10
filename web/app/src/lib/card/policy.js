// SPEC-341 R2, R5, R6; ADR-352 D1 to D3. The card frame's policy, in one module both the page
// policy (svelte.config.js) and the card frame read, so the two cannot drift apart.

/**
 * The page policy's `frame-src` (SEC01-F13). A frame's own navigation, one the card frame starts
 * itself included, is checked against the embedding page's `frame-src`, so 'none' holds every frame
 * to the document it was given. A `srcdoc` document is not fetched, so the card frame still renders.
 * Typed as the one source it holds, which SvelteKit's directive map accepts as a source list.
 * @type {'none'[]}
 */
export const PAGE_FRAME_SRC = ['none'];

/**
 * The card frame's `sandbox` attribute: present and empty, no token. The frame gets an opaque
 * origin, and runs no script, submits no form, opens no popup and navigates no other frame.
 * @type {string}
 */
export const FRAME_SANDBOX = '';

/**
 * The card frame's own policy, the meta element `frameDocument` writes first in its head: a card
 * loads `data:` images, media and fonts and inline styles, and nothing else; no script, connection,
 * frame, object, form target or base URL.
 */
export const FRAME_POLICY =
  "default-src 'none'; img-src data:; media-src data:; font-src data:; style-src 'unsafe-inline'; form-action 'none'; base-uri 'none'";

/**
 * The host policy, the meta element `frameHost` writes in the host's head. The card document
 * inherits it from its creation, before its first byte is parsed, so it holds an image-set
 * candidate an engine fetches ahead of the card document's own policy meta.
 */
export const HOST_POLICY = "img-src data:; script-src 'none'; object-src 'none'; base-uri 'none'";
