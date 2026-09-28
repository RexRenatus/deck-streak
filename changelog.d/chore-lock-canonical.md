### Fixed

- `Cargo.lock` is back in cargo's canonical form: an unlocked resolve no longer rewrites it, so a
  tool that runs cargo without `--locked` leaves the tree clean.
