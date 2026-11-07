### Added

- `scripts/perf-census.py`, the bench census (SPEC-630, ADR-652): `derive` reads the listing
  callgrind_annotate prints at the share, normalizes each name and writes `perf/hot-set.json` and
  `perf/bench-reach.json`; `check` judges the two files against the benches the tree defines and
  the release tags by arm (`shape`, `unreached`, `stale`, `void`, and the informational `edge`),
  report-only until `--enforce`.
