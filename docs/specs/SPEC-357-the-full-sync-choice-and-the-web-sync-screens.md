# SPEC-357: the full-sync choice is modelled first, one rule both clients share counts what each side loses and orders the backup and the snapshot check before any write, and the web client syncs at each session's start and end

- **Campaign row:** the app campaign, SPEC-334 row 1.4 (R5, R8) and its section 9. **Issue:** #631
  (the web sync screens and the full-sync choice). **Context(s):** `deck-streak-engine-core`
  (`crates/engine-core`), formal (`formal/tla`), and in the later parts `deck-streak-web-engine`
  (`crates/web-engine`), `miniapp` (`web/app/src`), `deck-streak-api` (`crates/api`) and the
  engine fork.
- **Decided by:** ADR-368 (this SPEC's own: where the shared rule lives, how the counts are
  read, what the backup holds, the re-check, the warning's source, the model's abstraction and
  the write's last check), under ADR-337 (the owner's tap and its token), ADR-340 (the offsite
  snapshot an upload checks for), ADR-356 (the core and its tables), ADR-058 (the fork) and
  ADR-361 (the media directory the sync fills). SPEC-342 measured the choice path this SPEC
  builds on.
- **Schematic:** `docs/schematics/full-sync-choice.md` (the components, a session's sync, the
  choice's states and an upload's order; this delivery adds it).
- **Status:** part a of four. This pull request writes the model first, then the one rule
  both clients share and the core's two new reads (sections 2 and 3). Parts b, c and d are
  excluded in section 5 (#631). **Mutation band:** S35700-S35799 (section 10).
  **Model:** `formal/tla/FullSyncChoice` (section 8).

## 1. The problem, measured

Each figure was read at DeckStreak `dev` `adb19bbd` (`D` below), or in the engine fork at its pin,
commit `c538de55` (`F` below, the fork's own tree). Nothing was run: every figure is a read.

### 1.1 What the engine and the core hold

| id | measured | figure | command |
|---|---|---|---|
| M1 | The fork refuses the sync transport on wasm32 | the twin of `zstd_request_with_timeout` answers `NOT_IMPLEMENTED`, "sync transport is not available on wasm32" (lines 101-115) | `grep -n 'not available on wasm32' rslib/src/sync/http_client/io_monitor.rs` in `F` |
| M2 | That refusal is a fork patch whose removal this issue owns | `browser-fetch`, commit `cebf6785`, removed when "a browser sync transport replaces the refusal (#631)" (line 206) | `git grep -n browser-fetch D -- 'docs/decisions/ADR-058-*'` |
| M3 | The backend runs every sync call to completion on a blocking runtime | `rt.block_on` at lines 259, 302, 339, 370, 428, 432 of the sync service | `grep -n block_on rslib/src/backend/sync.rs` in `F` |
| M4 | A sync's abort and its media sync each start a thread | `std::thread::spawn` at lines 207 and 376 | `grep -n 'thread::spawn' rslib/src/backend/sync.rs` in `F` |
| M5 | A full upload reads the collection file; a full download writes a temp file and renames it | `fs::read` at upload line 49; `new_tempfile_in_parent_of` and `atomic_rename` at download lines 37 and 44 | `grep -n -E 'fs::read\|atomic_rename\|new_tempfile' rslib/src/sync/collection/{upload,download}.rs` in `F` |
| M6 | An upload marks the schema changed before it sends | `before_upload` calls `set_schema_modified` (lines 192-203) | `grep -n -E 'fn before_upload\|set_schema_modified' rslib/src/collection/mod.rs` in `F` |
| M7 | The core admits the sync login on the native transport only, and holds the one-way sync nowhere | `ORDINARY` has 18 rows, `BackendSyncService.SyncLogin` (1,3) `native: true, web: false` (lines 113-120); `EXEMPT` has 6 rows (line 246) and none is (1,6); `decide` answers `NotAllowed` for a pair in neither (lines 295-308) | `git show D:crates/engine-core/src/table.rs \| sed -n '113,120p;246,247p;295,308p'` |
| M8 | The core's database door returns the first row only, for two closed reads | `Read` is `NoteCount` and `CardSnapshot` (line 63); `"first_row_only": true` (line 130) | `git grep -n -E 'pub enum Read\|first_row_only' D -- crates/engine-core/src/dispatch.rs` |
| M9 | The owner's gesture token is not built | 0 lines name `OwnerGesture` or `run_exempt` under `crates` (SPEC-345 part 2, #623) | `git grep -n -E 'OwnerGesture\|run_exempt' D -- crates \| wc -l` |
| M10 | The core holds no full-sync rule | six source files: `dispatch.rs`, `face.rs`, `lib.rs`, `login_guard.rs`, `media.rs`, `table.rs` | `git ls-tree -r --name-only D -- crates/engine-core/src` |

### 1.2 What the choice path already proved (SPEC-342, #660)

| id | measured | figure | command |
|---|---|---|---|
| M11 | The offer (F1) | both choices on a schema conflict with both sides non-empty; only the upload when the server is empty; only the download when the device is empty | `git grep -n -E '\(F[1-6]\)' D -- 'docs/specs/SPEC-342-*'` (lines 133 to 145) |
| M12 | The requests (F2, F3) | an upload sends one `upload` request with no `meta` before it; a download sends one `download` | same range |
| M13 | What each choice loses (F4, F6) | exactly the other side's review-log rows, by id; a review the device had already synced is lost by a download once the server was replaced without it, so a loss is a difference of ids, never the device's count of unsynced rows | same range |
| M14 | The window (F5) | another client's normal sync between the `meta` answer and the upload is overwritten: its row is absent from the server afterwards | same range; its consumer is this issue's model (M12 of SPEC-342, line 310) |

### 1.3 What the web client and the server hold

| id | measured | figure | command |
|---|---|---|---|
| M15 | The Worker's protocol has no sync operation | `OPS` holds 14 operations: `open`, `seed`, `next`, `answer`, `undo`, `snapshot`, `memory`, `close`, `decks`, `study`, `card`, `rate`, `bury`, `flag` (lines 7-22) | `git show D:web/app/src/lib/engine/protocol.ts \| sed -n '7,22p'` |
| M16 | The engine module exports no sync call | `EngineModule` lists 17 exports, none a sync (lines 13-33) | `git show D:web/app/src/lib/engine/session.ts \| sed -n '13,33p'` |
| M17 | The Worker's storage is one OPFS pool | directory `deck-streak`, collection `/deck-streak/collection.anki2`, the pool held once (lines 53-61); `install_storage` (line 106) | `git grep -n -E 'const DIRECTORY\|COLLECTION_PATH: \|static POOL\|fn install_storage' D -- crates/web-engine/src/wasm.rs` |
| M18 | The page may reach its own origin only | `connect-src` is `'self'` (line 30); the sync route is served under the web origin (SPEC-337 R4) | `git show D:web/app/svelte.config.js \| sed -n '30p'` |
| M19 | No route tells a client whether a snapshot exists | 0 lines mention a snapshot under `crates/api/src` | `git grep -n -i snapshot D -- crates/api/src \| wc -l` |
| M20 | The server's snapshot window is modelled; the client's choice is not | `formal/tla/SyncSnapshotWindow` (six witnesses, budget 60 s at `config/formal.json` line 13); no entry names a full-sync choice | `git ls-tree -r --name-only D -- formal/tla`; `git grep -n '"tla/' D -- config/formal.json` |
| M21 | The formal budget file is pinned by a test | `EXPECTED` (line 24) must equal the committed file byte for byte (line 476) | `git grep -n -E 'EXPECTED = \|committed_text\(\), json' D -- scripts/tests/test_formal_config.py` |
| M22 | The rows on the core's dispatcher | four rows anchor on `src/dispatch.rs` (S34511, S34512, S34706, S34823); their finds must stay unique | `git grep -l '"src/dispatch.rs"' D -- scripts/mutation-rows.d` |
| M23 | Where the web client keeps the sync credential is undecided | issue #654 is open and binds this issue: no browser sync is built before it is decided | `gh issue view 654 -R RexRenatus/deck-streak` |

**What follows.** Every piece the choice needs except its order is absent: no transport on the web
(M1 to M5), no token (M9), no rule (M10), no read of more than one row (M8), no snapshot answer
(M19) and no model (M20). The order is the one thing both clients must hold identically, and the
measured window (M14) shows it is the one thing a client alone cannot close: so the model comes
first, and the rule it covers lives once, in the core both clients already share.

### 1.4 One copy for both clients

| lives in | holds | the web client adds | the iOS client adds (#633) |
|---|---|---|---|
| `crates/engine-core/src/full_sync.rs` | the offer, the counts, the choice's states as types, backup-before-replace, the snapshot check and the re-check before an upload, the write's last check | nothing | nothing |
| `crates/engine-core/src/dispatch.rs` | the id reads and the unsynced read, by fixed statements | nothing | nothing |
| `formal/tla/FullSyncChoice` | the ordering both clients run | nothing | nothing |
| the client's adapter | where the backup and the server copy are stored, and the call that runs the write | `crates/web-engine` (the OPFS pool) and the Worker's operations | `crates/ffi` (the app's container) |
| the client's screens | the warning, the choice screen, its wording and its confirmation | `web/app/src` | the iOS app |

IOS-3 reuses the first three rows unchanged and writes only its own adapter and screens. A rule
the iOS client needs that the core lacks is added to the core, never written a second time.

## 2. Requirements (part a)

R1. **The model first.** `formal/tla/FullSyncChoice` models the owner's tap, this device's
    reviews, a second client's reviews and normal syncs, and the offsite snapshot, over review
    ids. It states five properties at `ramp=report`, each with a witness the formal check must
    catch, plus `TypeOK` (section 8). The model and its witnesses are committed before the rule
    they cover, and the red-first record says so.

R2. **Its budget.** `config/formal.json` gives the entry a time budget set from its measured run,
    and `scripts/tests/test_formal_config.py`'s `EXPECTED` pins it.

R3. **One rule, in the core.** `crates/engine-core/src/full_sync.rs` holds the choice's states as
    types: `Counted` (the offer, the device's ids and the server copy's ids), `Confirmed`,
    `BackedUp`, `Checked` (an upload only), `Ready` and `Write`. Each transition consumes the state
    before it, so no client reaches `Write` by another order, and `Write` has no public
    constructor.

R4. **The offer.** `Offer` is read from the engine's answer to a normal sync: both directions on
    a full sync, the upload only on a full upload, the download only on a full download, none
    otherwise (M11). A direction not offered is refused and the choice is kept.

R5. **The counts are differences of ids.** For each offered direction, `Losses::between` counts
    the review-log rows, cards and notes the replaced side holds and the kept side lacks: the
    upload replaces the server, the download replaces the device (M13). No count is taken from
    the unsynced rows.

R6. **Backup before replace.** `Confirmed::backed_up` accepts a backup only when its ids hold
    every review, card and note id of the side the write replaces: the device's collection for a
    download, the server copy for an upload. Otherwise it refuses and keeps the state.

R7. **The snapshot, then the re-check, before an upload.** An upload's `BackedUp` reaches
    `Checked` only from a snapshot answer whose `found` is true, and `Checked` reaches `Ready`
    only from a fresh server copy whose ids and modified stamp equal the counted copy's. A fresh
    copy that differs returns a new `Counted`, so the owner sees new counts and confirms again. A
    download reaches `Ready` from its backup directly and refuses the snapshot step.

R8. **The write's last check.** `Ready::at_write` re-reads the device's ids: a download whose
    device side gained a row its backup lacks is refused back to `Counted`; an upload admits a
    device gain, which it sends.

R9. **The core's reads.** `Dispatcher::id_sets` reads every review-log, card and note id and the
    collection's modified stamp, all rows, by fixed statements. `Dispatcher::unsynced` reads the
    reviews not yet synced and whether the collection changed, or its schema changed, since its
    last sync, by one fixed statement, with no network. `Unsynced::warns` is true when any holds.
    No adapter passes SQL.

R10. **Nothing else moves.** The core's tables, its dispatcher's `run`, its graph census and the
     rows on `src/dispatch.rs` (M22) are unchanged; the core gains no dependency.

## 3. Acceptance criteria (part a)

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | The formal budget file holds the new entry, and the test pins it | `EXPECTED` gains the entry before the file does: the committed text differs | `scripts/tests/test_formal_config.py` `the_committed_file_holds_exactly_the_declared_fields` |
| A2 | The offer follows the engine's answer in each of its five cases | an offer that admits both directions on every answer | `crates/engine-core/tests/full_sync.rs` `the_offer_follows_the_engines_answer` |
| A3 | Each direction counts the ids the replaced side alone holds, for reviews, cards and notes, and a review synced before the server was replaced is counted | counts of zero | `full_sync.rs` `each_direction_counts_the_ids_the_replaced_side_alone_holds` |
| A4 | A direction not offered is refused and the choice kept | a confirm that accepts any direction | `full_sync.rs` `a_direction_not_offered_is_refused_and_the_choice_kept` |
| A5 | A backup missing one id of the replaced side is refused, for each direction | a backup check that reads the kept side | `full_sync.rs` `a_backup_missing_an_id_of_the_replaced_side_is_refused` |
| A6 | An upload reaches `Ready` only after a found snapshot and an unchanged re-check; a changed copy returns new counts | an upload's backup that answers `Ready` | `full_sync.rs` `an_upload_reaches_ready_only_after_the_snapshot_and_the_recheck` |
| A7 | A download reaches `Ready` from its backup and refuses the snapshot step | a download held at the snapshot step | `full_sync.rs` `a_download_needs_no_snapshot_and_refuses_one` |
| A8 | A download's write refuses a device row its backup lacks; an upload's admits it | a write that does not re-read the device | `full_sync.rs` `a_write_refuses_a_device_row_its_backup_lacks` |
| A9 | The unsynced read counts reviews and changes since the last sync, with no network | a read of every review | `full_sync.rs` `the_unsynced_read_counts_reviews_and_changes_since_the_last_sync` |
| A10 | The id reads take every row | the first row only | `full_sync.rs` `the_id_reads_take_every_row` |
| A11 | Only the choice makes a `Write` | a `Write` built outside `Ready::at_write` | `full_sync.rs` `only_the_choice_makes_a_write` |
| A12 | The core still depends on no crate of this workspace | (unchanged; green at the base) | `crates/engine-core/tests/graph.rs`, whole |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_formal_config.py -k the_committed_file_holds_exactly_the_declared_fields
A2: cargo test -p deck-streak-engine-core --test full_sync -- --exact the_offer_follows_the_engines_answer
A3: cargo test -p deck-streak-engine-core --test full_sync -- --exact each_direction_counts_the_ids_the_replaced_side_alone_holds
A4: cargo test -p deck-streak-engine-core --test full_sync -- --exact a_direction_not_offered_is_refused_and_the_choice_kept
A5: cargo test -p deck-streak-engine-core --test full_sync -- --exact a_backup_missing_an_id_of_the_replaced_side_is_refused
A6: cargo test -p deck-streak-engine-core --test full_sync -- --exact an_upload_reaches_ready_only_after_the_snapshot_and_the_recheck
A7: cargo test -p deck-streak-engine-core --test full_sync -- --exact a_download_needs_no_snapshot_and_refuses_one
A8: cargo test -p deck-streak-engine-core --test full_sync -- --exact a_write_refuses_a_device_row_its_backup_lacks
A9: cargo test -p deck-streak-engine-core --test full_sync -- --exact the_unsynced_read_counts_reviews_and_changes_since_the_last_sync
A10: cargo test -p deck-streak-engine-core --test full_sync -- --exact the_id_reads_take_every_row
A11: cargo test -p deck-streak-engine-core --test full_sync -- --exact only_the_choice_makes_a_write
A12: cargo test -p deck-streak-engine-core --test graph
```

The model's properties are decided by the formal checker, which this repository's CI does not run;
section 8 names them and their witnesses.

## 4. File manifest

Part a of four; the later parts add their own files.

| file | context | change |
|---|---|---|
| `formal/tla/FullSyncChoice/FullSyncChoice.tla` | formal | added |
| `formal/tla/FullSyncChoice/MCFullSyncChoice.cfg` | formal | added |
| `formal/tla/FullSyncChoice/witness/a-download-before-its-backup.cfg` | formal | added |
| `formal/tla/FullSyncChoice/witness/an-upload-with-no-snapshot-found.cfg` | formal | added |
| `formal/tla/FullSyncChoice/witness/an-upload-with-no-re-check.cfg` | formal | added |
| `formal/tla/FullSyncChoice/witness/a-write-that-does-not-re-read-the-device.cfg` | formal | added |
| `formal/tla/FullSyncChoice/witness/an-upload-that-keeps-the-servers-schema.cfg` | formal | added |
| `config/formal.json` | formal | changed: one budget entry |
| `scripts/tests/test_formal_config.py` | scripts | changed: `EXPECTED` gains the entry |
| `crates/engine-core/src/full_sync.rs` | engine-core | added |
| `crates/engine-core/src/lib.rs` | engine-core | changed: `pub mod full_sync;` and its line in the module list |
| `crates/engine-core/src/dispatch.rs` | engine-core | changed: `id_sets`, `unsynced` and their fixed statements |
| `crates/engine-core/tests/full_sync.rs` | engine-core | added |
| `scripts/mutation-rows.d/S35700-S35799.json` | scripts | added |
| `docs/specs/SPEC-357-the-full-sync-choice-and-the-web-sync-screens.md` | docs | added |
| `docs/decisions/ADR-368-the-full-sync-choice-is-one-rule-in-the-core.md` | docs | added |
| `docs/schematics/full-sync-choice.md` | docs | added |
| `docs/red-first/SPEC-357.md` | docs | added |
| `changelog.d/full-sync-choice-357.md` | docs | added |

No `Cargo.toml` or `Cargo.lock` changes: the tests build their collections with the engine the core
already depends on.

## 5. What this does NOT do

- It does not decide where the web client holds the sync credential; parts b and c wait on that
  decision (#654).
- It does not build the owner's gesture token or the census that holds each engine write to a
  gesture handler; part b's write needs both (#623).
- It does not sign the owner in on the web (#627).
- It builds no iOS screen or adapter: IOS-3 reuses the core's rule and the model unchanged and
  adds its own storage and screens (#633).
- It does not close the window between the re-check and an upload at the server; a server that
  refuses an upload when it changed after the re-check is the sync server's own surface (#617).
- It counts no edit to a row both sides hold: a field edited only on the replaced side is lost
  without a count, and the choice screen says so (#620, which measured the path).
- It does not drop the engine fork; part b adds patches to it, and its removal stays tracked
  (#233).
- It proves nothing on a device; the owner's acceptance session does (#637).
- It builds no browser sync transport, no sync row in the core's tables and no one-way call:
  those are part b, which waits on #623's gesture token, designed on its own first, and on
  #654's decision (#631).
- It builds no page: the session's sync, the unsynced status, the choice screen, where the
  backups are stored and how many are kept, and the snapshot answer an upload asks for are
  part c, after part b (#631).
- It fills no media directory: the sync's media is part d, after part b (#631).

## 6. Risks

- **The residual window.** A second client's normal sync after the re-check and before the
  upload is overwritten on the server (M14). Its rows stay on that client, and the upload's
  schema change forces that client into a full sync of its own (M6), so the loss is never
  silent. Detected by the model's `AWindowSyncIsNotSilent` and, in part b, by the port of F5.
- **A stale count.** The counts are read once and re-checked before an upload; a download's
  device side is re-read at the write. A count shown and then changed by another client is
  caught by the re-check and shown again.
- **The rows on the dispatcher.** Part a's new reads must not repeat a line an existing row finds
  (M22). Detected by the row census.

## 7. Delivered by the next pull requests

Parts b, c and d are excluded in section 5 (#631). Each is specified, with its own
requirements and acceptance criteria, when it is built: part b after #623's gesture token has
its own design and #654 is decided, and parts c and d after part b.

## 8. Formal model

`formal/tla/FullSyncChoice` is written first, before the rule it covers. Its actors are the
owner's tap, this device's reviews, a second client's reviews and normal syncs, and the snapshot.
It covers `crates/engine-core/src/full_sync.rs` at `fn between`, `fn confirm`, `fn backed_up`,
`fn download_ready`, `fn snapshot_found`, `fn rechecked` and `fn at_write`, and cites #631, #620
and #660. Each property enters at `ramp=report`:

| property | states | witness the check must catch |
|---|---|---|
| `BackupBeforeReplace` | every row a write removes is in a backup, or is a row the second client synced after the re-check | `a-download-before-its-backup` |
| `SnapshotBeforeUpload` | no upload is written unless the snapshot was found first | `an-upload-with-no-snapshot-found` |
| `CountsCoverTheUpload` | every row an upload removes that the counts did not show was synced by the second client after the re-check | `an-upload-with-no-re-check` |
| `AWindowSyncIsNotSilent` | a row the second client synced is on the server, or that client must make a full sync | `an-upload-that-keeps-the-servers-schema` |
| `NoReviewLost` | every review ever made is on the device, the server, the second client, a backup, the server copy or the snapshot | `a-write-that-does-not-re-read-the-device` |

SPEC-334 section 9 words the last clause as "no normal sync between that check and the write". A
client cannot hold another client's sync off the server (M14), so the model states what a client
can hold: no sync before the re-check goes uncounted, and a sync after it is not silent. Closing
the window at the server is section 5's.

## 9. What only a device or a person proves

These are read by the owner, not by CI.

| id | criterion | who, and when |
|---|---|---|
| V1 | A session started offline on Safari on iPhone and iPad and on a desktop browser studies, names its unsynced reviews, and syncs them at the next session's start once online | the owner, in the acceptance session, after part c lands (#637) |
| V2 | An upload and a download on a real collection lose what the choice screen counted, read against Anki on the desktop | the owner, in the acceptance session (#637) |
| V3 | After the browser evicts the site's storage, the next session's start restores the collection from the server | the owner, in the acceptance session (#637) |

## 10. Mutation rows (part a)

`scripts/mutation-rows.d/S35700-S35799.json`, table `MUTATIONS`, each killed by the
named criterion's test:

| row | mutates | killed by |
|---|---|---|
| S35701 | `Losses::between` subtracts the replaced side from the kept side | A3 |
| S35702 | `confirm` admits a direction the offer does not | A4 |
| S35703 | `backed_up` accepts a backup that lacks a review id | A5 |
| S35704 | `backed_up` checks the kept side, not the replaced side | A5 |
| S35705 | `snapshot_found` ignores the answer's `found` | A6 |
| S35706 | `rechecked` ignores a changed id set | A6 |
| S35707 | `rechecked` ignores a changed modified stamp | A6 |
| S35708 | `download_ready` admits an upload | A6 |
| S35709 | `snapshot_found` admits a download | A7 |
| S35710 | `at_write` admits a device row the backup lacks | A8 |
| S35711 | the offer maps a full upload to both directions | A2 |
| S35712 | `id_sets` reads the first row only | A10 |
| S35713 | the unsynced read counts synced reviews | A9 |
| S35714 | `Unsynced::warns` needs every condition, not any | A9 |
