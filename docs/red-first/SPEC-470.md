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
