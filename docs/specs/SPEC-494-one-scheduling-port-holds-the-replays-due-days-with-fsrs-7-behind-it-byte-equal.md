# SPEC-494: One scheduling port holds the replay's due days, with FSRS-7 behind it and every due day byte-equal

| field | value |
|---|---|
| status | delivered by the pull request that adds this file |
| issue | #996 |
| decided by | ADR-512, under ADR-338, ADR-356 and ADR-400 |
| measured at | dev `75d555c4` |
| schematic | `docs/schematics/the-scheduling-port-and-its-fsrs-7-implementation.md` (new) |
| mutation band | S49400-S49499 |

## 1. The problem, measured

DeckStreak is to schedule through one interface, the scheduling port, so that a later delivery can put a second
implementation behind it. What was read, at dev `75d555c4`:

- **No trait holds a due day.** A fenced `git grep -l -i -E 'AnkiEngine|fsrs.?7|due_day|due day|SchedulerPort|scheduler'`
  over the tree names 240 files and no `SchedulerPort`.
- **DeckStreak computes one due day, and nothing calls it.** `Dispatcher::replay`
  (`crates/engine-core/src/replay.rs:92-146`) gives a review card `ivl`, its FSRS-7 interval rounded, at most the
  preset's maximum and at least 1, and `due`, the engine day of its last kept review plus `ivl` (`:164-182`,
  `:186-189`). No client adapter names it (`crates/engine-core/tests/graph.rs:582-630`), and only a test writes its
  result; the write is the switch's (#997).
- **Every queue a learner sees is the engine's.** The server's new-card queue asks the engine per top-level deck
  (`crates/ingest/src/engine.rs:544-570`); both clients ask the engine for `GetQueuedCards`
  (`crates/engine-core/src/table.rs:186-192`, `crates/web-engine/src/wasm.rs:263-270` and `:725-729`). An answer's
  next due day is the engine's own state (`crates/ffi/src/engine.rs:184-205`). DeckStreak's code holds no queue order
  rule and no sibling rule of its own.
- **Where DeckStreak consumes it.** Both clients reach the engine's queue only through the core's
  `Dispatcher::run` (`crates/engine-core/src/dispatch.rs:220-245`), from the web engine's `call` (`wasm.rs:103-105`)
  and the native adapter's `run` (`crates/ffi/src/engine.rs:104-116`); the server consumes it in ingest, which has no
  edge to the core (`crates/ingest/Cargo.toml:14-31`). The engine schedules by the released FSRS crate it locks, not
  by the FSRS-7 pin (`Cargo.toml:137-145`).
- **The day frames.** The replay's due counts in the engine's day, whole days since the collection's creation, read
  by the caller (`crates/engine-core/src/dispatch.rs:602-613`, `crates/engine-core/src/late.rs:17-26`). The kernel's
  study day is an epoch day from the rollover hour and the UTC offset (`crates/kernel/src/study_day.rs:104-114`).
- **The fixtures that pin due days.** `crates/engine-core/tests/replay.rs:400-457` (54 days, 1, the maximum 3) and
  `:459-482` (day 1051). The four first-review intervals (`crates/fsrs7/tests/stock.rs:10-15`) each round and round up
  to the same day, so no fixture tells the two apart. The formal vectors pin the history selection only.
- **Bits need the batch.** The single and batch replays agree within a relative 1e-6 only
  (`crates/fsrs7/tests/replay.rs:15-17`); the replay uses the batch (`replay.rs:120`).
- **Rows on the replay.** Rows S38609 to S38620 anchor on `replay.rs` lines this delivery keeps; 0 formal covers name
  `replay.rs`, and 5 name `crates/fsrs7/src/convert.rs`, which is unchanged.
- **Not measured:** which order and sibling-bury options each learner's engine presets set, whether any learner has
  them on, and the size of any learner's collection.

## 2. Requirements

R1. **The port.** `crates/engine-core/src/scheduler.rs`, in the crate where both clients consume the engine's queue,
defines `SchedulerPort` with the associated type `State` and the seven methods `version_id`, `new_state`, `fold`,
`predict`, `interval_days`, `checkpoint` and `restore`, `interval_days`'s doc naming each implementation's own
whole-day rule; `ReviewEvent` with `card`, `note`, `deck`, `preset`, `grade`, `answer_ms`, `key` and `study_day`;
`EventKey`, the event's instant in milliseconds, a total order; `Recall`, one `f32` always within 2^-13 and 1 - 2^-13
and never NaN; and `FoldError` and `LoadError`, plain enums with `Display`. No input panics.

R2. **FSRS-7 implements it.** `Fsrs7`, in an inline module of `crates/engine-core/src/replay.rs`, is the port's one
implementation. Its whole-day interval is the pinned revision's interval at the target rounded to the nearest day.

R3. **The replay's due day comes through the port.** `Dispatcher::replay` folds each card's kept reviews into an
`Fsrs7` state, each event carrying the engine day of its review as its study day, and takes a review card's whole-day
interval only from `interval_days`. Its composition is unchanged: at most the maximum, at least 1, due the last
review's engine day plus the interval. A retention that is no `Recall` is refused.

R4. **Every due day is byte-equal.** Every replayed card's `ivl`, `due`, stability and difficulty equal today's
direct path's, bit for bit.

R5. **The implementation reads no clock.** No clock and no timezone is read in `scheduler.rs` or the port module; the
producer computes each event's study day.

R6. **The day queue.** `scheduler::day_queue` builds a study day's queue from per-card due days: every card due on or
before the day, ordered by due day and then card id.

R7. **The sibling rule.** In that queue two cards of one note never share the study day: the first in the day's order
stays and each later one moves to the next day. A card with no note id (0 or less) takes no part in the rule, enters
the queue, and is counted.

R8. **The fold refuses.** A key equal to or before the last folded key, a grade outside 1 to 4 and an answer time above
3,600,000 milliseconds are refused, and the state is unchanged after any refusal.

R9. **Checkpoint and restore.** A checkpoint is byte-stable and holds the implementation's fixed identifier and the
folded events; a restored checkpoint gives equal bytes and equal intervals; different folded sets give different
checkpoints; truncated bytes, bytes that are no checkpoint and a checkpoint of another implementation are refused.

R10. **The probe is pure.** `predict` changes nothing, answers a `Recall` in range, refuses a day before the card's last
review and an unknown card; `version_id` is the implementation's fixed identifier, which names the model.

R11. **Only through the port.** No production source outside the FSRS-7 crate and the port module names a model call,
the replay's schedule names `interval_days`, and the engine's own queue is asked for only in the files that ask for it
at the cut.

R12. **Unchanged.** The FSRS-7 crate, its pin, arithmetic and parameters, the replay's statement, the core's doors to
the engine, every route, every screen, both client adapters, ingest, the root manifest, the engine core's manifest,
the lockfile, every formal entry and vector, and every existing test and row.

## 3. Acceptance criteria of SPEC-494

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | R2, R3, R4: on the replay fixtures the port's interval and stock values, and the replay's schedule, equal today's direct path bit for bit, and a retention that is no recall is refused | the stub port answers 0 days | ADR-512 D3 |
| A2 | R2, R4: over a fixed grid the port's whole day equals today's rounding, and the grid holds cells that round and round up apart | the stub port answers 0 days | ADR-512 D3 |
| A3 | R6, R7: two cards of one note never share a study day, the later moved to the next day, over named cases and every small queue | the stub queue keeps both siblings | ADR-512 D4 |
| A4 | R7: a card with no note id turns the rule off and is counted | the stub queue counts nothing | ADR-512 D4 |
| A5 | R3, R11: the due days come only through the port and no new file asks the engine for its queue, with planted bypasses refused by name | the stub replay keeps its direct path | ADR-512 D5 |
| A6 | R5: no clock or timezone read sits inside the port, with a planted clock read refused by name | the stub port reads the clock | ADR-512 D5 |
| A7 | R8: a fold refuses a key not after the last, a grade out of range and an answer above the cap, the state unchanged | the stub fold refuses nothing | ADR-512 D5 |
| A8 | R9: a checkpoint restores to equal bytes and equal intervals, and different sets give different checkpoints | the stub checkpoint is empty | ADR-512 D5 |
| A9 | R1, R10: predict is a pure probe in range, the errors read as text, and the version names the model | the stub probe answers one constant | ADR-512 D2 |

```acceptance
A1: cargo test -p deck-streak-engine-core --test scheduler_seam -- --exact fsrs7_behind_the_port_gives_todays_due_days_on_the_replay_fixtures
A2: cargo test -p deck-streak-engine-core --test scheduler_seam -- --exact fsrs7_whole_days_equal_todays_rounding_over_a_fixed_grid
A3: cargo test -p deck-streak-engine-core --test scheduler_seam -- --exact two_cards_of_one_note_never_share_a_study_day
A3: cargo test -p deck-streak-engine-core --test scheduler_seam -- --exact the_sibling_rule_holds_over_every_small_queue
A4: cargo test -p deck-streak-engine-core --test scheduler_seam -- --exact a_card_with_no_note_id_turns_the_rule_off_and_is_counted
A5: cargo test -p deck-streak-engine-core --test scheduler_census -- --exact the_due_days_come_only_through_the_port_and_no_new_file_asks_for_the_engine_queue
A6: cargo test -p deck-streak-engine-core --test scheduler_census -- --exact no_clock_or_timezone_read_sits_inside_the_port
A7: cargo test -p deck-streak-engine-core --test scheduler_seam -- --exact a_fold_refuses_a_key_not_after_the_last_and_leaves_the_state_unchanged
A8: cargo test -p deck-streak-engine-core --test scheduler_seam -- --exact a_checkpoint_restores_to_equal_bytes_and_equal_intervals
A9: cargo test -p deck-streak-engine-core --test scheduler_seam -- --exact predict_is_a_pure_probe_and_the_version_names_the_model
```

The replay suite (`--test replay`, SPEC-386 A11 to A21) and the graph suite (`--test graph`, SPEC-386 A9, A10 and A22)
run unchanged and stay green: they hold today's pins through the port.

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/engine-core/src/scheduler.rs` | engine core | new: the port, its event, key, recall and errors, the event check and the day queue with the sibling rule |
| `crates/engine-core/src/replay.rs` | engine core | changed: the FSRS-7 port module; the replay folds into it and takes its whole days from it |
| `crates/engine-core/src/lib.rs` | engine core | changed: the `scheduler` module and its doc line |
| `crates/engine-core/tests/scheduler_seam.rs` | engine core tests | new: A1 to A4 and A7 to A9 |
| `crates/engine-core/tests/scheduler_census.rs` | engine core tests | new: A5 and A6 |
| `docs/specs/SPEC-494-one-scheduling-port-holds-the-replays-due-days-with-fsrs-7-behind-it-byte-equal.md` | docs | new: this file |
| `docs/decisions/ADR-512-the-scheduling-port-is-one-engine-core-trait-and-fsrs-7-implements-it-in-the-replay-module.md` | docs | new: the decisions |
| `docs/schematics/the-scheduling-port-and-its-fsrs-7-implementation.md` | docs | new: the paths before and after |
| `docs/schematics/the-app-campaigns-surfaces-each-carry-a-stride-table-whose-every-control-cites-a-line-that-holds.md` | docs | changed: the engine-core forbid citation follows its moved line |
| `docs/red-first/SPEC-494.md` | docs | new: the red-first record |
| `scripts/mutation-rows.d/S49400-S49499.json` | rows | new: rows S49400 to S49428 |
| `changelog.d/scheduler-seam-494.md` | changelog | new: the fragment |

## 5. What this does NOT cover

- The second implementation behind the port, its switch and the switch's default; the engine's own queue, every due
  day after an answer and the server's new-card queue through the port; writing the scheduled values into a live
  collection, the client export of the replay, its call at open, the note ids and answer times its statement does not
  read, interval fuzz; and a hash of a folded state, which that implementation's crate brings (#997).
- Where the port runs for the web client, and the bundle bytes a reachable port adds to the web engine (#998).
- Any change to the engine's own scheduling code, its queue order or its sibling-bury options (#233).
- Measuring which learners' engine presets bury same-note siblings, and the Echo Test that measures sibling
  contamination (#138).

## 6. Risks

- **A due day moves.** Detected by A1 and A2, each against today's direct path built in the test, and by the
  unchanged replay suite's pins.
- **The batch's values depend on the batch's members.** The port replays the same cards in the same order as today;
  A1 compares every replayed card bit for bit.
- **A bypass of the port.** Detected by A5's census, whose planted bypasses must each be refused by name.
- **A census that reads nothing.** Each census prints its examined count and refuses zero.
- **A dependency moves.** None is added: the build reads the root manifest, the engine core's manifest and the
  lockfile byte-unchanged at its head; CI's `web-engine` job builds and sizes the bundle.
- **A sibling delivery moves a shared file.** `crates/engine-core/src/lib.rs` and the test support module the new
  tests read are touched by open pull requests; the builder re-measures them at the cut.

## 7. What only CI proves

The web engine's build and bundle size (`web-engine`), the whole workspace's clippy and tests (`rust`), and the
mutation jobs (`mutation-rust`, `mutation-rows`), each read by name at the head.

## 8. Formal model

- **Lean: NOT APPLICABLE.** The day queue and the fold's check are pure functions that decide a study queue and a
  refusal, not a gate verdict, a ledger, money or a release. The day queue is closed by A3's population of every queue
  of four cards over three note values and three due days, judged by an independent property check.
- **TLA+: NOT APPLICABLE.** One actor folds under exclusive access; nothing interleaves.
- **No cover moves.** The history selection's entry covers `crates/fsrs7/src/convert.rs`, which is unchanged; no entry
  covers `crates/engine-core/src/replay.rs`, `lib.rs` or the new module; `formal/vectors/replay-history.jsonl` is
  byte-unchanged.

## 9. Mutation rows

Rows S49400 to S49428 in `scripts/mutation-rows.d/S49400-S49499.json` hold the recall bounds and range check, the
answer cap, the grade range, the key order, the fold's check before its write, the day queue's due bound, order,
one-card-per-note rule, next day and missing-note rule, value and count, the rounding, the replay's retention check,
the checkpoint's header, model check and length check, the probe's last-day bound, the two censuses' planted
refusals, and each literal constant the port adds: the fixed identifier, the checkpoint's magic, its format byte and
its two lengths. Each is killed by a test of this SPEC's fence. The rows S38609 to S38620 on `replay.rs` keep their
anchors and their killers.
