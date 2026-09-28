### Changed

- CI runs the engine's slow sync and budget tests, SPEC-022's two test binaries, in an `engine` job
  of their own, beside the `rust` job that runs every other test. The aggregate `ci` check needs
  both, so every test stays required. `scripts/check.sh` defines that set once, as a nextest
  filterset: its `test` stage runs every test outside it, and its new `test-engine` stage runs it,
  so each test runs in exactly one of the two. The `engine` job runs the set in two slices, one per
  runner, and a warm pull request's gate took 2m39s and 2m48s, against 4m29s in one job.
