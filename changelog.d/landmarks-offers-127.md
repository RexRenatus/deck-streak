### Added

- The historical landmarks, each 25th earned study day and each anniversary of the first study
  day, are now celebrated through the notification router (SPEC-102 part b, #127; ADR-322). The
  sync cycle offers them between the recompute's writes, after the badges, records and band-ups,
  from a cursor that only the router's answers move: a landmark the router did not answer stays
  owed, and the next recompute offers it again.
- The first run stores the predecessor's high-water mark in its exact bytes, raises at most the
  first landmark due that day, and keeps the cursor; a mark imported from the predecessor raises the
  day's landmarks and none of its history. An anniversary on a day without language study reads the
  honest variant.
- The model LandmarkOnce now covers the offers' code, and mutation rows S10205 to S10207 and
  S10223 to S10228 hold the offers, the cursor, the seed and the mark.
