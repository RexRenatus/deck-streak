# ADR-530: The collection store is one file per user on rusqlite with the bundled SQLite, behind one opener, and ingest reads it through one port

| field | value |
|---|---|
| status | proposed |
| spec | SPEC-512 |
| builds on | ADR-002, ADR-003, ADR-020, ADR-022, ADR-029 (none amended) |
| measured at | integration head `bae990a2` |

## Context

DeckStreak is to keep its own collection: notes, cards, decks, presets and an append-only review log, in one design
the server and both clients share. Today every one of those lives in Anki's schema-18 file, which ingest copies and
reads through 19 statements spread over seven files (SPEC-512 section 1). This ADR decides the store's driver, its
file, its opener and pragmas, how a file is recognised, how its schema moves forward, its tables, how the review log
stays append-only, how every statement is checked, where ingest's statements go, what the read port carries, how CI
proves both clients' targets, and how the store meets the privacy inventory. Nothing live changes: the store is
dormant, and ingest reads exactly what it reads today.

## D1. The driver: rusqlite 0.39.0 with the bundled SQLite, admitted here

- **Chosen: `rusqlite = { version = "0.39.0", features = ["bundled"] }` in the workspace, named by the collection crate
  and by ingest alone.** It is the version and source the lock already holds through the engine (`Cargo.lock:4323-4326`),
  so the lock gains no package: its two changes are the collection's own package and the edges to it.
- **Measured for the admission.** rusqlite 0.39.0: licence MIT, its `.crate` 175,688 bytes, unpacked 992 KiB, checksum
  `a0d2b014...bcd4e` (the lock's). libsqlite3-sys 0.37.0, which `bundled` compiles: licence MIT, its `.crate` 5,295,554
  bytes, unpacked 20,956 KiB, carrying SQLite 3.51.3's amalgamation (public domain, 265,977 lines of `sqlite3.c`). On
  `wasm32-unknown-unknown` rusqlite links sqlite-wasm-rs 0.5.5 instead (licence MIT, its `.crate` 5,954,429 bytes),
  which the web engine already builds. MIT is on `deny.toml`'s allow list (`deny.toml:39`).
- **sqlx:** lost, because it has no `wasm32` build and no session API; the kernel keeps it for the service's own
  database (ADR-003).
- **diesel:** lost, because its iOS build and its crate count were not measured, and its schema macros add a code
  generator the clients would carry.
- **A Swift-only store on iOS:** lost, because it shares no SQL with the server or the web client.
- **rusqlite 0.40 or later:** lost, because it needs a libsqlite3-sys that cannot sit beside sqlx-sqlite 0.9.0, which
  asks for libsqlite3-sys below 0.38.0.
- **ADR-003's objection to run-time-checked statements** is answered by D7's census, which prepares every statement
  against the migrated schema in the crate's tests.

## D2. One file per user, one opener, and the pragmas by role

- **Chosen: one file per user, the same schema on a device and on the server, opened only by
  `crates/collection/src/store.rs`.** A per-user erase is then a file delete or one emptied file, never a row hunt.
  `Store::open(path, role)` sets `foreign_keys=ON`, `secure_delete=ON` (an erased row's bytes are overwritten), a busy
  timeout of 30,000 ms (the kernel's, `crates/kernel/src/db.rs:26`), and WAL on native targets. On `wasm32` the web
  engine installs its file system first and the journal mode is left as it is; its measurement is owed (SPEC-512
  section 7).
- **`synchronous` by role.** The server's store runs `FULL`, because the server answers a device only after its commit
  returns, so a committed answer must survive a power loss. A device's store runs `NORMAL`, as the kernel's database
  does, because a device keeps an operation pending until the server has judged it.
- **The role is recorded.** At creation the opener writes the store's record (`store_meta`): its role, its writer id
  (16 bytes) and its slot (0 for the server, 1 to 63 for a device). A later open in another role, writer id or slot is
  refused, so a server's file is never written as a device's.
- **A reader.** `Store::open_reader(path)` opens an existing store read-only with `query_only=ON` and refuses every
  write before it reaches SQLite; it refuses a store that is not at head.
- **No other file builds a connection**, the rule the kernel holds for sqlx (`crates/kernel/src/db.rs:9-11`); a census
  reads every `crates/*/src` file for a connection constructor. Test fixtures build their own.
- **Two pragma sets in one opener, or one for both roles:** lost, because one set either loses a server's committed
  answer or slows every device write for a guarantee a device does not need.

## D3. A file is recognised by its `application_id`

- **Chosen: the store stamps `PRAGMA application_id = 1146311747` (`0x44535443`, "DSTC") in its first migration and
  reads it before anything else at every open.** A file with another id, or an id of 0 and any schema, is refused, and
  the opener changes nothing in it: no pragma is set and no journal mode moves before the check. An empty file is the
  one file it stamps.
- **The foreign opener** (`ForeignFile::open_read_only`) opens any other SQLite file read-only and `query_only`, and
  refuses a file stamped with the store's id. The schema-18 adapter opens through it, so each adapter refuses the
  other's file.
- **Recognising a file by its tables:** lost, because a foreign file can hold a table of any name, and the check would
  run after a pragma had already touched it.

## D4. Migrations move forward under `user_version`

- **Chosen: migrations embedded from `crates/collection/migrations/`, named in sqlx's grammar
  (`<VERSION>_<DESCRIPTION>.sql`), each run in one `BEGIN IMMEDIATE` transaction ending with
  `PRAGMA user_version = n`.** The runner re-reads `user_version` inside the transaction, so two openers racing on one
  new file apply each migration once. A store above the newest migration was written by a newer app: it is refused,
  and nothing downgrades it.
- **The kernel's sqlx migrator and its `_sqlx_migrations` table:** lost, because the store is not the service's
  database and its clients link no sqlx; `user_version` lives in the file's header and needs no table.
- **The workspace's `migrations/` directory:** lost, because the kernel embeds every file there into the service's
  database (ADR-020).

## D5. The tables

- **Chosen: 20 `STRICT` tables** (SPEC-512 R3), with the checks the tests plant against. Replicated: the study rule,
  notetypes, fields, templates, notes, note tags, note field history, presets, decks, cards, the review log,
  retractions and the client record. Device-local: the store's record, the erase gate, the outbox, applied operations,
  the undo journal, the schedule and the device setting.
- **No engine column.** Cards hold no frozen engine card fields and the review log no engine interval and factor:
  those belong to the import, which keeps every source column Anki needs back in tables of its own, so no fact is
  stored twice.
- **The review log's key and order.** Each row carries its writer id and sequence (unique together), the basis key of
  the review the scheduler folded before it (`basis_t`, `basis_origin`, `basis_seq`, all three or none), the
  scheduler's version id that committed it (`scheduler`, 32 bytes), and `server_seq`, the server's order (unique).
- **The client record is a side table**, one row per review, so the category's own erase deletes rows and never edits
  a review; an absent row reads unknown.
- **The engine's protobuf blob columns become JSON columns:** the `config` of notetypes, fields, templates, presets and
  decks, each held to `json_valid`, as are the outbox's payload and the undo journal's two states. The typed readers
  that keep unknown keys come with each column's first reader.
- **A column per engine field:** lost, because the store would then carry two answers to one question, the engine's
  and its own.

## D6. The review log is append-only, and one gate admits an erase

- **Chosen: on `review_log`, `review_retraction`, `review_client` and `note_field_history`, a `BEFORE UPDATE` trigger
  that always aborts, and a `BEFORE DELETE` trigger that aborts unless the erase gate's one row reads open.** The
  gate's test is `IS NOT 1`, so a missing row reads closed. The gate is created closed, and outside the migrations only
  `crates/collection/src/data_rights.rs` names it.
- **The server's order.** A trigger refuses a review row without `server_seq` unless the store's record says it is a
  device's. A device's own row keeps no `server_seq`; the order the server gives it is recorded beside it, in the
  outbox and the applied operations, so no log row is ever updated.
- **Admitting an update that only fills a missing `server_seq`:** lost, because every exception to "a log row never
  changes" is one more path a defect can edit history through.
- **Permissions in code alone:** lost, because any statement of any later reader could then edit history; the trigger
  holds every writer, the store's own included.

## D7. Every statement is a registered constant, and one census prepares each

- **Chosen: every statement of the store is a `const` in `sql.rs` or `data_rights.rs`, listed in
  `sql::STATEMENTS`; a census prepares each against a store at head, and a source census refuses SQL text anywhere
  else in the crate.** The schema-18 adapter's statements have their own register, prepared against a collection the
  engine made. Statements run through `prepare_cached`.
- **Compile-time checked queries:** lost with sqlx (D1).

## D8. Ingest's statements move into one schema-18 adapter, and the live readers keep their driver

- **Chosen: the 19 statements move byte for byte into `crates/ingest/src/schema18.rs`, and the seven files import them
  from it.** Twelve still run through the kernel's foreign connection and `CARD_STATE` on the engine's connection, as
  today, so ingest reads exactly what it reads today. The adapter also runs them through the collection's foreign
  opener for the port. A census pins each statement's SHA-256 to its text at the base.
- **The context map.** The fence gains `deck-streak-collection`, depending on nothing, and ingest depends on it.
  Ingest's anti-corruption row names the port: Anki's collection schema is read through the port's schema-18 adapter.
- **Switching the live readers to the port now:** lost, because it changes the live path's driver beside a dormant
  store; the cut-over switches them.
- **The statements moved into the collection crate:** lost, because Anki's vocabulary then enters the store's crate;
  ingest stays the one anti-corruption layer until a crate for Anki's file formats takes them.

## D9. What the read port carries

- **Chosen: `CollectionRead` answers in domain values both files hold** (`CardRead`, `ReviewRead`, `TemplateRead`,
  `FieldRead`, `NoteRead`, `LogCounts`): deck names; the cards of a deck scope with their note, home deck, suspension
  and tags; the study reviews after a floor and their ids and count; the newest review; the notetypes' templates and
  fields of a scope; the reviewed notes; a note's fields; the cards studied after an id; and the collection's counts.
  A study review is one of kinds 0 to 3 with an ease of 1 or more; the store adapter leaves out a retracted review.
- **One rule on purpose:** a review's deck is its card's current home deck in both adapters (the card's original deck
  while a filtered deck borrows it), never the deck recorded at the answer.
- **The engine-only reads stay outside the port:** the collection's creation stamp, the cards' fingerprint, a card's
  scheduling state and the cards by queue and type read columns the store does not keep, so they remain statements of
  the schema-18 adapter alone and leave with the engine.
- **Parity.** A test builds a schema-18 file from every case of the engine's reader golden and loads the same
  collection into a store, as an import will, and both adapters must answer the case's expected values.
- **A port that answers the engine's columns:** lost, because the store adapter would have to invent them.

## D10. CI builds the store for both clients' targets

- **Chosen: the `web-engine` job builds the crate's library for `wasm32-unknown-unknown` with `clang-18` and
  `llvm-ar-18`, which SQLite's source needs there; the Apple workflow builds it for `aarch64-apple-ios` on the Xcode
  image.** Both with `--locked`.
- **Both targets on Ubuntu, as for the study day crate:** lost, because the bundled SQLite is C, and compiling it for
  iOS needs the iOS SDK's headers, which only the Xcode image carries.

## D11. The store in the privacy inventory

- **Chosen: `privacy.json` reads `crates/collection/migrations/*.sql` beside `migrations/*.sql`, and six categories
  name the store's 19 personal tables** (every table but the erase gate): `collection-notes`, `collection-cards`,
  `collection-review-log`, `collection-client-record`, `collection-store-record` and `collection-device-state`.
- **Export** writes every declared table as JSON, a blob as lowercase hex.
- **Erase.** On the server one transaction opens the gate, deletes every declared table but the store's record,
  clears the record's import digest (the record keeps its role, writer id, slot and counters, so no id is made twice),
  and closes the gate before it commits; then `VACUUM` runs, with `secure_delete` on. On a device the erase removes
  the store's file and its WAL and shared-memory files. The client record's own erase deletes its rows under the gate
  and keeps every review.
- **The symmetry census.** The service database's census judges the service's ports; a category table that is one of
  the store's tables is judged by the store's own census instead, which holds each to exactly one category with the
  port's erase mode. A category table in neither schema stays refused.
- **No declaration, because the file holds one user's data and no user column:** lost, because an erase or an export
  must still reach every table, and the inventory is where an auditor looks for them.

## Consequences

- The workspace gains one crate and no external package. Its tests run in the `rust` job; its client targets in CI
  alone.
- Nothing live reads or writes the store, so no owner data moves at this change.
- Every later write of history is held by D6's triggers and every later statement by D7's census.
