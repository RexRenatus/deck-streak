### Added

- Two bounds that no test observed on its own are pinned, each by a test and a hand-proved mutation
  row. An hour of the day stops at 23: `Hour::new` refuses 24 and every later value, so a rollover
  or digest hour of 24 refuses start (SPEC-020, row S02005). Launch data may be dated at most 60
  seconds ahead of the server's clock: the test spells 60 and 61 seconds literally, so a changed
  skew can no longer move the test's dates with it (SPEC-024, row S02406).
