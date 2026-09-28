### Added

- Every pull request is mutation tested on its diff. `mutation-rust` runs cargo-mutants 27.1.0 over
  the changed Rust in place, proves the hand-proved rows the diff selects, and refuses a row that
  leaves while its target stays; `mutation-web` runs StrykerJS 10.0.0 over each changed Mini App
  file. Both are needs of `ci`. The verdict reads the tools' own reports: a missed mutant fails the
  job, and a class of production code that examined nothing, or whose report the tool left
  partial, is VOID, never green. A comment opener inside a string opens no comment.
- Hand-proved rows (`scripts/mutation-rows.d/`, one band per SPEC) hold the invariants the tool
  cannot mutate: constants, methods named `new`, guards, the constant-time compare and the cookie
  flags. `scripts/mutation_rows.py` censuses and proves them, restoring each file byte for byte.
- A weekly battery (`.github/workflows/mutation-weekly.yml`) sweeps `dev` in 32 shards and files
  each file's survivors as a scrubbed, deduplicated issue, then fails naming any shard, rows or
  Stryker report that is missing or partial. It goes live once a release carries it to `main`; until
  then a pull request that changes it runs a rehearsal.
- The tools' configurations are judged by what each tool itself refuses
  (`python3 scripts/mutation-verdict.py configs`), with no vendored pack file.
