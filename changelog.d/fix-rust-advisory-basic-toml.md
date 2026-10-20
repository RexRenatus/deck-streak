### Fixed

- The Rust advisory audit reads clean again: `deny.toml` admits RUSTSEC-2026-0374, an unmaintained notice
  (not a vulnerability) on `basic-toml`, which reaches `deck-streak-ffi` only through the pinned `uniffi`
  bindings generator (ADR-345). Issue #1001 tracks the move that drops it.
