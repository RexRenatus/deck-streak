### Changed

- `PRIVACY.md` says what DeckStreak does with your data that could be called training (SPEC-368,
  ADR-379): it does not train or fit a model on your data, it reads the scheduling parameters
  your Anki app stores, and it sends an AI duty's inputs only as the context of that one run.
