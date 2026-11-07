# ADR-621: Media files are kept as files under their hash, the index row is the commit point, orphans go at open and bytes are verified where they enter

| field | value |
|---|---|
| status | proposed |
| spec | SPEC-601 |
| builds on | ADR-530, ADR-531; amends ADR-345 D1 and ADR-400 D1; beside ADR-348; ADR-359 D1 unchanged |
| measured at | the delivery's base `74ad834d` |

## Context

The media core names a file by the SHA-256 of its bytes and gives its blob path (ADR-531), and the collection store
is one file per user behind one opener (ADR-530). No store keeps a media file, each client reads a face's media by
name from a folder, and the core's face rewrite reads references through the engine's grammar. This ADR decides where
the bytes live, what commits a put, what happens to a file a crash leaves, where bytes are verified, the layout, which
crate owns the store, how each client switches, and how the face rewrite leaves the engine.

## D1. Files, not database blobs

- **Chosen: each media file is a file of its own, and the collection store holds only its index rows.** A face's file
  runs up to the core's 4 MiB file cap, and SQLite's own measurements put the file system ahead of a blob column for
  files that large; a file also streams in and out without a transaction holding it.
- **Blobs in the store:** lost, because every media byte would pass through the one writer and grow the file that
  every erase vacuums.

## D2. The row is the commit point

- **Chosen: a put writes the file first and the `media_file` row second.** A file on disk without a committed row is
  garbage; a row in state 0 promises bytes that hash to its key. The tables are `WITHOUT ROWID, STRICT`, with checks
  on every bounded column, so a row the rules refuse never enters.

## D3. Orphans are deleted at open, never adopted

- **Chosen: opening the media store empties `tmp/` and deletes every hash path without a state-0 row.** An erase that
  commits and crashes before it unlinks leaves files this pass removes, so the erase depends on it.
- **Adopting a file whose name is a hash:** lost, because its bytes were never verified where they entered.

## D4. Verify where bytes enter, check the size where they are read, quarantine a mismatch

- **Chosen: every entry that takes bytes hashes them while they stream into `tmp/`, refuses past its cap, syncs and
  renames; `put_claimed`, the sync's and the port's entry, refuses bytes whose hash is not the claimed one before any
  rename or row.** A read is capped at its limit and its length compared with the row's size; a mismatch moves the
  file to `bad/`, sets the row to state 1 (fetch again), records the hash as wanted and reads nothing.
- **Hashing at every read:** lost for now, because its cost on the phone and in wasm is unmeasured; an idle re-hash is
  a later delivery's.

## D5. One layout per user

- **Chosen: `<root>/sha256/<blob path>` for the bytes, the root's `tmp/` folder for a put in flight, `<root>/bad/<hex>`
  for a quarantined file, one root per user.** The blob path is the media core's shipped function, so the layout and its
  proof target are one.

## D6. The store lives in the collection crate

- **Chosen: `deck-streak-collection` holds `BlobStore`, `FsBlobStore`, the index over the three tables and
  `CasReader`, and gains its edge to the media core.** `CasReader` answers its own `read(name, limit)`, so the
  collection has no edge to the engine core, and every statement stays a registered constant of `sql.rs` or
  `data_rights.rs`.
- **Edges.** The iOS adapter's new edges to the collection and the media core amend ADR-345 D1 (one adapter crate
  depending on the engine alone); the core's edge to the media core amends ADR-400 D1 (the core's one workspace
  edge); the web engine's edge to the media core sits in its all-target table, beside ADR-348's wasm32-only table.

## D7. The two client readers are built switched off

- **Chosen: `StoreMedia` (iOS) and the handed files (web) are built and tested, and no client selects them yet.**
  Each client switches when it opens its store: the iOS adapter replaces `MediaFolder` with `StoreMedia` there, and
  `MediaFolder` leaves in that change; the Worker carries hashes in place of names there, and the web core admits a
  handed file only when its bytes hash to its hash. Until then `CardFace`, `MediaFolder`, `study.rs`, `wasm.rs` and the
  Worker are unchanged, so public builds keep the live path.

## D8. The face rewrite reads the media core's grammar

- **Chosen: `rewrite(html, to)` in the media core calls `to` for each reference the grammar finds, in the engine's
  order (every tag's, then every sound tag's), and writes the answer in the reference's place, every other byte
  kept; `Budget::rewrite` calls it.** It visits a remote reference too, which the face's rules refuse, while `scan`
  drops it. `scan.rs` stays as it is, so the rewrite finds each place by the same rules over `scan`'s public
  constants and `each_match`, and a test holds its names equal to `scan`'s over every input of both goldens.
- **Held:** the media references golden, 854 cases, with 0 divergences; the 18 face tests and the review templates'
  golden byte-equal. Every cap, table, budget rule and `data:` writing is unchanged: ADR-359 D1 is unchanged.

## D9. `k_put` behind `cfg(kani)`, VOID until the verifier is installed

- **Chosen: the put's control flow is a function over an abstract hasher and sink, and `k_put` drives it behind
  `cfg(kani)`, harness only.** The crate gets no feature table and no witness feature, because the progression census
  counts every member's features against its witness list. A native test holds the harness text as written. K-put
  stays `planned` in the ledger, with its `at`.

## Consequences

- The store is dormant: nothing live reads or writes it, and each client's switch is its own later change.
- The collection's schema is 2; a store at 1 migrates on its next writer open.
- The face rewrite no longer reads the engine's grammar; the engine core keeps the engine for rendering.
