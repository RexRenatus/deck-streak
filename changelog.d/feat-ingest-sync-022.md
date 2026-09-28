### Added

- Ingest reaches Anki's own Rust engine through one port, pinned to the release the predecessor's
  package comes from. A measured spike records the engine's cold build, binary size, peak memory
  and incremental sync time against budgets fixed before anything was measured, and every budget
  held.
- Ingest keeps a private copy of the collection from the configured sync server: one scheduled sync
  per study day and the owner's own trigger, the predecessor's retries, a full download when the
  server demands one, and a refusal when the server holds no collection. Nothing is ever uploaded,
  and every run is recorded with one bounded reason code.
