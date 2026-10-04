// SPEC-341 R2, R5, R6; ADR-352 D1 to D3. The card frame's policy, in one module both the page
// policy (svelte.config.js) and the card frame read. A stub until the layers are written.

/**
 * The page policy's `frame-src`.
 * @type {string[]}
 */
export const PAGE_FRAME_SRC = [];

/**
 * The card frame's `sandbox` attribute.
 * @type {string | null}
 */
export const FRAME_SANDBOX = null;

/** The card frame's own policy, the meta element `frameDocument` writes first in its head. */
export const FRAME_POLICY = '';
