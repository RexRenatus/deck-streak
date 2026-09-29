### Changed

- A mutation run for one package now sizes its number of parallel legs from that package's own
  list of mutants, so a small package no longer fans out to thirty-two legs. The same mutants run
  with the same limits, and the whole-tree and scheduled runs keep thirty-two legs.
