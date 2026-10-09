### Added

- A decision on grades that arrive by sync (ADR-404; refs #715). A grade recorded on another client
  and brought here by sync is outside the owner-press rule: the rule holds the one call that records
  a grade to a press made in DeckStreak's own client, and sync records no grade of its own, so the
  reviews it brings in stay as they were recorded. No code, test, table row, model or ruling changes.
