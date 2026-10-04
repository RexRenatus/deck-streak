### Added

- The app campaign's PRD (SPEC-334): a web study client and a universal iPhone and iPad client
  over Anki's engine, with the owner's own sync server behind HTTPS, and the order its deliveries
  keep.
- Ten decisions for the campaign (ADR-335 to ADR-344): the SwiftUI client over FFI, the engine on
  WASM in the browser, the owner's own taps and the never-list, FSRS-7 on one preset, the
  reorder-only rule for every other model, the sync server's move, native push through the one
  router, one universal app driven by touch, keys or an 8BitDo remote, in-app personas behind the
  persona-core gates, and the internal TestFlight lane.
- A schematic of the clients, the engine, the sync server and the one router
  (`docs/schematics/app-clients-engine-and-sync.md`).

### Changed

- The charter's constraints 2, 4 and 14, the PRD's third non-goal and the design system's first two
  principles each carry a note quoting the owner's app-surfaces ruling and naming the ADR that
  carries it; their earlier text is kept.
