### Added

- A host role, `deckstreakd preset`, lists every preset of the private copy (its parameter field,
  whether it is on the scheduler's defaults, its desired retention, its decks and its non-new
  cards, the main preset first) and proposes the scheduler's defaults for one preset. A proposal
  records the values it would replace and prints the values, the steps that apply them, what the
  change recomputes and the undo; `verify` later settles it moved or diverged by what the copy
  holds. DeckStreak writes nothing to the collection: the owner applies the change in their own
  app (SPEC-387, ADR-401).

### Changed

- The privacy page's section on models and your data says that DeckStreak proposes the
  scheduler's defaults for a preset when you ask and keeps a record of the proposal and of the
  parameters it would replace, and the page's table names that record, which is exported and
  erased with the rest of your data.
