// SPEC-341 R2 to R4; ADR-352 D3, D4. The card frame's document: the card's markup and CSS composed
// under the frame policy. A stub until the layers are written: it returns the card unchanged.

/** The frame document `CardFrame` sets as its `srcdoc`, or the refusal of a card that escaped it. */
export type FrameDocument =
  | { srcdoc: string; refused?: undefined }
  | { srcdoc?: undefined; refused: 'escaped' };

export function frameDocument(html: string, css: string): FrameDocument {
  void css;
  return { srcdoc: html };
}
