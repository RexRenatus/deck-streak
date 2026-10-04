# SPEC-345: the engine core: one allow-listed dispatcher both clients share, and an owner-gesture token only the UI adapters construct

- **Wave:** the app campaign, Phase 1 (SPEC-334 rows 1.4 and 1.5, R3 and R7). **Issue:** #623.
  **Context(s):** `deck-streak-engine-core` (added: the one client-side holder of Anki's engine),
  `deck-streak-ffi` and `deck-streak-web-engine` (rewired through it).
- **Decided by:** ADR-337 (the allow-listed dispatcher, the `OwnerGesture` token and the
  containment test), ADR-335 and ADR-336 (the two transports share one allow-list), ADR-345 (the
  FFI adapter), ADR-348 (the web engine), the signed owner-taps ruling in `docs/rulings/`, and
  ADR-356 (the core crate, its per-transport table, the gesture's target check and the
  containment test's form).
- **Status:** part 1 (sections 2 to 4, R1 to R6) is delivered by the pull request that adds this
  file, with `docs/red-first/SPEC-345.md`; part 2 (R7 to R10) by the next pull request
  (section 7). **Base:** live `dev`. **Mutation band:** `S345`.

## 1. The problem, measured

Every figure was read at `dev` `1eec0870e67e801fc3aa279318219d6986116ec7` (DEV below) unless the row
names another ref. The SwiftUI harness's open pull request (#656, issue #616) was read at
`33fb20d928cace083b59023c0787418598455453` (HARNESS below), whose merge-base with DEV is
`7507d8a22014a3effc02429bb855167a9e0d39ba` (BASE below). The engine was read at the pinned fork rev
`c538de55a23e695234e794029fce0dafff2d36a9` (PIN below) in a clone of the fork.

| id | what | figure | command |
|---|---|---|---|
| M1 | The native adapter's allow-list | five pairs: (3,0) `BackendCollectionService.OpenCollection`, (7,13) `DecksService.GetDeckNames`, (13,3) `SchedulerService.GetQueuedCards`, (13,4) `SchedulerService.AnswerCard`, (3,8) `CollectionService.Undo`; `allowed()` finds a pair or `None` | `git show DEV:crates/ffi/src/allow_list.rs \| sed -n 11,57p` |
| M2 | How the native adapter refuses | `Engine::run` checks `allowed()` before the engine sees the call, then calls `Backend::run_service_method` (line 94); `EngineRefusal` has three variants (lines 12-32) whose text (lines 34-49) `tests/refusal_text.rs` pins, the killer of row `S33600` | `git show DEV:crates/ffi/src/engine.rs \| sed -n 12,97p`; `git show DEV:crates/ffi/tests/refusal_text.rs` |
| M3 | What the harness changes in the native adapter | one pair added, (27,6) `CardRenderingService.RenderExistingCard` (`[Call; 6]`); the round-trip tests split into `tests/support/`; `tests/render.rs` and an example added; row `S33900` anchored on `src/allow_list.rs`; `src/engine.rs` and `Cargo.toml` untouched (7 files, +385/-197 in `crates/ffi`) | `git diff --stat BASE HARNESS -- crates/ffi scripts/mutation-rows.d`; `git diff DEV HARNESS -- crates/ffi` |
| M4 | Whether the web engine routes through an allow-list | only `run_method` (`src/wasm.rs:277-280`) calls `admit` (`src/study.rs:89-95`, over the eight `STUDY_CALLS` at lines 74-83); `call()` (lines 61-68) reaches `run_service_method` directly (line 63) for `open`, `close`, `seed`, `next_card`, `answer` and `undo`, and `query()` (lines 74-86) passes SQL to `run_db_command_bytes` (line 82) | `git show DEV:crates/web-engine/src/wasm.rs \| sed -n 40,90p`; `git show DEV:crates/web-engine/src/study.rs \| sed -n 61,95p` |
| M5 | What the web client calls | the named exports only (`web/app/src/lib/engine/session.ts` lines 92-167); `run_method` is not a protocol operation (`session.test.ts:127`) | `git grep -n -E 'engine\.[a-z_]+\(' DEV -- web/app/src/lib/engine/session.ts`; `git show DEV:web/app/src/lib/engine/session.test.ts \| sed -n 127p` |
| M6 | The engine's pin | `Cargo.lock` lines 60-62 hold `anki` at the fork's PIN; the root manifest's `[patch]` (lines 132-134) moves `anki` and `anki_proto` to it | `git show DEV:Cargo.lock \| sed -n 58,63p`; `git show DEV:Cargo.toml \| sed -n 131,134p` |
| M7 | How a pair is numbered | `run_service_method` is generated (`rslib/rust_interface.rs:141`); services are indexed in sorted proto order (`rslib/proto/rust.rs:72-91`); each odd index is a backend service holding its own methods, then its collection service's (`rslib/proto_gen/src/lib.rs:46-80`). Derived and equal to all 17 distinct pairs this tree pins: the 5 native, the 8 web study calls (4 shared with the native list) and the 8 refused writes | `git show PIN:<path> \| sed -n <range>p` for each file named |
| M8 | The never-list's taps, mapped to the engine at PIN | see the table below: entries 2, 3, 4, 6 and 7 map to one method each with one target, and entry 8 to two; entries 1 and 5 map to no single-target method | `git show PIN:<file> \| sed -n <line>p` per row |
| M9 | The never-list's enforcement today | ADR-301 (a)'s rows (lines 112-119) are held by `scripts/tests/test_declared_write_classes.py` (its `ROW` regex, line 136, and `test_each_never_list_entry_names_what_it_protects`, line 305); the web engine's `run_method_admits_only_the_study_calls` refuses eight exempt pairs; no Rust code holds an exempt write behind a token | `git grep -n -i 'never.list\|never_list\|exempt' DEV -- crates scripts docs/decisions` (the hits in other senses, such as data-rights exemptions, read and set aside) |
| M10 | The dependency graph | `anki` is named by `crates/ffi` (normal), `crates/ingest` (normal), `crates/readings` (dev only) and `crates/web-engine` (its `wasm32` table only); no member names `deck-streak-ffi` or `deck-streak-web-engine`; the context map declares both as `depends on: nothing` (`docs/CONTEXT-MAP.md` lines 37 and 40) | `for f in $(git ls-tree --name-only DEV crates/ \| sed 's#$#/Cargo.toml#'); do git show DEV:$f \| grep -n -E '^\[\|anki\|deck-streak-(ffi\|web-engine)'; done` |
| M11 | Where workspace code reaches an engine write or a raw door | 11 call- or path-shaped lines: `ffi/src/engine.rs:94`; `web-engine/src/wasm.rs:11` (a `use` of `init_backend`), `:63`, `:82`; the bare `init_backend` calls at `wasm.rs:124` and `ffi/src/engine.rs:72` follow a `use` and match only as a name in it; `ingest/src/engine.rs:221` (the mirror's full download, SPEC-022 R6) and `ingest/tests/support/mod.rs:665` (an upload to the scratch test server) reach the engine; five more name the mirror's own port method (`ingest/src/sync.rs:340`, `ingest/examples/engine_probe.rs:72`, `ingest/tests/engine_budget.rs:111` and `:173`, `ingest/tests/sync.rs:461`) | `git grep -n -E '(\.(NAMES)[[:space:]]*\(\|::(NAMES)\b)' DEV -- crates`, NAMES the 22 engine names of R10 joined by `\|` |
| M12 | The census conventions | every member is `deck-streak-<dir>` with `[lints] workspace = true` (`crates/daemon/tests/workspace.rs`); the settle census refuses a member whose feature another member turns on (`crates/progression/tests/xp_census.rs:805-836`), so a feature-gated constructor cannot pass it; the wallet census reads every crate's sources, prints its examined count and refuses a planted file by name (`crates/economy/tests/wallet_census.rs:45-175`) | `git show DEV:<file> \| sed -n <range>p` |
| M13 | The rows on the files this touches | `S33600` on `ffi/src/engine.rs`'s `Display` body; `S33804` and `S33805` on `web-engine/src/study.rs`; `S33900` (HARNESS) on `ffi/src/allow_list.rs`; none on `web-engine/src/wasm.rs`; a row may not leave while its file stays (`scripts/mutation_rows.py` lines 45-46, 1228-1256) | `git show DEV:scripts/mutation-rows.d/S33600-S33699.json`; the same for `S33800-S33899.json` and HARNESS's `S33900-S33999.json` |
| M14 | What CI builds per crate | the `rust` job runs `scripts/check.sh fmt clippy test doctest audit-rust` over the workspace (`.github/workflows/ci.yml` lines 30-94); `mutation-rust` runs `cargo mutants --in-diff`, sharded (lines 344-492); the `web-engine` job builds the module for `wasm32` and runs the engine's browser tests (lines 734-796) | `git show DEV:.github/workflows/ci.yml \| sed -n 30,94p` and the other ranges |
| M15 | Undo after a sync | a normal sync discards the undo queue (`rslib/src/sync/collection/normal.rs:86`, `discard_undo_and_study_queues`), so the ordinary `Undo` (3,8) cannot reach an answer that has synced | `git show PIN:rslib/src/sync/collection/normal.rs \| sed -n 80,90p` |

The never-list's taps (the owner-taps ruling's table, ADR-301 lines 112-119) at PIN:

| entry | the tap | engine method, pair | implementation at PIN | request's target field(s) | target |
|---|---|---|---|---|---|
| 1 | undo an answer after it has synced | none: the undo queue is gone after a sync (M15) | `rslib/src/collection/service.rs:27` (`undo`, the ordinary call) | none | none |
| 2 | Forget | `SchedulerService.ScheduleCardsAsNew`, (13,17) | `scheduler/service/mod.rs:132`, `scheduler/new.rs:153` | `card_ids` (`scheduler.proto:219`) | one card |
| 3 | delete a preset | `DeckConfigService.RemoveDeckConfig`, (11,5) | `deckconfig/service.rs:70` | `dcid` (`deck_config.proto:22`) | one preset |
| 4 | the one-way sync, upload or download | `BackendSyncService.FullUploadOrDownload`, (1,6) | `backend/sync.rs:146`; `sync/collection/upload.rs:38`, `download.rs:21` | `upload` (`sync.proto:85-90`) | the collection, one direction |
| 5 | switching a preset's scheduler | no one-preset method: `UpdateDeckConfigs` (11,7) carries a collection-wide `fsrs` (`deck_config.proto:261`), a `fsrs_reschedule` and `removed_config_ids` (`deckconfig/update.rs:171`); `UpgradeScheduler` (13,26) is collection-wide | `deckconfig/update.rs:84`; `scheduler/service/mod.rs:238` | none single | none |
| 6 | set due date | `SchedulerService.SetDueDate`, (13,19) | `scheduler/service/mod.rs:159`, `scheduler/reviews.rs:128` | `card_ids` (`scheduler.proto:240`) | one card |
| 7 | change note type | `NotetypesService.ChangeNotetype`, (23,15) | `notetype/service.rs:189`, `notetype/notetypechange.rs:115` | `note_ids` (`notetypes.proto:205`) | one note |
| 8 | delete a reviewed card or note | `CardsService.RemoveCards`, (5,2); `NotesService.RemoveNotes`, (25,7) | `card/service.rs:48`; `notes/service.rs:101`, which removes the notes of `card_ids` when `note_ids` is empty | `card_ids` (`cards.proto:68`); `note_ids` and `card_ids` (`notes.proto:89`) | one card; one note |

So the two clients reach the engine through two tables that nothing holds equal, the web engine's
named exports bypass its own table, the engine's raw database door takes SQL from the adapter, and
no exempt write exists behind a token: ADR-337's dispatcher, token and containment test are not
built.

## 2. Requirements

Part 1, delivered by this pull request:

R1. A new crate, `crates/engine-core` (package `deck-streak-engine-core`), is the one client-side
    holder of Anki's engine. It holds the engine's `Backend` privately and exposes a `Dispatcher`:
    `start(transport, init)`, `run(service, method, input)` and `read(read)`. It declares no
    feature and no `staticlib`, and inherits the workspace lints. A native `Dispatcher` is `Send`
    and `Sync`, so the native adapter can hold it behind UniFFI.
R2. The dispatcher's table is per transport (ADR-356 D2). A `Dispatcher` started on
    `Transport::Native` admits exactly (3,0), (3,8), (7,13), (13,3) and (13,4); one started on
    `Transport::Web` admits exactly (3,0), (3,1), (3,8), (13,3), (13,4), (23,8), (25,0) and (25,2).
    `run` refuses an exempt pair (R3) with `Refusal::NeedsGesture` and every other pair with
    `Refusal::NotAllowed`, before the engine sees the call.
R3. The exempt table names, with the engine's name and the kind of target each takes, exactly the
    six methods M8 maps for entries 2, 3, 6, 7 and 8: (13,17) Forget, card; (13,19) set due date,
    card; (11,5) delete a preset, preset; (23,15) change note type, note; (5,2) delete a card, card;
    (25,7) delete a note, note. Every other never-list method stays unlisted and refused by `run`:
    the one-way sync (1,6), which M8 maps for entry 4 and which waits for its measurement and its
    guards (section 5), `UpdateDeckConfigs` (11,7), `UpgradeScheduler` (13,26), `GradeNow` (13,20),
    `RemoveDecks` (7,16) and `RemoveNotetype` (23,11) included.
R4. The engine's database door takes no SQL from an adapter: `Dispatcher::read` takes a closed
    `Read` (the note count, and one card's scheduling fields by id), and the core holds the two
    read-only statements the web engine passes today.
R5. Both adapters reach the engine only through the core. The native `Engine` holds a `Dispatcher`
    started on `Transport::Native`, checks its own allow-list first as today, and keeps
    `EngineRefusal` and its text byte for byte (row `S33600`). The web engine's `call()` and
    `query()` go through a `Dispatcher` started on `Transport::Web`. `ffi/src/allow_list.rs` and
    `web-engine/src/study.rs` are unchanged, and each equals its transport's column of the core's
    table.
R6. The graph holds it. `deck-streak-ffi` and `deck-streak-web-engine` (under its `wasm32` table
    only, ADR-348) name `deck-streak-engine-core`, and no other member does. No member names `anki`
    in a normal, build or target table except `deck-streak-engine-core` and `deck-streak-ingest`;
    the native adapter keeps `anki` as a dev-dependency for its synthetic collections. No member
    names either adapter. The context map declares the core as `depends on: nothing` and each
    adapter as `depends on: engine-core`.

Part 2, delivered by the next pull request (section 7):

R7. `OwnerGesture` names one exempt write and one target of that write's kind. Its fields are
    private, it is neither `Clone` nor `Copy`, and its one constructor, `OwnerGesture::from_tap`,
    refuses a target of another kind.
R8. `Dispatcher::run_exempt(gesture, input)` consumes the gesture, takes the pair from the
    gesture's write, decodes the input as that write's request, refuses it unless it names exactly
    the gesture's target (`card_ids == [card]`; `dcid == preset`; `note_ids == [note]`, and for
    `RemoveNotes` `card_ids` empty), and passes the engine the message it checked, re-encoded,
    never the caller's bytes.
R9. Each adapter has one exempt entry, and only it constructs a gesture: the native `Engine`'s
    `run_exempt(write, target, input)` (a UniFFI export, its own refusal type beside
    `EngineRefusal`) and the web engine's `run_exempt` export (`wasm32` only, no Worker route).
R10. A containment test in the core enumerates every caller in the workspace and prints the count:
    outside the core, `OwnerGesture::from_tap` and `run_exempt` are named only in
    `crates/ffi/src/engine.rs` and `crates/web-engine/src/wasm.rs`. The engine's 22 write and door
    names (`schedule_cards_as_new`, `reschedule_cards_as_new`, `reschedule_cards_as_new_defaults`,
    `set_due_date`, `remove_deck_config`, `update_deck_configs`, `upgrade_scheduler`,
    `upgrade_to_v2_scheduler`, `change_notetype`, `change_notetype_of_notes`, `remove_cards`,
    `remove_cards_and_orphaned_notes`, `remove_notes`, `full_upload_or_download`, `full_upload`,
    `full_download`, `grade_now`, `remove_decks`, `remove_decks_and_child_decks`,
    `run_service_method`, `run_db_command_bytes`, `init_backend`), as a call (`.name(` or
    `::name(`), a path (`::name`) or a name in a `use`, are reached outside the core only at the
    seven held `ingest` lines of M11, each with its reason. No member includes a core file by
    `#[path]` or `include!`, and a planted caller in a non-UI crate is refused by name.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | Over every pair in 0..=64 by 0..=64, a dispatcher on each transport admits exactly that transport's ordinary pairs, holds exactly the six exempt pairs for a gesture, and refuses every other pair as not allowed; it prints the pairs it examined | `cargo test -p deck-streak-engine-core --test table -- --exact every_pair_is_admitted_held_or_refused_by_its_transport` |
| A2 | The exempt table names each of the six writes with its pair, the engine's name and the kind of target it takes | `cargo test -p deck-streak-engine-core --test table -- --exact each_exempt_write_names_its_engine_call_and_its_target_kind` |
| A3 | On a synthetic collection, an ordinary call reaches the engine (an answer raises the card's repetitions) and an exempt pair through `run` is refused as needing a gesture with the card's row unchanged | `cargo test -p deck-streak-engine-core --test dispatch -- --exact an_ordinary_call_reaches_the_engine_and_an_exempt_one_does_not` |
| A4 | A fixed read returns the one card it names, of two | `cargo test -p deck-streak-engine-core --test dispatch -- --exact a_fixed_read_returns_its_one_card` |
| A5 | The native adapter's allow-list equals the native column, and the web engine's study calls equal the web column | `cargo test -p deck-streak-engine-core --test parity -- --exact each_adapter_table_equals_its_transport_column` |
| A6 | Each adapter starts its dispatcher on its own transport and never names the other's | `cargo test -p deck-streak-engine-core --test parity -- --exact each_adapter_starts_its_dispatcher_on_its_own_transport` |
| A7 | Only the two adapters name the core, only the core and ingest name the engine outside dev-dependencies, nothing names an adapter, and a planted member naming the core is refused by name; it prints the manifests it examined | `cargo test -p deck-streak-engine-core --test graph -- --exact only_the_client_adapters_reach_the_engine_core` |
| A8 | The context map declares the core with no dependency and each adapter as depending on it | `cargo test -p deck-streak-engine-core --test graph -- --exact the_context_map_declares_the_core_and_its_two_edges` |
| A9 | The native adapter's refusals read as before | `cargo test -p deck-streak-ffi --test refusal_text -- --exact each_refusal_reads_as_its_own_sentence` |
| A10 | The native adapter's round trip runs as before through the core | `cargo test -p deck-streak-ffi --test round_trip` |
| A11 | The web engine's `run_method` admits only the study calls, as before | `cargo test -p deck-streak-web-engine --test study -- --exact run_method_admits_only_the_study_calls` |

```acceptance
A1: cargo test -p deck-streak-engine-core --test table -- --exact every_pair_is_admitted_held_or_refused_by_its_transport
A2: cargo test -p deck-streak-engine-core --test table -- --exact each_exempt_write_names_its_engine_call_and_its_target_kind
A3: cargo test -p deck-streak-engine-core --test dispatch -- --exact an_ordinary_call_reaches_the_engine_and_an_exempt_one_does_not
A4: cargo test -p deck-streak-engine-core --test dispatch -- --exact a_fixed_read_returns_its_one_card
A5: cargo test -p deck-streak-engine-core --test parity -- --exact each_adapter_table_equals_its_transport_column
A6: cargo test -p deck-streak-engine-core --test parity -- --exact each_adapter_starts_its_dispatcher_on_its_own_transport
A7: cargo test -p deck-streak-engine-core --test graph -- --exact only_the_client_adapters_reach_the_engine_core
A8: cargo test -p deck-streak-engine-core --test graph -- --exact the_context_map_declares_the_core_and_its_two_edges
A9: cargo test -p deck-streak-ffi --test refusal_text -- --exact each_refusal_reads_as_its_own_sentence
A10: cargo test -p deck-streak-ffi --test round_trip
A11: cargo test -p deck-streak-web-engine --test study -- --exact run_method_admits_only_the_study_calls
```

The red each shows first, recorded in `docs/red-first/SPEC-345.md`:

- A1 to A6: red at the tests commit, over the shape commit's core, whose tables are empty and whose
  `run` and `read` refuse everything. A1 fails on the admitted set (empty against five and eight),
  A2 on the exempt table (empty against six), A3 on the answer (refused, repetitions unchanged), A4
  on the read (refused), A5 on each column (empty against five and eight), and A6 on the adapters'
  sources (neither starts a dispatcher yet).
- A7 and A8: committed alone, before any manifest or map change, and red by assertion: the
  adapters still name `anki` and not the core, and the map still reads `depends on: nothing` for
  both. Each test also judges a planted tree inside the same test, so a census that went blind
  fails it.
- A9, A10 and A11: not red. They pin the adapters' behaviour the base already has, and guard the
  rewire.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/engine-core/Cargo.toml` | `deck-streak-engine-core` | added: `anki`, `anki_proto`, `prost`, `serde_json`; no feature, no `staticlib` (part 1) |
| `crates/engine-core/src/lib.rs` | `deck-streak-engine-core` | added: the crate root, `#![forbid(unsafe_code)]` (part 1) |
| `crates/engine-core/src/table.rs` | `deck-streak-engine-core` | added: `Transport`, the ordinary table with its transport columns, the exempt table with its `ExemptWrite` (part 1) |
| `crates/engine-core/src/dispatch.rs` | `deck-streak-engine-core` | added: `Dispatcher`, `Refusal`, `Read` (part 1); `run_exempt` (part 2) |
| `crates/engine-core/src/gesture.rs` | `deck-streak-engine-core` | added: `OwnerGesture`, `Target` (part 2) |
| `crates/engine-core/tests/table.rs` | `deck-streak-engine-core` | added: A1, A2 (part 1) |
| `crates/engine-core/tests/dispatch.rs` | `deck-streak-engine-core` | added: A3, A4 (part 1) |
| `crates/engine-core/tests/parity.rs` | `deck-streak-engine-core` | added: A5, A6 (part 1) |
| `crates/engine-core/tests/graph.rs` | `deck-streak-engine-core` | added: A7, A8 (part 1) |
| `crates/engine-core/tests/support/mod.rs` | `deck-streak-engine-core` | added: the synthetic collection and the `examined` helper (part 1) |
| `crates/engine-core/tests/gesture.rs` | `deck-streak-engine-core` | added: A12 (part 2) |
| `crates/engine-core/tests/exempt.rs` | `deck-streak-engine-core` | added: A13, A14 (part 2) |
| `crates/engine-core/tests/containment.rs` | `deck-streak-engine-core` | added: A15, A16 (part 2) |
| `crates/ffi/Cargo.toml` | `deck-streak-ffi` | the core replaces `anki` in `[dependencies]`; `anki` moves to `[dev-dependencies]` (part 1) |
| `crates/ffi/src/engine.rs` | `deck-streak-ffi` | `Engine` holds a `Dispatcher`; the `Display` body of `EngineRefusal` unchanged (part 1); `run_exempt` and its refusal type (part 2) |
| `crates/ffi/src/lib.rs` | `deck-streak-ffi` | the crate's doc names the core (part 1) |
| `crates/ffi/tests/exempt.rs` | `deck-streak-ffi` | added: A17, A18 (part 2) |
| `crates/web-engine/Cargo.toml` | `deck-streak-web-engine` | the core replaces `anki` in the `wasm32` table (part 1) |
| `crates/web-engine/src/wasm.rs` | `deck-streak-web-engine` | `call()` and `query()` through the dispatcher; the two statements move to the core (part 1); the `run_exempt` export (part 2) |
| `Cargo.toml` | workspace | `deck-streak-engine-core` in `[workspace.dependencies]` (part 1) |
| `Cargo.lock` | workspace | changed by `cargo` only (part 1) |
| `docs/CONTEXT-MAP.md` | campaign | the core's line, and the two adapters' `depends on: engine-core` (part 1) |
| `docs/decisions/ADR-356-the-engine-core-holds-the-engine-for-both-clients-behind-a-per-transport-table.md` | campaign | added (part 1) |
| `docs/schematics/engine-core-dispatcher-and-owner-gesture.md` | campaign | added: the component diagram and the call paths, before and after (part 1) |
| `docs/specs/SPEC-345-the-engine-core-one-allow-listed-dispatcher-and-an-owner-gesture-token.md` | campaign | added (part 1) |
| `docs/red-first/SPEC-345.md` | campaign | added (part 1), extended (part 2) |
| `scripts/mutation-rows.d/S34500-S34599.json` | campaign | added (part 1), extended (part 2) |
| `changelog.d/engine-core-345.md` | campaign | added (part 1); a second fragment for part 2 |

## 5. What this does NOT do

- It builds no screen that calls an exempt entry, and it does not prove which Swift or TypeScript
  code calls the adapters' entries: the first tap screens add those callers and the census that
  holds each to a gesture handler (#631, #633), and the later parity taps (Forget, set due date,
  change note type, delete, deck options) ride the later parity phases (#611, as SPEC-334
  section 5 cites them).
- It builds no undo after a sync: the engine discards its undo queue at a normal sync (M15), so
  entry 1 has no engine method at the pin; the measurement of undo's review-log row (#620) and
  undo with sync on iPhone and iPad (#633) decide it.
- It leaves the one-way sync (1,6) out of the exempt table: the full-sync path is measured by #620,
  and SYNC-01's guards (each side's losses shown, the on-device backup, the server snapshot check)
  and the full-sync model are built with the web sync screens (#631) and on iPhone and iPad
  (#633).
- It builds no scheduler switch: `UpdateDeckConfigs` (11,7) and `UpgradeScheduler` (13,26) stay
  refused, and the one-preset FSRS-7 switch is the FSRS-7 crate's (#641).
- It builds no umbrella crate, XCFramework, Swift package or static library (#624).
- It adds no Worker protocol route for `run_exempt` or `run_method` (#630, #631).
- It does not add the render call (27,6) to the native column: the harness's delivery adds it to
  the allow-list (#616), and whichever of the two lands second carries the pair into the core's
  native column, which A5 names.
- It adds no dated note to ADR-301 line 107: the delivery that builds the first exempt tap adds it,
  as the ruling's order note says (#631, #633).
- It adds no Swift or TypeScript mutation rows (#650).

## 6. Risks

- **The adapters' tables drift from the core's.** Two lists of one boundary can diverge, most
  likely when the harness's render pair lands. A5 compares each adapter's table with its column,
  pair by pair, and names the pair that differs.
- **The engine renumbers a pair at a new pin.** A1 pins every pair; A3 and the native round trip
  (A10) call real pairs on a synthetic collection, so a renumbered pair fails by decode or by its
  effect.
- **A gesture carries a wider write than its target.** A request can name many ids, and
  `RemoveNotes` removes the notes of `card_ids` when `note_ids` is empty (M8). R8 checks the
  decoded request against the one target and passes the engine the message it checked (A14).
- **A non-UI path reaches the core or the engine.** The graph census (A7) refuses a new
  dependent; the containment census (A15) refuses a new caller, a `#[path]` or `include!` of a
  core file, and a new line that names an engine write, each by name.
- **The mirror's own engine port.** `ingest` names the engine and so is not held by the graph. Its
  full download writes only DeckStreak's read copy of the collection, which no sync carries to the
  server, and A15 holds its seven lines by text, so any new one fails until a reviewer admits it.
- **Changing a note's type forces a one-way sync next.** The core runs the write; the tap screen
  must say so before the owner confirms, and the next sync's full-sync choice is the owner's own
  tap (#631, #633).
- **The web adapter's exempt export compiles only for `wasm32`.** The `web-engine` CI job builds
  it; its behaviour is the core's, which A13 and A14 test natively.

## 7. Delivered by the next pull request

| id | criterion | delivered by |
|---|---|---|
| A12 | `OwnerGesture::from_tap` takes one target of its write's kind and refuses another kind | `cargo test -p deck-streak-engine-core --test gesture -- --exact a_gesture_takes_one_target_of_its_writes_kind` |
| A13 | On a synthetic collection of two cards, a Forget gesture on one card resets that card and leaves the other unchanged | `cargo test -p deck-streak-engine-core --test exempt -- --exact a_forget_gesture_resets_its_one_card_and_no_other` |
| A14 | For each of the six writes, a request naming no target, another target, or the target and another (and `RemoveNotes` with any `card_ids`) is refused before the engine sees it, every card's row unchanged | `cargo test -p deck-streak-engine-core --test exempt -- --exact a_request_naming_more_than_its_gestures_target_is_refused` |
| A15 | Outside the core, the gesture's constructor and `run_exempt` are named only in the two adapters' entry files, and the engine's write and door names only at the seven held lines; no `#[path]` or `include!` of a core file; planted callers in the daemon, the bot, coordination and ingest are refused by name; it prints the callers and files examined | `cargo test -p deck-streak-engine-core --test containment -- --exact no_non_ui_caller_reaches_an_exempt_function` |
| A16 | `OwnerGesture` derives and implements neither `Clone` nor `Copy` | `cargo test -p deck-streak-engine-core --test containment -- --exact the_gesture_is_neither_clone_nor_copy` |
| A17 | The native adapter's exempt entry runs a Forget tap on its one card | `cargo test -p deck-streak-ffi --test exempt -- --exact a_native_forget_tap_resets_its_one_card` |
| A18 | Each exempt refusal of the native adapter reads as its own sentence | `cargo test -p deck-streak-ffi --test exempt -- --exact each_exempt_refusal_reads_as_its_own_sentence` |

A12 to A17 are red at their own tests commit over a `run_exempt` that refuses every gesture and a
census whose callers are not yet written: A15 fails on its positive artifact, the two adapter
entries it must find. A18 is mutation coverage, as A9 is for the refusals it mirrors.
