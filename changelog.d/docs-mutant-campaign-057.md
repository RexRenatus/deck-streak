### Added

- A plan for the first release that mutation testing judges (SPEC-057, planned): every surviving
  mutant is killed by a test or recorded equivalent, one delivery per crate with the vault first,
  and a table per crate that must read 0 unexplained before that release. Recounted from the weekly
  battery's dispatch run 36384080819: 310 missed Rust mutants in 33 files, and in the Mini App 53
  survived and 28 uncovered (#240). Re-listed at `dev` 16ed8e2, the release's merge diff holds all
  2,207 of the tree's mutants, in 27 shards.
- ADR-070 (proposed) records an equivalent mutant in a committed fragment per package, bound by an
  anchor to exactly one listed mutant, and excused only while the tool keeps reporting it missed; it
  rejects `exclude_re`, `mutants::skip`, a `Stryker disable` comment, a survivor budget and skipped
  files. The schematic `docs/schematics/mutation-equivalence-record.md` shows the record's path to
  the verdict, and ADR-057 carries a note pointing at ADR-070.
