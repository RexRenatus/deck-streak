### Added

- Every Swift mutant row is held on each pull request and proved by its package's own macOS job
  (SPEC-397, ADR-411): one module, `scripts/swift_mutants.py`, checks every row's form on Linux,
  refuses a row that leaves while its source file stays unless an approval records it, and sweeps a
  package's rows in that package's job, each killer run bounded and the source file restored and
  checked after each mutant. Each sweeping job's time limit is sized from its measured cost, and a
  change to the module starts the macOS jobs.
