### Fixed

- The Rust advisory audit reads clean again: `Cargo.lock` moves `webpki-roots` (RUSTSEC-2026-0381),
  `rustls-webpki` (RUSTSEC-2026-0382) and `webpki-root-certs` (RUSTSEC-2026-0383), each to the lowest
  patch release that clears its advisory. No `Cargo.toml` changes, and no other package moves.
