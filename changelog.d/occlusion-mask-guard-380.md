### Added

- Both reviews withhold an image occlusion card whose masks this app does not draw (SPEC-380,
  ADR-391). Shown as it renders, such a question would show the very parts its masks hide, so
  neither side of the card is shown. In its place the review shows one line: the card cannot be
  shown here, because this app does not draw its masks, and it can still be buried or flagged.
- A withheld card offers no Show Answer and no rating, on the web or in the native app, so no
  review is recorded for it and it stays due. Bury and flag work as they do on a question, and a
  bury moves the review on to the next card.
- The engine core decides it from the question the engine renders, by the engine's own mask layer
  or one of its shapes. The web line is in all seven locales, each with its own word for bury; the
  native line is English, as every native string is.
