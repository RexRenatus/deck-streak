### Added

- The skip day, part one of three (SPEC-083, #108): DeckStreak records a skip once per study day and
  can undo the most recent one; a standing skip bridges both streaks without spending a freeze,
  leaves the consistency run as it was, and neither counts nor ends the governor's silent run.
- The skip tariff, read from the ladder in `economy.json`: it is taken on the skip's day by a
  debit the wallet clips to what it holds, outside the daily loss limit; an unfunded skip still
  applies and records its shortfall; an undo refunds what the skip paid, on the undo's day; and a
  retry moves no coin twice.
- The preview's fields, with no card list, and the skip tables in the owner's export and erase.
- A model of one skip per day and one charge and refund per skip, a proof of the tariff with the
  Rust test that answers its vectors, and mutation rows for the ladder, the settlement's keys and
  days, and the skip constants. The take that writes to the collection lands in a later part.
