### Security

- The `rmcp` package moves within its declared range to clear advisory RUSTSEC-2026-0462.
- The workspace's `rusqlite`, and the engine dependency `anki` with it, move to the release line
  whose `libsqlite3-sys` bundles the fixed SQLite, clearing advisory RUSTSEC-2026-0459. The engine
  fork takes only the call sites that line forces, and the `sqlx` fork admits that
  `libsqlite3-sys` line, so the Rust advisory audit reads clean again.
