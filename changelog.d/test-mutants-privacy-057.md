### Changed

- Every mutant of the privacy crate is now killed by a test: the words of an export mismatch
  (`ExportProblem`'s display form), the only mutant the weekly battery left unexplained, are
  pinned by a test.
- The export's format name, `deckstreak.export.v1`, is pinned by a test in the privacy crate and by row S05780, since the weekly battery never mutates a constant.
