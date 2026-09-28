### Added

- The owner's data rights: `deckstreakd data export` writes every table a context exports or resets
  as one JSON document, and `deckstreakd data erase --confirm ERASE` erases them in one transaction
  that zeroes the pages it frees, then rebuilds the database file and empties its write-ahead log.
  A port that fails, or leaves a table as its declaration forbids, rolls the whole erase back, and
  the cron-fire ledger and the schema table are never touched (SPEC-021).
- One registry of every context's data-rights port, and tests that hold the tables the export
  carries equal to the tables the erase clears or resets, and every table of the schema declared by
  exactly one port, the one that owns it.
- `privacy.json` and `PRIVACY.md`: each category of the owner's data with its lawful basis and its
  retention, and every copy an erase cannot reach with its window; and the Litestream and journald
  templates whose windows the policy states.
