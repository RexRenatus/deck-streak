### Fixed

- The public scrub's owned email rule now passes a systemd instance unit's path (a unit type, then
  `/`, `.d/`, `.wants/`, `.requires/` or `.upholds/`) and still finds a real address in every
  context. The owned copy equals its source again over every field it keeps (SPEC-056 section 9).
