### Added

- The repository's own Python is mutated (SPEC-087, ADR-073, accepted; #218, #219). A pull request
  runs the new job `mutation-python` over the mutants on its changed lines in the guard scripts and
  the parity oracle's Python, one job per shard, and the verdict judges the reports: a survivor
  fails, a hung mutant, a missing or partial shard and a failed restore read VOID by name, and an
  equivalent mutant is recorded, with its argument, in `scripts/mutation-equivalent.d/python.json`.
  The weekly battery gains a `python` job of sixteen shards over the whole population, and its
  `package` input accepts `python`.
- Hand-proved rows `S08701-S08725` for the runner's and the verdict's own guards and the runner's
  constants, each proved KILLED.
