# Red-first record: SPEC-470

SPEC-470 (R1 to R8, A1 to A6). Its acceptance fence, one test per criterion:

```text
A1: cargo test -p deck-streak-coordination --test instruments_lock -- --exact a_run_whose_caller_went_away_keeps_the_lock_until_its_read_ends
A2: cargo test -p deck-streak-coordination --test instruments_lock -- --exact a_run_whose_caller_went_away_still_stores_its_report_and_frees_the_lock
A3: cargo test -p deck-streak-coordination --test instruments_lock -- --exact a_run_its_runtime_dropped_keeps_the_lock_until_the_process_ends
A4: cargo test -p deck-streak-ingest --test lock -- --exact a_held_lock_handed_to_a_task_runs_to_its_end_when_its_caller_stops_waiting
A5: cargo test -p deck-streak-ingest --test lock -- --exact a_task_that_panics_releases_the_lock_and_its_caller_sees_the_panic
A6: cargo test -p deck-streak-ingest --test lock -- --exact a_task_its_runtime_drops_keeps_the_lock_until_the_process_ends
```

```red-first
A1: red at 1d515b8da27b8bcac5744b52efd6a4b69ca33e84: left: (false, true, ["alpha"], 1) right: (true, true, ["alpha"], 1)
A1: green at 3063d2b983e656e80e304213b499b5ecef66fb78
A2: red at 1d515b8da27b8bcac5744b52efd6a4b69ca33e84: left: (None, true) right: (Some("alpha"), true)
A2: green at 3063d2b983e656e80e304213b499b5ecef66fb78
A3: red at 1d515b8da27b8bcac5744b52efd6a4b69ca33e84: the lock read free after the runtime stopped while the read ran: Ok(())
A3: green at 3063d2b983e656e80e304213b499b5ecef66fb78
A4: not red: calls the added Held::hold_to_end, which the base lacks, so it cannot compile there; its red is hand row S47003, killed by A4
A5: not red: calls the added Held::hold_to_end, which the base lacks, so it cannot compile there; its reds are hand rows S47001 and S47004, killed by A5
A6: not red: calls the added Held::hold_to_end, which the base lacks, so it cannot compile there; its red is hand row S47002, killed by A6
```

A1 to A3 were committed alone in `crates/coordination/tests/instruments_lock.rs`, at
1d515b8da27b8bcac5744b52efd6a4b69ca33e84, over the base's production code, and each was run there
by its fence line: each run read `running 1 test` and failed by its own assertion. A1's tuple
differs from the expected one only in its first element: the dropped caller unlocked while the
gated read still ran, so the test's own take found the lock free (and then held it, which is why
the second request and the weekly step were refused). A2 found no stored report after the caller
went, because the dropped run never stored. A3 found the lock free once the runtime stopped while
the read ran. Each test reads a blocking read held at a gate the test opens itself, with a bounded
wait and a guard that opens the gate when dropped, so a failed assertion still ends the read. The
coordination suite at that commit (`--lib --test instruments_lock --test instruments_step --test
instruments_cycle --test instrument_reports`) read 12 passed and 3 failed, the three being A1, A2
and A3 and no other.

The cure, 3063d2b983e656e80e304213b499b5ecef66fb78, changes `crates/ingest/src/lock.rs` and
`crates/coordination/src/instruments.rs`, and each of the six fence lines then read
`running 1 test` and `1 passed`. That commit changes no line of
`crates/coordination/tests/instruments_lock.rs`. It edits one existing test file,
`crates/ingest/tests/lock.rs`, only by adding: A4, A5 and A6, the imports they need
(`std::fs::{OpenOptions, TryLockError}` and `tokio::sync::mpsc` beside `Notify`) and one module-doc
line; no existing test or assertion there changes.

A4, A5 and A6 call `Held::hold_to_end`, which the base does not have, so they cannot compile at the
base and are not red first. Their reds are the hand-proved rows of
`scripts/mutation-rows.d/S47000-S47099.json`, each a mutant of `crates/ingest/src/lock.rs` that the
named test kills.

## The rows' hand proofs

Each row of `scripts/mutation-rows.d/S47000-S47099.json` was proved by hand on the committed cure:
its target's sha256 was recorded, its one mutant installed by script (each `find` occurs once in
its target), the file touched and rebuilt, and its killer run by its exact name, each run reading
`running 1 test` and failing by the assertion below; the saved bytes were then written back, the
sha256 compared equal, the file touched and rebuilt before the next measurement.
`crates/ingest/src/lock.rs` read sha256
`a9ad604e69227a06d4e9979bc8c1d6ed0f24176a87b251dfd61d5535ec7bf0d0` before and after each of its four
proofs, and `crates/coordination/src/instruments.rs` read
`d92399566ec990ea8b09b309d41ce1f07d150197f99a78032e5adfc195496fd2` before and after its one.

- `S47001-A-PANIC-RELEASES-THE-INSTRUMENT-LOCK` (`if false {`): A5 failed with
  `left: (true, false) right: (true, true)`: the caller saw the panic, and the lock stayed held.
- `S47002-A-STOPPED-TASK-KEEPS-THE-LOCK` (`drop(held);`): A6 failed with
  `the lock read free after the runtime dropped the task: Ok(())`.
- `S47003-THE-HELD-LOCK-RUNS-IN-ITS-OWN-TASK` (the work awaited in the caller): A4 failed with
  `left: (true, true, None, false) right: (true, true, Some(7), true)`: the dropped caller took the
  work with it, so it never ended, and the lock was never freed.
- `S47004-THE-CALLER-SEES-THE-WORKS-PANIC` (the panic answered as an error): A5 failed with
  `left: (false, true) right: (true, true)`.
- `S47005-AN-INSTRUMENT-RUN-HOLDS-THE-LOCK-TO-ITS-END` (the run awaited in the caller, then
  released): A1 failed with `left: (false, true, ["alpha"], 1) right: (true, true, ["alpha"], 1)`,
  its red at the base, and A2 failed under it too with `left: (None, true) right: (Some("alpha"), true)`.

After the last restore, A1 was rebuilt and read `1 passed`.

`cargo mutants --no-shuffle --list --json --in-diff` over this branch's diff lists 7 mutants and
builds nothing: `Instruments::step`, `Instruments::run_on_demand` and `Instruments::run_held` (two)
in `crates/coordination/src/instruments.rs`, and `Held::hold_to_end`, `ToEnd::release` and
`<impl Drop for ToEnd>::drop` in `crates/ingest/src/lock.rs`. None was run here; the pull
request's mutation stage judges them.
