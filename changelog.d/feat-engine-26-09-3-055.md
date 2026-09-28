### Changed

- Anki's engine moves from tag 26.05 to 26.09.3 (SPEC-055). The dependency keeps naming the
  upstream tag, and a `[patch]` entry replaces it with a commit of the maintainer's fork, pinned by
  revision: that tag plus one fix, so the engine's protobuf build script no longer reruns on every
  cargo command. A cargo command with nothing to do now finishes in under a second instead of about
  half a minute, and 685 locked packages become 477. ADR-022's budgets were measured again at the
  pinned commit, and every one holds (ADR-058, accepted).
- `deny.toml` allows exactly the fork and `ankitects/rust-url` as git sources, and drops the two
  advisory exceptions and the `Unlicense` allowance that 26.09.3 no longer needs. A guard test
  refuses any exception or allowed source the dependency graph no longer uses.
