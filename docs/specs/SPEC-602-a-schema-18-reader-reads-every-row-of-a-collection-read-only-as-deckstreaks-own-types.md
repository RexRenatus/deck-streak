# SPEC-602: A schema-18 reader reads every row of a collection read-only as DeckStreak's own types

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
| issue | none filed; the pull request names this SPEC |
| decided by | ADR-622 (the crate and its opener) and ADR-623 (the `unicase` admission), under ADR-530 and ADR-095 |
| measured at | DeckStreak `dev` at the commit this pull request branches from (every file this SPEC cites is read there) |
| schematic | `docs/schematics/anki-format.md` (components); `docs/schematics/context-map.md` (changed) |
| mutation band | S60200-S60299 (rows S60201 and S60202: section 9) |

## 1. The problem, measured

- Ingest is the anti-corruption layer for Anki, and ADR-530 D8 keeps it the one adapter of Anki's schema-18 file
  "until a crate for Anki's file formats takes them". Its schema-18 adapter (`crates/ingest/src/schema18.rs`) holds
  19 statements, and its wire walk (`crates/ingest/src/wire.rs`, ADR-095) reads protobuf blobs with no schema.
- Those statements read a subset: no reader keeps `config`, `tags`, `graves`, the media database or any blob but
  `templates.config`, so no DeckStreak type holds a whole collection.
- The one foreign opener today, the store's `ForeignFile`, opens the user's file in place, read-only and
  `query_only`. A file whose `notes` is a view, or which carries a trigger or a virtual table, runs its own SQL in that
  reader, and nothing bounds the bytes it reads.
- Four engine goldens pin what a reader must answer: `anki.schema.snapshot` (one case: the twelve tables, `col.ver`
  18, `user_version` 0), `anki.collation.order` (40 cases), `anki.blob.encode` (48 cases over the 8 blob columns)
  and `anki.read.collection_view` (112 cases).

## 2. Requirements

- **R1** One crate, `deck-streak-anki-format` at `crates/anki-format`, under the workspace lints (unsafe code
  forbidden), whose `[dependencies]` are exactly `rusqlite` (the workspace entry plus the `collation` and `limits`
  features), `unicase`, `sha2` and `thiserror`; no DeckStreak crate, no engine crate, no clock, network or async
  crate. The engine crate, `serde_json` and `tempfile` are dev-dependencies only, and no `src` file names the engine.
- **R2** One defensive opener, `src/opener.rs`, the only file of the crate that builds a connection, opens every
  foreign file the crate reads: it refuses a non-empty `-wal` beside the source before copying; copies the bytes to
  a private file (mode 600, in a directory it made) counting them against a `Limits` bound; never opens the user's
  file; opens the copy with `SQLITE_OPEN_READ_ONLY`; before any statement touches the schema sets defensive on,
  trusted schema off, `query_only`, `cell_size_check` on, `mmap_size` 0 and the length and SQL-length limits;
  registers `unicase`; and refuses anything but `ok` from `PRAGMA quick_check`.
- **R3** Five refusals before any row is read: the copy's sha256 differs from the caller's recorded one; a non-empty
  `-wal` sits beside the source; `col.ver` is not 18; the table set (`sqlite_` tables excluded) differs from the
  snapshot's twelve; `PRAGMA user_version` differs from the snapshot's pinned value. A refusal names the check and
  the object kind or table, never a value.
- **R4** A `notes` view, a trigger, a virtual table and a table outside the snapshot's set are each refused before
  any row is read; a source over its `Limits` bound is refused while it is copied; a file built from
  `tools/parity-oracle/anki/schema-snapshot.sql` with the snapshot's one `col` row opens and reads.
- **R5** All 14 tables are read, every column kept, into DeckStreak's own row types: the twelve collection tables and
  the media database's `media` and `meta`, with a count per table equal to `SELECT count(*)`.
- **R6** The `unicase` collation is registered on every connection the opener makes, and orders names as the
  engine's collation does.
- **R7** The 8 blob columns (`config.val`, `deck_config.config`, `decks.common`, `decks.kind`, `fields.config`,
  `notetypes.config`, `tags.config`, `templates.config`) are read as typed views plus their raw bytes; a view encodes
  to the engine's bytes, and an unchanged view keeps its raw bytes byte for byte.
- **R8** Each collection of `anki.read.collection_view` reads through the crate as the golden's per-table view.
- **R9** The 19 statements of ingest's schema-18 adapter live once, in `src/statements.rs`, byte for byte, and ingest
  imports them; the wire walk moves here the same way, and ingest re-exports it, so every ingest path compiles
  unchanged. This completes ADR-530 D8.
- **R10** The workspace's connection-builder census admits exactly two opener files, the store's and this crate's.
- **R11** The wire walk (truncation, unsupported wire type, the 70-bit varint limit, no progress) and the field codec
  (`join(split(s)) == s`, `split(join(fs)) == fs` when no field holds U+001F, the separator U+001F) are held by
  tests over fixed vectors and a fixed-seed generator, by two mutation rows, and by two Kani harnesses behind
  `cfg(kani)`.
- **R12** The crate builds for `x86_64-unknown-linux-gnu`, `wasm32-unknown-unknown` and `aarch64-apple-ios`; the
  context map, its schematic and the lexicon carry it.

## 3. Acceptance criteria of SPEC-602

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | R1: the manifest census and the source census | not red: the crate is born with its dependencies; its planted manifest reads red | ADR-622 D1, ADR-623 |
| A2 | R2: each setting read back, a write refused, the source unchanged, a damaged page refused | the inert opener opens the source in place with no setting | ADR-622 D2 |
| A3 | R3: the five refusals, each by name | the inert opener refuses nothing | ADR-622 D3 |
| A4 | R4: the four planted objects and the bound refused, the snapshot file read | the inert opener refuses nothing and the inert reader answers no row | ADR-622 D3 |
| A5 | R5: every column of the 14 tables kept, each count equal | the inert reader answers no row | ADR-622 D2 |
| A6 | R6 and R7: the collation golden's order and the unregistered control; each blob golden case both ways, raw bytes kept | the inert opener registers no collation, and the inert codec answers a fixed wrong value | ADR-623, ADR-622 D2 |
| A8 | R8: each collection view golden case | the inert reader answers no row | ADR-622 D2 |
| A9 | R9: each statement defined once over the workspace's sources, in `statements.rs` | the statements sit in ingest's adapter | ADR-622 D4 |
| A10 | R10: two opener files, a planted third refused by file and line | not red: the inert opener builds a connection in `opener.rs` | ADR-622 D2 |
| A11 | R11: the walk's and the codec's tests | the inert walk answers nothing and the inert codec a fixed wrong value | ADR-622 D6, D7 |
| A12 | R12: the three builds and the map's tests | the map and the push filter lack the crate | ADR-622 D1 |

```acceptance
A1: cargo test -p deck-streak-anki-format --test manifest -- --exact the_crate_depends_on_exactly_its_four_admitted_crates
A1: cargo test -p deck-streak-anki-format --test manifest -- --exact no_source_of_the_crate_names_the_engine
A2: cargo test -p deck-streak-anki-format --test opener -- --exact the_opener_sets_every_defensive_setting_on_the_copy
A2: cargo test -p deck-streak-anki-format --test opener -- --exact a_write_through_the_reader_is_refused_as_read_only
A2: cargo test -p deck-streak-anki-format --test opener -- --exact the_source_keeps_its_bytes_and_modification_time
A2: cargo test -p deck-streak-anki-format --test opener -- --exact a_copy_with_a_damaged_page_is_refused_at_the_quick_check
A3: cargo test -p deck-streak-anki-format --test refusals -- --exact a_copy_whose_digest_differs_is_refused
A3: cargo test -p deck-streak-anki-format --test refusals -- --exact a_non_empty_wal_beside_the_source_is_refused
A3: cargo test -p deck-streak-anki-format --test refusals -- --exact a_collection_of_version_17_is_refused
A3: cargo test -p deck-streak-anki-format --test refusals -- --exact a_table_set_off_the_snapshot_is_refused
A3: cargo test -p deck-streak-anki-format --test refusals -- --exact a_user_version_off_the_pin_is_refused
A4: cargo test -p deck-streak-anki-format --test refusals -- --exact a_planted_notes_view_is_refused
A4: cargo test -p deck-streak-anki-format --test refusals -- --exact a_planted_trigger_is_refused
A4: cargo test -p deck-streak-anki-format --test refusals -- --exact a_planted_virtual_table_is_refused
A4: cargo test -p deck-streak-anki-format --test refusals -- --exact a_planted_table_outside_the_snapshot_is_refused
A4: cargo test -p deck-streak-anki-format --test refusals -- --exact a_source_over_its_bound_is_refused_while_copied
A4: cargo test -p deck-streak-anki-format --test refusals -- --exact the_snapshot_file_opens_and_reads_its_one_col_row
A5: cargo test -p deck-streak-anki-format --test tables -- --exact every_column_of_the_fourteen_tables_is_kept
A5: cargo test -p deck-streak-anki-format --test tables -- --exact each_table_count_equals_its_select_count
A6: cargo test -p deck-streak-anki-format --test collation -- --exact the_unicase_order_equals_the_collation_golden
A6: cargo test -p deck-streak-anki-format --test collation -- --exact a_connection_without_the_registration_has_no_unicase
A6: cargo test -p deck-streak-anki-format --test blobs -- --exact each_blob_golden_case_encodes_and_decodes_both_ways
A6: cargo test -p deck-streak-anki-format --test blobs -- --exact an_unchanged_view_keeps_its_raw_bytes
A8: cargo test -p deck-streak-anki-format --test collection_view -- --exact each_golden_collection_reads_as_its_per_table_view
A9: cargo test -p deck-streak-ingest --test collection_read -- --exact the_moved_statements_live_once_in_the_schema_18_adapter
A10: cargo test -p deck-streak-collection --test statement_census -- --exact no_other_file_of_the_workspace_builds_a_sqlite_connection
A11: cargo test -p deck-streak-anki-format --test wire_walk -- --exact the_walk_refuses_a_truncated_varint_and_field
A11: cargo test -p deck-streak-anki-format --test wire_walk -- --exact the_walk_refuses_an_unsupported_wire_type
A11: cargo test -p deck-streak-anki-format --test wire_walk -- --exact the_walk_refuses_a_varint_at_seventy_bits
A11: cargo test -p deck-streak-anki-format --test wire_walk -- --exact the_walk_refuses_a_step_that_makes_no_progress
A11: cargo test -p deck-streak-anki-format --test wire_walk -- --exact the_walk_reads_fixed_vectors_and_seeded_messages
A11: cargo test -p deck-streak-anki-format --test field_codec -- --exact join_of_split_is_the_text
A11: cargo test -p deck-streak-anki-format --test field_codec -- --exact split_of_join_is_the_fields_without_a_separator
A11: cargo test -p deck-streak-anki-format --test field_codec -- --exact the_separator_is_the_unit_separator
A12: cargo build --locked --offline -p deck-streak-anki-format --lib
A12: python3 -m unittest discover -s scripts/tests -p test_context_map_register.py
A12: python3 -m unittest discover -s scripts/tests -p test_context_map_schematic.py
A12: python3 -m unittest discover -s scripts/tests -p test_testflight_workflows.py
```

Ingest's own suites run unchanged and stay green: the moved statements keep their text, the moved walk keeps its
bytes, and every ingest path to them compiles through a re-export.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-602-a-schema-18-reader-reads-every-row-of-a-collection-read-only-as-deckstreaks-own-types.md` | docs | new: this SPEC |
| `docs/decisions/ADR-622-ankis-file-formats-are-read-by-one-anti-corruption-crate-over-rusqlite-and-every-foreign-file-opens-through-its-defensive-opener.md` | docs | new: the crate and its opener |
| `docs/decisions/ADR-623-unicase-enters-as-a-direct-dependency-for-the-collation-schema-18-names-carry.md` | docs | new: the `unicase` admission |
| `docs/schematics/anki-format.md` | docs | new: the component schematic |
| `docs/red-first/SPEC-602.md` | docs | new: the red-first record |
| `changelog.d/anki-format-602.md` | docs | new: the changelog fragment |
| `scripts/mutation-rows.d/S60200-S60299.json` | rows | new: rows S60201 and S60202 |
| `crates/anki-format/Cargo.toml` | crate | new: the manifest of R1 |
| `crates/anki-format/src/lib.rs` | crate | new: the crate's doc and modules |
| `crates/anki-format/src/opener.rs` | crate | new: the defensive opener and its refusals |
| `crates/anki-format/src/statements.rs` | crate | new: the 19 statements, moved from ingest |
| `crates/anki-format/src/wire.rs` | crate | moved from `crates/ingest/src/wire.rs`, byte for byte |
| `crates/anki-format/src/fields.rs` | crate | new: the field codec |
| `crates/anki-format/src/blob.rs` | crate | new: the 8 blob views |
| `crates/anki-format/src/read.rs` | crate | new: the 14 tables' row types and readers |
| `crates/anki-format/src/kani_proofs.rs` | crate | new: K-wire-walk and K-field-split behind `cfg(kani)` |
| `crates/anki-format/tests/manifest.rs` | tests | new: A1 |
| `crates/anki-format/tests/opener.rs` | tests | new: A2 |
| `crates/anki-format/tests/refusals.rs` | tests | new: A3, A4 |
| `crates/anki-format/tests/tables.rs` | tests | new: A5 |
| `crates/anki-format/tests/collation.rs` | tests | new: A6 |
| `crates/anki-format/tests/blobs.rs` | tests | new: A6, the blobs |
| `crates/anki-format/tests/collection_view.rs` | tests | new: A8 |
| `crates/anki-format/tests/wire_walk.rs` | tests | new: A11, the walk |
| `crates/anki-format/tests/field_codec.rs` | tests | new: A11, the codec |
| `crates/anki-format/tests/support/mod.rs` | tests | new: the fixtures the tests share |
| `Cargo.toml` | workspace | changed: the member entry, `unicase`, the rusqlite comment |
| `Cargo.lock` | workspace | changed: the new member and ingest's edge |
| `crates/ingest/Cargo.toml` | ingest | changed: the edge to this crate |
| `crates/ingest/src/lib.rs` | ingest | changed: `wire` re-exported from this crate |
| `crates/ingest/src/schema18.rs` | ingest | changed: the 19 statements imported from this crate |
| `crates/ingest/tests/collection_read.rs` | ingest | changed: the census of where the 19 statements are defined |
| `scripts/mutation-rows.d/S09400-S09499.json` | rows | changed: three rows' target follows the moved walk |
| `crates/collection/tests/statement_census.rs` | collection | changed: the second opener file admitted |
| `docs/CONTEXT-MAP.md` | docs | changed: the crate's line, ingest's edge, its paragraph |
| `docs/LEXICON.md` | docs | changed: the crate's terms |
| `docs/schematics/context-map.md` | docs | changed: the node, ingest's arrow, the header commit |
| `.github/workflows/ci.yml` | ci | changed: the wasm32 build step |
| `.github/workflows/xcframework.yml` | ci | changed: the iOS build step |
| `.github/workflows/testflight-internal.yml` | ci | changed: the push filter line |
| `scripts/tests/test_testflight_workflows.py` | ci | changed: the internal path list line |

## 5. What this does NOT cover

- The store's `ForeignFile` and every live reader stay as they are; switching the read port to this crate's opener
  is a later change (ADR-622 D2).
- No writer, no `.apkg` or `.colpkg` package, no merge, and no web or iOS file.
- `formal/obligations.json` is unchanged: its K-wire-walk and K-field-split entries stay planned.

## 6. Risks

- The engine is a test-only fixture maker: if its collection changes shape, the goldens and these tests move together.
- A blob view models only the fields it names; every other field rides in the raw bytes and is re-emitted unchanged.

## 7. What only CI proves

- The `wasm32-unknown-unknown` build (the `web-engine` job) and the `aarch64-apple-ios` build (`xcframework.yml`).
- The whole-tree mutation row census and the hygiene censuses over every crate's sources and tests.

## 8. Formal model

Kani harnesses K-wire-walk (the shipped walk over a 16-byte array) and K-field-split (split and join over strings
of at most 8 bytes) sit behind `cfg(kani)` in `src/kani_proofs.rs`. They are not run here, because the verifier is
not installed; the tests of A11 and the rows of section 9 hold the walk and the codec.

## 9. Mutation rows

| row | target | change | killed by |
|---|---|---|---|
| S60201 | `crates/anki-format/src/wire.rs` | the varint limit moved one step, from 70 to 71 | `wire_walk::the_walk_refuses_a_varint_at_seventy_bits` |
| S60202 | `crates/anki-format/src/fields.rs` | the separator changed from U+001F to U+001E | `field_codec::the_separator_is_the_unit_separator` |
