### Changed

- ADR-497, SPEC-362 and SPEC-416 gain notes naming ADR-509 beside the four lines that describe
  cargo-mutants' own baseline: each Rust mutation leg now runs its own unmutated baseline under a
  time budget, and cargo-mutants skips its own. Every earlier line stays as written, and no code,
  test, requirement or row changes.
