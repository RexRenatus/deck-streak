### Fixed

- The Rust advisory audit reads clean with no exception: `deny.toml` ignores no advisory and checks
  unmaintained crates across the whole graph. The engine pin moves to a fork commit that drops the
  five unmaintained `unic` crates and `rustls-pemfile`, and `matches` leaves with them (ADR-058).
  Issues #248 to #253 and #256 track them.
