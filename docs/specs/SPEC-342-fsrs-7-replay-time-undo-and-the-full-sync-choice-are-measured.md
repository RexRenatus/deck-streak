# SPEC-342: FSRS-7 replay time, undo's review-log row and the full-sync choice path are measured

- **Issue:** #620, SPEC-334 row 1.2's scheduler, undo and full-sync measurements (R8, R10).
  **Context(s):** `deck-streak-fsrs7`, a new context: the isolated FSRS-7 scheduler (ADR-338), which
  depends on no DeckStreak crate; and `deck-streak-ingest`'s tests, which measure the engine through
  the harness they already hold. No product code of an existing context changes.
- **Decided by:** ADR-353 (D1 to D6), resting on ADR-338 (the isolated crate, its `allow-git`
  entry, replay at open) and ADR-337 (never-list entries 1 and 4, `OwnerGesture`, the containment
  test).
- **Status:** a measurement delivery. **Mutation band:** S34200-S34299.

## 1. The problem, measured

SPEC-334 row 1.2 asks for three measurements before any client is built on them. None exists.
What was read before this SPEC was written:

- **FSRS-7 replay.** ADR-338 rebuilds each card's FSRS-7 memory state from its review history
  whenever the collection opens, and names the replay time as what would make it wrong. FSRS-7
  is in no release of the upstream scheduler crate; it lives on fsrs-rs's development line. Read
  there: the scheduler selects FSRS-7 when it is given 34 parameters or none, a review's interval
  is a fractional number of days (`FSRSReview { rating: u32, delta_t: f32 }`), and the first review
  of an item takes an interval of 0. The development line's package carries the same name and the
  same version string as the released crate the engine pins by checksum. So the two packages can
  coexist in one lockfile only by their source, never by their version, which refines ADR-338's
  "per-version disambiguation". The builder re-reads each of these facts at the pinned revision.
- **Undo's review-log row.** Read in the engine's source at the workspace's pinned engine commit
  (`c538de5`, whose native code at every line cited here is the earlier pin's, `57382da`, by
  ADR-058's note):
  - an answer writes its review-log row through the undoable path
    (`rslib/src/scheduler/answering/mod.rs:417`, `rslib/src/revlog/undo.rs:30-33`);
  - undoing that change removes the row (`rslib/src/revlog/undo.rs:14-17`);
  - a normal sync empties the undo queue when it starts (`rslib/src/sync/collection/normal.rs:85-86`,
    `rslib/src/undo/mod.rs:228`);
  - an undo with nothing queued is `UndoEmpty` (`rslib/src/undo/mod.rs:180-191`).

  The browser engine spike measured the deletion in Chromium and WebKit (SPEC-335 M7). Nothing
  has measured it natively at the workspace's pin, or across a sync.
- **The full-sync choice path.** Read at the same commit:
  - a schema mismatch answers `FullSyncRequired { upload_ok, download_ok }`. `upload_ok` is
    false only when the local collection is empty and the server's is not, and `download_ok` the
    reverse (`rslib/src/sync/collection/meta.rs:70-78`).
  - The upload (`rslib/src/sync/collection/upload.rs:44-69`):
    1. it first changes the local collection (`before_upload`);
    2. it closes the collection;
    3. it sends one `upload`, with no second `meta`.
  - The server's handler checks the file's integrity and replaces its collection
    (`rslib/src/sync/collection/upload.rs:73-104`).
  - The download (`rslib/src/sync/collection/download.rs:21-44`):
    1. it closes the local collection;
    2. it writes the server's file to a temporary path;
    3. it checks that file's integrity;
    4. it renames the file over the collection atomically.

  So, unmeasured:
  - the `meta` answer carries no count of what either side would lose;
  - nothing between that answer and the upload re-checks the server.

  A normal sync from another client in that window would be overwritten.

## 2. Requirements

R1. `deck-streak-fsrs7` (`crates/fsrs7`) is a new context crate. It depends on the upstream
    scheduler crate alone, taken from fsrs-rs at one full 40-hex revision (`4bc0a0979f95dd01cc653031b6f2a01429e4c32b`) through
    the workspace's dependency table. It never depends on the engine or on any DeckStreak crate,
    and its line in `docs/CONTEXT-MAP.md` reads `depends on: nothing` (ADR-338; ADR-353 D1).
R2. `deny.toml`'s `allow-git` names the fsrs-rs repository once, beside the engine's two sources,
    with a comment citing ADR-338, and `unknown-git` still refuses any other source. The lockfile
    holds exactly two packages of the upstream scheduler crate:
    - the released one, from the registry, which only the engine depends on;
    - the pinned one, from fsrs-rs at `4bc0a0979f95dd01cc653031b6f2a01429e4c32b`, which only `deck-streak-fsrs7` depends on.
R3. One test binary links both packages. Each answers from its own model: the released package
    holds the engine's FSRS-6 defaults and the pinned one FSRS-7's (ADR-338's coexistence;
    ADR-353 D5).
R4. A card's review history becomes one FSRS-7 item. The input is review-log rows shaped as the
    engine's table holds them: card id, an id that is the answer's time in milliseconds, the
    ease, and the kind.
    - Within a card, rows are taken in id order.
    - Manual and rescheduled entries, and entries with no ease, are dropped.
    - The first kept review takes an interval of 0, and each later one the difference of the
      two ids divided by 86,400,000, a fractional number of days.
R5. The measured history is synthetic and deterministic: no randomness, and no collection is read.
    For a cell of R review rows at a mean of m reviews per card:
    - card lengths cycle through 1 to 2m-1, so their mean is m;
    - the last card is cut so that the total is exactly R;
    - ratings and intervals follow fixed cycles.

    The sizes come from the shape of SPEC-334 row 1.2's collection-size measurement. The grid
    spans review-row counts, and a consumer reads the cost per review at the review-log row count
    that measurement records.
R6. A replay runs from the pinned revision's FSRS-7 defaults by two methods:
    - `single`, card by card;
    - `batch`, the upstream batch call, kept only if the pinned revision offers one.

    Every replay returns a checksum, the sum of the cards' stabilities.
R7. The example `replay_timing` runs the grid of R5 and R6:
    - review rows 10,000, 100,000 and 1,000,000;
    - a mean of 8 or of 32 reviews per card;
    - the method `single` or `batch`.

    It takes one untimed warm-up and then 5 timed runs per cell. It prints one line per cell, in
    the fixed form `fsrs7-replay target=<t> method=<m> reviews=<R> cards=<n> mean=<m> runs=5
    median_ms=<x> min_ms=<x> max_ms=<x> per_review_us=<x> checksum=<x>`. It runs:
    - natively on one thread;
    - as `wasm32-wasip1` under Node's V8, through `node:wasi` (ADR-353 D2).
R8. The example `replay_report` reads both targets' lines and writes the report. It refuses by
    name:
    - a missing or doubled cell;
    - a line not in the fixed form;
    - a cell whose two targets' checksums differ by more than one part in 10,000.

    It also records the web target's check (`cargo check --target wasm32-unknown-unknown`) as
    `pass`, or as `fail` with the failing crates named, and refuses a report without that line. The
    harness lives in the crate's `measure` module, which the examples only call (ADR-353 D6).
R9. A new workflow, `.github/workflows/fsrs7-measure.yml`, runs R7 and R8 on a pull request into
    `dev` that changes `crates/fsrs7/**`, `Cargo.lock` or the workflow itself. It holds every
    workflow's hardening rules:
    - a read-only token;
    - actions pinned by full commit;
    - no secret;
    - the pull request's concurrency group.

    It publishes the report as the run's summary and as the `fsrs7-measure-report` artifact.
R10. The undo probe runs against the engine at the workspace's pin, in
     `crates/ingest/tests/undo_and_full_sync.rs`, with the engine's own sync server started by the
     test (ADR-353 D3, D4). It measures:
     - (U1) after one answer the card has one review-log row; after the undo it has none, and the
       card's row is restored byte for byte;
     - (U2) after an answer and a normal sync, nothing can be undone: the undo is `UndoEmpty`, and
       both the local row and the server's row stay;
     - (U3) an answer undone before a normal sync never reaches the server: no pushed chunk
       carries a review-log row for the card, and the server holds none.
R11. The full-sync probe runs in the same binary, against the same kind of server. It measures:
     - (F1) the choice offered: both choices on a schema conflict with both sides non-empty; only
       the upload when the server is empty; only the download when the local collection is
       empty;
     - (F2) the upload choice sends exactly one request, `upload`, with no `meta` before it;
     - (F3) the download choice sends exactly one request, `download`, and the local collection
       is then the server's;
     - (F4) each choice loses exactly the other side's review-log rows, by id: the upload the
       server's, the download the local's;
     - (F5) another client's normal sync between the `meta` answer and the upload is
       overwritten: its review-log row is absent from the server afterwards;
     - (F6) the download loses a review the local client had already synced, once the server has
       been replaced without it. So what a side loses is a difference of ids, never its count of
       unsynced rows.
R12. `scripts/check.sh` does not change: the probes use tiny synthetic collections and run in the
     `test` stage, outside the engine set. Section 7 records every figure, with the run and the
     commit that produced it.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the lockfile holds exactly two packages of the scheduler crate: the engine's from the registry, and `deck-streak-fsrs7`'s from fsrs-rs at the manifest's 40-hex revision; neither consumer reaches the other's; a planted lock that crosses them is refused by name | `python3 -m unittest discover -s scripts/tests -p test_fsrs7_pin.py -k test_the_engine_and_the_fsrs7_crate_each_resolve_their_own_scheduler` |
| A2 | `allow-git` names exactly the engine's fork, `rust-url` and fsrs-rs, and `unknown-git` is `deny` | `python3 -m unittest discover -s scripts/tests -p test_engine_pin.py -k test_the_engine_is_patched_by_rev_to_a_commit_of_the_fork` |
| A3 | every allowed git source, fsrs-rs included, is one a crate of the graph comes from | `python3 -m unittest discover -s scripts/tests -p test_engine_pin.py -k test_every_advisory_exception_and_git_source_in_deny_toml_is_live` |
| A4 | one test binary links both packages, and each answers from its own model | `cargo test -p deck-streak-fsrs7 --test coexistence -- --exact both_schedulers_link_into_one_binary_and_answer_from_their_own_models` |
| A5 | a card's kept reviews become fractional-day intervals, from 0 | `cargo test -p deck-streak-fsrs7 --test convert -- --exact a_cards_reviews_become_fractional_day_intervals_from_zero` |
| A6 | manual, rescheduled and unrated entries are dropped | `cargo test -p deck-streak-fsrs7 --test convert -- --exact manual_rescheduled_and_unrated_entries_are_dropped` |
| A7 | a cell holds exactly its review rows, at its mean length | `cargo test -p deck-streak-fsrs7 --test history -- --exact a_cell_holds_exactly_its_review_rows_at_its_mean_length` |
| A8 | one Good review replays to the initial stability that the pinned model defines for Good | `cargo test -p deck-streak-fsrs7 --test replay -- --exact one_good_review_replays_to_the_models_initial_stability_for_good` |
| A9 | the two methods agree on every card of a cell | `cargo test -p deck-streak-fsrs7 --test replay -- --exact the_single_and_batch_methods_agree_on_every_card` |
| A10 | a cell's line carries its median, minimum and maximum in the fixed form | `cargo test -p deck-streak-fsrs7 --test report -- --exact a_cells_line_carries_its_median_min_and_max_in_the_fixed_form` |
| A11 | the grid is three sizes by two lengths by two methods, five runs each | `cargo test -p deck-streak-fsrs7 --test report -- --exact the_grid_is_three_sizes_by_two_lengths_by_two_methods_five_runs_each` |
| A12 | a report missing a cell, or holding one twice, is refused by name | `cargo test -p deck-streak-fsrs7 --test report -- --exact a_report_missing_or_doubling_a_cell_is_refused_by_name` |
| A13 | checksums that disagree across the targets are refused, and a report with no web-target line is refused | `cargo test -p deck-streak-fsrs7 --test report -- --exact disagreeing_checksums_and_a_missing_web_check_are_refused` |
| A14 | U1 | `cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact u1_undo_deletes_the_answers_review_log_row_and_restores_the_card` |
| A15 | U2 | `cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact u2_a_normal_sync_empties_undo_and_the_synced_row_stays` |
| A16 | U3 | `cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact u3_an_answer_undone_before_a_sync_never_reaches_the_server` |
| A17 | F1 | `cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact f1_the_choice_offered_follows_which_side_is_empty` |
| A18 | F2 | `cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact f2_the_upload_choice_sends_one_upload_and_no_second_meta` |
| A19 | F3 | `cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact f3_the_download_choice_sends_one_download_and_replaces_the_collection` |
| A20 | F4 | `cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact f4_each_choice_loses_exactly_the_other_sides_review_log_rows` |
| A21 | F5 | `cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact f5_a_normal_sync_between_the_meta_and_the_upload_is_overwritten` |
| A22 | F6 | `cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact f6_a_download_loses_a_synced_review_the_server_no_longer_holds` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_fsrs7_pin.py -k test_the_engine_and_the_fsrs7_crate_each_resolve_their_own_scheduler
A2: python3 -m unittest discover -s scripts/tests -p test_engine_pin.py -k test_the_engine_is_patched_by_rev_to_a_commit_of_the_fork
A3: python3 -m unittest discover -s scripts/tests -p test_engine_pin.py -k test_every_advisory_exception_and_git_source_in_deny_toml_is_live
A4: cargo test -p deck-streak-fsrs7 --test coexistence -- --exact both_schedulers_link_into_one_binary_and_answer_from_their_own_models
A5: cargo test -p deck-streak-fsrs7 --test convert -- --exact a_cards_reviews_become_fractional_day_intervals_from_zero
A6: cargo test -p deck-streak-fsrs7 --test convert -- --exact manual_rescheduled_and_unrated_entries_are_dropped
A7: cargo test -p deck-streak-fsrs7 --test history -- --exact a_cell_holds_exactly_its_review_rows_at_its_mean_length
A8: cargo test -p deck-streak-fsrs7 --test replay -- --exact one_good_review_replays_to_the_models_initial_stability_for_good
A9: cargo test -p deck-streak-fsrs7 --test replay -- --exact the_single_and_batch_methods_agree_on_every_card
A10: cargo test -p deck-streak-fsrs7 --test report -- --exact a_cells_line_carries_its_median_min_and_max_in_the_fixed_form
A11: cargo test -p deck-streak-fsrs7 --test report -- --exact the_grid_is_three_sizes_by_two_lengths_by_two_methods_five_runs_each
A12: cargo test -p deck-streak-fsrs7 --test report -- --exact a_report_missing_or_doubling_a_cell_is_refused_by_name
A13: cargo test -p deck-streak-fsrs7 --test report -- --exact disagreeing_checksums_and_a_missing_web_check_are_refused
A14: cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact u1_undo_deletes_the_answers_review_log_row_and_restores_the_card
A15: cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact u2_a_normal_sync_empties_undo_and_the_synced_row_stays
A16: cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact u3_an_answer_undone_before_a_sync_never_reaches_the_server
A17: cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact f1_the_choice_offered_follows_which_side_is_empty
A18: cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact f2_the_upload_choice_sends_one_upload_and_no_second_meta
A19: cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact f3_the_download_choice_sends_one_download_and_replaces_the_collection
A20: cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact f4_each_choice_loses_exactly_the_other_sides_review_log_rows
A21: cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact f5_a_normal_sync_between_the_meta_and_the_upload_is_overwritten
A22: cargo test -p deck-streak-ingest --test undo_and_full_sync -- --exact f6_a_download_loses_a_synced_review_the_server_no_longer_holds
```

Every test builds its own synthetic history or collection, and no real collection is read. A14
to A22 measure the engine at the workspace's pin, and no DeckStreak code makes them pass. So each
is recorded `not red` in `docs/red-first/SPEC-342.md`, with that reason. If one of them
measures the opposite of the hypothesis read in section 1, the hypothesis is wrong: the measured
fact is recorded, and the test is never bent to agree. A2 and A3 are SPEC-055's own tests:
- A2's expected set gains fsrs-rs, and A2 is red by assertion until `deny.toml` names it;
- A3 is red until a crate of the graph comes from the new entry.

Section 7 records R7 to R9, measured by the workflow's run.

## 4. File manifest

| file | context | change |
|---|---|---|
| `Cargo.toml` | the workspace | the pinned scheduler crate in `[workspace.dependencies]` by git and revision, and the released one by its exact version for A4's dev-dependency |
| `Cargo.lock` | the workspace | the pinned scheduler crate and the packages it adds |
| `deny.toml` | the workspace | `allow-git` gains fsrs-rs, with its comment (R2) |
| `crates/fsrs7/Cargo.toml` | `deck-streak-fsrs7` | added |
| `crates/fsrs7/src/lib.rs` | `deck-streak-fsrs7` | added |
| `crates/fsrs7/src/convert.rs` | `deck-streak-fsrs7` | added: R4 |
| `crates/fsrs7/src/replay.rs` | `deck-streak-fsrs7` | added: R6 |
| `crates/fsrs7/src/measure/mod.rs` | `deck-streak-fsrs7` | added: the measurement harness, R5, R7 and R8 |
| `crates/fsrs7/src/measure/history.rs` | `deck-streak-fsrs7` | added: R5 |
| `crates/fsrs7/src/measure/report.rs` | `deck-streak-fsrs7` | added: R7's line, R8's report |
| `crates/fsrs7/examples/replay_timing.rs` | `deck-streak-fsrs7` | added: R7 |
| `crates/fsrs7/examples/replay_report.rs` | `deck-streak-fsrs7` | added: R8 |
| `crates/fsrs7/tests/coexistence.rs` | `deck-streak-fsrs7` | added: A4 |
| `crates/fsrs7/tests/convert.rs` | `deck-streak-fsrs7` | added: A5, A6 |
| `crates/fsrs7/tests/history.rs` | `deck-streak-fsrs7` | added: A7 |
| `crates/fsrs7/tests/replay.rs` | `deck-streak-fsrs7` | added: A8, A9 |
| `crates/fsrs7/tests/report.rs` | `deck-streak-fsrs7` | added: A10 to A13 |
| `crates/ingest/tests/undo_and_full_sync.rs` | `deck-streak-ingest` (tests only) | added: A14 to A22 |
| `.github/workflows/fsrs7-measure.yml` | none (CI) | added: R9 |
| `scripts/tests/test_fsrs7_pin.py` | none (the gate) | added: A1 |
| `scripts/tests/test_engine_pin.py` | none (the gate) | A1's expected `allow-git` gains fsrs-rs, and so does its planted default |
| `scripts/mutation-rows.d/S34200-S34299.json` | none (the gate) | added |
| `docs/CONTEXT-MAP.md` | the map | the new context's line and paragraph |
| `docs/decisions/ADR-058-the-engine-pins-a-patched-fork-of-26-09-3-until-upstream-carries-the-fix.md` | the record | an appended amendment: `allow-git` names three sources, by ADR-338 and ADR-353 |
| `docs/specs/SPEC-055-the-engine-moves-to-26-09-3-on-a-patched-fork.md` | the record | R3 and A1 point to the amendment |
| `docs/schematics/fsrs-7-replay-undo-probe-and-full-sync-choice.md` | the record | added |
| `docs/red-first/SPEC-342.md` | the record | added |
| `docs/specs/SPEC-342-fsrs-7-replay-time-undo-and-the-full-sync-choice-are-measured.md` | the record | added |
| `docs/decisions/ADR-353-the-fsrs-7-crate-sits-beside-the-engine-and-is-timed-under-v8-and-the-probes-use-the-engines-own-server.md` | the record | added |
| `changelog.d/fsrs7-measure-342.md` | the record | added |

## 5. What this does NOT do

- It writes no stock field and does not convert FSRS-7 state into the 90 percent stability the
  stock field holds. The crate and the replay in the client are the stretch row's (#641).
- It does not decide between replay at open and a cache of FSRS-7 state. That is ADR-338's
  "What would make this wrong", decided by the delivery that reads section 7 (#641).
- It times no FSRS-7 parameter fitting, and no replay with fitted parameters: only the pinned
  revision's defaults (#641).
- It runs nothing in a browser or on a phone, and builds no Worker or bindings. Browser and
  device figures belong to the web engine (#626) and the iPhone and iPad delivery (#633).
- It does not make the crate build for the web target. R8 records whether it builds, and why
  not (#626).
- It adds no backend call to the FFI allow-list and no `OwnerGesture`. It changes no containment
  test: the engine core's allow-list is #623's.
- It builds no full-sync screen, loss display, on-device backup or snapshot check, and no TLA+
  model. The full-sync choice and its model, written first, are the web sync screens' (#631).
- It builds no undo button. The web study screens' undo is #630's, and the iPhone and
  iPad undo is #633's.
- It measures no media sync, and no full sync of a large collection. Only the protocol's order
  and its losses are measured, on tiny collections (#631).
- It changes no engine code and no fork commit (#233).

## 6. Risks

- **The pinned revision collides with the released crate.** The two packages share a name and a
  version string, so the lockfile tells them apart by source alone. If cargo refuses them, the
  build fails at resolution. A1 reads the lockfile, and A4 links both.
- **The pinned revision adds a crate under a licence `deny.toml` does not allow.** `cargo deny`
  fails, and A3's run names the crate. Admitting a licence is not this delivery's to decide: the
  build stops and asks.
- **A crate under the pinned revision does not build for `wasm32-wasip1`.** Example causes are a
  random source with no backend, or a parallel-iterator pool that cannot start a thread. The
  workflow's wasm step fails, and R8 refuses a report missing the wasm lines.
- **The development line moves after the pin.** The revision is a full commit, so nothing moves
  until a delivery re-pins. A1 refuses a branch name or a short revision.
- **The engine's behaviour differs from the source reading in section 1.** A14 to A22 measure
  it. A contradiction is a finding, recorded as measured, and it changes the consumer's input.
- **The probes slow the `test` stage.** Each starts the engine's server in a child process, as
  `recorder_control` already does, on collections of a few cards. The stage's duration in the CI
  run detects it. If a probe joins the slow class, it moves into the engine set by a change to
  `scripts/check.sh`, in a delivery of its own.
- **A shared runner's timing is noisy.** Each cell reports its median of five runs together with
  their minimum and maximum. Native runs on one thread, so that it compares with the
  single-threaded wasm run. A consumer reads the cost per review, never one run.

## 7. Measurements

Filled from the workflow's run and from CI's `rust` job at the head of the pull request. The
`<measured>` cells are written by the builder from that run, and never from a local run.

| id | figure | unit | command (job) | consumer |
|---|---|---|---|---|
| M1 | replay time natively, per cell: median, minimum and maximum of 5 runs, and the cost per review | ms; µs per review | `RAYON_NUM_THREADS=1 cargo run --release --locked -p deck-streak-fsrs7 --example replay_timing` (`fsrs7-measure`) | #641: replay at open, or a cache (ADR-338) |
| M2 | replay time as `wasm32-wasip1` under Node's V8, per cell, the same figures | ms; µs per review | `cargo build --release --locked --target wasm32-wasip1 -p deck-streak-fsrs7 --example replay_timing`, then run under `node:wasi` (`fsrs7-measure`) | #641, and #626's budget for the Worker |
| M3 | native and wasm checksums agree, per cell | pass or fail | `cargo run --release --locked -p deck-streak-fsrs7 --example replay_report` (`fsrs7-measure`) | #641: that the wasm figures measure the same work |
| M4 | the crate's web-target check | pass, or fail with the crates named | `cargo check --locked --target wasm32-unknown-unknown -p deck-streak-fsrs7` (`fsrs7-measure`) | #626 |
| M5 | the scheduler crate's packages in the lockfile, each with its source and its one consumer | count | A1 (`ci` `hygiene` job, `python` stage) | ADR-338's Confirmation |
| M6 | review-log rows for the card: after the answer, then after the undo; the card's row restored | rows; equal or not | A14 (`ci` `rust` job, `test` stage) | #623, #630, #633 |
| M7 | after a normal sync: what can be undone, the undo's answer, and the rows locally and on the server | rows; `UndoEmpty` or not | A15 (`ci` `rust` job) | ADR-337 entry 1, #623's exempt set, #630, #633 |
| M8 | an answer undone before a sync: its review-log rows pushed, and its rows on the server | rows | A16 (`ci` `rust` job) | #630, #633 |
| M9 | the choice offered in each of the three cases | `upload_ok` and `download_ok` | A17 (`ci` `rust` job) | #631 |
| M10 | the requests each choice sends, in order | method names | A18, A19 (`ci` `rust` job) | #631, and its TLA+ model's actions |
| M11 | what each choice loses | review-log ids | A20, A22 (`ci` `rust` job) | #631: what each side loses (SPEC-334 R8) |
| M12 | a normal sync in the window before an upload | rows lost | A21 (`ci` `rust` job) | #631: the model's property that no normal sync lies between the check and the write |

| id | measured |
|---|---|
| M1 | the `native` column of the report below: 0.1778 to 0.1937 µs per review row across the twelve cells (`fsrs7-measure` run 37237452262 at `9afb08b`) |
| M2 | the `wasm32-wasip1` column of the report below: 0.2122 to 0.2329 µs per review row across the twelve cells (the same run) |
| M3 | pass in all twelve cells: each cell's two checksums are equal to the printed three decimals, and the report renders only when every cell agrees to one part in 10,000 (the same run) |
| M4 | fail (getrandom): the crate does not build for `wasm32-unknown-unknown`, and the one crate that fails is getrandom, whose web backend is not configured (the same run) |
| M5 | 2: the engine's from the registry and `deck-streak-fsrs7`'s from fsrs-rs, each reached by its own consumer alone; A1 read `examined 2 scheduler package(s) in Cargo.lock` in `ci` run 37237452199's `hygiene` job, whose `python` stage read `ok` |
| M6 to M12 | PASS, each, in `ci` run 37237452199's `rust` job at `9afb08b`, whose `test` stage read `ok` (`1479 tests run: 1479 passed`): A14 `u1_undo_deletes_the_answers_review_log_row_and_restores_the_card`, A15 `u2_a_normal_sync_empties_undo_and_the_synced_row_stays`, A16 `u3_an_answer_undone_before_a_sync_never_reaches_the_server`, A17 `f1_the_choice_offered_follows_which_side_is_empty`, A18 `f2_the_upload_choice_sends_one_upload_and_no_second_meta`, A19 `f3_the_download_choice_sends_one_download_and_replaces_the_collection`, A20 `f4_each_choice_loses_exactly_the_other_sides_review_log_rows`, A21 `f5_a_normal_sync_between_the_meta_and_the_upload_is_overwritten`, A22 `f6_a_download_loses_a_synced_review_the_server_no_longer_holds` |

The report, as `fsrs7-measure` run 37237452262 wrote it at `9afb08b` (median of 5 runs in ms, the
minimum to the maximum, and the median's microseconds per review row):

| method | reviews | mean | cards | native | wasm32-wasip1 | checksum |
|---|---|---|---|---|---|---|
| single | 10000 | 8 | 1254 | 1.792 (1.782 to 1.835), 0.1792 µs | 2.164 (2.141 to 2.286), 0.2164 µs | 9177.331 |
| single | 10000 | 32 | 314 | 1.931 (1.926 to 1.935), 0.1931 µs | 2.310 (2.306 to 2.319), 0.2310 µs | 2137.139 |
| single | 100000 | 8 | 12504 | 17.988 (17.891 to 18.101), 0.1799 µs | 21.288 (21.229 to 21.396), 0.2129 µs | 91336.968 |
| single | 100000 | 32 | 3136 | 19.312 (19.279 to 19.323), 0.1931 µs | 23.224 (23.146 to 24.123), 0.2322 µs | 21305.548 |
| single | 1000000 | 8 | 125004 | 183.603 (182.947 to 184.800), 0.1836 µs | 212.226 (211.959 to 213.451), 0.2122 µs | 912933.338 |
| single | 1000000 | 32 | 31259 | 193.577 (193.246 to 200.275), 0.1936 µs | 231.378 (231.185 to 231.819), 0.2314 µs | 212283.373 |
| batch | 10000 | 8 | 1254 | 1.778 (1.769 to 1.786), 0.1778 µs | 2.125 (2.121 to 2.135), 0.2125 µs | 9177.331 |
| batch | 10000 | 32 | 314 | 1.933 (1.925 to 1.939), 0.1933 µs | 2.317 (2.307 to 2.321), 0.2317 µs | 2137.139 |
| batch | 100000 | 8 | 12504 | 17.999 (17.949 to 18.011), 0.1800 µs | 21.520 (21.416 to 22.106), 0.2152 µs | 91336.968 |
| batch | 100000 | 32 | 3136 | 19.295 (19.275 to 19.338), 0.1930 µs | 23.288 (23.227 to 23.352), 0.2329 µs | 21305.548 |
| batch | 1000000 | 8 | 125004 | 184.193 (183.811 to 187.123), 0.1842 µs | 212.429 (212.280 to 212.607), 0.2124 µs | 912933.338 |
| batch | 1000000 | 32 | 31259 | 193.657 (193.485 to 193.737), 0.1937 µs | 232.014 (231.775 to 232.527), 0.2320 µs | 212283.373 |

web target (`wasm32-unknown-unknown`) check: fail (getrandom)

#641 reads M1 and M2's cost per review against the review-log row count that SPEC-334 row 1.2's
collection-size measurement records. This delivery reads no figure from any real collection.

## 8. Amendments

### SPEC-387: the ingest crate reads the released scheduler's defaults

R2 gains no user. The ingest crate reads the released scheduler package's default parameters
through the engine it already depends on (SPEC-387 R11), so it names no scheduler package, and
`RELEASED_USERS` in `scripts/tests/test_fsrs7_pin.py` still names only the engine and the
FSRS-7 crate. An engine upgrade that moves the released package moves the defaults SPEC-387
proposes with it. `PINNED_USERS` is unchanged, and so is every other line of this SPEC.
