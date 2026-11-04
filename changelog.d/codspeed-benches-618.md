### Added

- `scripts/perf-plan.py`, the plan of each CodSpeed bench lane: a push always applies, a pull
  request applies when its diff reaches the lane, and a fork pull request builds and never uploads,
  since GitHub mints a fork no OIDC token (SPEC-618, ADR-640).
- `scripts/perf-record.py check`, the checker of the regression record a pull request carries when
  it keeps a slower change: a closed schema, refused by arm.
