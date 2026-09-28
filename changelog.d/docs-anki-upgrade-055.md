### Added

- A plan to move Anki's engine from 26.05 to 26.09.3 together with the predecessor (SPEC-055,
  planned, #235). The engine keeps naming the upstream tag, and a `[patch]` entry replaces it with a
  commit of the maintainer's fork that carries the fix for the engine's rebuild on every cargo
  command, until an upstream release does (ADR-058, proposed). The schematic
  `docs/schematics/engine-pin-lifecycle.md` shows the pin's lifecycle and its removal path, and
  ADR-022 carries a note pointing at ADR-058.
