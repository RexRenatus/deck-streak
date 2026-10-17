# SPEC-364: the web client syncs through the engine's own transport from its Worker, and the full-sync write runs in the order the core's choice checks

- **Campaign row:** the app campaign, SPEC-334 row 1.4 (R8) and section 9. **Issue:** #631, part b
  (SPEC-357 section 5 names it: "no browser sync transport, no sync row in the core's tables and no
  one-way call"). **Context(s):** `deck-streak-engine-core` (`crates/engine-core`),
  `deck-streak-web-engine` (`crates/web-engine`), `miniapp` (`web/app/src`), the engine fork (its
  pin, ADR-058 and ADR-348).
- **Decided by:** `ADR-375` (this SPEC's own: where the browser's request is made, how the full-sync
  files move on `wasm32`, where the write's steps live, how the backup is made, how a part's fork
  patches are pinned, the clock and thread gates), under ADR-368 (the choice is one rule in the
  core; D5, D7 and D10 left to parts b and c), ADR-374 (the sync key sealed, opened only in the
  Worker; "SPEC-357's part b proceeds as designed and plugs its login and sync into this store's
  obtain, send and settle", line 258), ADR-358 D4 (the endpoint guard), ADR-337 (a one-way sync is
  the owner's tap), ADR-301 (write classes), ADR-058 and ADR-348 (the fork and its pin). It amends ADR-356 D6,
  which kept (1,6) refused until the full-sync path was measured and the sync's guards were built
  (M27; `ADR-375` D5).
- **Schematic:** `docs/schematics/web-sync-core.md` (this delivery adds it: the components, a normal
  sync's data flow, the full-sync write's data flow, the fork pin and the gates).
- **Status:** part b1 of three. This pull request (b1) moves the core: the table's sync rows, the
  endpoint guard on every sync call, the one-way sync as an exempt write that runs only with the
  choice's `Write`, and the write's steps (the server copy, the backup, the re-check, the write)
  as one driver both clients call. Part b2 (the browser transport and the normal sync from the
  Worker) and part b3 (the full-sync write in the browser) are section 7. **Mutation band:**
  `S364` (section 10). **Model:** `formal/tla/FullSyncChoice`, unchanged (section 8).
- **Part b2:** sections 11 to 18 record what part b2 delivered: the browser transport, the two
  sync exports, the normal sync from the Worker, persistence at every session start and the test
  server. Where they differ from section 7's outline, sections 11 to 18 hold. **Model:**
  `formal/tla/SyncCredential`, unchanged (section 16).

## 1. The problem, measured

Each figure was read at DeckStreak `dev` `dee337bc` (`D` below), in the engine fork at its pin,
commit `c538de55` (`F` below, the fork's own tree), or in the crate source the lock file pins.
Nothing was run: every figure is a read.

### 1.1 What the engine fork does on `wasm32`

| id | measured | figure | command |
|---|---|---|---|
| M1 | The fork refuses the sync transport on `wasm32` | the `wasm32` twin of `zstd_request_with_timeout` answers `NOT_IMPLEMENTED`, "sync transport is not available on wasm32" (lines 103-115) | `grep -n -E 'cfg\(target_arch = "wasm32"\)\|not available on wasm32' rslib/src/sync/http_client/io_monitor.rs` in `F` |
| M2 | That refusal is the `browser-fetch` patch, whose removal this issue owns | removal condition "a browser sync transport replaces the refusal (#631)" (ADR-058 line 206) | `git grep -n browser-fetch D -- 'docs/decisions/ADR-058-*'` |
| M3 | Every sync call runs to completion on the backend's blocking runtime | `rt.block_on` at lines 259, 302, 339, 370, 428 and 432; the abort thread runs its own `block_on` at line 377 | `grep -n block_on rslib/src/backend/sync.rs` in `F` |
| M4 | The runtime is built with every driver on `wasm32` too | `new_current_thread()` on `wasm32` (line 139), then `enable_all()` for both targets (line 140) | `sed -n '130,141p' rslib/src/backend/mod.rs` in `F` |
| M5 | The sync paths read tokio's clock | `use tokio::time::Instant` (line 23) and `Instant::now()` in `IoMonitor::new` (line 45); the full-sync progress monitor's `interval` (lines 8, 30) | `grep -n -E 'Instant\|interval' rslib/src/sync/http_client/io_monitor.rs rslib/src/sync/http_client/full_sync.rs` in `F` |
| M6 | On a platform with no timers tokio's timing functions panic, and an idle runtime panics rather than blocks | the crate's "WASM support" section | the tokio crate documentation, read through Context7 |
| M7 | A sync's abort and a media sync each start a thread | `std::thread::spawn` at lines 207 (media) and 376 (abort) | `grep -n 'thread::spawn' rslib/src/backend/sync.rs` in `F` |
| M8 | A media sync starts only on request | after a normal sync only when `input.sync_media`; after a full sync only when `server_usn` is present | `grep -n -E 'input.sync_media\|server_usn.is_some' rslib/src/backend/sync.rs` in `F` |
| M9 | A full upload reads the collection file; a full download writes a temporary file and renames it | `fs::read` (upload line 49); `new_tempfile_in_parent_of` (37), `set_check_integrity(true)` (40), `update col set ls=mod` (42) and `atomic_rename` (44) in download | `grep -n -E 'fs::read\|atomic_rename\|new_tempfile\|check_integrity\|set ls' rslib/src/sync/collection/{upload,download}.rs` in `F` |
| M10 | The fork's SQLite binding builds neither SQLite's serialize nor its backup interface; the browser's SQLite omits neither | the workspace's `rusqlite` line names `trace`, `functions`, `collation`, `bundled`, `fallible_uint` (line 120); `OMIT_DESERIALIZE` appears 0 times in the browser SQLite crate's build script | `grep -n '^rusqlite' Cargo.toml` in `F`; `grep -c OMIT_DESERIALIZE build.rs` in the `sqlite-wasm-rs` source the lock pins |
| M11 | The engine opens a collection in exclusive locking mode with a write-ahead log | `locking_mode exclusive` (line 64), `journal_mode wal` (line 68) | `sed -n '60,70p' rslib/src/storage/sqlite.rs` in `F` |
| M12 | The fork's pin and its rules | `[patch]` takes `anki` and `anki_proto` by `rev` `c538de55` (lines 163-171); one commit per patch, a tag per pin, nothing force-pushed or deleted while pinned (ADR-348); ten patch rows (ADR-058 lines 198-209) | `git show D:Cargo.toml \| sed -n '163,171p'`; `git grep -n '' D -- 'docs/decisions/ADR-058-*' \| sed -n '181,209p'` |

### 1.2 What the core holds

| id | measured | figure | command |
|---|---|---|---|
| M13 | The core admits the sync login on the native transport only, and holds neither the normal sync nor the one-way sync | `ORDINARY` 18 rows (line 113), `BackendSyncService.SyncLogin` (1,3) `web: false` (lines 113-120); `EXEMPT` 6 rows (line 246); no (1,5) and no (1,6) row; `decide` (line 295) | `git show D:crates/engine-core/src/table.rs \| grep -n -E 'SyncLogin\|pub const (ORDINARY\|EXEMPT)\|pub fn decide'` |
| M14 | The endpoint guard reads the sync login only | `check` decodes a `SyncLoginRequest` (line 39); `run` applies it to (1,3) alone (lines 115-117) | `git show D:crates/engine-core/src/login_guard.rs \| grep -n 'pub fn check'`; `git show D:crates/engine-core/src/dispatch.rs \| sed -n '112,118p'` |
| M15 | The choice is typed, and the adapter feeds it every id set | `Counted::show(offer, device, server)`, `Confirmed::backed_up(&IdSets)`, `Checked::rechecked(IdSets)`, `Ready::at_write(&IdSets)` and `Write::direction` (lines 85-347); nothing in the core fetches a server copy or makes or reads a backup | `git show D:crates/engine-core/src/full_sync.rs \| grep -n -E 'pub (const )?fn '` |
| M16 | One dispatcher holds one engine and one open collection | `struct Dispatcher` (line 46), `id_sets` (line 183) reads the open collection | `git show D:crates/engine-core/src/dispatch.rs \| grep -n -E 'pub struct Dispatcher\|pub fn id_sets'` |
| M17 | The gesture runs only through `run_exempt`, which decodes the caller's bytes | `run_exempt(gesture, input)` (line 142) | `git show D:crates/engine-core/src/dispatch.rs \| sed -n '142,151p'` |
| M18 | The containment census holds three gesture names, two entry files, and no core line | `GESTURE_NAMES` (line 53), `ENTRY_FILES` (line 56), `ENTRY_CALLS` (line 59), `CORE` (line 62); a core line costs no census edit (SPEC-345 section 8) | `git show D:crates/engine-core/tests/containment.rs \| grep -n -E '^const (GESTURE_NAMES\|ENTRY_FILES\|ENTRY_CALLS\|CORE)'` |
| M19 | The model covers the choice's transitions by digest | `@phx covers` `full_sync.rs` at `between`, `confirm`, `backed_up`, `download_ready`, `snapshot_found`, `rechecked`, `at_write`; `NoReviewLost` counts a backup and the server copy as places a review survives in | `git show D:formal/tla/FullSyncChoice/FullSyncChoice.tla \| grep -n -E '@phx\|^NoReviewLost' -A2` |
| M26 | The core's web column is held equal to the web engine's study calls, which are pinned at 16 | `parity.rs` includes `study.rs` by path (line 29) and compares the web column with `STUDY_CALLS` (lines 70, 85); `STUDY_CALLS` holds 16 pairs, none of the sync service (line 83); the web engine's `run_method_admits_only_the_study_calls` (line 60) and `the_study_calls_are_the_reviews_pairs` (line 148) pin them | `git show D:crates/engine-core/tests/parity.rs \| grep -n -E 'path = \|fn each_adapter_table_equals\|STUDY_CALLS'`; `git show D:crates/web-engine/src/study.rs \| grep -n 'pub const STUDY_CALLS'`; `git show D:crates/web-engine/tests/study.rs \| grep -n -E 'fn (run_method_admits\|the_study_calls)'` |
| M27 | ADR-356 keeps the one-way sync refused until this part | D6 names (1,6) among the pairs that stay refused (lines 165-166); its options reject admitting (1,6) while the full-sync path is unmeasured (line 117); the `EXEMPT` doc comment calls the one-way sync unlisted (`table.rs` line 244) | `git grep -n -F '(1,6)' D -- 'docs/decisions/ADR-356-*'`; `git show D:crates/engine-core/src/table.rs \| grep -n 'stay unlisted'` |

### 1.3 What the web client holds

| id | measured | figure | command |
|---|---|---|---|
| M20 | The web engine exports no sync call | exports `run_exempt` (line 297) and `credential_*` (lines 316-362); none syncs | `git show D:crates/web-engine/src/wasm.rs \| grep -n -E 'pub (async )?fn '` |
| M21 | The Worker's protocol has no sync operation | `OPS`, 17 operations (lines 9-27), the last two `credential-status` and `credential-forget` | `git show D:web/app/src/lib/engine/protocol.ts \| sed -n '9,27p'` |
| M22 | The sealed store hands a send its key and settles its answer | `SYNC_ROUTE` (line 15), `Login` (line 52), `obtain` (line 193), `forSend` (line 220), `settle` (line 254) | `git show D:web/app/src/lib/engine/credential.ts \| grep -n -E 'SYNC_ROUTE =\|export type Login\|async (obtain\|forSend\|settle)'` |
| M23 | The storage pool starts with room for six files, and grows on request | `initial_capacity: 6` (line 791); `reserve_minimum_capacity` (line 862) | `grep -n -E 'initial_capacity: *[0-9]\|pub async fn reserve_minimum_capacity' src/sahpool.rs` in the `sqlite-wasm-vfs` source the lock pins |
| M24 | The page never asks for persistent storage | `requestPersistence` is defined (line 11) and called only by the engine harness (`engine-harness/main.ts` lines 5 and 46; line 19 names its type) and its own test | `git grep -n requestPersistence D -- web/app/src web/app/engine-harness` |
| M25 | A collection the browser evicted opens as new and says so | `open` answers `existed` (line 154); the browser test "a collection lost to eviction opens as new and says so" | `git show D:crates/web-engine/src/wasm.rs \| sed -n '154,175p'`; `git grep -n 'lost to eviction' D -- web/app/tests-engine` |

**What follows.** The engine's transport is the one piece the browser lacks (M1), and the engine
reaches it from inside a blocking call (M3): a request that waits for the browser's event loop
never completes, because the Worker's loop does not turn while the engine blocks, and an idle
runtime panics (M6). The engine's clock and threads (M4, M5, M7) and its file reads and renames
(M9) are the other walls. The core holds the choice's order as types (M15) but takes every id set
on trust from the adapter, so each client would write the fetch, the backup and the re-read
itself: three steps where the order is the rule. The table's web column is held equal to the web
engine's study calls (M26), so a sync pair admitted on the web moves the web engine's list too. And the web client's storage promise (it keeps
asking for persistent storage, the server copy stays authoritative, and a deck the browser evicts
is downloaded again) is unmet today: nothing asks for persistent storage (M24), and an evicted
collection is new (M25) with no path back to the server's copy.

### 1.4 One copy for both clients

| lives in | holds | the web client adds | the iOS client adds (#633) |
|---|---|---|---|
| `crates/engine-core/src/table.rs` | which sync calls each transport may make, and that the one-way sync needs a gesture | its column | its column, when its sync lands |
| `crates/engine-core/src/login_guard.rs` | the endpoint rule for the login and every sync call; no media until part d | nothing | nothing |
| `crates/engine-core/src/one_way.rs` | the write's steps in the model's order: the server copy, the backup, the re-check, the device's re-read, the write | the paths it names | the paths it names |
| `crates/engine-core/src/full_sync.rs` | the choice's states and counts (SPEC-357) | nothing | nothing |
| `crates/engine-core/src/credential.rs` | the key's generation rule (SPEC-363) | nothing | nothing |
| `crates/web-engine/src/study.rs`, `crates/ffi/src/allow_list.rs` | each adapter's list of the pairs it admits, held equal to its column by the parity guard | `SYNC_CALLS`, beside `STUDY_CALLS` | its allow-list, when its sync lands |
| the engine fork | the transport and the file moves on each target | the `wasm32` patches (parts b2, b3) | nothing: the native engine is unchanged |
| the client's adapter | where files live, and holding the choice between the owner's taps | `crates/web-engine`, the Worker's sync module | `crates/ffi` |

The loss counts both clients show are part a's `Counts`, read by `one_way.rs`; no client counts.

## 2. Requirements (part b1)

R1. **The table.** `BackendSyncService.SyncLogin` (1,3) is admitted on the web transport too.
    `BackendSyncService.SyncCollection` (1,5) joins `ORDINARY`, `native: false` (the iOS sync is
    #633's), `web: true`. `BackendSyncService.FullUploadOrDownload` (1,6) joins `EXEMPT` as
    `ExemptWrite::OneWaySync` with `TargetKind::Collection`, so `decide` answers `NeedsGesture` for
    it on both transports. `ORDINARY` holds 19 rows and `EXEMPT` 7, and the doc comment over
    `EXEMPT` that calls the one-way sync unlisted is rewritten (M27). The web engine's `study.rs`
    gains `service::SYNC` (1) and `SYNC_CALLS`, `(1,3,"sync_login")` and `(1,5,"sync_collection")`,
    and the parity guard's web side is `STUDY_CALLS` with `SYNC_CALLS` (M26). `run_method` still
    admits `STUDY_CALLS` alone, so no sync pair reaches the engine through it (`ADR-375` D11).
R2. **One endpoint rule for every sync call.** The login guard's rule (an endpoint present,
    parsing as a URL, with no user or password, `https` or plain `http` to a loopback literal)
    checks the `SyncAuth` of every admitted (1,5) and of every one-way write, before the engine sees
    it. A (1,5) request with `sync_media: true` is refused: media is part d (#631). Each refusal is
    in the engine's error shape, naming the rule broken and never the endpoint, the user or a key.
R3. **The one-way sync runs only with the choice's write.** `run_exempt` refuses a `OneWaySync`
    gesture with `GestureRefusal::NeedsTheChoice` before it decodes anything.
    `Dispatcher::run_one_way(gesture, write, auth)`, `pub(crate)` so that only R7's
    `one_way::write` reaches it, consumes a `OneWaySync` gesture and the choice's `Write`, refuses a gesture of another write (`WrongKind`), and builds the engine's
    `FullUploadOrDownloadRequest` itself: `upload` is `write.direction() == Direction::Upload`,
    `server_usn` is absent, and `auth` has passed R2. No caller's request bytes reach the engine.
R4. **The server copy.** `one_way::count(dispatcher, answer, auth, copy)` reads the offer from the
    sync's answer, fetches the server's collection into the path `copy` by the engine's own full
    download on a private engine started with the dispatcher's own start message (its `auth`
    checked by R2 first; the private engine's collection is the empty `copy`), reads the
    device's and the copy's ids by the core's fixed statements, and answers part a's `Counted`.
    It refuses a `copy` that names the dispatcher's open collection, and a `copy` whose collection
    already holds any row: a copy is fetched only into an empty file, so a fetch never overwrites a
    backup.
R5. **The backup, made and verified by the core.** `one_way::back_up(dispatcher, confirmed, backup,
    copy)` makes the side the write replaces durable before anything destructive runs. For a
    download it writes the open collection into the path `backup` with SQLite's `VACUUM INTO`, as
    one fixed statement through the engine's database door with the path bound as a parameter, and
    reads the backup's ids from the written file on a private engine. For an upload the backup is
    the counted server copy (ADR-368 D3), and its ids are read again from `copy`. Either way it
    calls `Confirmed::backed_up` with ids read from the file, never ids the adapter passes. A
    `backup` that names the open collection or a file that holds rows is refused.
R6. **The re-check.** `one_way::recheck(dispatcher, checked, auth, fresh)` fetches a fresh server
    copy into the empty path `fresh` (R4's rules) and calls `Checked::rechecked`; a changed server
    answers a new `Counted` over the fresh copy. The stamp `rechecked` compares beside the ids is
    the upload re-check stamp of the owner's ruling, `docs/rulings/OWNER-RULING-2026-10-08-upload-recheck-stamp.md`:
    `Dispatcher::id_sets` reads it by one fixed integer statement, `fnvhash` of the greatest row
    usn over every synced table that carries one (`cards`, `notes`, `revlog`, `graves`, `decks`,
    `deck_config`, `notetypes`, `templates`, `tags` and `config`) and `col.scm`. A normal sync and
    a full upload move it; a download's restamp, which changes only `col.usn`, `col.mod` and
    `col.ls`, leaves it still. `rechecked` is unchanged (ADR-375 D13).
R7. **The write re-reads the device.** `one_way::write(dispatcher, ready, gesture, auth)` re-reads
    the open collection's ids, calls `Ready::at_write`, and only on its `Write` calls R3's
    `run_one_way`. A device that gained a row its backup lacks answers a new `Counted`.
R8. **Nothing the choice makes is deleted.** No function of this part removes a server copy, a
    fresh copy or a backup; where they are kept and how many is part c's (ADR-368 D3), and part c
    owes the model's `NoReviewLost` its reason for each removal it adds.
R9. **The census.** `run_one_way` joins `GESTURE_NAMES`: outside the core it is named only in the
    two entry files, and a planted caller in the daemon, the bot, coordination or ingest is refused
    by name. `ENTRY_CALLS` is unchanged, so the native entry owes no one-way call before #633.
    `run_one_way` is `pub(crate)`, so the entry is defence in depth: a gesture is built only by
    `from_tap`, which the census already holds to the entry files.
R10. **The evicted device restores from the server.** An empty collection syncing against a server
    that holds reviews is offered the download alone; the download's counts lose nothing of the
    device; and after the owner's write the device holds every review id the server holds. The
    restore is still the owner's tap (ADR-337).
R11. **The model is unchanged and still covers the code.** `formal/tla/FullSyncChoice` and its
    `config/formal.json` entry are unchanged, and no covered anchor of `full_sync.rs` changes text,
    so every `@phx covers` digest holds. Section 8 maps each step of `one_way.rs` to the model's
    action it realises.
R12. **The records.** ADR-301 gains the note SPEC-345 section 5 deferred to the first exempt
    tap's delivery: the one-way sync is the owner's tap through the core's choice, never a write
    class. The note names its issue (#631) and carries no date. ADR-356 gains an amendment: D6's
    (1,6) is the exempt `OneWaySync` of `ADR-375` D5, and the gesture still names one call on the
    open collection. SPEC-357 section 7 names this SPEC as part b.
R13. **Nothing else moves.** The fork's pin, the Worker, the page and the core's existing
    transitions are unchanged in b1. The web engine changes only by R1's `SYNC_CALLS` and its
    `service::SYNC` (no export; `wasm.rs` is unchanged, since its `refuse` prints any refusal's
    sentence). The native adapter gains no export: its exempt refusal gains the case
    `NeedsTheChoice`, an error case of the existing call that no native tap reaches before #633.

## 3. Acceptance criteria (part b1)

The native proofs run the engine against the engine's own sync server, started by the core's test
support as `crates/ffi/tests/support/sync_server.rs` starts it, on a loopback port.

| id | criterion | red at its tests commit | decided by |
|---|---|---|---|
| A1 | The web column admits (1,3) and (1,5); (1,5) is refused natively; (1,6) needs a gesture on both; the web engine names exactly those two sync pairs | the table unchanged: (1,3) and (1,5) refused on the web, (1,6) `NotAllowed`; `SYNC_CALLS` stubbed empty | `table::the_web_column_admits_the_sync_login_and_the_normal_sync`; `study::the_sync_calls_are_the_login_and_the_normal_sync` (web engine) |
| A2 | A normal sync or a one-way write whose endpoint breaks the rule, and a normal sync asking for media, are refused before the engine sees them, naming only the rule | the guard reads the login alone: the planted endpoints reach the engine | `login_guard::every_sync_call_reaches_only_a_guarded_endpoint_and_never_media` |
| A3 | `run_exempt` refuses a one-way gesture before decoding, and the write refuses a gesture of any other write | `checked` decodes a `OneWaySync` gesture as no message and answers `Undecodable`; a stub `run_one_way` that runs any gesture's write answers the engine's reply to a `Forget` gesture | `exempt::run_exempt_refuses_the_one_way_sync`; `one_way::a_write_takes_only_a_one_way_gesture` |
| A4 | The one-way write sends the direction the owner confirmed and no media, from the core's own request | a stub that always sends `upload: true`: the download leaves the server's reviews off the device | `one_way::the_write_sends_the_direction_the_owner_confirmed_and_no_media` |
| A5 | A server copy is never the open collection nor a file that holds rows | a stub that fetches into any path: the live collection is replaced by the server's | `one_way::a_server_copy_is_fetched_only_into_an_empty_file` |
| A6 | A download's backup is written before the write and holds every device id; the device is unchanged by it | a stub that passes the device's ids to `backed_up` and writes no file: the backup opens empty | `one_way::a_downloads_backup_holds_every_device_id_before_the_write` |
| A7 | An upload's backup is the counted server copy, read again from its file | a stub that passes the counted ids without reading the file: a copy emptied after the count passes | `one_way::an_uploads_backup_is_the_counted_server_copy` |
| A8 | A second client's sync after the count returns the upload to the counts, with its review counted | a stub re-check that compares the counted copy with itself: `Ready` | `one_way::a_server_changed_after_the_count_returns_to_the_counts` |
| A9 | A review made on the device after the backup refuses the download and returns to the counts | a stub write that passes the backup's ids to `at_write`: the review is lost | `one_way::a_device_review_after_the_backup_refuses_the_download` |
| A10 | After a write, the backup and every server copy the choice made still open with their ids | a stub that removes the copy after the upload | `one_way::the_backup_and_the_server_copies_outlive_the_write` |
| A11 | An evicted (empty) device is offered the download alone, loses nothing, and after the owner's write holds every server review | a stub `run_one_way` that refuses every write: the device holds none of the server's reviews | `one_way::an_evicted_device_is_offered_the_download_alone_and_restores_every_review` |
| A12 | Outside the core, `run_one_way` is named only in the entry files; a planted caller in each non-UI crate is refused by name; the census prints its count | `GESTURE_NAMES` without `run_one_way`: the planted caller passes | `containment::no_non_ui_caller_reaches_an_exempt_function` |
| A13 | No new dependent of the core; the core's test support adds a dev-dependency only | not red: the graph census is green at the base and must stay green; it guards the dev-dependency, its planted dependent is the target's own control | `graph` (whole target) |
| A14 | Each adapter's list still equals its transport's column, the web side read as `STUDY_CALLS` with `SYNC_CALLS` | not red: green at the base; it reddens only in a tree that flips the web column without `SYNC_CALLS` | `parity::each_adapter_table_equals_its_transport_column` |
| A15 | A second client's note edit that adds no id, synced after the count, returns the upload to the counts | not red: the header stamp refuses every re-check, so a changed server already returns to the counts | `one_way::an_edit_that_adds_no_id_returns_the_upload_to_the_counts` |
| A16 | A second client's new tag name alone, and its config change alone, each synced after the count, return the upload to the counts | not red: likewise | `one_way::a_tag_or_config_change_alone_returns_the_upload_to_the_counts` |
| A17 | A second client's removal of an empty deck, synced after the count, returns the upload to the counts | not red: likewise | `one_way::a_deletion_returns_the_upload_to_the_counts` |
| A18 | A second client's note edit dated at or below the greatest row's, synced after the count, returns the upload to the counts | not red: likewise | `one_way::an_edit_dated_at_or_below_the_greatest_returns_the_upload_to_the_counts` |
| A19 | A second client's full upload after the count returns the upload to the counts | not red: likewise | `one_way::a_full_upload_after_the_count_returns_the_upload_to_the_counts` |
| A20 | An unchanged server's copy, fetched again, carries the counted copy's stamp, and the upload is ready | the header stamp: the fetch's download moves `col.mod` | `one_way::an_unchanged_server_rechecks_equal` |
| A21 | The stamp moves with each synced table's greatest usn and with `col.scm`, and equals the engine's hash of them read apart from the core | the header stamp moves with none of them | `one_way::the_stamp_reads_every_synced_tables_greatest_usn_and_the_schema` |

```acceptance
A1: cargo test -p deck-streak-engine-core --test table -- --exact the_web_column_admits_the_sync_login_and_the_normal_sync
A1: cargo test -p deck-streak-web-engine --test study -- --exact the_sync_calls_are_the_login_and_the_normal_sync
A2: cargo test -p deck-streak-engine-core --test login_guard -- --exact every_sync_call_reaches_only_a_guarded_endpoint_and_never_media
A3: cargo test -p deck-streak-engine-core --test exempt -- --exact run_exempt_refuses_the_one_way_sync
A3: cargo test -p deck-streak-engine-core --test one_way -- --exact a_write_takes_only_a_one_way_gesture
A4: cargo test -p deck-streak-engine-core --test one_way -- --exact the_write_sends_the_direction_the_owner_confirmed_and_no_media
A5: cargo test -p deck-streak-engine-core --test one_way -- --exact a_server_copy_is_fetched_only_into_an_empty_file
A6: cargo test -p deck-streak-engine-core --test one_way -- --exact a_downloads_backup_holds_every_device_id_before_the_write
A7: cargo test -p deck-streak-engine-core --test one_way -- --exact an_uploads_backup_is_the_counted_server_copy
A8: cargo test -p deck-streak-engine-core --test one_way -- --exact a_server_changed_after_the_count_returns_to_the_counts
A9: cargo test -p deck-streak-engine-core --test one_way -- --exact a_device_review_after_the_backup_refuses_the_download
A10: cargo test -p deck-streak-engine-core --test one_way -- --exact the_backup_and_the_server_copies_outlive_the_write
A11: cargo test -p deck-streak-engine-core --test one_way -- --exact an_evicted_device_is_offered_the_download_alone_and_restores_every_review
A12: cargo test -p deck-streak-engine-core --test containment -- --exact no_non_ui_caller_reaches_an_exempt_function
A13: cargo test -p deck-streak-engine-core --test graph
A14: cargo test -p deck-streak-engine-core --test parity -- --exact each_adapter_table_equals_its_transport_column
A15: cargo test -p deck-streak-engine-core --test one_way -- --exact an_edit_that_adds_no_id_returns_the_upload_to_the_counts
A16: cargo test -p deck-streak-engine-core --test one_way -- --exact a_tag_or_config_change_alone_returns_the_upload_to_the_counts
A17: cargo test -p deck-streak-engine-core --test one_way -- --exact a_deletion_returns_the_upload_to_the_counts
A18: cargo test -p deck-streak-engine-core --test one_way -- --exact an_edit_dated_at_or_below_the_greatest_returns_the_upload_to_the_counts
A19: cargo test -p deck-streak-engine-core --test one_way -- --exact a_full_upload_after_the_count_returns_the_upload_to_the_counts
A20: cargo test -p deck-streak-engine-core --test one_way -- --exact an_unchanged_server_rechecks_equal
A21: cargo test -p deck-streak-engine-core --test one_way -- --exact the_stamp_reads_every_synced_tables_greatest_usn_and_the_schema
```

Existing tests the implementation commit grows with the table, insert-only (no assertion
weakened, each population printed): `tests/table.rs` (`WEB` 16 to 18, `HELD` 6 to 7, and the two
tests that enumerate the rows), `tests/review_pairs.rs` (`HELD` 6 to 7), `tests/gesture.rs`
(`TARGETS` 3 to 4, `TAKEN` 6 to 7), `tests/exempt.rs` (the exhaustive `target()` and the two
per-write request tables gain the `OneWaySync` arm; the loop over every exempt write expects
`NeedsTheChoice` for it, by name), `tests/parity.rs` (the web side reads `SYNC_CALLS` too), and the
web engine's and native adapter's tests R1 and R13 name.

R6's stamp changes three existing tests' oracle or fixture, and drops no assertion: the `held`
oracles of `tests/one_way.rs` and `tests/full_sync.rs` read the stamp through the support's
`stamp`, apart from the core, and
`full_sync::a_reply_that_is_not_integers_is_the_engines_database_error` plants a schema stamp
that is not an integer, which the engine's hash refuses as its own database error, since the
stamp's statement answers no fraction. The core's own refusal of a reply that is not the integers
its statement selects stays asserted, its message and kind byte for byte, by
`full_sync::a_review_id_that_is_not_an_integer_is_the_cores_own_refusal`, which plants a review id
that is not an integer.

Three mutation coverage tests, each green at its own commit, hold what no criterion's test reached:
`one_way::a_server_copy_is_fetched_into_a_file_whose_collection_holds_no_row` fetches the server's
copy into an existing file whose collection holds no row, so a file is refused only for a row it
holds; `one_way::a_copy_path_that_is_not_utf8_is_the_cores_own_refusal` asserts the core's refusal
of a copy path that is not UTF-8, its kind and message byte for byte, before any network; and the
crate's unit test `dispatch::tests::a_close_with_no_collection_open_is_the_engines_refusal` holds a
private engine's close to the engine's own refusal when no collection is open.

The model's properties are decided by the formal checker, which this repository's CI does not run;
section 8 maps the driver's steps to them.

## 4. File manifest

Part b1.

| path | change |
|---|---|
| `crates/engine-core/src/table.rs` | R1: (1,3) web, the (1,5) row, the `OneWaySync` exempt row, `TargetKind::Collection` |
| `crates/engine-core/src/login_guard.rs` | R2: the rule applied to a `SyncAuth`; the media refusal |
| `crates/engine-core/src/dispatch.rs` | R2 on (1,5) in `run`; R3 `run_one_way`; the dispatcher keeps its start message and its open collection's path; R6: `MODIFIED_SQL` reads the upload re-check stamp; a unit test, `a_close_with_no_collection_open_is_the_engines_refusal`, holds a private engine's close to the engine's own refusal (section 3) |
| `crates/engine-core/src/gesture.rs` | R3: `Target::Collection`, `GestureRefusal::NeedsTheChoice`; `checked` refuses `OneWaySync` before it decodes |
| `crates/engine-core/src/one_way.rs` (new) | R4 to R8: `count`, `back_up`, `recheck`, `write` |
| `crates/engine-core/src/full_sync.rs` | one insert-only crate-private accessor, `Confirmed::direction`, on which `back_up` branches (`ADR-375` D12); no covered anchor changes |
| `crates/engine-core/src/lib.rs` | the module |
| `crates/engine-core/Cargo.toml`, `Cargo.lock` | the test support's dev-dependency for the engine's own sync server, and its lock entry |
| `crates/engine-core/tests/support/sync_server.rs` (new), `crates/engine-core/tests/support/mod.rs` | the engine's own sync server on a loopback port; the stamp's oracle `stamp` (R6) |
| `crates/engine-core/tests/one_way.rs` (new) | A4 to A11, A15 to A21; two mutation coverage tests, `a_server_copy_is_fetched_into_a_file_whose_collection_holds_no_row` and `a_copy_path_that_is_not_utf8_is_the_cores_own_refusal` (section 3) |
| `crates/engine-core/tests/full_sync.rs` | R6: the `held` oracle reads the stamp, the not-integers test plants a schema stamp that is not an integer, and a review id that is not an integer keeps the core's own refusal asserted (section 3) |
| `crates/engine-core/tests/table.rs`, `crates/engine-core/tests/login_guard.rs`, `crates/engine-core/tests/exempt.rs`, `crates/engine-core/tests/containment.rs` | A1, A2, A3, A12 |
| `crates/engine-core/tests/gesture.rs`, `crates/engine-core/tests/review_pairs.rs`, `crates/engine-core/tests/parity.rs` | the existing tests that grow with the table (section 3) |
| `crates/ffi/src/engine.rs`, `crates/ffi/tests/exempt.rs` | the exempt refusal gains the case `NeedsTheChoice`, mapped and read as its own sentence (an error case, no new export); the four earlier sentences stay in one match of a private type, so their mutation row keeps its anchor |
| `crates/web-engine/src/study.rs`, `crates/web-engine/tests/study.rs` | R1: `service::SYNC` and `SYNC_CALLS`, and A1's web-engine test (no export; `run_method` unchanged) |
| `docs/specs/SPEC-364-the-web-client-syncs-from-its-worker.md`, `docs/decisions/ADR-375-the-web-client-syncs-from-its-worker.md`, `docs/schematics/web-sync-core.md` | this SPEC, its ADR, its schematic |
| `docs/decisions/ADR-301-deckstreak-writes-to-the-collection-only-through-declared-write-classes.md` | R12's note |
| `docs/decisions/ADR-356-the-engine-core-holds-the-engine-for-both-clients-behind-a-per-transport-table.md` | R12's amendment of D6 |
| `docs/specs/SPEC-357-the-full-sync-choice-and-the-web-sync-screens.md` | section 7 names this SPEC as part b; an insert-only amendment of R7 names the upload re-check stamp (R6) |
| `scripts/mutation-rows.d/S36400-S36499.json` (new) | section 10 |
| `scripts/mutation-equivalent.d/deck-streak-engine-core.json` | the equivalence record of the core's own refusal's kind, `INVALID_INPUT`, the protobuf default; the login guard's record re-bound from `check` to `refused`, where part c moved the same construction |
| `docs/red-first/SPEC-364.md` | each criterion's red |
| `changelog.d/web-sync-core-364.md` | the delivery's fragment |

## 5. What this does NOT do

- It makes no fork write and moves no fork pin; whether a part's patches may be pushed to the fork
  is decided with part b2 (#631).
- It builds no browser transport: the synchronous request from the Worker,
  the clock and thread gates, the web engine's login and sync exports and the Worker's sync
  operations are part b2 (#631).
- It runs no full-sync write in the browser: the full-sync file moves on `wasm32`, the web
  engine's choice exports, the pool's reserve and the browser proof of the restore are part b3
  (#631).
- It builds no page: the session's sync, the unsynced status, the choice screen, the backups' list
  and retention, the `needs-sign-in` state and the snapshot answer an upload asks for are part c
  (#631).
- It decides no snapshot route: until part c, an upload's `SnapshotAnswer` is the caller's, and a
  `found: false` is refused by part a's rule (#631, #161).
- It fills no media directory and syncs no media: R2 refuses it until part d (#631).
- It adds no sync to the native adapter's surface; the iOS client calls the same driver when its
  sync lands (#633).
- It does not close the window between the re-check and an upload at the server (#617).
- It counts no edit to a row both sides hold (#620).
- It does not drop the engine fork; its removal stays tracked (#233).
- It proves nothing on a device; the owner's acceptance session does (#637).

## 6. Risks

- **`VACUUM INTO` through the engine's door.** The statement cannot run inside a transaction, and
  the engine's door runs whatever the core passes. Measured first, by A6's test over the real
  statement before any other red: if the door refuses it between operations, the builder stops
  and reports the engine's error, and the seat chooses the fallback (`ADR-375` D4 names the fork
  patch), never an adapter's copy.
- **A private engine on the same start message.** Opening the copy may upgrade or check it; the
  copy is the engine's own download, so it opens as the engine wrote it. Detected by A5 to A8.
- **A copy fetched over a backup.** R4 refuses a non-empty target; A5 holds it.
- **The rows on the dispatcher and the table.** New finds must not repeat a line an existing row
  finds. Detected by the row census.
- **The parity guard and the tests that enumerate the table.** A web column flipped without
  `SYNC_CALLS` reddens A14; a table test left at its old length reddens its own population.
  Detected by the whole target set, run after each implementation commit.
- **The model's digests.** A change inside a covered anchor of `full_sync.rs` breaks its digest.
  R11 keeps every change outside them; the formal check outside this repository's CI reads them.

## 7. Delivered by the next pull requests

### Part b2: the browser transport and the normal sync (after b1)

Part b2 is built: sections 11 to 18 hold what it delivered, and where they differ from this outline
(the export's name, R18's study clause, B1's full-sync arm, B2's and B3's reds), they hold.

R14. **The transport.** A fork patch, `browser-xhr`, replaces `browser-fetch`'s refusal (M1): the
    `wasm32` twin sends the engine's request with a synchronous `XMLHttpRequest` from the dedicated
    Worker, its body zstd-encoded in memory, the engine's own headers (the `anki-sync` header
    among them) set on it, its response read as bytes and decoded by the size header, as the native
    twin reads it. A response that was redirected is refused, as the native twin refuses a 308; a
    stall past the engine's timeout answers the engine's own timeout error; a non-success status
    answers the error the native twin's status answers, so a 403 reaches the core's `classify` as
    the sync server's refusal. `browser-fetch`'s other change (no `http1_only`) stays.
R15. **The clock and thread gates.** A fork patch, `wasm-clock-threads`: on `wasm32` the runtime is
    built with no time driver, `IoMonitor` reads no clock, a sync's abort is sent inline instead of
    on a thread, and a media sync in the background refuses instead of starting a thread. Every
    patch is a `cfg` on the target; the native engine is unchanged.
R16. **The pin.** The two patches are commits on the fork's pin branch over `c538de55`, tagged with
    a new tag; the old tag stays. ADR-058's table gains their rows and amends `browser-fetch`'s; the
    root manifest's `[patch]` `rev` moves to the new tag's commit.
R17. **The login.** The web engine exports `sync_login(endpoint, user, password)`, which runs (1,3)
    through the core's `run` (the guard first) and answers the host key to its one caller, the
    Worker's sync module's `Login`, which `CredentialStore.obtain` calls. The password lives only
    for the call.
R18. **The normal sync.** The Worker's `sync` operation takes `{generation, key}` from
    `CredentialStore.forSend(origin + SYNC_ROUTE)`, calls the web engine's `sync(key, endpoint)`
    (the core's (1,5) with `sync_media: false`), and passes the engine's error bytes, or none, to
    `CredentialStore.settle(generation, error)`. Its reply is a status word and the sync's required
    change, never a value of the key. Study is not held while it runs.
R19. **Persistent storage, every session.** The study page asks for persistent storage at every
    session start, through `requestPersistence`, and a refusal changes nothing else.

| id | criterion | delivered by |
|---|---|---|
| B1 | In Chromium and WebKit, a login and a normal sync from the Worker reach the engine's own sync server through the page's own origin, and a review made in the browser is on the server afterwards | `pnpm --dir web/app exec playwright test --config playwright.engine.config.ts -g "a normal sync from the worker reaches the server"`; red: the refusal of M1 |
| B2 | A 403 from the sync route reaches `settle` as a refusal and drops the key; a network failure keeps it | `... -g "a refused key is dropped and a lost network keeps it"`; red: the twin answers every failure as a network error |
| B3 | A redirected answer is refused and nothing is synced | `... -g "a redirected sync answer is refused"`; red: the twin reads the redirected body |
| B4 | The Worker's sync module takes the key only from `forSend` and settles every send | `pnpm --dir web/app exec vitest run src/lib/engine/sync.test.ts -t "each sync takes its key from the store and settles"`; red: a stub that keeps the key between syncs |
| B5 | No Worker reply carries the key, the password or the host key, across the new operations; only the Worker imports the sync module | `pnpm --dir web/app exec vitest run src/lib/engine/credential-reach.test.ts`; red: a planted reply of the key |
| B6 | Each new web export owes its statements | `cargo test -p deck-streak-web-engine --test boundary`; red: the census's entry before the export |
| B7 | Every session start asks for persistent storage | `pnpm --dir web/app exec vitest run src/lib/study/engine.test.ts -t "every session start asks for persistent storage"`; red: no caller (M24) |

### Part b3: the full-sync write in the browser (after b2)

R20. **The files.** A fork patch, `browser-full-sync-files`: on `wasm32`, an upload reads the closed
    collection through SQLite's serialize, and a download deserializes the received bytes into
    memory, checks their integrity, sets `ls` to `mod`, and replaces the collection with SQLite's
    backup interface in one transaction, through the storage pool the web engine installs as the
    default file system. The binding's `serialize` and `backup` features are turned on for `wasm32`
    only. The full-sync progress monitor never ticks on `wasm32`. Pinned as R16 pins.
R21. **The choice's exports.** The web engine exports `full_sync_count`, `full_sync_confirm`,
    `full_sync_snapshot` and `full_sync_cancel`. It holds the choice's stage between the owner's
    taps and calls only `one_way`'s functions; `full_sync_confirm` is the owner's tap, builds the
    gesture, and runs the backup and, for a download, the write; `full_sync_snapshot(found)` runs an
    upload's snapshot check, re-check and write. Each answers the counts, a status word, or that
    the counts changed. Paths are new pool files, and the pool is reserved before each.
R22. **The restore in the browser.** After eviction, a sync offers the download alone and the owner's tap
    restores every review the server holds.

| id | criterion | delivered by |
|---|---|---|
| C1 | In Chromium and WebKit, a download replaces the browser's collection with the server's after a backup that opens with every device review | `... playwright.engine.config.ts -g "a download is written after its backup"`; red: `fs::read` and the rename of M9 |
| C2 | An upload is refused without a found snapshot and written with one; the server holds the device's reviews | `... -g "an upload waits for its snapshot"`; red: the same |
| C3 | An evicted collection restores every server review on the owner's tap | `... -g "an evicted collection is restored from the server"`; red: the same |
| C4 | Each choice export owes its statements, and none answers an id set or a path the page passed | `cargo test -p deck-streak-web-engine --test boundary`; red: the census's entries before the exports |

## 8. Formal model

`formal/tla/FullSyncChoice` is unchanged, and so is its entry. Part b realises its steps; it adds
no state, no actor and no transition the model lacks, and each step keeps the model's
abstractions: "a write is one step" (the engine's replace on `wasm32` is one transaction of
SQLite's backup interface, R20; natively the engine's rename) and "a backup and the server copy are
places a review survives in" (R8 deletes neither).

| `one_way.rs` step | the model's action | the property it serves |
|---|---|---|
| `count`: fetch into an empty file, read both sides | `Count` / `Recount` | `CountsCoverTheUpload` |
| `back_up`: `VACUUM INTO`, or the counted copy, read back from the file; then part a's `download_ready` for a download | `Backup`, `DownloadReady` | `BackupBeforeReplace` (`a-download-before-its-backup`) |
| the caller's `SnapshotAnswer`, then `recheck`'s fresh fetch | `SnapCheck`, `Rechecked` | `SnapshotBeforeUpload` (`an-upload-with-no-snapshot-found`), `CountsCoverTheUpload` (`an-upload-with-no-re-check`) |
| `write`: re-read the device, `at_write`, then `run_one_way` | `AtWriteRefused`, `WriteUpload`, `WriteDownload` | `NoReviewLost` (`a-write-that-does-not-re-read-the-device`) |
| `run_one_way`'s upload: the engine's own full upload, which changes the server's schema | `WriteUpload` | `AWindowSyncIsNotSilent` (`an-upload-that-keeps-the-servers-schema`) |
| R8: no copy or backup removed | the `backup` and `copy` places | `NoReviewLost` |

The model's comment that the adapter writes the backup (`FullSyncChoice.tla` line 54) predates
this driver. It is prose outside every covered span, and it stays unchanged here; `one_way.rs` is
a new caller of the covered transitions, so a later formal delivery may cover it and re-word that
comment (#631).

## 9. What only a device or a person proves

| id | criterion | who, and when |
|---|---|---|
| V1 | A sync from Safari on iPhone and iPad and from a desktop browser reaches the server through the page's origin | the owner, in the acceptance session, after part c lands (#637) |
| V2 | After the browser evicts the site's storage, the owner's download restores the collection (SPEC-357 V3) | the owner, in the acceptance session (#637) |

## 10. Mutation rows (part b1)

Band `S364`: rows `S36401` to `S36421` in `scripts/mutation-rows.d/S36400-S36499.json`, each find
unique in its file at the cut; the build brief names each row's find, replacement and killer.
`S36401` to `S36418` are `crates/engine-core`'s; `S36419` to `S36421` pin `SYNC_CALLS`'s literal
values in `crates/web-engine/src/study.rs`, killed by the web engine's own test (a new row's
killer lives in its row's crate). `S36423` to `S36434` pin R6's stamp in
`crates/engine-core/src/dispatch.rs`: one row per synced table that drops it from the statement,
one that drops `col.scm`, and one that reads the greatest modified time of cards, notes, decks,
presets and note types instead; `S36422` is unused. The delivery also re-kills every existing row on each file it
edits, and re-anchors none: `table.rs` 23, `dispatch.rs` 6, `gesture.rs` 3, `login_guard.rs` 5,
`crates/ffi/src/engine.rs` 6 and `crates/web-engine/src/study.rs` 27 at `dee337bc`.

## 11. Requirements (part b2), as built

R14. **The transport.** A fork patch, `browser-xhr`, replaces `browser-fetch`'s refusal (M1). On
    `wasm32` the engine's sync request is a synchronous `XMLHttpRequest` from the dedicated Worker:
    its body zstd-encoded in memory, the engine's own headers (the `anki-sync` header among them)
    set on it, the method and URL read from the request the engine built, its answer read as bytes
    and decoded by the size header, as the native twin reads it. An answer whose response URL is
    not the request's is refused as `MISDIRECTED_REQUEST` with no source, which the engine reads as
    a failure (ADR-375 D15). A non-success status reaches the engine's own status mapping through a
    `wasm32`-only status source in the fork's `network.rs`, which `From<HttpError>` downcasts to
    `error_for_status_code`, so a 403 reads as the engine's auth refusal, as natively (D14). The
    request's timeout is the engine's stall duration (`SyncAuth.io_timeout_secs`, or the engine's
    default), and its expiry answers the engine's own timeout error (D19).

R15. **The runtime.** A fork patch, `wasm-clock-threads`: on `wasm32` the engine's runtime is
    `Builder::new_current_thread().build()` with no `enable_*` call, so no timer and no IO driver
    exist; the native engine keeps `enable_all()`. On `wasm32` the IO monitor reads no clock, a
    sync's abort is sent inline rather than from a thread, and a media sync in the background
    refuses.

R16. **The pin.** The fork's branch `wasm32-26.09.3` gains exactly three commits over
    `c538de55a23e695234e794029fce0dafff2d36a9`, one per patch (`browser-xhr`, then
    `wasm-clock-threads`, then `wasm-collection-size`), each body `Patch:`, `Reason:` and `Removal condition:` as the ten before
    it. A NEW lightweight tag, `deckstreak-pin-26.09.3-wasm32-sync`, names the third; the earlier
    tag stays where it is, unmoved. The root manifest's `[patch]` takes `anki` and `anki_proto` by
    `rev` from the new tag's commit, its comment still naming ADR-058 and #233; `Cargo.lock` moves
    by `cargo update -p anki -p anki_proto` alone; ADR-058 gains a note naming the three patches and the
    new commit; and the pin's description in `crates/ingest/src/engine.rs` names the new commit.
    The native engine is unchanged: every edit is a `cfg` on `wasm32` or a dependency line both
    targets share. The third patch, `wasm-collection-size`, exists because a normal sync reads the
    collection's size from the file system before its first request, and `wasm32` has none: on
    `wasm32` the sync meta reads the open collection's SQLite page count times its page size, so
    the upload size check still reads the collection's real size.

R17. **`sync_login`.** The web engine exports `sync_login(endpoint, user, password)`, which sends
    the engine's `SyncLoginRequest` (the endpoint set) through the dispatcher on (1,3) and answers
    the host key. An engine refusal answers the engine's error bytes as a `Uint8Array`; any other
    refusal answers the boundary's refusal (ADR-375 D16). The Worker's `sync-login` operation takes
    a non-empty bounded `user` and `password`, calls it through `sync.ts`, keeps the key only
    through the credential module, and answers a status word: a refused login answers
    `needs-sign-in`; any other failure answers `offline`, and the store keeps no key.

R18. **`sync_collection`.** The web engine exports `sync_collection(key, endpoint)`, which sends the
    engine's `SyncCollectionRequest` (its `SyncAuth` holding the key and the endpoint, no timeout
    override, and `sync_media` false) on (1,5) and answers the engine's `required` as a number. The
    Worker's `sync` operation answers `{status, required}`, `required` being one of `no-changes`,
    `normal-sync`, `full-sync`, `full-download` or `full-upload` (the engine's number indexes that
    list, in `protocol.ts`) or null. Both operations need an open session (`not-open` otherwise),
    and a Worker with no sync store answers as a store with no key. Each sync takes its key from
    the store at the send and settles it: a success settles the send, the engine's bytes settle it
    with those bytes, and any other throw settles it with empty bytes and is thrown again (D17).
    The sync runs on the session's queue (D18): a study request waits, then is answered; none is
    refused.

R19. **Persistence.** Every session start asks the browser for persistent storage, whatever it
    answered before; the server copy stays the authoritative one.

## 12. Acceptance criteria (part b2)

| id | criterion | red at its tests commit | decided by |
|---|---|---|---|
| B1 | In Chromium and WebKit, a login and a normal sync from the Worker reach the engine's own sync server through the page's own origin: after the collection is opened and seeded, `sync-login` answers `held` and `sync` answers `{status: 'held', required: 'full-upload'}`, the server saw the login and at least one sync request, every one under `/anki-sync/`, and no CSP violation was reported | red at the implementation commit, not the tests commit: `browser-fetch`'s refusal (M1) makes `sync-login` answer `offline`, not `held` (at the tests commit `sync-login` is an unknown operation, a red for another reason); green at the pin-move commit | `sync.spec.ts`, "a login and a normal sync from the worker reach the server" |
| B2 | A server's 403 drops the key, and a lost network keeps it: in forbid mode `sync` answers `{status: 'absent', required: null}`; in drop mode the status stays `held` | not red: the 403 mapping lives in the pinned fork. Control: with the status source's downcast arm reverted, B2 is red | `sync.spec.ts`, "a refused key is dropped and a lost network keeps it" |
| B3 | A redirected answer is refused: a redirected login answers `offline` and the status stays `absent`; a redirected sync answers `required: null` and the status stays `held` | not red. Control: with the response-URL check dropped, B3 is red | `sync.spec.ts`, "a redirected sync answer is refused" |
| B4 | Each sync takes its key from the store and settles every send, and a send that throws anything but the engine's bytes still settles | red: a stub `sync.ts` keeps the key between syncs and never settles | `sync.test.ts`, two tests |
| B5 | The key reaches no reply and no page module: no reply to `sync-login` or `sync` carries the host key, no page module imports `sync.ts`, and each new operation answered ok, so the census is not vacuous | not red: an absence census, held by planted controls (a reply carrying the host key, a page module importing `sync.ts`), each refused by name | `credential-reach.test.ts` |
| B6 | The two sync exports reach the engine through the dispatcher and never through `admit(` or `run_method(` | red: "`fn sync_login(` occurs 0 times, not once" | the `boundary` census |
| B7 | Every session start asks for persistent storage | red: the constructor takes `persist` and calls it 0 times | `engine.test.ts`, "every session start asks for persistent storage" |

```acceptance
B1: pnpm --dir web/app exec playwright test --config playwright.engine.config.ts sync.spec.ts -g "a login and a normal sync from the worker reach the server"
B2: pnpm --dir web/app exec playwright test --config playwright.engine.config.ts sync.spec.ts -g "a refused key is dropped and a lost network keeps it"
B3: pnpm --dir web/app exec playwright test --config playwright.engine.config.ts sync.spec.ts -g "a redirected sync answer is refused"
B4: pnpm --dir web/app exec vitest run src/lib/engine/sync.test.ts -t "each sync takes its key from the store and settles"
B4: pnpm --dir web/app exec vitest run src/lib/engine/sync.test.ts -t "a send that throws anything but the engine's bytes still settles"
B5: pnpm --dir web/app exec vitest run src/lib/engine/credential-reach.test.ts
B6: cargo test -p deck-streak-web-engine --test boundary
B7: pnpm --dir web/app exec vitest run src/lib/study/engine.test.ts -t "every session start asks for persistent storage"
```

## 13. File manifest (part b2)

- `Cargo.toml`
- `Cargo.lock`
- `crates/ingest/src/engine.rs`
- `crates/web-engine/src/wasm.rs`
- `crates/web-engine/tests/boundary.rs`
- `crates/engine-core/examples/sync_server.rs` (new)
- `web/app/src/lib/engine/sync.ts` (new)
- `web/app/src/lib/engine/sync.test.ts` (new)
- `web/app/src/lib/engine/protocol.ts`
- `web/app/src/lib/engine/protocol.test.ts`
- `web/app/src/lib/engine/session.ts`
- `web/app/src/lib/engine/session.test.ts`
- `web/app/src/lib/engine/worker.ts`
- `web/app/src/lib/engine/worker.test.ts`
- `web/app/src/lib/engine/client.ts`
- `web/app/src/lib/engine/client.test.ts`
- `web/app/src/lib/engine/credential-reach.test.ts`
- `web/app/src/lib/study/engine.ts`
- `web/app/src/lib/study/engine.test.ts`
- `web/app/vite.engine.config.ts`
- `web/app/playwright.engine.config.ts`
- `web/app/tests-engine/sync.spec.ts` (new)
- `docs/specs/SPEC-364-the-web-client-syncs-from-its-worker.md`
- `docs/decisions/ADR-375-the-web-client-syncs-from-its-worker.md`
- `docs/decisions/ADR-058-the-engine-pins-a-patched-fork-of-26-09-3-until-upstream-carries-the-fix.md`
- `docs/schematics/web-sync-core.md`
- `docs/red-first/SPEC-364.md`
- `scripts/mutation-rows.d/S36400-S36499.json`
- `changelog.d/web-sync-worker-364.md` (new)

The fork's files, outside this repository, at the new tag's commit: Cargo.lock, rslib/Cargo.toml,
rslib/src/backend/mod.rs, rslib/src/backend/sync.rs, rslib/src/error/network.rs,
rslib/src/sync/collection/meta.rs, rslib/src/sync/http_client/io_monitor.rs and
rslib/src/sync/http_client/mod.rs.

## 14. What this does NOT do (part b2)

- No full-sync write, and no review proven on the server: part b3 (#631).
- The server's `new_endpoint` is not followed (#631).
- No media sync (#631).
- No page: the sync screens are part c (#631).
- No full-sync timeout (#631).
- No CI workflow change (#631).
- No native sync (#633).
- The fork is not dropped: its removal conditions stand (#233).
- No device proof (#637).

## 15. Risks (part b2)

- A cross-origin redirect is stopped by the Worker's `connect-src 'self'` before the hop, so only a
  same-origin redirect reaches the response-URL check. The builder confirms the CSP header on the
  Worker's script, and B1 asserts no violation.
- The request's timeout is a total bound, the engine's default stall duration: a long normal sync
  fails as a timeout, and the key is kept. Part b3 sizes it for a full sync (section 17).
- The example's cold compile in CI: the second webServer's timeout is sized for a cold build. Past
  the `web-engine` job's time bound, the CI change is its own reviewed change, outside this part.
- `--locked` with the moved `[patch]`: `Cargo.lock` moves only by `cargo update -p anki -p
  anki_proto`, or every locked build refuses.
- An export beside `call()` that skips the dispatcher: B6's census holds both sync exports by
  name, and its new control refuses a sync export that reaches `admit(` or `run_method(`.
- The native engine must not change: `cargo test -p anki` in the fork before and after the three
  commits, with equal results.
- The synchronous request holds the Worker for the sync's length: study waits (R18), bounded by
  the request's timeout (D19).

## 16. Formal model (part b2)

`formal/tla/SyncCredential` is unchanged. `forSend` is `StartSend`; `settle(gen)` is `Accepted`;
`settle(gen, a refusal's bytes)` is `Refused`; `settle(gen, other or empty bytes)` is `Failed`; a
Worker restart is `Restart`. The condition this part holds: every send settles (B4's second line).
A send that does not settle is a step the model lacks, so the model would change first.

## 17. What part b3 owes after part b2

- (i) B1's full-sync arm, verbatim from section 7: "and a review made in the browser is on the
  server afterwards". Section 7's whole B1 reads: "In Chromium and WebKit, a login and a normal sync
  from the Worker reach the engine's own sync server through the page's own origin, and a review
  made in the browser is on the server afterwards". The arm is unreachable in part b2, because the
  fork's `meta.rs` (lines 72-80) answers FullSyncRequired across schemas, so it joins C2.
- (ii) R20's "never ticks on `wasm32`" is mandatory: the full-sync `interval` in the fork's
  `http_client/full_sync.rs` (lines 8 and 30-32) panics with no time driver.
- (iii) The request's total timeout is sized for a full sync, or a stall bound is decided.
- (iv) Part b3's patch builds on the `deckstreak-pin-26.09.3-wasm32-sync` commit, under a new tag
  and a new ADR-058 note of the same shape.
- (v) It reuses `crates/engine-core/examples/sync_server.rs`, the Vite proxy and the test routes.
- (vi) The status source's arm exists (D14).
- (vii) The boundary census is shared: C4 grows its owed statements.
- (viii) The #730 overlap is shared: the paths part b2 shares with #730 are part b3's to re-measure too.
- Part b3 is delivered by SPEC-377 part c1.

## 18. Mutation rows (part b2)

Band `S364`, rows `S36435` to `S36439` in `scripts/mutation-rows.d/S36400-S36499.json`: crate
`web-engine`, file `src/wasm.rs`, killer
`boundary::each_boundary_function_reaches_the_engine_through_the_dispatcher`. Each find is spelled
from the committed text after `cargo fmt` and occurs once in the file.

| row | find | replace |
|---|---|---|
| `S36435-THE-SYNC-LOGIN-EXPORT-IS-METHOD-THREE` | `.run(service::SYNC, 3,` | `.run(service::SYNC, 4,` |
| `S36436-THE-SYNC-EXPORT-IS-METHOD-FIVE` | `.run(service::SYNC, 5,` | `.run(service::SYNC, 6,` |
| `S36437-THE-SYNC-EXPORT-ASKS-NO-MEDIA` | `sync_media: false` | `sync_media: true` |
| `S36438-AN-ENGINE-REFUSAL-KEEPS-ITS-BYTES` | `Refusal::Engine { error } => Uint8Array::from(error.as_slice()).into()` | `Refusal::Engine { error } => refuse("engine error")` |
| `S36439-THE-LOGIN-ANSWERS-THE-HOST-KEY` | `Ok(auth.hkey)` | `Ok(String::new())` |

The TypeScript is held by StrykerJS with `break` 100 over each changed `.ts` file, with no hand
rows. The fork carries no rows of this repository.
