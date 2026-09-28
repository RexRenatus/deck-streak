### Changed

- No scheduled job but the sync runs the sync cycle, even after a restart (SPEC-027). ADR-037 carries
  the note, and the planned readings-jobs SPEC (SPEC-053) now reads a missed sync as not succeeded
  instead of running it, with the readings unit ordered after the sync's.
