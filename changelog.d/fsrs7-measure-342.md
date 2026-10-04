### Added

- FSRS-7's replay time, undo's review-log row and the full-sync choice path are measured
  (SPEC-342). A new context, `deck-streak-fsrs7`, takes the upstream scheduler crate's FSRS-7 from
  fsrs-rs at one pinned revision beside the engine's released crate, turns review-log rows into
  FSRS-7 items, and replays them card by card and in one batch. The `fsrs7-measure` workflow times
  the replay natively on one thread and as `wasm32-wasip1` under Node's V8, and reports both with
  their checksums and the web target's check. ADR-353 records where the crate sits and how each
  figure is taken.
- `crates/ingest/tests/undo_and_full_sync.rs` measures the engine's undo of an answer, before and
  after a normal sync, and the full-sync choice: what each side is offered, the requests each
  choice sends, and the review-log rows each choice loses, against the engine's own sync server.
