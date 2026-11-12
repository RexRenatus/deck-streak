# ADR-622: Anki's file formats are read by one anti-corruption crate over rusqlite, and every foreign file opens through its defensive opener

| field | value |
|---|---|
| status | proposed |
| spec | SPEC-602 |
| builds on | ADR-530 (D2 and D7 amended for foreign formats; D8 completed), ADR-095, ADR-623 |
| measured at | DeckStreak `dev` at the commit this pull request branches from |

## Context

Ingest is the anti-corruption layer for Anki, and ADR-530 D8 made its schema-18 adapter the one place the 19
statements live, "until a crate for Anki's file formats takes them". The store's `ForeignFile` opens another
application's file in place, read-only and `query_only`; nothing stops a file whose `notes` is a view, or which
carries a trigger or a virtual table, from running its own SQL in that reader, and nothing bounds what it reads.
The engine's goldens `anki.schema.snapshot`, `anki.collation.order`, `anki.blob.encode` and
`anki.read.collection_view` pin what a reader of the format must answer.

## D1. The crate, its dependencies and its context-map line

A new crate, `deck-streak-anki-format` at `crates/anki-format`, owns Anki's file formats: Anki's vocabulary (`nid`,
`mid`, `did`, `odid`, `usn`, `csum`, `sfld`, the blob layouts) lives here and nowhere else. It depends on no
DeckStreak crate, so the server and both clients can link it: its context-map line reads `depends on: nothing`, and
ingest gains the edge `anki-format`. Its `[dependencies]` are exactly the workspace's `rusqlite` with the
`collation` and `limits` features (each an empty feature in rusqlite 0.39.0's manifest, so the lock gains no
package), `unicase` (ADR-623), `sha2` and `thiserror`. A census test reads the manifest and refuses any other
entry. The crate reads no clock, opens no socket and runs no async code.

## D2. One defensive opener, and the second opener file

`src/opener.rs` is the crate's only file that builds a connection, and every foreign file the crate reads opens
through it:

1. A non-empty `-wal` beside the source is refused before any byte is copied.
2. The source's bytes are copied to a private file only the opener holds, mode 600 in a directory it made, counted
   against a `Limits` bound while they are copied; past the bound the copy is refused. The user's file is never
   opened by SQLite.
3. The copy opens with `SQLITE_OPEN_READ_ONLY`. Before any statement touches the schema the opener sets
   `SQLITE_DBCONFIG_DEFENSIVE` on and `SQLITE_DBCONFIG_TRUSTED_SCHEMA` off (rusqlite's `set_db_config`),
   `PRAGMA query_only = ON`, `PRAGMA cell_size_check = ON`, `PRAGMA mmap_size = 0`, and the `SQLITE_LIMIT_LENGTH`
   and `SQLITE_LIMIT_SQL_LENGTH` limits (rusqlite's `set_limit`, the `limits` feature).
4. It registers the `unicase` collation (rusqlite's `create_collation`, the `collation` feature).
5. It runs `PRAGMA quick_check` and refuses anything but `ok`.

This amends ADR-530 D2 and D7's one-opener rule for foreign formats: the workspace has two opener files, the
store's `crates/collection/src/store.rs` and this crate's `crates/anki-format/src/opener.rs`, and the
connection-builder census admits exactly those two. The store's `ForeignFile` is left as it is, and is the one left
to re-point when the read port switches to this crate.

The private copy's mode-600 file and the directory it makes need `std::os::unix`, which `wasm32-unknown-unknown`
lacks. That code sits behind `#[cfg(not(target_family = "wasm"))]`, with a `#[cfg(target_family = "wasm")]` arm, in
the store's precedent (`crates/collection/src/store.rs` lines 197, 232 and 305), so the wasm32 build holds.

The crate reads all 14 tables (the collection's twelve and the media database's `media` and `meta`) into its own
row types, every column kept, and the 8 blob columns as typed views plus their raw bytes: a view re-emits every
field it does not model from its raw bytes, so an unchanged view keeps them byte for byte.

## D3. The refusals, before any row is read

Five refusals come first: the copy's sha256 differs from the one the caller recorded; a non-empty `-wal` sits beside
the source; `col.ver` is not 18; the table set (`sqlite_` tables excluded) differs from the snapshot's twelve; and
`PRAGMA user_version` differs from the snapshot's pinned value. Then the schema itself: any view, trigger, virtual
table or table outside the snapshot's set is refused. A refusal names its check and the object kind or table,
never a value read from the file, so a refusal can be logged and shown.

## D4. The statements and the wire walk move here

The 19 statements move from ingest's adapter into `src/statements.rs` byte for byte, and ingest's adapter imports
them through one re-export, so every ingest path to them compiles unchanged and its statement register stays in
ingest. The wire walk moves by a rename, byte for byte, and ingest re-exports it. This completes ADR-530 D8: ingest
keeps its live drivers, and the format's text now lives in the format's crate.

## D5. The engine is a test-only fixture maker

The engine crate is a dev-dependency only, as ingest's tests use it: tests build collections and media databases
through it, and the goldens it answered are the expected values. No `src` file names it, and a source census holds
that.

## D6. No property-test crate: a fixed-seed generator

No property-test crate is admitted. The walk's and the codec's tests run fixed vectors plus a small generator seeded
with a constant in the test file, so every run reads the same inputs and prints how many it examined.

## D7. Kani harnesses behind `cfg(kani)`

K-wire-walk runs the shipped walk over a `kani::any` 16-byte array, and K-field-split runs split and join over
strings of at most 8 bytes, both behind `#[cfg(kani)]` in `src/kani_proofs.rs`. They are void until the verifier
is installed where they run; their ledger entries stay planned.

## Consequences

- Every foreign file a later reader opens (a package's inner collection, a leftover schema-18 file) has one opener
  to go through.
- Ingest's adapter no longer holds the format's text; a change to a statement is a change to this crate.
- The wasm32 and iOS builds of the crate are CI steps beside the store's.
