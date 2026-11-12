### Fixed

- The review fixture's day tests no longer fail when a run's process gets the id of an earlier
  run that left its scratch directory behind: the directory is removed before the test writes it
  (#779; SPEC-615).
