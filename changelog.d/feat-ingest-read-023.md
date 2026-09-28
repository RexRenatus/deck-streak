### Added

- The collection is read read-only inside the owner's deck scope: a card by its home deck's top-level
  name, even while a filtered deck borrows it, and a review when it is a study event, with no
  collation registered for the collection's name columns (SPEC-023).
- Each cycle reads only the last 400 days of reviews, and every older study event is one count,
  recounted once it is a rebase period stale; a recount that shrinks is logged as a self-check.
- A change gate after every sync skips the recompute only when nothing it could observe changed,
  records the skip as a `skipped` run, and runs it for a pending rescore, a failed sync or run, a new
  study day, a settings change, a new review, a changed card, or a registered deadline that came due.
- `DECKSTREAK_INCLUDE_DECKS` and `DECKSTREAK_LAW_DECK_ROOT`, with one warning at start when the
  include list cannot reach the law root.
