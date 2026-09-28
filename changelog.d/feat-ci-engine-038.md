### Changed

- CI runs the engine's slow sync and budget tests, SPEC-022's two test binaries, in an `engine` job
  of their own, beside the `rust` job that runs every other test. The aggregate `ci` check needs
  both, so every test stays required. `scripts/check.sh` defines that set once, as a nextest
  filterset: its `test` stage runs every test outside it, and its new `test-engine` stage runs it,
  so each test runs in exactly one of the two. The `engine` job runs the set in two slices, one per
  runner. A warm pull request's gate, measured without each job's wait for a runner, took 2m35s to
  3m09s, against 4m26s with those tests in the `rust` job.
