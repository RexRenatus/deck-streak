### Fixed

- The streak-break cap now applies. Every celebration reads the language streak as stored before
  it is routed: each award, band-up and landmark through the one `Celebrate` door, the level-up and
  the relight. Every flush re-caps each held celebration with the stored streak at its own study
  day: the scheduled held flush, the flush after a sync, and the bot's flush after the owner's
  `/sync`. A celebration on the day the streak broke therefore renders at most at the policy's cap,
  T1. A streak that cannot be read routes and flushes nothing: the award stays owed, the relight
  stays due and the held queue keeps its holds (SPEC-326, ADR-327, #572).
