### Added

- The personal board, in the progression and coordination crates (SPEC-075, #79): the best day
  among the 365 most recent stored rollups, on a tie the most recent; today's score; the language
  streak; and the level both XP tables' total reaches, equal to the predecessor's
  `ReadApiLayer.leaderboard`. `GET /api/board` answers it to the owner alone.
- The XP exchange readout (SPEC-075, #80): each source bucket's XP over the graduations of the
  distinct days it paid on, read from `xp_ledger`, `xp_settlement` and the rollups in one read
  transaction, with no rate where no card graduated; and its window, the last N study days capped
  at 3650 or every day. `GET /api/xp/exchange?days=N` answers it to the owner alone, an undefined
  rate as `null`, and refuses a `days` that is not an integer.
- A Lean proof of the bucket rule, the readout's fold (every row kept, a day counted once per
  bucket, a rate defined exactly when a card graduated), the best day and the window, checked
  against `exchange_rates` over 777 recorded vectors.
