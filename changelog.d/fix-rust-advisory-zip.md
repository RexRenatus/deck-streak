### Security

- The `zip` package, reached through the engine dependency `anki`, moves to its next major line to
  clear advisories RUSTSEC-2026-0367, RUSTSEC-2026-0368 and RUSTSEC-2026-0369, so the Rust advisory
  audit reads clean again. The engine fork takes only the API changes that line forces, and its
  behaviour is unchanged.
