### Added

- Every pull request is mutation tested on its diff. `mutation-rust` runs cargo-mutants 27.1.0 over
  the changed Rust in place, proves the hand-proved rows the diff selects, and refuses a row that
  leaves while its target stays; `mutation-web` runs StrykerJS 10.0.0 over each changed Mini App
  file. Both are needs of `ci`. The verdict reads the tools' own reports: a missed mutant fails the
  job, and a class of production code that examined nothing is VOID, never green.
- Hand-proved rows (`scripts/mutation-rows.d/`, one band per SPEC) hold the invariants the tool
  cannot mutate: constants, methods named `new`, guards, the constant-time compare and the cookie
  flags. `scripts/mutation_rows.py` censuses and proves them, restoring each file byte for byte.
- A weekly battery (`.github/workflows/mutation-weekly.yml`) sweeps `dev` in 32 shards and files
  each file's survivors as a scrubbed, deduplicated issue. It goes live once a release carries it to
  `main`; until then a pull request that changes it runs a rehearsal.
- The mutation-rows pack is vendored and enforced; its blocking `tool-config-valid` row judges the
  tools' configurations.
