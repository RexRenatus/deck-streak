### Fixed

- The predecessor's table register in `docs/CONTEXT-MAP.md` no longer lists `xp_settlement`, a
  DeckStreak-only table, or a second `buffs` row (issue #420): it holds 64 unique names, equal to
  the number its prose states, and a docs test counts it. "DeckStreak's own tables" is unchanged.
- SPEC-024 gains an insert-only amendment stating the log-capture killer's own exemption from its
  census, and its red-first record names the nested-capture test and carries the unfiltered
  passed-test count (issue #511).
