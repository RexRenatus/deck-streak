### Changed

- The packs stay box-only, as the owner decided, and no pack runner is published (ADR-056). Public
  CI keeps running DeckStreak's own gate, including the vendored probes. The packs built into the
  maintainer's runner keep running on the maintainer's box against the pinned commit. The pack
  runner script and the vendored manifest now say so.
