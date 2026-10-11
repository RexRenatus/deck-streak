### Added

- The review's Undo reaches its last bury or flag, not only its last answer (SPEC-383, ADR-397,
  #753). The review keeps one undo slot: an answer, a bury or a flag, whichever came last. After a
  bury the next card's bar offers "Undo bury", and after a flag the card's bar offers "Undo flag". A
  press asks for the offer and writes nothing: a dialog names the card as one line of text and what
  the undo changes (the state a buried card returns to, or whether the undo removes the red flag,
  puts it back, or puts back the flag it replaced), with focus on "Keep it", and only its
  confirmation undoes.
- The engine core checks the undo at the write, through the same owner's exempt Undo an answer uses:
  the change must still be on the card, the card must not have synced, and the change must still be
  the front of the engine's undo queue. A later change, a sync or another card refuses it, and
  nothing moves. The restore puts back the card's queue, flag and sync mark as they were, and writes
  no review row.
- A synced change is shown with a disabled control that says so, and a refused undo of a change
  reads its own notice. The ten new messages are in all seven locales.
