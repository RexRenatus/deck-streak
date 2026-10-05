### Added

- The card view on iPhone and iPad (SPEC-349, ADR-360): one factory in the `CardIsolation` package
  builds every card web view with seven named layers (a non-persistent store, page JavaScript off,
  a compiled rule list that blocks every load, no message handler, a navigation gate that allows
  only the first load, a window refusal, and no file access). A tree guard holds it to one
  constructor; a test-only probe host on one iPhone and one iPad simulator plants a card for every
  channel and proves each reaches its listener from a reference view and nothing from the card
  view. It closes SEC01-F14 and SEC01-F15 for iPhone and iPad.
