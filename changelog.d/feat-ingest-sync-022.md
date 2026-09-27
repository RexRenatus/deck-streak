### Added

- Ingest reaches Anki's own Rust engine through one port, pinned to the release the predecessor's
  package comes from. A measured spike records the engine's cold build, binary size, peak memory
  and incremental sync time against budgets fixed before anything was measured.
