### Added

- Every pull request is mutation tested on its diff, a release into `main` included.
  `mutation-plan` lists the diff's mutants with cargo-mutants 27.1.0 and takes the fewest
  round-robin shards each projected within an hour; `mutation-rust` runs one shard a job, in place;
  `mutation-rows` proves the hand-proved rows the diff selects and refuses a row that leaves while
  its target stays; `mutation-verdict` counts every shard the plan promised; `mutation-web` runs
  StrykerJS 10.0.0 over each changed Mini App file. All five are needs of `ci`. The verdict reads
  the tools' own reports: a missed mutant, or one tested in two shards, fails the job, and a class
  of production code that examined nothing, a shard that never reported, or a report the tool left
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
