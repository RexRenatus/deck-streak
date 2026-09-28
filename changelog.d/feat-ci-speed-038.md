### Changed

- CI runs the local gate in four jobs that start together: `rust` (fmt, clippy, the tests, the
  doctests and the Rust audit), `web` (the Mini App and its audit), `packs` and `hygiene` (the guard
  tests, the public scrub and the secret scan). Each calls `scripts/check.sh` with its own stages,
  and a test holds every stage to exactly one job, so none can drop out of CI.
- CI restores a cache of Rust's downloads and build (keyed on the toolchain pin and `Cargo.lock`),
  the pnpm store and Playwright's browser (keyed on the Playwright version the lockfile locks). Only
  a push to `dev` or `main` saves a cache; a pull request, a fork's included, restores and never
  saves.
- A newer push to a pull request cancels the CI run it supersedes. A push to `dev` or `main` is never
  cancelled, nor replaced while it waits.
- Each stage of `scripts/check.sh` checks its own tools, so a missing tool fails the stage that needs
  it, by name. The `toolchain` stage is gone, and the `audit` stage is split into `audit-rust` and
  `audit-web`.
- `scripts/pack-rows.py` runs the pack rows in a bounded pool (`--jobs`, by default the smaller of 8
  and the CPUs available), with each row's own timeout, the same verdicts and the rows in row order.
- `scripts/check.sh` records each stage's seconds, verdict and exit in `timings.tsv` beside its logs.

### Fixed

- The stage logs reach each CI run. They were written to a hidden directory, which the upload skips
  by default, so no run ever held them.
