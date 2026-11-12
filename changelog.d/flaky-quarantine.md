### Added

- A census reads every test file and test runner configuration for the markers that keep a
  test from running or from failing its run, and a quarantine listing where a flaky test is kept
  with the runs on one head that disagreed, still running. The listing refuses an entry with no
  such record and an entry whose test is gone; an unlisted marker is reported at warn (SPEC-677).
