### Fixed

- The Rust advisory audit no longer admits `paste` or `basic-toml`: `deny.toml` drops their two
  unmaintained-crate exceptions, and forks of `frankenstein` (on `pastey`), `uniffi` (without its
  unused `paste`) and `rinja` (parsing its config with `toml`) take their place in the build
  (ADR-657). Issue #1001 tracked the `basic-toml` move.
