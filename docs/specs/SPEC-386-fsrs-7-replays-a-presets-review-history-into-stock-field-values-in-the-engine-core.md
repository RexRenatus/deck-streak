# SPEC-386: FSRS-7 replays a preset's review history into stock-field values in the engine core

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
| issue | #641 |
| campaign row | SPEC-334 stretch row 2.4 (R10) |
| decided by | ADR-400, under ADR-338 and ADR-353 |
| measured at | dev `e7ecf10d` |
| schematic | `docs/schematics/fsrs-7-replay-undo-probe-and-full-sync-choice.md`, sections 6 to 8 (insert-only) |

## 1. The problem, measured

Every fact below was read at `e7ecf10d` with `git show e7ecf10d:<path>` or `git grep -n` at that commit.

- **The crate exists and is isolated.** `crates/fsrs7` is the package `deck-streak-fsrs7`. Its one dependency is the
  workspace key `fsrs7`, the upstream scheduler at the 40-hex revision `Cargo.toml:141` pins; `fsrs6`, the released
  scheduler, is a dev-dependency only (`crates/fsrs7/Cargo.toml:14-21`). `scripts/tests/test_fsrs7_pin.py:121`
  (`crate_findings`) holds that dependency list exact. `docs/CONTEXT-MAP.md:39` reads `deck-streak-fsrs7 ... depends on:
  nothing`, and no crate depends on it.
- **No published release carries FSRS-7.** The package registry's newest release of the scheduler is the one
  `Cargo.toml:145` pins for the dev-dependency, with 21 parameters; FSRS-7 has 34 and exists only on the upstream
  development line. The library documentation index holds no FSRS-7 API either, so the pinned revision's own source is
  the only record of it (ADR-353, pin facts table, `:134-145`).
- **The history conversion follows SPEC-342 R4 and has no reset cut.** `crates/fsrs7/src/convert.rs:18-26` defines
  `RevlogRow { cid, id, ease, kind }`, with no factor column. `:73` (`kept`) drops ease 0 and the kinds
  `DROPPED_KINDS = [4, 5]` (`:14`), and `:43` (`histories`) groups by card, orders by id and gives the first kept review
  the delta 0. A card's reviews from before a reset are kept. `CardHistory` (`:31-36`) carries no last-review id.
- **The replay exists, and nothing calls it outside the crate.** `crates/fsrs7/src/replay.rs:40` (`replay`) runs the
  single or the batch method over histories, and `:60` (`checksum`) sums stabilities. `crates/fsrs7/tests/replay.rs:13`
  types one reference value, the initial stability for Good, from the pinned revision's default parameters.
- **SPEC-342 measured the cost.** Per review: native 0.1778-0.1937 µs, wasm32-wasip1 0.2122-0.2329 µs. At one million
  rows the slowest cell is 232.014 ms (wasm, batch) and 193.657 ms native
  (`docs/specs/SPEC-342-fsrs-7-replay-time-undo-and-the-full-sync-choice-are-measured.md:312-339`). `:341` asks #641 to
  read that cost against SPEC-334 row 1.2's collection-size measurement; no document at `e7ecf10d` records a review-row
  count (`git grep` over `docs` returns none).
- **The engine core holds the engine and depends on no workspace crate.** `crates/engine-core/src/lib.rs:41-42` says so,
  and `crates/engine-core/tests/graph.rs:444-476` asserts the map line `docs/CONTEXT-MAP.md:37` reads `nothing`. Its two
  dependents are the native adapter and the web engine (`graph.rs:32-35`; `CONTEXT-MAP.md:38`, `:42`).
- **The engine core reads the collection only through fixed statements.** `crates/engine-core/src/dispatch.rs:27-83`
  holds the fixed statements; `:431-451` (`read`, `query`) sends one statement per call through the engine's database
  door and asks for the first row only. Every method takes `&self` (`:203`, `:431`, `:481`, `:500`).
- **The scheduler switch is an unlisted write.** `crates/engine-core/src/table.rs:279-285` keeps the scheduler switch and
  every never-list method unlisted, so `run` refuses them. The owner-taps ruling makes a preset's scheduler switch the
  mass-reschedule entry (the owner-taps ruling under `docs/rulings/`, `:16`), admitted only from the owner's tap on one
  preset, and binds every batch, background job and sync repair (`:26-41`).
- **The stock fields.** A card's memory state lives in its `data` field as `s`, `d`, `decay`, `dr` and `lrt`
  (`crates/ingest/src/memory_state.rs:105-124`), beside the card's `due` and `ivl` columns. ADR-338 decides the replay
  writes only the stock fields, the stability as the 90-percent stability (`ADR-338 :39-54`).
- **No client reaches the replay.** `crates/ffi/src/engine.rs:85-559` exports no replay, and neither does
  `crates/web-engine/src/wasm.rs`. The native engine object is shared (`Arc`, `crates/ffi/src/engine.rs:69-70`, `:87`)
  and holds no lock of its own.
- **The web bundle has a size budget.** `scripts/web-engine-size.py:33` holds it, measured by CI's `web-engine` job
  (`.github/workflows/ci.yml:767-826`). The web build turns the browser random source on through the engine's own graph
  (`scripts/web-engine-build.sh:10-11`).

## 2. Requirements

R1. **The crate and its pin are unchanged.** `crates/fsrs7` keeps the revision `Cargo.toml:141` pins and its one
dependency; the lockfile keeps exactly two scheduler packages. No random-source feature is added to the crate.

R2. **A reset cuts a card's history.** `RevlogRow` gains `factor`. `histories` drops every row of a card up to and
including the card's last reset row, then applies SPEC-342 R4's drops. A reset row is the engine's own Forget row:
kind 4 with factor 0, confirmed against the engine at its pin by R12.

R3. **A history names its last kept review.** `CardHistory` gains `last_id`, the id of the card's last kept row.

R4. **The selection is a function of the row set.** Any arrival order of one set of rows gives the same histories,
in card-id order, as the id-ordered rows give.

R5. **The replay equals the reference.** Over fixed histories, the replay's stability and difficulty equal the values
the pinned revision's own tests assert, within the tolerance those tests use, with each history's inputs fixed the way
that test fixes them. Each of the four first ratings replays to its initial stability, typed from the pinned
revision's default parameters.

R6. **The stock projection.** A new pure module `crates/fsrs7/src/stock.rs` maps a memory state to stock values: the
stability is the interval at which the pinned revision's forgetting curve reads 0.9 (the 90-percent stability), by
the revision's own interval function; the difficulty is clamped to 1 to 10; the interval is the revision's interval at
the preset's desired retention.

R7. **One seam.** The engine core gains one dependency, `deck-streak-fsrs7`, named by exactly one source file,
`crates/engine-core/src/replay.rs`. `docs/CONTEXT-MAP.md`'s engine-core line reads `depends on: fsrs7`; ADR-356's
"no crate of this workspace" sentence is amended, insert-only.

R8. **The replay reads one statement.** `Dispatcher::replay` takes a deck set, a parameter vector (empty for the
pinned defaults, or 34 values), the desired retention, the maximum interval and the engine day its caller read. The
replay itself makes no engine call: the engine's own day read unburies cards on a new day, which R10 forbids. It reads
the review rows of the cards
whose home deck (the original deck when the card sits in a filtered deck) is in the set, with each card's type, by ONE
fixed statement through the database door, ordered by card and id. A review row whose card no longer exists is not
read. A card with no kept review gets no entry, and an empty deck set gives an empty result.

R9. **The schedule.** For a card of the review type, the result carries `ivl`, the interval rounded and clamped to 1
and the maximum interval, and `due`, the engine day of the card's last kept review plus `ivl`, with no fuzz. A card of
another type carries the stability and difficulty only.

R10. **The replay writes nothing.** A replay leaves every row of the collection, its modification stamp and its undo
status as they were.

R11. **The stock fields survive the engine.** Written through the engine's own card update in a test, the projected
values read back after a sync round trip with `s`, `d`, `ivl` and `due` equal to the projection, and every other card
column and data key unchanged.

R12. **The selection agrees with the engine.** The engine's own Forget writes a row the selection reads as a reset;
its set-due-date row is not one; and a replayed card's last kept review is the engine's own last-review time.

R13. **An undone answer is invisible.** After an answer and its undo, the replay equals the replay before the answer.

R14. **No client reaches it.** No source of the native adapter or the web engine names the replay.

R15. **The selection is proved.** A Lean entry ports the selection over integer rows and proves the reset cut, the
drops, the first delta of 0, non-negative deltas and order independence; its vectors are read by a Rust test of the
crate.

## 3. Acceptance criteria of SPEC-386

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | R1: the pin, the one dependency and the two scheduler packages hold | not red: SPEC-342's pin test already holds them; it guards the new edge | ADR-400 D1 |
| A2 | R2: a reset drops every earlier row of its card | the review before the reset is kept | ADR-400 D2 |
| A3 | R3: a history names its last kept review | the field does not exist yet: the assertion on its value fails | ADR-400 D2 |
| A4 | R4: every arrival order gives the same histories | a permuted set with a reset keeps a pre-reset review | ADR-400 D2 |
| A5 | R5: the replay equals the pinned revision's own vectors | no vector test exists; the asserted values are absent | ADR-400 D4 |
| A6 | R5: each first rating replays to its initial stability | Again, Hard and Easy are unasserted | ADR-400 D4 |
| A7 | R6: the stock stability is the 90-percent interval | the module does not exist | ADR-400 D2 |
| A8 | R6: the difficulty is clamped and the interval meets the retention | the module does not exist | ADR-400 D2 |
| A9 | R7: only the replay module names the crate | the census finds no such module | ADR-400 D1 |
| A10 | R7: the map declares the core's edge to fsrs7 | the map line still reads nothing | ADR-400 D1 |
| A11 | R8: a deck set's replay reads only its home decks' cards | the method does not exist | ADR-400 D2 |
| A12 | R8: a deleted card's reviews are not replayed | the method does not exist | ADR-400 D2 |
| A13 | R8: a card with no kept review has no entry | the method does not exist | ADR-400 D2 |
| A14 | R9: a review card's schedule is its last day plus its interval | the method does not exist | ADR-400 D2 |
| A15 | R9: a learning card keeps its due and interval | the method does not exist | ADR-400 D2 |
| A16 | R10: the replay moves no stamp, no undo and no row | the method does not exist | ADR-400 D3 |
| A17 | R11: the stock fields survive a sync round trip alone | the mapping does not exist | ADR-400 D3 |
| A18 | R12: the engine's Forget reads as a reset and its set-due-date does not | the row has no factor to read | ADR-400 D2 |
| A19 | R12: the last kept review is the engine's last-review time | the history names no last review | ADR-400 D2 |
| A20 | R13: an undone answer leaves the replay as before it | the method does not exist | ADR-400 D2 |
| A21 | R8: two replays of one collection are equal | the method does not exist | ADR-400 D4 |
| A22 | R14: no client adapter names the replay | not red: no adapter names it today; the census guards it, with a planted control | ADR-400 D3 |
| A23 | R15: the selection equals the Lean vectors | the vectors file does not exist | ADR-400 D4 |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_fsrs7_pin.py
A2: cargo test -p deck-streak-fsrs7 --test convert -- --exact a_reset_cuts_every_earlier_review_of_its_card
A3: cargo test -p deck-streak-fsrs7 --test convert -- --exact the_history_names_its_last_kept_review
A4: cargo test -p deck-streak-fsrs7 --test convert -- --exact any_arrival_order_gives_the_same_histories
A5: cargo test -p deck-streak-fsrs7 --test replay -- --exact the_replay_equals_the_pinned_revisions_own_vectors
A6: cargo test -p deck-streak-fsrs7 --test replay -- --exact each_first_rating_replays_to_its_initial_stability
A7: cargo test -p deck-streak-fsrs7 --test stock -- --exact the_stock_stability_is_the_ninety_percent_interval
A8: cargo test -p deck-streak-fsrs7 --test stock -- --exact the_difficulty_is_clamped_and_the_interval_meets_the_retention
A9: cargo test -p deck-streak-engine-core --test graph -- --exact only_the_replay_module_names_the_fsrs7_crate
A10: cargo test -p deck-streak-engine-core --test graph -- --exact the_context_map_declares_the_core_and_its_two_edges
A11: cargo test -p deck-streak-engine-core --test replay -- --exact a_deck_sets_replay_reads_only_its_home_decks_cards
A12: cargo test -p deck-streak-engine-core --test replay -- --exact a_deleted_cards_reviews_are_not_replayed
A13: cargo test -p deck-streak-engine-core --test replay -- --exact a_card_with_no_kept_review_has_no_entry
A14: cargo test -p deck-streak-engine-core --test replay -- --exact a_review_cards_due_is_its_last_review_day_plus_its_interval
A15: cargo test -p deck-streak-engine-core --test replay -- --exact a_learning_card_keeps_its_due_and_interval
A16: cargo test -p deck-streak-engine-core --test replay -- --exact the_replay_moves_no_stamp_no_undo_and_no_row
A17: cargo test -p deck-streak-engine-core --test replay -- --exact the_stock_fields_survive_a_sync_round_trip_alone
A18: cargo test -p deck-streak-engine-core --test replay -- --exact the_engines_forget_reads_as_a_reset_and_set_due_does_not
A19: cargo test -p deck-streak-engine-core --test replay -- --exact the_last_kept_review_is_the_engines_last_review_time
A20: cargo test -p deck-streak-engine-core --test replay -- --exact an_undone_answer_leaves_the_replay_as_before_it
A21: cargo test -p deck-streak-engine-core --test replay -- --exact two_replays_of_one_collection_are_equal
A22: cargo test -p deck-streak-engine-core --test graph -- --exact no_client_adapter_names_the_replay
A23: cargo test -p deck-streak-fsrs7 --test formal_vectors_replay_history -- --exact the_history_selection_matches_the_lean_vectors
```

Each enumerating test (A4's permutations, A9's and A22's source walks, A23's vectors) prints its examined count and
refuses zero. A9 and A22 each carry a planted control the census must refuse by name. A2, A4 and A23 assert the kept
rows by value, never an absence alone. A5's values are typed from the pinned revision's own test, cited by file and
line, never computed by the code under test.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-386-fsrs-7-replays-a-presets-review-history-into-stock-field-values-in-the-engine-core.md` | document | new: this SPEC |
| `docs/decisions/ADR-400-the-fsrs-7-replay-reads-one-statement-through-one-engine-core-seam-and-writes-nothing.md` | document | new |
| `docs/red-first/SPEC-386.md` | document | new: red-first record |
| `changelog.d/fsrs-replay-386.md` | document | new: fragment |
| `scripts/mutation-rows.d/S38600-S38699.json` | rows | new: rows band |
| `crates/fsrs7/src/stock.rs` | fsrs7 | new: the stock projection |
| `crates/fsrs7/tests/stock.rs` | fsrs7 | new: A7, A8 |
| `crates/fsrs7/tests/formal_vectors_replay_history.rs` | fsrs7 | new: A23 |
| `crates/engine-core/src/replay.rs` | engine-core | new: the one seam |
| `crates/engine-core/tests/replay.rs` | engine-core | new: A11 to A21 |
| `formal/lean/Formal/ReplayHistory.lean` | formal | new: the selection's port, theorems and witness |
| `formal/lean/Formal/ReplayHistoryVectors.lean` | formal | new: its vectors writer |
| `formal/vectors/replay-history.jsonl` | formal | new: written by the writer, never by hand |
| `crates/fsrs7/src/convert.rs` | fsrs7 | `factor`, the reset cut, `last_id` |
| `crates/fsrs7/src/lib.rs` | fsrs7 | `pub mod stock` |
| `crates/fsrs7/src/measure/history.rs` | fsrs7 | the generator sets a non-zero factor |
| `crates/fsrs7/tests/convert.rs` | fsrs7 | A2 to A4; the row helper's factor |
| `crates/fsrs7/tests/replay.rs` | fsrs7 | A5, A6 |
| `crates/fsrs7/tests/history.rs` | fsrs7 | the generator golden's row literal gains `factor` |
| `crates/engine-core/Cargo.toml` | engine-core | the one edge |
| `crates/engine-core/src/lib.rs` | engine-core | `pub mod replay`, and the doc sentence of `:41-42` |
| `crates/engine-core/src/dispatch.rs` | engine-core | the fixed history statement and its many-row call |
| `crates/engine-core/tests/graph.rs` | engine-core | A9, A10, A22 |
| `Cargo.lock` | workspace | the new edge |
| `docs/CONTEXT-MAP.md` | document | the engine-core line and its prose |
| `docs/decisions/ADR-356-the-engine-core-holds-the-engine-for-both-clients-behind-a-per-transport-table.md` | document | an insert-only amendment of D4 |
| `docs/schematics/fsrs-7-replay-undo-probe-and-full-sync-choice.md` | document | sections 6 to 8, insert-only |
| `formal/lean/Formal/Vectors.lean` | formal | one writer arm |
| `formal/lean/Formal.lean` | formal | one import |
| `.github/workflows/apple-on-change.yml` | ci | the change caller watches `crates/fsrs7/**` |
| `.github/workflows/testflight-internal.yml` | ci | the internal push filter watches `crates/fsrs7/**` |
| `scripts/tests/test_ci_workflows.py` | tests | `APPLE_PATHS` gains `crates/fsrs7/**` |
| `scripts/tests/test_testflight_workflows.py` | tests | `INTERNAL_PATHS` gains `crates/fsrs7/**` |
| `docs/schematics/context-map.md` | document | the `engine-core --> fsrs7` arrow and its layer, and the prose beside it |

No migration. No file under `web/`, `ios/`, `crates/ffi/` or `crates/web-engine/` changes.

## 5. What this does NOT cover

- Writing the stock fields into a live collection: that is the scheduler switch, the owner's tap on one preset, made
  by the preset screen (#611). This delivery computes the values and proves their write only in a test.
- The client export of the replay, its call at open, and the bundle bytes a reachable replay adds to the web engine
  (#611).
- Fitting FSRS-7 parameters and storing a fitted vector; the replay accepts a vector, and nothing fits or keeps one
  here (#611).
- Interval fuzz at the switch (#611).
- Resolving a preset's decks; the caller passes the deck set (#611).
- A standalone browser build of the crate, SPEC-342's M4 (#626).
- A full or one-way sync caused by any write (#631), and the undo of a scheduler switch (#630).
- Linking the XP crate into the client adapters (#639).
- Any change to the engine's own scheduling code (#233).

## 6. Risks

- **The reset predicate differs from the engine's.** Detected by A18, which writes the engine's own Forget and
  set-due-date rows and reads them through the selection.
- **The day of a review differs from the engine's day.** Detected by A14, whose fixture's review times and day cutoff
  are fixed the way the engine's own scheduling test fixes them, and by A19.
- **The read's cost at a large collection is unmeasured.** The compute is bounded per review by SPEC-342 (at most
  0.2329 µs a review in the browser target); the one-statement read returns every row at once. Detected when the first
  caller times the replay at open (#611).
- **The JSON array the statement binds needs the database's JSON functions in every target.** The native target is
  tested here; the web target is reached only by #611, whose web test runs the statement.
- **A tolerance hides a difference.** A5 uses the tolerance of the pinned revision's own test, cited by file and line;
  a looser one is a weakening.

## 7. What only CI proves

- The `web-engine` job's size check passes with the new edge in the graph; the build records its reading beside
  dev's in the hand-back. No export reaches the replay, so the reachable code is unchanged.
- The `fsrs7-measure` workflow runs on the crate's change; its checksums match SPEC-342's, because the generator writes
  no reset.

## 8. Formal model

- **Lean: REQUIRED** for the history selection, a total pure function over integer rows that decides which reviews
  count. The entry `lean/ReplayHistory` covers `crates/fsrs7/src/convert.rs::histories` and `::kept` and the new reset
  predicate, proves the reset cut, the drops, the first delta of 0, non-negative deltas and order independence, and
  its witness shows the merge-base's `histories` keeps a review from before a reset. Its vectors are read by A23.
- **No Lean for the model's arithmetic:** the replay over 32-bit floats is the pinned revision's own code, held by A5
  against that revision's own test values.
- **TLA+: NOT APPLICABLE by surface.** The replay reads one statement and writes nothing; it adds no actor and no
  write path, and composes no check with an act. The act that reads its result, the switch, is #611's.
- No `@phx covers` anchor of `crates/engine-core/src/dispatch.rs` changes: the new statement and read sit outside
  every covered item.

## 9. Mutation rows

Band `scripts/mutation-rows.d/S38600-S38699.json`, from S38600, in the `MUTATIONS` table. Each `find` is written after
`cargo fmt` and occurs exactly once at its target.

| stem | file | mutant | killer |
|---|---|---|---|
| S38600-RESET-FACTOR | `crates/fsrs7/src/convert.rs` | the reset predicate's factor test inverted | `convert::a_reset_cuts_every_earlier_review_of_its_card` |
| S38601-RESET-KIND | `crates/fsrs7/src/convert.rs` | the reset kind constant moved off 4 | `convert::a_reset_cuts_every_earlier_review_of_its_card` |
| S38602-LAST-RESET | `crates/fsrs7/src/convert.rs` | the cut finds the first reset, not the last | `convert::a_reset_cuts_every_earlier_review_of_its_card` |
| S38603-LAST-ID | `crates/fsrs7/src/convert.rs` | `last_id` takes the first kept row | `convert::the_history_names_its_last_kept_review` |
| S38604-STOCK-RETENTION | `crates/fsrs7/src/stock.rs` | the stock retention constant moved off 0.9 | `stock::the_stock_stability_is_the_ninety_percent_interval` |
| S38605-D-CLAMP-LOW | `crates/fsrs7/src/stock.rs` | the difficulty's lower clamp moved | `stock::the_difficulty_is_clamped_and_the_interval_meets_the_retention` |
| S38606-D-CLAMP-HIGH | `crates/fsrs7/src/stock.rs` | the difficulty's upper clamp moved | `stock::the_difficulty_is_clamped_and_the_interval_meets_the_retention` |
| S38607-HOME-DECK | `crates/engine-core/src/dispatch.rs` | the statement reads the current deck, not the home deck | `replay::a_deck_sets_replay_reads_only_its_home_decks_cards` |
| S38608-DECK-SET | `crates/engine-core/src/dispatch.rs` | the deck-set filter is negated | `replay::a_deck_sets_replay_reads_only_its_home_decks_cards` |
| S38609-IVL-FLOOR | `crates/engine-core/src/replay.rs` | the interval's floor of 1 dropped | `replay::a_review_cards_due_is_its_last_review_day_plus_its_interval` |
| S38610-DUE-SUM | `crates/engine-core/src/replay.rs` | the due drops the interval | `replay::a_review_cards_due_is_its_last_review_day_plus_its_interval` |
| S38611-REVIEW-TYPE | `crates/engine-core/src/replay.rs` | every card type gets a schedule | `replay::a_learning_card_keeps_its_due_and_interval` |
| S38612-EMPTY-ENTRY | `crates/fsrs7/src/convert.rs` | a card with no kept review gets an entry | `convert::manual_rescheduled_and_unrated_entries_are_dropped` |
| S38613-ONE-SEAM | `crates/engine-core/tests/graph.rs` | the seam census admits a second file | `graph::only_the_replay_module_names_the_fsrs7_crate` |
| S38614-IVL-CEILING | `crates/engine-core/src/replay.rs` | the interval's ceiling at the preset's maximum dropped | `replay::a_review_cards_due_is_its_last_review_day_plus_its_interval` |
| S38615-PARAMS-LENGTH | `crates/engine-core/src/replay.rs` | the parameter count constant moved off 34 | `replay::a_parameter_vector_of_another_length_is_refused_whole` |
| S38616-RESET-SINCE | `crates/fsrs7/src/convert.rs` | a card with no reset keeps its history from its second row | `convert::a_cards_reviews_become_fractional_day_intervals_from_zero` |
| S38617-REVIEW-TYPE-VALUE | `crates/engine-core/src/replay.rs` | the review card type constant moved off 2 | `replay::a_review_cards_due_is_its_last_review_day_plus_its_interval` |
| S38618-MS-PER-SECOND | `crates/engine-core/src/replay.rs` | the milliseconds-per-second constant moved off 1000 | `replay::a_review_cards_due_is_its_last_review_day_plus_its_interval` |
| S38619-SECONDS-PER-DAY | `crates/engine-core/src/replay.rs` | the seconds-per-day constant moved off 86400 | `replay::a_review_cards_due_is_its_last_review_day_plus_its_interval` |
| S38620-STOCK-SCHEDULE | `crates/engine-core/src/replay.rs` | the mapping writes no interval and no due | `replay::the_stock_fields_survive_a_sync_round_trip_alone` |

The builder may add rows for any further constant or branch it writes; a new literal constant owes a row pinning its
value. Every row's killer lives in its row's own crate.
