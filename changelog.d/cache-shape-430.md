### Added

- The workflow tests hold the CI cache shape: every job that runs cargo restores the one shared
  Rust key as its class requires, only the warm job or a push to `dev` or `main` saves it, and
  every cache key hashes its lock and toolchain inputs and nothing that changes from run to run
  (SPEC-430).
