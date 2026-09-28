### Added

- The equivalence record (SPEC-057, ADR-070, now accepted): a mutant no test can tell apart is
  recorded in `scripts/mutation-equivalent.d/<package>.json`, `miniapp.json` for the Mini App, bound
  by an anchor to exactly one listed mutant, with its reason, the evidence a reviewer checks, the
  test that reaches it and its issue. The gate's `python` stage holds every record whole
  (`mutation-verdict.py census`). The mutant keeps running: a pull request's verdict and the weekly
  battery bind every record against the whole tree's listing, count a recorded missed or survived
  mutant as equivalent, apart from unexplained, and fail a record whose mutant a test catches
  (REFUTED), that binds no mutant (STALE) or two (AMBIGUOUS), whose mutant never builds (UNNEEDED)
  or that excuses an uncovered Mini App mutant (UNCOVERED). The battery drafts no issue for an
  equivalent mutant.
- `mutation-verdict.py table` prints a battery's row per package, listed, killed, equivalent,
  unexplained and unviable, and names every listed mutant no report tested. A dispatch of the weekly
  battery takes a `package`, a crate of the workspace or `miniapp`, sweeps only it, counts only the
  reports it promises, and ends with its table line.
- The gate's `audit-rust` stage runs an unlocked resolve and fails, by name, when it rewrites
  `Cargo.lock`, which `--locked` alone accepts though the lock is not in cargo's canonical form.
- The vault's content rails gain tests of how the pack's probe reads a note: where a fence
  opens and closes, what a code span, a comment, a tag and its attributes, and each form of
  link take, how a path is percent-decoded, where its extension starts, and the line each
  rail names. They kill 138 of the mutants of `crates/vault/src/rails.rs` that the opening
  sweep left, unit tests of where a tag, a link's tail and an autolink end and of an attribute
  with no value kill 6 more, and the file's 5 equivalent mutants are recorded in
  `scripts/mutation-equivalent.d/deck-streak-vault.json`.
- The vault's other six files gain tests of what they already do: the start check's refusals and
  the failures it reports by step, the real file system's own errors, a sync the system refuses
  included, a note's key, hash and body shape, the readings tree's calendar, rails and emptied day
  folders, and the staged executor's moves, updates and new folders, the gate's classes, the class
  that examined nothing and its time limit. They kill 70 more of the vault's mutants, 6 more
  equivalent mutants are recorded (11 in all), and a test that rolls or archives a note fails
  within seconds when a mutant stops its search for a free archive name.

### Changed

- No exclusion hides a mutant: `mutation-verdict.py exclusions` refuses every `exclude_re`,
  `exclude_globs`, `examine_re`, `examine_globs` and `skip_calls` key, even an empty one, every
  `mutants::skip` and `mutants::exclude_re` attribute, every `Stryker disable` comment, and an
  excluded, ignored or static-ignored Stryker mutant. SPEC-039's A20, which let a justified
  exclusion pass, is retired.
- A push whose subject ends with a squash merge's ` (#N)` reads not-applicable, naming `#N`, the
  last number when the title carries an issue's.
- The builder brief, `.cargo/mutants.toml`, `web/app/stryker.config.json` and the survivors' issue
  drafts teach the record, and the weekly battery's header says that a dispatch runs at any branch
  that holds the workflow.
- SPEC-057's plan gives `deck-streak-agent` and `deck-streak-progression`, which gained their
  mutants after the plan's base, rows 8 and 9 of its table and their ids in the band;
  `deck-streak-privacy` is row 10, and the Mini App stays last. Both rows have their own
  criteria, A26 and A27, in the form the other crates' rows have.

### Fixed

- A pull request whose only changes under `crates/*/src` are lines inside a `#[cfg(test)]` or
  test-attribute item, which cargo-mutants never mutates, reads its Rust mutation class
  not-applicable by name, where it read VOID; one that also changes a production line still
  applies, and the plan prints each class's case by name. The empty output cargo-mutants prints
  when no mutant overlaps a diff is read as an empty listing, so a changed production line that
  no tool can mutate still reads VOID without a covering row.
