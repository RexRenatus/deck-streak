# SPEC-023: ingest reads the private copy read-only inside the window and scope, and a change gate skips only what nothing would change

- **Wave:** W0. **Issue:** #16 (epic #1). **Context(s):** `deck-streak-ingest`, `deck-streak-coordination` (the obligations registry and the sync cycle).
- **Decided by:** ADR-009 (reads stay read-only SQLite over the copy, bounded to the 400-day window), ADR-008 (the kernel's database base), ADR-002 (cross-context work in coordination), ADR-012 (goldens), ADR-020 (the settings generation in the kernel).
- **Status:** judged: delivered with its tests, `docs/red-first/SPEC-023.md`, and five goldens
  (`allowed_deck_ids`, `study_event`, `ingest_rebase`, `change_probe`, `ingest.constants`) generated
  at the predecessor's `27ee2bc`. The delivery made R1, R3, R4, R8, R11, R12 and the manifest exact
  where the code decided them (§7).

## 1. The problem, measured

- **The predecessor's read** (predecessor `27ee2bc`, names only): `anki_reader.py:read_collection`
  opens the copy as `file:<path>?mode=ro` and reads revlog rows newer than the ingest floor, every
  card with its original deck (`types.py:Card.true_did`), the collection's creation time and the
  deck names; `deck_filter.py:allowed_deck_ids` keeps the decks whose top-level name (before the
  first `\x1f`) starts with a configured prefix, and an empty prefix list keeps every deck; a study
  event is a review of type 0 to 3 with ease 1 or more (`types.py:Review.is_study_event`); names
  are matched in Python, never in SQL, because the collection's name columns use a `unicase`
  collation SQLite does not ship.
- **The window** (`pipeline.py:GamifyPipeline._maybe_rebase_ingest`, `constants.py:INGEST_WINDOW_DAYS`,
  `INGEST_REBASE_DAYS`): only the last window of reviews is read each cycle; older study events are
  one persisted count, recounted in SQL whenever the floor is a rebase period stale, and a count
  that shrinks is logged as a self-check (cards deleted or the collection replaced). This bounded
  window keeps the read inside DeckStreak's memory budget (ADR-032).
- **The change gate** (`pipeline.py:GamifyPipeline._maybe_skip_recompute`,
  `anki_reader.py:probe_change_signal`): the sync always runs, but the recompute is skipped when a
  cheap probe proves nothing observable would change: the same newest review id, card count and
  weighted card fingerprint, the same study day, the same owner-config generation, no explicit
  rescore request, this cycle's sync healthy and the last run not an error.
- **The defect the gate once shipped.** A skipped cycle is exactly the cycle in which a
  deadline-bearing obligation comes due (a window closes unreviewed, a wager expires, a free spin
  lapses): nothing changed in the collection, so an input-keyed gate skipped the recompute that
  should have settled it, and the obligation was stranded past midnight. The predecessor repaired
  it by making every open deadline its own invalidation term
  (`pipeline.py:GamifyPipeline._obligation_deadlines`, `_first_due_obligation`): a deadline inside
  the half-open interval from the last full recompute to now forces the recompute. DeckStreak must
  keep that term, and keep it OPEN to every later wave's deadlines.
- **What the parity oracle proves** (registered in `tools/parity-oracle/registry/spec_023.py`): the
  deck scope (`goldens/allowed_deck_ids.json`, an adapter over `deck_filter.py:allowed_deck_ids`
  converting the JSON keys to deck ids), the study-event rule (`goldens/study_event.json`, an
  adapter over `types.py:Review.is_study_event`), the window and rebase
  (`goldens/ingest_rebase.json`, an adapter over `_maybe_rebase_ingest` with a stub store and a
  patched offload returning the case's count), the probe's fingerprint (`goldens/change_probe.json`,
  an adapter that writes the case's synthetic card rows into a temporary collection and calls
  `probe_change_signal`), and the constants (`goldens/ingest.constants.json`).

**Order.** After SPEC-022 (same crate: the copy, the collection lock, `sync_runs`, and the sync
cycle this SPEC extends). It can run beside SPEC-027, which schedules the cycle without editing
it. SPEC-021 declares this SPEC's table, so it lands after this one.

## 2. Requirements

The read (#16)

R1. `ingest::CollectionReader` opens the copy with the kernel's `Db::open_foreign_read_only`
    (`mode=ro`, `query_only`) under the shared collection lock, on the kernel's `Offload`; any write
    through it fails.
R2. Scope: a card is read when its ORIGINAL deck's top-level name (the text before the first
    `\x1f`) starts with a prefix of `DECKSTREAK_INCLUDE_DECKS` (comma-separated; empty or unset
    reads every deck); a review is read when its card is in scope and it is a study event (type 0 to
    3, ease 1 or more). The deck set equals `goldens/allowed_deck_ids.json`, and the study-event rule
    `goldens/study_event.json`, for every case.
R3. The track of a card is `law` when its top-level deck name starts with `DECKSTREAK_LAW_DECK_ROOT`
    (optional) and `language` otherwise. When the include list is non-empty and no prefix covers
    the law root, start logs one WARN naming both settings (the predecessor's warning, ported).
R4. No SQL predicate, ordering or aggregate touches a deck, note-type or tag name column; names are
    read by id and matched in Rust, so a collection whose name columns use `COLLATE unicase` is read
    with no collation registered.
R5. The reader returns only DeckStreak's own types (reviews, cards with their deck id, original deck
    id and track, the collection's creation time, deck names); no engine or SQL type leaves `ingest`.
R6. The window: reviews newer than the ingest floor are read; the floor is the persisted base, or
    `INGEST_WINDOW_DAYS` days before now when there is none; the base (floor and pre-floor
    study-event count) is recounted in SQL when it is more than `INGEST_REBASE_DAYS` days staler
    than a fresh floor; a recount lower than the stored count logs one WARN (the self-check). The
    kept or written base equals `goldens/ingest_rebase.json` for every case.
R7. The study-day rule is DeckStreak's configuration (SPEC-020); the reader never takes a rollover
    hour from the collection.

The change gate

R8. `ingest::gate::decide` is a pure function of the gate's inputs, returning `Run` with a reason or
    `Skip`: it runs when an owner rescore is pending (consumed once), when this cycle's sync
    failed, when there is no successful run on record or the last run failed, when the persisted
    anchor is missing or unreadable, when the kernel's settings generation changed, when the study
    day changed, when the probe's newest review id, card count or fingerprint changed, or when a
    registered deadline lies after the anchor's last recompute and at or before now; otherwise it
    skips.
R9. The probe reads the newest revlog id and the count and weighted fingerprint of the cards
    (integer columns only, never a name column); its values equal `goldens/change_probe.json`.
R10. `coordination::obligations` is the registration port for time-driven obligations: a context's
    delivery registers a named source of open deadlines (a label and an instant each); every cycle
    collects them and passes them to `decide`. A deadline not yet due, or at or before the last
    recompute, never holds the gate open, so each deadline costs exactly the one recompute that
    serves it.
R11. The anchor (newest review id, card count, fingerprint, study day, the instant of the last full
    recompute and the settings generation) and the pending rescore flag live in `ingest_state` (one
    row; `migrations/002301_ingest_state.sql`, `STRICT`, `created_at`), with the window's base. A
    full recompute writes a fresh anchor; a skip writes a `skipped` row in `sync_runs` and leaves
    the anchor as it was.
R12. `coordination::sync_cycle` runs: sync (SPEC-022), then the probe and `decide`, then either the
    read and the recompute (at W0 the recompute has no domain consumer and only writes the anchor)
    or the skip record.
R13. `ingest`'s data-rights port resets `ingest_state` in place; the table is registered in
    docs/CONTEXT-MAP.md's register of DeckStreak's own tables.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a write through the collection reader fails | `reader` test |
| A2 | only scoped cards and their study reviews are read | `reader` test over a synthetic collection |
| A3 | the deck scope equals the predecessor's | `scope` test over `goldens/allowed_deck_ids.json` |
| A4 | the study-event rule equals the predecessor's | `scope` test over `goldens/study_event.json` |
| A5 | a card in a filtered deck is scoped by its original deck | `scope` test |
| A6 | a deck table collated `unicase` is read with no collation registered | `reader` test |
| A7 | a law root the include list does not cover is warned by name at start | `scope` test |
| A8 | the ingest window and its rebase equal the predecessor's | `window` test over `goldens/ingest_rebase.json` |
| A9 | a shrinking pre-window count is reported by the self-check | `window` test |
| A10 | the change probe equals the predecessor's | `gate` test over `goldens/change_probe.json` |
| A11 | an unchanged cycle skips the recompute and records `skipped` | `gate` test |
| A12 | a new review runs the recompute | `gate` test |
| A13 | a changed card runs the recompute | `gate` test |
| A14 | a new study day runs the recompute | `gate` test |
| A15 | a settings change runs the recompute | `gate` test |
| A16 | a failed sync, or a failed last run, never skips | `gate` test |
| A17 | an owner rescore forces exactly one recompute | `gate` test |
| A18 | a registered deadline that passes runs the recompute on an otherwise unchanged cycle | coordination `change_gate` test |
| A19 | a deadline not yet due, or already served by a recompute, lets the gate skip | coordination `change_gate` test |
| A20 | the ingest port resets `ingest_state` in place | `data_rights` test |

```acceptance
A1: cargo test -p deck-streak-ingest --test reader -- --exact a_write_through_the_collection_reader_fails
A2: cargo test -p deck-streak-ingest --test reader -- --exact only_scoped_cards_and_their_study_reviews_are_read
A3: cargo test -p deck-streak-ingest --test scope -- --exact the_deck_scope_matches_the_predecessors_golden
A4: cargo test -p deck-streak-ingest --test scope -- --exact the_study_event_rule_matches_the_predecessors_golden
A5: cargo test -p deck-streak-ingest --test scope -- --exact a_card_in_a_filtered_deck_is_scoped_by_its_original_deck
A6: cargo test -p deck-streak-ingest --test reader -- --exact a_unicase_collated_deck_table_is_read_without_the_collation
A7: cargo test -p deck-streak-ingest --test scope -- --exact a_law_root_outside_the_include_list_is_warned_by_name
A8: cargo test -p deck-streak-ingest --test window -- --exact the_ingest_window_and_its_rebase_match_the_predecessors_golden
A9: cargo test -p deck-streak-ingest --test window -- --exact a_shrinking_pre_window_count_is_reported_by_the_self_check
A10: cargo test -p deck-streak-ingest --test gate -- --exact the_change_probe_matches_the_predecessors_golden
A11: cargo test -p deck-streak-ingest --test gate -- --exact an_unchanged_cycle_skips_the_recompute_and_records_skipped
A12: cargo test -p deck-streak-ingest --test gate -- --exact a_new_review_runs_the_recompute
A13: cargo test -p deck-streak-ingest --test gate -- --exact a_changed_card_runs_the_recompute
A14: cargo test -p deck-streak-ingest --test gate -- --exact a_new_study_day_runs_the_recompute
A15: cargo test -p deck-streak-ingest --test gate -- --exact a_settings_change_runs_the_recompute
A16: cargo test -p deck-streak-ingest --test gate -- --exact a_failed_sync_or_a_failed_last_run_never_skips
A17: cargo test -p deck-streak-ingest --test gate -- --exact an_owner_rescore_forces_exactly_one_recompute
A18: cargo test -p deck-streak-coordination --test change_gate -- --exact a_registered_deadline_that_passes_runs_the_recompute_on_an_unchanged_cycle
A19: cargo test -p deck-streak-coordination --test change_gate -- --exact a_deadline_not_yet_due_or_already_served_lets_the_gate_skip
A20: cargo test -p deck-streak-ingest --test data_rights -- --exact the_ingest_port_resets_its_state_in_place
```

Each gate test holds every input but one fixed and changes that one (the issue's "a test per
input"). A18 registers a synthetic obligation source in the coordination registry; no feature's
real deadlines exist yet. Every clock is a `ManualClock`.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/ingest/src/reader.rs` | `deck-streak-ingest` | added: the read-only read, scope and track |
| `crates/ingest/src/window.rs` | `deck-streak-ingest` | added: the floor, the base and the self-check |
| `crates/ingest/src/gate.rs` | `deck-streak-ingest` | added: the probe and `decide` |
| `crates/ingest/src/state.rs` | `deck-streak-ingest` | added: `ingest_state` |
| `crates/ingest/src/settings.rs` | `deck-streak-ingest` | changed: the include list and the law root |
| `crates/ingest/src/data_rights.rs`, `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed |
| `crates/ingest/tests/reader.rs`, `scope.rs`, `window.rs`, `gate.rs` | `deck-streak-ingest` | added: A1 to A17 |
| `crates/ingest/tests/data_rights.rs` | `deck-streak-ingest` | changed: A20 |
| `crates/ingest/tests/support/synthetic.rs` | `deck-streak-ingest` | changed: filtered decks and a `unicase` deck table |
| `crates/ingest/tests/support/logs.rs`, `crates/ingest/tests/support/mod.rs` | `deck-streak-ingest` | added, changed: the log recorder A7 and A9 read their WARN through (§7) |
| `crates/ingest/src/sync_runs.rs` | `deck-streak-ingest` | changed: the `skipped` row and the run history the gate reads (§7) |
| `crates/coordination/src/obligations.rs` | `deck-streak-coordination` | added: the registration port |
| `crates/coordination/src/sync_cycle.rs`, `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: probe, gate, read or skip |
| `crates/coordination/tests/change_gate.rs` | `deck-streak-coordination` | added: A18, A19 |
| `crates/coordination/Cargo.toml`, `Cargo.lock` | `deck-streak-coordination` | changed: `thiserror`; dev `tokio` and `tempfile` (§7) |
| `crates/daemon/src/role_job.rs` | `deck-streak-daemon` | changed: the `sync` job builds the whole cycle, and a unit test of its reason codes (§7) |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: the `sync` job pages a malformed scope before it syncs (§7) |
| `migrations/002301_ingest_state.sql` | `deck-streak-ingest` | added |
| `.sqlx/` | workspace | changed |
| `tools/parity-oracle/registry/spec_023.py` | repo | added |
| `tools/parity-oracle/goldens/allowed_deck_ids.json`, `study_event.json`, `ingest_rebase.json`, `change_probe.json`, `ingest.constants.json` | repo | added |
| `docs/CONTEXT-MAP.md` | repo | changed: `ingest_state` registered to `ingest` |
| `.env.example` | repo | changed: the include list and the law root, by name, with neutral examples |
| `docs/schematics/sync-cycle-and-change-gate.md` | repo | changed: the gate's decision (shared with SPEC-022) |
| `docs/red-first/SPEC-023.md` | repo | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It registers no feature's deadlines: each deadline-bearing feature registers its own source in
  the delivery that builds it (#109, #112,
  #113, #119, #118,
  #110, #115, #105,
  #100, and the readings' jobs, #39).
- It registers no end-of-quiet-hours term, which the router contributes with the deferral it
  serves (#27).
- It recomputes no domain fact: the rollup, XP, streaks and the rest arrive in W3
  (#66).
- It flushes no deferred celebration on a skipped cycle; the router does (#27).
- It parses no Bloom-tier tag and no FSRS card state (#71,
  #90).
- It resolves no day set of new cards (#31).
- It offers no rescore command; the bot's `/sync` sets the flag this gate consumes
  (#19).

## 6. Risks

- **A later feature forgets to register its deadline** and reproduces the stranded-obligation
  defect. Mitigated by R10's port and by each later SPEC's own criterion; the architect's W3 and W5
  SPECs carry "the feature registers its deadline" as an acceptance criterion.
- **The fingerprint's weights overflow differently in Rust.** The predecessor sums in SQL with a
  modulus per card; the port runs the same SQL, and A10's golden includes cards whose weighted terms
  exceed 32 bits.
- **A deck renamed on another device changes the scope silently.** Scope is re-derived from names
  on every read, as in the predecessor; a card leaving scope drops out of the read and the next
  recompute.
- **The recount runs during a busy cycle.** It is one indexed `COUNT` on the read-only connection,
  bounded to once per rebase period, on the offload rail, so it never blocks the async runtime.

## 7. Amendments at delivery

- **Manifest: three more files, no new crate.** `sync_runs.rs` is the one writer of `sync_runs`, so
  the skip's `skipped` row (R11) and the record the gate's run-history term reads (R8) are two methods
  there. The WARNs R3 and R6 promise are read as positive artifacts through a recording subscriber in
  the ingest tests' support, which needs only the `tracing` crate ingest already depends on. The
  sync cycle's error type is a `thiserror` enum like every library's, and the change-gate tests run
  whole cycles over a temporary database and copy, so `coordination` names `thiserror`, and `tokio`
  and `tempfile` as dev-dependencies: three crates the workspace already locks, which adds three
  lines to `Cargo.lock` and no version.
- **R1: one read is one session.** Every read of the copy, the probe and the window's recount
  included, takes the shared collection lock, then a worker of the kernel's `Offload`, then a
  connection from `Db::open_foreign_read_only`; `sqlx`'s queries run to their end on the offload's
  blocking thread, so the offload's bound and its slow-operation WARN cover the whole read. A1 proves
  the refusal through `CollectionReader::execute_statement`, the reader's one door that runs a
  caller's statement: it returns a count of changed rows and never a row, so no Anki data leaves the
  crate through it, and every write is refused, the one after lifting `query_only` included.
- **R3: the law root is read as a prefix.** R3 reads the root as the include list reads a prefix. The
  predecessor's law set (`pipeline.py:GamifyPipeline._run_sync_cycle_impl`) compared the top-level
  name with its root for equality; the two differ only for a top-level deck whose name extends the
  root, which a test pins as R3 says. The WARN names both settings and never a value, since deck names
  are private configuration.
- **R4: names by id, deck ids in SQL.** The card and review reads, and the window's recount, filter by
  the allowed deck ids, integers bound as one JSON array that `json_each` expands; the scope itself is
  decided in Rust over the names read by id, and the study-event rule is applied in Rust to the rows
  read (the recount's count needs its SQL mirror, as the predecessor's did). A6 shows the trap is live
  on the reader's own connection: an ordering by name there fails for want of `unicase`.
- **R8: the record is read before the cycle's own sync.** The predecessor's gate read its last run
  before its own cycle wrote a row; here the sync writes its row first, so the cycle reads the record
  before it syncs, and "no successful run on record" is read at the same moment. A sync refused for
  the day or debounced made no request, so the gate counts it healthy; a failed earlier run still
  holds the gate open through the run-history term. A partial anchor, or a row that is gone, reads as
  unreadable and runs the recompute, whose write puts the row back.
- **R11: the anchor is the one the cycle decided on.** A full recompute writes the probe, study day,
  instant and settings generation its cycle decided on, read before the read, so a sync that lands
  between them makes the next cycle run again rather than skip. The same write clears the rescore
  flag: a request is consumed by the recompute that served it, and a recompute that fails serves none.
- **R12 and A18: whole cycles.** A18 and A19 run `sync_cycle` end to end over an engine whose every
  sync finds no change and a copy the engine itself created, with a synthetic source registered in
  `coordination::obligations`. Tests beyond the fence pin R8's order, R10's window, R8's unreadable
  anchor, R3's prefix and the settings' parse.
- **R12: the cycle's parts, and the `sync` job.** `sync_cycle` runs over `CycleParts`: the syncer,
  the reader, the gate, the obligations and the clock. SPEC-027's runner reaches the cycle through its
  port `runner::SyncCycle`, which keeps that name; the daemon's `sync` job implements the port by
  building the parts from the job's own settings when it runs (an empty registry until a feature
  registers a source), and hands the port the cycle's sync report. A cycle that cannot run to its end
  is recorded `sync_record_failed`, `obligations_unreadable` or `recompute_failed`. That wiring came
  in the merge commit that absorbed SPEC-027, so its two tests followed it and were proved by hand
  mutation instead of red first (`docs/red-first/SPEC-023.md`): a unit test holds every kind of
  `CycleError` to its code, and a `roles` test runs `deckstreakd job sync` with a malformed scope and
  reads the page `scope_settings_refused`, the fire recorded `error`, and no sync run.
- **R9: the fingerprint's constants are bound.** The weights and the modulus are bound parameters
  of one static statement, never formatted into it (web-security's `ws.db-rules`); bound integers
  take the same integer arithmetic as the predecessor's literals, and A10's golden holds all 16
  cases.
- **R2: the include list.** Unset, empty or blank reads every deck; the predecessor also read `ALL` as
  every deck, which R2 does not name, so it is read as a prefix.
- **Goldens.** `change_probe`'s adapter writes the case's rows into a temporary collection through
  the `sqlite3` module the predecessor's reader already imports: the registry module imports no
  database module and reads nothing, and only the predecessor's probe reads the file back.

Amendment (2026-09-28): a passage describing another service's operations was replaced with
DeckStreak's own memory budget under the public-text rule (ADR-059).
