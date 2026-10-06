### Added

- The skip day, part two of three (SPEC-083, #108): the take's write. On the owner's confirm,
  DeckStreak converges a working copy of the collection by one normal sync, moves exactly the
  previewed review cards the search still selects with the engine's own Set Due Date, and pushes
  them by one more normal sync; a card that changed or joined since the preview is left alone and
  named, and a card studied during the take is listed to the owner.
- A full or one-way sync demand at either sync aborts the take writing nothing. A changed preview,
  a set of more than 5,000 cards, an engine day or a zone that differs from the study day, and an
  unpinned zone each refuse before any request.
- Before any card changes, each moved card's prior state is recorded and a backup of the working
  copy is written beside the private copy, owner-only, and kept only after its restore check
  passes; one backup is kept, and an erase removes every backup and nothing else.
- Counts before and after the reschedule: any count other than the review-log rows and the due
  count that moves sets the class's stop and ends the take before its push. While the stop is set,
  the preview says who set it and why, and a take refuses; a stop set during a take ends it before
  its push.
- A model of the take against another client, the stop's setters, the server and the start-up
  settlement, with one witness per property, and mutation rows for the abort, the selection, the
  prior state, the card guard, the digest, the working copy, the read-back, the backup's mode and
  the stop's one row. Nothing in the service calls the take yet; the undo, the commands and the
  wiring land in part three.
