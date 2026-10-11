### Added

- The engine core holds the reorder-only input of due-card scores and its one reorder: scores keyed by the
  engine's day count and the collection's schema stamp order today's due cards, highest first, only when both
  equal the client's own; a stale, absent, switched-off or invalid score leaves the stock order, and no card
  field is written (SPEC-414, ADR-428). No client calls it yet.
