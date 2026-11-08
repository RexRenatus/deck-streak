# SPEC-601: Media files live in a content-addressed store on the server, the phone and the browser, and a face reads them through it

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
| issue | none filed; the pull request names this SPEC |
| decided by | ADR-621, under ADR-530, ADR-531 and ADR-359 (ADR-359 D1 unchanged) |
| measured at | the delivery's base `74ad834d` (every file this SPEC cites is byte-equal there) |
| schematic | `docs/schematics/media-store-601.md` (new: the data flow from a face to the store, and the sequence of a put); `docs/schematics/context-map.md` (changed: the five new edges and the collection's own layer) |
| mutation band | S60100-S60199 (no hand row: section 9) |

## 1. The problem, measured

- **No store keeps a media file by its hash.** The media core (`crates/media`, SPEC-513) knows a file by its SHA-256
  and gives its blob path, and the collection store (`crates/collection`, SPEC-512) holds one file per user at schema
  1, but no table names a media file and no code writes, reads or deletes one. Each client still reads a face's media
  by name from a folder: the iOS adapter through `MediaFolder` (`crates/ffi/src/face.rs`), the web core through the
  names its Worker hands in (`crates/web-engine/src/study.rs`).
- **The face rewrite is the engine's.** `crates/engine-core/src/media.rs` line 8 imports the engine's media reference
  rewrite, and `Budget::rewrite` (lines 168 to 173) calls it, so the core's one face rule reads references through a
  grammar it does not own, while the media core's `scan` already reads the same references for the sweep.
- **The engine's reference finder has a golden.** `tools/parity-oracle/goldens/anki.render.media_refs.json` holds 854
  cases (an input text and the names the engine's finder reports, in order); nothing of DeckStreak's reads it yet.
- **The face suite.** `crates/engine-core/tests/face.rs` holds 18 `#[test]`: 10 build a `Memory` reader and 7 call
  the `face` helper, whose `Folder` reader reads the dispatcher's media folder; their expected texts hold every
  `data:` URL written out. `web/app/src/lib/study/review-templates.golden.json` holds the review templates' faces.

## 2. Requirements

- **R1. Three STRICT tables at schema 2.** Migration `000002_media_store.sql` creates `media_file` (a 32-byte hash
  key, its size, kind 0 to 4, state 0 bytes here, 1 bytes elsewhere or 2 swept, synced 0 or 1, and two instants),
  `media_name` (a name of 1 to 255 bytes as key, its hash referencing `media_file`, origin 1 to 3, its instant and a
  tombstone flag, 0 by default), the index `media_name_by_hash`, and `media_pin` (a hash referencing `media_file`, a
  reason 1 to 3 and an instant, keyed by hash and reason). Each is `WITHOUT ROWID, STRICT`, and `HEAD` is 2.
- **R2. The row is the commit point.** A put writes the file first, in the root's `tmp/` folder, synced and renamed
  to its hash path, and commits its `media_file` row second. Opening the media store empties the root's `tmp/` folder
  and deletes every hash path that has no state-0 row; it never adopts a file.
- **R3. A claimed hash is verified while the bytes stream.** `put_claimed`, the entry the sync and the port take,
  hashes while it streams into `tmp/` and refuses bytes whose hash is not the claimed one before any rename or row.
- **R4. A read is capped and its length checked.** `FsBlobStore::get(hash, limit)` returns at most `limit` bytes. When
  the limit exceeds the row's size, `CasReader` compares the length read with the size; on a mismatch it moves the
  file to `<root>/bad/<hex>`, sets the row's state to 1, records the hash as wanted and reads nothing.
- **R5. A face reads by name through `CasReader`.** A live `media_name` row gives the hash; state 0 reads the blob,
  state 1 or 2 records the hash as wanted and reads nothing; then the length check of R4. An unbound name or a
  tombstoned one reads nothing. `CasReader` answers its own `read(name, limit)`, so the collection has no edge to the
  engine core.
- **R6. The iOS reader over the store, built switched off.** `crates/ffi/src/store_media.rs` holds `StoreMedia`, the
  core's `Reader` over `CasReader`, answering `inlines_fonts` and `shows_video` true as `MediaFolder` answers.
  `face.rs` and `engine.rs` are unchanged: `CardFace` is byte for byte the base's and `MediaFolder` stays live until
  the iOS client opens its store (ADR-621 D7).
- **R7. The web core admits handed bytes only by their hash, built switched off.** `crates/web-engine/src/handed.rs`
  holds the hash-keyed handed files the Worker's switched protocol will carry: its constructor admits a `(hash,
  bytes)` pair only when `MediaHash::of(bytes)` is the hash, lists every refused hash, and reads an admitted file at
  most a limit. `study.rs`, `wasm.rs` and the Worker are unchanged.
- **R8. The blob layout and the put's control flow are proof targets.** `FsBlobStore` places a file at
  `<root>/sha256/` joined with `MediaHash::blob_path`, the shipped function `k_blob_path` covers. The put's control
  flow is a function over an abstract hasher and sink that `k_put` drives behind `cfg(kani)`, harness only, with no
  feature table; a native test holds the harness text as written.
- **R9. The face rewrite reads DeckStreak's own grammar.** `crates/media/src/rewrite.rs` holds `rewrite(html, to)`: it
  calls `to` for each media reference the grammar finds, in the engine's order, and writes the answer in the
  reference's place, every other byte kept. `Budget::rewrite` calls it, `crates/engine-core/src` names the engine's
  rewrite 0 times, every cap, table, budget rule and `data:` writing stays byte for byte (ADR-359 D1), and the face
  suite and the review templates' golden stay byte-equal.

## 3. Acceptance criteria of SPEC-601

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | R1: the three tables exist, STRICT, with their columns, every new check refuses its planted row, and a store at head reads `user_version` 2 | `media_file is missing`: the migration is absent | ADR-621 D1, D2 |
| A2 | R2: a file placed whose row's transaction is dropped is deleted at the next open, no row is left, and a planted state-0 file beside it is kept | the inert open pass deletes nothing | ADR-621 D2, D3 |
| A3 | R3: a claimed hash with other bytes is refused, no file under either hash, no row, `tmp/` empty | the inert `put_claimed` admits | ADR-621 D4 |
| A4 | R4: `get` returns at most the limit; a size mismatch quarantines the file to `bad/`, sets state 1 and reports the hash wanted | the inert `get` reads nothing | ADR-621 D4, D5 |
| A5 | R5: the 18 face tests pass with both test readers answering through `CasReader`; an unbound or tombstoned name reads nothing while a live one reads its bytes | each face test whose expected value holds a media file's bytes; the inert reader reads nothing | ADR-621 D6 |
| A6 | R6: a put file reads back by name through `StoreMedia`; an unbound name reads nothing; a state-1 name is wanted; both flags are true | the inert reader reads nothing | ADR-621 D7 |
| A7 | R7: a matching pair is admitted and read back capped; a pair whose bytes differ by one byte is refused and listed; an empty set admits nothing while a matching pair is admitted | the inert constructor admits every pair | ADR-621 D7 |
| A8 | R8: a put file lies at `sha256/` and its blob path; the harness text reads as written. K-blob-path and K-put: VOID (section 8) | the inert put places no file; the inert harness is not the text held | ADR-621 D9 |
| A9 | R9: the rewrite visits the engine's names in every case of the media references golden, 0 divergences, and its names equal `scan`'s; the core's source names the engine's rewrite 0 times; the review templates and the face suite stay green | the inert rewrite visits no name; the census finds the engine's rewrite at line 8 | ADR-621 D8 |

```acceptance
A1: cargo test -p deck-streak-collection --test schema -- --exact every_table_of_the_schema_is_strict_with_its_columns
A1: cargo test -p deck-streak-collection --test schema -- --exact each_check_refuses_its_planted_row
A1: cargo test -p deck-streak-collection --test media_store -- --exact a_store_at_head_reads_user_version_two
A2: cargo test -p deck-streak-collection --test media_store -- --exact a_file_without_its_row_is_deleted_at_the_next_open
A3: cargo test -p deck-streak-collection --test media_store -- --exact a_claimed_hash_with_other_bytes_is_refused
A4: cargo test -p deck-streak-collection --test media_store -- --exact get_returns_at_most_the_limit
A4: cargo test -p deck-streak-collection --test media_store -- --exact a_size_mismatch_quarantines_the_file_and_reports_it_wanted
A5: cargo test -p deck-streak-engine-core --test face
A5: cargo test -p deck-streak-collection --test media_store -- --exact an_unbound_or_tombstoned_name_reads_nothing
A6: cargo test -p deck-streak-ffi --test store_media
A7: cargo test -p deck-streak-web-engine --test handed
A8: cargo test -p deck-streak-collection --test media_store -- --exact a_file_lies_at_its_blob_path
A8: cargo test -p deck-streak-collection --test media_store -- --exact each_kani_harness_reads_as_it_was_written
A9: cargo test -p deck-streak-media --test rewrite -- --exact rewrite_visits_the_engines_reference_names
A9: cargo test -p deck-streak-engine-core --test media_refs_census -- --exact the_engine_core_names_no_engine_media_rewrite
A9: cargo test -p deck-streak-engine-core --test review_templates
```

A9 also holds `git diff --quiet 74ad834d HEAD -- web/app/src/lib/study/review-templates.golden.json`, and A6 and the seventh criterion
hold `git diff --quiet 74ad834d HEAD -- crates/ffi/src/face.rs crates/ffi/src/engine.rs` and leave
`crates/web-engine/src/study.rs`, `crates/web-engine/src/wasm.rs` and every `web/` file unchanged. The census test
plants a source line naming the engine's rewrite and reads it red, so its count can fail.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-601-media-files-live-in-a-content-addressed-store-on-the-server-the-phone-and-the-browser-and-a-face-reads-them-through-it.md` | docs | new: this SPEC |
| `docs/decisions/ADR-621-media-files-are-kept-as-files-under-their-hash-the-index-row-is-the-commit-point-orphans-go-at-open-and-bytes-are-verified-where-they-enter.md` | docs | new: the decision |
| `docs/schematics/media-store-601.md` | docs | new: the data flow and the put's sequence |
| `docs/red-first/SPEC-601.md` | docs | new: the red-first record |
| `changelog.d/media-store-601.md` | changelog | new: the fragment |
| `crates/collection/migrations/000002_media_store.sql` | collection | new: the three tables and the index |
| `crates/collection/src/blob.rs` | collection | new: `BlobStore`, `FsBlobStore`, the put's control flow |
| `crates/collection/src/media_store.rs` | collection | new: the index over the three tables, the put flow, the open pass, `CasReader` |
| `crates/collection/src/kani_proofs.rs` | collection | new: the `k_put` harness, behind `cfg(kani)` |
| `crates/collection/tests/media_store.rs` | collection | new: A1 to A5 and A8 |
| `crates/media/src/rewrite.rs` | media | new: `rewrite`, `MediaRef`, `Site` |
| `crates/media/tests/rewrite.rs` | media | new: A9 over the golden |
| `crates/engine-core/tests/media_refs_census.rs` | engine-core | new: A9's census |
| `crates/ffi/src/store_media.rs` | ffi | new: `StoreMedia` |
| `crates/ffi/tests/store_media.rs` | ffi | new: A6 |
| `crates/web-engine/src/handed.rs` | web-engine | new: the handed files |
| `crates/web-engine/tests/handed.rs` | web-engine | new: the seventh criterion |
| `crates/collection/Cargo.toml` | collection | changed: the media core edge |
| `crates/collection/src/lib.rs` | collection | changed: the modules, the re-exports, the harness module |
| `crates/collection/src/migrate.rs` | collection | changed: the second migration, `HEAD` 2 |
| `crates/collection/src/sql.rs` | collection | changed: the media statements, each registered |
| `crates/collection/src/data_rights.rs` | collection | changed: the three tables declared, erased and exported |
| `crates/collection/tests/schema.rs` | collection | changed: the three tables and their planted rows |
| `crates/collection/tests/support/mod.rs` | collection | unchanged: the new tests call no new helper |
| `privacy.json` | privacy | changed: the `collection-media` entry |
| `PRIVACY.md` | privacy | changed: the `collection-media` row |
| `crates/media/src/lib.rs` | media | changed: the rewrite module and its re-export |
| `crates/engine-core/Cargo.toml` | engine-core | changed: the media core edge; the collection and `tempfile` for tests |
| `crates/engine-core/src/media.rs` | engine-core | changed: the import and `Budget::rewrite`'s body |
| `crates/engine-core/tests/face.rs` | engine-core | changed: the `Folder` and `Memory` readers over `CasReader` |
| `crates/ffi/Cargo.toml` | ffi | changed: the collection and media core edges |
| `crates/ffi/src/lib.rs` | ffi | changed: the `store_media` module |
| `crates/web-engine/Cargo.toml` | web-engine | changed: the media core edge |
| `crates/web-engine/src/lib.rs` | web-engine | changed: the `handed` module |
| `formal/obligations.json` | formal | changed: K-put's `at` |
| `Cargo.lock` | lock | changed: four dependency lists only |
| `docs/CONTEXT-MAP.md` | docs | changed: four dependency lines, the sentences they amend, three table rows |
| `docs/LEXICON.md` | docs | unchanged: the context map's register test holds the register, never the lexicon |
| `docs/schematics/context-map.md` | docs | changed: five arrows, the collection's layer, the base, two paragraphs |
| `crates/engine-core/tests/graph.rs` | engine-core | changed: three expected dependency values |
| `.github/workflows/apple-on-change.yml` | ci | changed: two path-filter lines |
| `scripts/tests/test_ci_workflows.py` | ci | changed: two lines of `APPLE_PATHS` |
| `.github/workflows/testflight-internal.yml` | ci | changed: one path-filter line |
| `scripts/tests/test_testflight_workflows.py` | ci | changed: one line of `INTERNAL_PATHS` |
| `crates/kernel/tests/allow_list_arms.rs` | kernel | changed: the census pin counts the six new source files |
| `docs/schematics/the-app-campaigns-surfaces-each-carry-a-stride-table-whose-every-control-cites-a-line-that-holds.md` | docs | changed: the line citations below the two workflow path-filter inserts move down one line |
| `scripts/mutation-rows.d/S39400-S39499.json` | mutation | changed: one row anchor names the new base commit of the context map schematic |
| `crates/coordination/tests/relight_order.rs` | coordination | changed: the static census writes out the media store's staging-name sequence |
| `crates/ingest/tests/skip_census.rs` | ingest | changed: the engine port list drops the media module, so its planted copy is refused |

## 5. What this does NOT cover

- The switch of either client: the iOS adapter keeps `MediaFolder` and the web Worker keeps its name-keyed protocol
  (ADR-621 D7). `crates/ffi/src/face.rs`, `crates/ffi/src/engine.rs`, `crates/web-engine/src/study.rs`,
  `crates/web-engine/src/wasm.rs` and every `web/` file are unchanged.
- The media sync, the sweep's deletion of files, eviction, the editor's put and the port, each a later delivery. A
  pin row is written by none of them yet.
- A full re-hash as an idle job; verifying the hash at every read (ADR-621 D4).
- Nothing live reads or writes the store yet: the crate doc's sentence stays true.

## 6. Risks

- **A crash between file and row.** The row commits second, so a crash leaves an unreferenced file, which the next
  open deletes; a crash between row and file cannot happen, because the file is synced and renamed first.
- **A file changed on disk after it was put.** The length check catches a changed size; equal-size corruption is
  caught only by the re-hash, which is not built here.
- **A name written with an entity.** The rewrite writes the answer as `to` returns it; no face test and no golden
  case holds such a name.
- **The rewrite reads places, `scan` reads names.** `scan.rs` is unchanged here, so `rewrite.rs` finds each place by
  the same rules over `scan`'s public constants and `each_match`; A9 holds the two equal over every golden input.

## 7. What only CI proves

The whole workspace's clippy and tests (`rust`), the web engine's wasm32 build with the collection store's step
(`web-engine`), the fragment check (`fragment`) and the mutation verdict over the new files (`mutation-verdict`),
each read by name at the head. The XCTest half of A6 is UNMEASURED until the Mac gate: no iOS client opens the store
here.

## 8. Formal model

- **Kani: K-blob-path, VOID.** The existing `k_blob_path` covers the shipped `MediaHash::blob_path`, which
  `FsBlobStore` uses for its layout; `a_file_lies_at_its_blob_path` holds the layout.
- **Kani: K-put, VOID.** `k_put` drives the put's control flow over an abstract hasher and sink behind `cfg(kani)`;
  the verifier is not installed, so it is unratcheted and the ledger's `K-put` gains its `at` and stays `planned`. The
  content side is A3, with real SHA-256. `each_kani_harness_reads_as_it_was_written` holds the harness text, so a
  mutant of the harness cannot go unread.
- **Lean and TLA+: NOT APPLICABLE here.** The media sync model is a later delivery's.

## 9. Mutation rows

None. The new files' mutants are generated by cargo-mutants, and A1 to A9 are their killers. The band S60100-S60199
is claimed and unused.
