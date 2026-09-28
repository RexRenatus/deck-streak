### Added

- A plan to move Anki's engine from 26.05 to 26.09.3 together with the predecessor (SPEC-055,
  planned, #235). The engine is pinned by revision to the maintainer's fork, which carries the fix
  for the engine's rebuild on every cargo command until an upstream release does (ADR-058,
  proposed). The schematic `docs/schematics/engine-pin-lifecycle.md` shows the pin's lifecycle and
  its removal path, and ADR-022 carries a note pointing at ADR-058.
