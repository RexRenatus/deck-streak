### Fixed

- The review pairs test no longer fails for ten minutes before the collection's day rollover
  (SPEC-348 R1, A2). The engine reads a learning interval that would cross the rollover in days,
  so a new card's 10-minute interval read `1d` then. The test now pins the synthetic collection's
  rollover to the hour furthest from the test's read and still asserts the three intervals exactly.
