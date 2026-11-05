### Added

- `scripts/perf-plan.py`, the plan of each CodSpeed bench lane: a push always applies, a pull
  request applies when its diff reaches the lane, and a fork pull request builds and never uploads,
  since GitHub mints a fork no OIDC token (SPEC-618, ADR-640).
- `scripts/perf-record.py check`, the checker of the regression record a pull request carries when
  it keeps a slower change: a closed schema, refused by arm.
- The CodSpeed bench lanes inside `ci`: per lane a build job with no token and an upload job with
  an OIDC grant, over wave-1 benches of the scheduler, the FSRS replay, the native adapter's answer
  and face, and the web card frame, review step and review screen (SPEC-618, ADR-640).
- The regression record check on each pull request, over the records it adds or changes, against
  the benchmark list `perf/benchmarks.txt`; the record guide is `perf/regressions.d/README.md`.
- `docs/OWNER-SETUP.md` section 10, the one-time CodSpeed setup, with no token and no secret.

### Changed

- Bench files live outside the app's source tree, which Stryker mutates, and join the coverage
  exclusions.
