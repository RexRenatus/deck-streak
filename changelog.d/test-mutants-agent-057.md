### Changed

- The agent crate meets SPEC-057: a sweep of every cargo-mutants mutant of `deck-streak-agent`
  finds each one killed by an existing test or unviable, so the crate needs no equivalence
  record. Two hand-proved rows pin the roster setting's name and the shape it must have, since
  cargo-mutants never mutates a constant, and a test now holds its row of the campaign table to
  that reading.
