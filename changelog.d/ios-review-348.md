### Added

- The engine side of the iPhone and iPad review screen (SPEC-348, part 1 of 2, #632): three
  ordinary pairs, the deck tree (7,4), the current deck (7,22) and the next states' intervals
  (13,24), join the native adapter's allow-list and the engine core's native column. The core
  completes a card's face as the engine's own reviewer does: the question's extracted text fills
  `{{FrontSide}}`, the AV tags are stripped from the shown text and become sound and speech clips,
  autoplay and replay follow the card's preset and the client's wish, and every media reference is
  inlined as a `data:` URL under a name rule, a closed type table and two size caps, or emptied and
  named as omitted. The native adapter returns the face as one closed HTML document, keeps a
  per-language voice choice in a small file of its own, resolves a UI test's collection directory
  from one checked launch argument, and writes a review fixture the Apple job uploads. ADR-359
  records the choices and what each was chosen against.
