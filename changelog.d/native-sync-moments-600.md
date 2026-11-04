### Added

- The iPhone and iPad app runs its normal sync on its own at the session's moments: after the
  collection opens, on a return from the background with no review open, after a sign-in stores
  its key and when a review closes, beside the Sync button. After each sync the deck list is read
  again, so a new install lists the downloaded decks with no relaunch. A sync that needs a full
  sync still stops at the counts, and the download runs only from a tap (SPEC-600, ADR-620).
