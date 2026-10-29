# SPEC-512: The collection store: one opener, the schema, its migrations, the append-only review log, and ingest's readers behind one port

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
| issue | none filed; the pull request names this SPEC |
| decided by | ADR-530, under ADR-002, ADR-003, ADR-020, ADR-022 and ADR-029 |
| measured at | integration head `bae990a2` (every file this SPEC cites is read there) |
| schematic | `docs/schematics/collection-store-tables-512.md` (data flow), `docs/schematics/collection-store-components-512.md` (components), `docs/schematics/collection-store-open-512.md` (sequence); `docs/schematics/context-map.md` (changed) |
| mutation band | S51200-S51299 (no hand row: section 9) |

## 1. The problem, measured

DeckStreak keeps no collection of its own: every note, card and review it reads lives in Anki's schema-18 file, which
ingest copies and reads (`crates/ingest/src/reader.rs:1-14`). The store that will replace it must be one design on the
server and on both clients, and it must not disturb the live read. What was read, at `bae990a2`:

- **No crate of the workspace names `rusqlite`.** `git grep -l rusqlite -- crates` reads 0 files; the lock holds
  rusqlite 0.39.0 and libsqlite3-sys 0.37.0 only through the engine (`Cargo.lock:4323-4326`, `:2894-2897`), and the
  engine's own manifest asks for its `bundled` feature.
- **The bundled SQLite is 3.51.3** (`libsqlite3-sys-0.37.0/sqlite3/sqlite3.c:470`), so `STRICT` tables and the JSON
  functions are built in.
- **The kernel opens the service's database in one place** with WAL, `synchronous=NORMAL`, `foreign_keys=ON` and a
  busy timeout of 30,000 ms (`crates/kernel/src/db.rs:1-11`, `:26`), and opens a foreign file read-only beside it.
- **Ingest's SQL is spread over seven files.** Thirteen statement constants sit in `reader.rs:37-53` (4),
  `study_days.rs:15-18` (1), `window.rs:34-36` (1), `gate.rs:31-38` (2), `structure.rs:70-86` (4) and
  `engine.rs:442-443` (1); six more are written inline in `skip_write.rs:719-772`, inside `studied_after` and
  `read_counts`. Each runs on the copy, read-only: twelve through the kernel's foreign connection, `CARD_STATE` on the
  engine's own connection.
- **The context map** gives ingest one edge, to the kernel (`docs/CONTEXT-MAP.md:14`), and its anti-corruption row
  names "Anki's collection schema, its sync protocol" (`:126`). The schematic's census holds every crate directory
  to one node (`scripts/tests/test_context_map_schematic.py`).
- **`privacy.json` reads one SQL glob,** `migrations/*.sql` (`privacy.json:445`), and its every category names tables
  of the service's database; the symmetry census refuses a category table that no service port exports or erases
  (`crates/coordination/tests/data_rights_symmetry.rs:655-689`).
- **The engine's golden of schema-18 collections is in the tree:** `tools/parity-oracle/goldens/
  anki.read.reader_fixture.json` holds 112 cases (normal, suspended, buried, filtered-deck, notetype-kinds, review
  types 0 to 5, and 24 drawn), each a synthetic collection and the schema-18 rows the engine wrote for it, keyed by
  corpus index.
- **CI** builds the study day crate for `wasm32-unknown-unknown` and `aarch64-apple-ios` on Ubuntu
  (`.github/workflows/ci.yml:1065-1068`); the Apple workflow builds the umbrella FFI crate for iOS on the Xcode image
  (`.github/workflows/xcframework.yml:57-65`).
- **Not measured:** the store's weight in either client's binary, the journal mode under the web engine's file system
  on `wasm32`, and a build of the store for either client target (section 7).

## 2. Requirements

R1. **One opener.** `crates/collection`, package `deck-streak-collection`, is the only file set of the workspace that
builds a `rusqlite` connection, and `crates/collection/src/store.rs` the only file that calls a connection
constructor. `Store::open(path, role)` reads `application_id` and `user_version` before it writes anything, then sets
`foreign_keys=ON`, `secure_delete=ON`, a busy timeout of 30,000 ms, WAL on native targets, and `synchronous=FULL` for
the server's store or `synchronous=NORMAL` for a device's. `Store::open_reader(path)` opens an existing, migrated store
read-only and `query_only`, and refuses every write. A store opened in a role other than the one it was made in is
refused. A writer begins `IMMEDIATE`; a read runs in one deferred transaction.

R2. **Migrations to head under `user_version`.** The store's migrations are embedded from `crates/collection/migrations/`
in sqlx's grammar, `<VERSION>_<DESCRIPTION>.sql`. Each pending migration runs in one `BEGIN IMMEDIATE` transaction that
ends with `PRAGMA user_version = n`. A store whose `user_version` is above the newest migration is refused and never
downgraded. After the migrations an empty file is a store at head whose `integrity_check` reads `ok` and whose
`foreign_key_check` is empty.

R3. **The tables, each with its checks.** One file per user holds 20 `STRICT` tables: the store's own record
(`store_meta`), the study rule, the erase gate, notetypes, fields, templates, notes, note tags, note field history,
presets, decks, cards, the review log, retractions, the client record, the outbox, applied operations, the undo
journal, the schedule and the device setting. Engine-only columns have no column here: cards carry no engine card
fields and the review log no engine interval columns, which an import keeps beside them. The review log carries the
basis key and the server's order (`server_seq`), and its scheduler column holds the version that committed the row.
Every value a check bounds is refused outside its bound.

R4. **The append-only log and the erase gate.** An `UPDATE` of a row of `review_log`, `review_retraction`,
`review_client` or `note_field_history` aborts. A `DELETE` of one aborts unless the erase gate's one row reads open;
a missing gate row reads closed. The gate is closed at creation, and outside the migrations only
`crates/collection/src/data_rights.rs` names it. On the server's store a review row without its `server_seq` is
refused; a device's row may hold none.

R5. **The statement census.** Every SQL statement of the store is a constant of `crates/collection/src/sql.rs` or
`crates/collection/src/data_rights.rs`, listed in the crate's statement register, and every constant of the register
prepares against a store at head. No source of the crate holds SQL text outside a registered constant. The census
prints how many statements it examined, refuses zero, and refuses a planted statement that names a missing column.

R6. **`CollectionRead`, with two adapters.** The collection crate declares `CollectionRead`, the read port the
server's contexts read through, with the store adapter as its implementation over a read of the store. Ingest gains
`crates/ingest/src/schema18.rs`, the schema-18 adapter: the 19 statements of section 1 move into it byte for byte,
the seven files import them from it, and it implements `CollectionRead` over a read-only, `query_only` schema-18
file opened through the collection's foreign opener. Ingest's live readers keep their driver and read exactly what
they read today; the port is dormant until the cut-over switches ingest's readers to it. The context map gains the
collection's row and ingest's edge to it, and ingest's anti-corruption row names the port.

R7. **Reader parity over the engine's golden.** For every case of `anki.read.reader_fixture.json`, a schema-18 file
built from the case's rows and a store loaded with the same collection give equal domain values through both adapters,
and each equals the value the case's rows give by the port's rules. A review's deck is its card's current home deck in
both adapters.

R8. **Each adapter refuses the other's file by `application_id`.** The store's opener and reader refuse a file whose
`application_id` is not `0x44535443` ("DSTC") and leave it untouched; an empty file is the one file it stamps. The
foreign opener refuses a file whose `application_id` is the store's.

R9. **CI builds the store for both clients' targets.** The `web-engine` job builds the crate's library for
`wasm32-unknown-unknown` with the C compiler SQLite's source needs there, and the Apple workflow builds it for
`aarch64-apple-ios` where the iOS SDK is, each with `--locked`. Since the Apple workflow then builds the crate, the
internal TestFlight lane's push filter watches it, as its census of the crates that workflow builds requires.

R10. **Privacy and the store's data-rights port.** `privacy.json` reads the store's migrations and declares the store's
tables in six categories: its notes, its cards, its review log, its client record (a category of its own), the
store's record and the device's state. The port exports every declared table as JSON. On the server an erase opens
the gate, deletes every declared table but the store's record, clears the record's import digest, closes the gate in
the same transaction, and runs `VACUUM`; on a device an erase removes the store's file. The client record's own erase
deletes its rows under the gate and keeps every review. `PRIVACY.md` states each category's basis and retention.

## 3. Acceptance criteria of SPEC-512

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | R1: the opener's pragmas by role, the read-only reader, the role check, and no other file building a connection | the stub opens with no pragma, no role check and no read-only reader | ADR-530 D2, D3 |
| A2 | R2: migrations from an empty file to head, a newer store refused | the stub embeds no migration | ADR-530 D4 |
| A3 | R3: every table `STRICT` with its columns, every planted out-of-bound row refused | the stub's file holds no table | ADR-530 D5 |
| A4 | R4: append-only rows, the gate fail-closed and named by the port alone, the server's order required | the stub's file holds no trigger | ADR-530 D6 |
| A5 | R5: the store's and the schema-18 adapter's statements prepare, every SQL text registered | the stub's registers are empty | ADR-530 D7 |
| A6 | R6 and R7: the moved statements live once in the adapter; the store adapter excludes a retracted review; the map carries the edge; both adapters read every golden case as its expected domain values | the statements sit in the seven files; the stub adapters answer nothing | ADR-530 D8, D9 |
| A8 | R8: each adapter refuses the other's file by `application_id` | the stub openers check no `application_id` | ADR-530 D3 |
| A9 | R9: CI builds the crate for `wasm32-unknown-unknown` and `aarch64-apple-ios` | none: only CI proves it (section 7) | ADR-530 D10 |
| A10 | R10: export, the server's and the device's erase, the client record's erase, and `privacy.json` against the port | the stub exports nothing and erases nothing | ADR-530 D11 |

```acceptance
A1: cargo test -p deck-streak-collection --test opener -- --exact the_opener_sets_each_roles_pragmas_and_stamps_the_application_id
A1: cargo test -p deck-streak-collection --test opener -- --exact the_reader_opens_query_only_and_refuses_every_write
A1: cargo test -p deck-streak-collection --test opener -- --exact a_store_opened_in_another_role_is_refused
A1: cargo test -p deck-streak-collection --test statement_census -- --exact no_other_file_of_the_workspace_builds_a_sqlite_connection
A2: cargo test -p deck-streak-collection --test opener -- --exact migrations_run_from_an_empty_file_to_head_under_user_version
A2: cargo test -p deck-streak-collection --test opener -- --exact a_store_one_schema_above_head_is_refused_and_never_downgraded
A3: cargo test -p deck-streak-collection --test schema -- --exact every_table_of_the_schema_is_strict_with_its_columns
A3: cargo test -p deck-streak-collection --test schema -- --exact each_check_refuses_its_planted_row
A4: cargo test -p deck-streak-collection --test append_only -- --exact a_log_row_never_changes_and_leaves_only_while_the_erase_gate_is_open
A4: cargo test -p deck-streak-collection --test append_only -- --exact a_server_review_row_needs_its_server_order
A4: cargo test -p deck-streak-collection --test append_only -- --exact only_the_data_rights_port_names_the_erase_gate
A5: cargo test -p deck-streak-collection --test statement_census -- --exact every_statement_of_the_store_prepares_against_the_migrated_schema
A5: cargo test -p deck-streak-collection --test statement_census -- --exact every_sql_text_of_the_crate_is_a_registered_statement
A5: cargo test -p deck-streak-ingest --test collection_read -- --exact every_schema_18_statement_prepares_against_an_engine_collection
A6: cargo test -p deck-streak-ingest --test collection_read -- --exact the_moved_statements_live_once_in_the_schema_18_adapter
A6: cargo test -p deck-streak-collection --test read_port -- --exact a_retracted_review_leaves_every_read_of_the_store_adapter
A6: python3 -m unittest discover -s scripts/tests -p test_context_map_schematic.py
A6: cargo test -p deck-streak-ingest --test collection_read -- --exact both_adapters_read_each_golden_collection_as_its_expected_domain_values
A8: cargo test -p deck-streak-collection --test opener -- --exact the_store_opener_refuses_a_file_of_another_application_and_leaves_it_untouched
A8: cargo test -p deck-streak-ingest --test collection_read -- --exact each_adapter_refuses_the_others_file_by_application_id
A9: cargo build --locked -p deck-streak-collection --lib --target wasm32-unknown-unknown
A9: cargo build --locked -p deck-streak-collection --lib --target aarch64-apple-ios
A9: python3 -m unittest discover -s scripts/tests -p test_testflight_workflows.py
A9: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py
A10: cargo test -p deck-streak-collection --test data_rights -- --exact the_export_writes_every_declared_table_as_json
A10: cargo test -p deck-streak-collection --test data_rights -- --exact the_server_erase_empties_every_table_through_the_gate_and_closes_it
A10: cargo test -p deck-streak-collection --test data_rights -- --exact the_client_record_erase_deletes_only_the_client_rows_and_keeps_every_review
A10: cargo test -p deck-streak-collection --test data_rights -- --exact a_device_erase_removes_the_store_file
A10: cargo test -p deck-streak-collection --test data_rights -- --exact privacy_json_declares_every_store_table_once_with_the_ports_erase_mode
A10: cargo test -p deck-streak-coordination --test data_rights_symmetry -- --exact privacy_json_names_every_table_the_ports_export_or_erase
A10: python3 -m unittest discover -s scripts/tests -p test_privacy_policy.py
```

Ingest's own suites run unchanged and stay green: the moved statements keep their text, and every reader keeps its
driver.

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/collection/Cargo.toml` | store | new: rusqlite with the bundled SQLite, serde_json for the export, thiserror; tempfile in tests |
| `crates/collection/migrations/000001_collection_base.sql` | store | new: the 20 tables, their checks, indexes and triggers |
| `crates/collection/src/lib.rs` | store | new: the crate's doc and its modules |
| `crates/collection/src/store.rs` | store | new: the one opener, the reader, the foreign opener, reads and writes |
| `crates/collection/src/migrate.rs` | store | new: the embedded migrations and their runner |
| `crates/collection/src/sql.rs` | store | new: every statement of the store and the statement register |
| `crates/collection/src/read.rs` | store | new: `CollectionRead`, its domain values and the store adapter |
| `crates/collection/src/data_rights.rs` | store | new: export, the erases and the gate |
| `crates/collection/src/error.rs` | store | new: `StoreError` |
| `crates/collection/tests/*.rs` | store tests | new: A1 to A6 and A8, A10 (six files and a support module) |
| `crates/ingest/Cargo.toml` | ingest | changed: the edges to the collection and to rusqlite |
| `crates/ingest/src/schema18.rs` | ingest | new: the 19 moved statements and the schema-18 adapter |
| `crates/ingest/src/{reader,study_days,window,gate,structure,engine,skip_write}.rs` | ingest | changed: each moved statement imported from the adapter, its text unchanged |
| `crates/ingest/src/lib.rs` | ingest | changed: the adapter's module |
| `crates/ingest/tests/collection_read.rs` | ingest tests | new: A5, A6 and A8 for the adapter |
| `crates/coordination/tests/data_rights_symmetry.rs` | coordination tests | changed: a table of the store's migrations is judged by the store's own census |
| `Cargo.toml`, `Cargo.lock` | workspace | changed: the crate's entry and rusqlite's; the crate's package and its edges |
| `docs/CONTEXT-MAP.md`, `docs/schematics/context-map.md` | docs | changed: the crate's row, ingest's edge, the anti-corruption row, the store's tables |
| `docs/LEXICON.md` | docs | changed: the store's words |
| `privacy.json`, `PRIVACY.md` | privacy | changed: the store's six categories and its migrations glob |
| `.github/workflows/ci.yml`, `.github/workflows/xcframework.yml` | CI | changed: the store's library for wasm32 and iOS |
| `.github/workflows/testflight-internal.yml`, `scripts/tests/test_testflight_workflows.py` | CI | changed: the internal lane's push filter, and its census's expected filter, watch the store's crate |
| this SPEC, ADR-530, the three schematics, `docs/red-first/SPEC-512.md`, `changelog.d/collection-store-512.md` | docs | new |

## 5. What this does NOT cover

- Any write of the domain: ids and operations, notes and cards, answers, retractions, the queue and the day. The tables
  exist; their repositories come with the changes that first write them.
- The import of a schema-18 collection into the store, and its carry of the engine's own columns.
- Switching ingest's readers or any client to the store: the port is dormant and nothing live reads the store.
- The typed readers of the JSON columns, which come with each column's first reader.
- A client linking the store, and the store's weight under the web engine's budget.

## 6. Risks

- **A moved statement changes its text.** Detected by A6, which pins each moved statement's SHA-256 to its text at the
  base, and by ingest's unchanged suites.
- **A store statement that names a missing column.** Detected by A5's census, which prepares every registered
  statement and refuses a planted misspelling.
- **A second writer of history.** Detected by A4: every update of a log row aborts, and a delete needs the gate, which
  only the data-rights port opens.
- **A foreign file opened as a store, or a newer store downgraded.** Detected by A2 and A8, which leave the refused file
  byte for byte as it was.
- **A census that reads nothing.** Each census prints its examined count and refuses zero.

## 7. What only CI proves

The store's builds for `wasm32-unknown-unknown` (`web-engine`) and `aarch64-apple-ios` (`xcframework`), A9, the whole
workspace's clippy and tests (`rust`), and the mutation jobs over the new files (`mutation-rust`), each read by name at
the head. The journal mode under the web engine's file system is not measured by this change.

## 8. Formal model

- **TLA+: NOT APPLICABLE at this change.** Nothing writes history yet; the append-only log under concurrent devices is
  modelled with the operations that first write it.
- **Lean: NOT APPLICABLE.** No arithmetic rule is added.
- **No cover moves.** Each formal entry naming an ingest file anchors an item this change leaves byte-equal
  (`skip_write.rs`'s `take`, `restore_check` and `moved_counts`; `skip.rs`; `sensitive.rs`; `settings.rs`'s
  `DECK_SEPARATOR`; `write_class_stop.rs`).

## 9. Mutation rows

None. The opener, the migrations and the adapters are killed by the crate's own suites under cargo-mutants; the band
S51200-S51299 holds no row.
