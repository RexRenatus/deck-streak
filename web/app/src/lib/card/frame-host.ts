import type { FrameDocument } from './frame-document';
import { HOST_POLICY } from './policy.js';

// SPEC-407; ADR-421. The card frame's host: the document `CardFrame` sets as its `srcdoc`, whose
// one frame holds the frame document, so the card document inherits the host's policy from its
// creation.

/** The host for a frame document, under a policy the card document inherits. */
export function frameHost(card: string, title: string, policy: string = HOST_POLICY): FrameDocument {
  return { srcdoc: card };
}
