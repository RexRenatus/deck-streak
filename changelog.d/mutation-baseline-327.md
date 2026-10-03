### Fixed

- A pull request that changes `deck-streak-progression` gets a mutation verdict again (SPEC-327,
  ADR-328). Every `cargo mutants` command now runs under `--timeout 1200 --build-timeout 600`,
  admitted by the owner's signed ruling: the settle census's two tests need about 788 s, and at
  300 s every shard's unmutated baseline timed out and the verdict read VOID. The shard sizer
  charges the census 788 s for each progression mutant and once in the baseline of a plan that
  lists one, so each shard stays within its one-hour projection. The weekly battery keeps its 32
  shards until a weekly run under the new budget re-sizes it (#597).
