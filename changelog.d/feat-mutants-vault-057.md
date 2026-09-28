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
  `deck-streak-privacy` is row 10, and the Mini App stays last.
