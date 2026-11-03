### Fixed

- The weekly mutation battery's sweep of the whole Mini App is no longer cut at 100 minutes before
  it writes its report. Its bound, and the `mutation-web` check's, is now 325 minutes, re-derived
  by SPEC-379's rule from the sweep's measured static mutants, which run every test and come last,
  and the rest, each at its worst measured cost, with the same margin of one and a half; a test
  holds both jobs inside 325 to 420 minutes (SPEC-379).
