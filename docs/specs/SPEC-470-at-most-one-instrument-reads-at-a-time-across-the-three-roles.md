# SPEC-470: at most one instrument reads at a time across the three roles

- **Issue:** #476 (this delivery closes it). **Context(s):** `deck-streak-ingest` (the lock file
  and `Held::hold_to_end`); `deck-streak-coordination` (`run_held`, the weekly step and the
  on-demand run); `formal` (the model `OneInstrumentReader`).
- **Decided by:** ADR-487 (an instrument run holds the instrument lock until its read ends), under
  ADR-094 (the instruments run weekly after the sync or on demand, one at a time on the host).
- **Prerequisites:** SPEC-094 (the instruments' frame and its lock, R8), SPEC-022 (the collection
  lock's pattern, R7) and SPEC-020 (the offload). **Mutation band:** `S47000-S47099`.
- **Status:** accepted; delivered with its tests, its model, its rows and
  `docs/red-first/SPEC-470.md` by the pull request that closes #476.

## 1. The problem, measured

Read at dev `36c4783a` by `git show` and `grep -n`; ADR-487 D1 cites each fact by `path:line`.

- One lock file serves the three roles: `instruments.lock` under the one `StateDirectory=` the api,
  bot and job units share (`crates/coordination/src/instruments.rs:23`, `:253`;
  `deploy/systemd/deck-streak-api.service:19`, `deploy/systemd/deck-streak-bot.service:22`,
  `deploy/systemd/deck-streak-job@.service:20`).
- `run_held` takes it exclusively without waiting (`instruments.rs:337`), runs the instrument
  (`:342`), stores (`:350`) and releases (`:351-353`); `Held` also unlocks when dropped
  (`crates/ingest/src/lock.rs:111-118`).
- The read runs to its end on the offload's blocking thread (`crates/ingest/src/reader.rs:381-414`,
  `crates/kernel/src/offload.rs:64`), which nothing can stop.
- The api route awaits the run inline (`crates/api/src/insights_routes.rs:110`) under the request
  limit (`crates/api/src/router.rs:320-323`), whose timeout drops the handler's future; the api and
  bot runtimes end by `shutdown_timeout(EXIT_GRACE)` (`crates/daemon/src/main.rs:111-112`), which
  drops every async task.
- So a dropped caller or a stopping api or bot process unlocks while its read runs, and the job
  process's weekly step (`instruments.rs:284-312`) can take the lock and read beside it. ADR-094's
  "at most one instrument reads at a time on the host" and SPEC-094 R8's "so no two instruments read
  at once on the host" are false at `36c4783a`.
- The weekly step reads whether the report is due (`instruments.rs:293-298`) before it takes the
  lock (`:302`); that split allows at worst a redundant run after the other one, never two readers.
- No `@phx covers` line names `instruments.rs`, `lock.rs`, `insights_routes.rs` or `reader.rs` at
  `36c4783a`.

## 2. Requirements

R1. Every instrument run, weekly or on demand and in any role process, takes
    `<state directory>/instruments.lock` exclusively without waiting, as SPEC-094 R8 says and
    `run_held`'s acquire does today, byte for byte: a run that finds it held starts nothing; an
    on-demand request is answered that a run is in progress, and a weekly turn leaves the
    instrument due.
R2. A run that took the lock holds it until its read has ended and its report or its failure is
    stored, whether or not its caller still waits. A request the api's request limit ends, a client
    that goes away, or any other dropped caller leaves the run to end, store its report and release
    the lock.
R3. A run whose process stops while its read runs keeps the lock until that process exits: the lock
    is never released while the run's read may still run.
R4. A run whose work panics releases the lock, and its caller sees the panic.
R5. `Held::hold_to_end(work)` in `crates/ingest/src/lock.rs` runs `work` in its own task that owns
    the held lock. It returns the work's value with the release's result when the work ends;
    it unlocks and resumes the panic in its caller when the work panics; and when its runtime drops
    the task, it keeps the lock until the process exits. `run_held` hands it the lock and the whole
    run: the read, the failure envelope, the clock read and the store. No `Cargo.toml` or
    `Cargo.lock` changes.
R6. The TLA+ entry `formal/tla/OneInstrumentReader/` models `run_held`, the weekly `step` and the
    on-demand runs of the job, api and bot roles, with two api runs in one process. It holds the
    invariant `AtMostOneReader` (at most one instrument run holds the lock and reads at any time)
    and the liveness `ADueWeeklyRunRunsOnceTheLockIsFree` (a due weekly run eventually runs once the
    lock is free), each with its fairness stated, and one witness per way the property can break,
    which the checker must report (section 3a).
R7. The entry's `@phx covers` lines name `run_held`, `step` and `run_on_demand` in
    `crates/coordination/src/instruments.rs`, `run_instruments` in
    `crates/coordination/src/sync_cycle.rs`, `run` in `crates/api/src/insights_routes.rs`,
    `try_exclusive` and `hold_to_end` in `crates/ingest/src/lock.rs` and `with_copy` in
    `crates/ingest/src/reader.rs`, each stamped with its digest after the last edit.
R8. Rows S47001 to S47005 (section 7) are each killed by the test they name, and each killer lives
    in the crate its row mutates.

## 3. Acceptance criteria of SPEC-470

| id | criterion | decided by |
|---|---|---|
| A1 | a run whose caller went away keeps the lock until its read ends: the lock reads held, a second request is answered in progress, the weekly step defers the instrument, and at most one read runs | `a_run_whose_caller_went_away_keeps_the_lock_until_its_read_ends` |
| A2 | a run whose caller went away still stores its report and then frees the lock | `a_run_whose_caller_went_away_still_stores_its_report_and_frees_the_lock` |
| A3 | a run its runtime dropped while its read runs keeps the lock until the process ends | `a_run_its_runtime_dropped_keeps_the_lock_until_the_process_ends` |
| A4 | a held lock handed to a task runs its work to the end, and releases it, when its caller stops waiting | `a_held_lock_handed_to_a_task_runs_to_its_end_when_its_caller_stops_waiting` |
| A5 | a task whose work panics releases the lock, and its caller sees the panic | `a_task_that_panics_releases_the_lock_and_its_caller_sees_the_panic` |
| A6 | a task its runtime drops keeps the lock until the process ends | `a_task_its_runtime_drops_keeps_the_lock_until_the_process_ends` |

```acceptance
A1: cargo test -p deck-streak-coordination --test instruments_lock -- --exact a_run_whose_caller_went_away_keeps_the_lock_until_its_read_ends
A2: cargo test -p deck-streak-coordination --test instruments_lock -- --exact a_run_whose_caller_went_away_still_stores_its_report_and_frees_the_lock
A3: cargo test -p deck-streak-coordination --test instruments_lock -- --exact a_run_its_runtime_dropped_keeps_the_lock_until_the_process_ends
A4: cargo test -p deck-streak-ingest --test lock -- --exact a_held_lock_handed_to_a_task_runs_to_its_end_when_its_caller_stops_waiting
A5: cargo test -p deck-streak-ingest --test lock -- --exact a_task_that_panics_releases_the_lock_and_its_caller_sees_the_panic
A6: cargo test -p deck-streak-ingest --test lock -- --exact a_task_its_runtime_drops_keeps_the_lock_until_the_process_ends
```

A1 to A3 are red first: each compiles at the base and fails there by its assertion. A4 to A6 call
the added `Held::hold_to_end`, so they cannot compile at the base; they are not red, and their reds
are rows S47001 to S47004. Every test drives a gate it opens itself, with a bounded wait and an
open-on-drop guard, so no red can hang.

## 3a. What the formal check decides

The repository's formal check reads the entry `formal/tla/OneInstrumentReader/` on the default
entry budget; its verdict is read in CI by name. No test in the fence above runs it.

| id | criterion | decided by |
|---|---|---|
| F1 | across the job, api and bot processes, two api runs and one bot run beside the weekly run, at most one instrument run holds the lock and reads at any time | `AtMostOneReader` clean in `MCOneInstrumentReader.cfg` |
| F2 | a due weekly run eventually runs once the lock is free, under the fairness the property states | `ADueWeeklyRunRunsOnceTheLockIsFree` clean in `MCLiveness.cfg` |
| F3 | an acquire that is not exclusive lets two runs read at once | `witness/one-reader-needs-the-exclusive-acquire.cfg` reported |
| F4 | the code at `36c4783a`, whose lock goes with its caller and its process, lets two runs read at once | `witness/the-reader-keeps-the-lock-after-its-caller-goes.cfg` reported |
| F5 | a task alone, whose lock goes when its process stops, lets two runs read at once | `witness/the-reader-keeps-the-lock-until-its-process-exits.cfg` reported |
| F6 | a guard alone, which keeps a dropped run's lock in a live process, leaves a due weekly run unrun for ever | `witness/a-due-weekly-run-runs-once-the-lock-is-free.cfg` reported |
| F7 | the entry's covers name each item of R7 with its digest at the head | the formal check reports no stale cover |

The model's switches are `Exclusive`, `HoldToEnd` and `KeepOnStop`; the cure is all three TRUE,
and each witness changes only what its row says (ADR-487 D2).

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/ingest/src/lock.rs` | `deck-streak-ingest` | changed: `Held::hold_to_end`, its private guard `ToEnd`, and one module-doc sentence on the release at the process's exit |
| `crates/ingest/tests/lock.rs` | `deck-streak-ingest` | changed: A4, A5 and A6 |
| `crates/coordination/src/instruments.rs` | `deck-streak-coordination` | changed: `run_held` hands its lock and its run to `Held::hold_to_end`; its two callers pass the runner's `Arc` |
| `crates/coordination/tests/instruments_lock.rs` | `deck-streak-coordination` | added: A1, A2 and A3 |
| `formal/tla/OneInstrumentReader/OneInstrumentReader.tla` | `formal` | added: the model, its covers, its properties and its witnesses |
| `formal/tla/OneInstrumentReader/MCOneInstrumentReader.cfg` | `formal` | added: the invariant's configuration |
| `formal/tla/OneInstrumentReader/MCLiveness.cfg` | `formal` | added: the liveness configuration |
| `formal/tla/OneInstrumentReader/witness/one-reader-needs-the-exclusive-acquire.cfg` | `formal` | added: F3 |
| `formal/tla/OneInstrumentReader/witness/the-reader-keeps-the-lock-after-its-caller-goes.cfg` | `formal` | added: F4 |
| `formal/tla/OneInstrumentReader/witness/the-reader-keeps-the-lock-until-its-process-exits.cfg` | `formal` | added: F5 |
| `formal/tla/OneInstrumentReader/witness/a-due-weekly-run-runs-once-the-lock-is-free.cfg` | `formal` | added: F6 |
| `scripts/mutation-rows.d/S47000-S47099.json` | `mutation rows` | added: S47001 to S47005 |
| `docs/specs/SPEC-470-at-most-one-instrument-reads-at-a-time-across-the-three-roles.md` | `docs` | added: this SPEC |
| `docs/decisions/ADR-487-an-instrument-run-holds-the-instrument-lock-until-its-read-ends.md` | `docs` | added: its decision |
| `docs/red-first/SPEC-470.md` | `docs` | added: the red-first record |
| `docs/schematics/insights-instrument-frame.md` | `docs` | changed: one section appended, insert-only |
| `changelog.d/instrument-lock-model-470.md` | `docs` | added: the changelog fragment |

No Cargo manifest or lockfile, no formal configuration and no daemon wiring changes.

## 5. What this does NOT cover

- The instruments' own readings and reports: the property is who reads when, not what a read returns (#142).
- The weekly report's content (#130).
- The sync that schedules the weekly step (#142; ADR-094).
- The collection lock's same release on a dropped caller in `with_copy` (`crates/ingest/src/reader.rs:395`, `:411-413`), for every other api and bot ingest read and for an instrument read at a runtime stop (#957).

## 6. Risks

- **A red that hangs instead of failing.** Every test drives a gate it opens itself, waits a
  bounded time, and opens the gate from a guard's drop, so a failed assertion still ends the read.
- **A test that passes while examining nothing.** Each fence line selects one test by `--exact`,
  and its run reads `running 1 test`.
- **A model check that reads clean while examining less.** Each witness must be reported against its
  own property, the model's explored state count is quoted, and the model must reach each situation
  ADR-487 D2 lists, each checked as a trap that must be violated.
- **A lock kept by a process that lives on after its runtime stops.** The forgotten lock is held
  until the process exits; ADR-487 names it under what would make the decision wrong.
- **A report stored after its caller left.** By design (R2): the caller already had its answer, and
  the report is the instrument's latest.

## 7. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S47001-A-PANIC-RELEASES-THE-INSTRUMENT-LOCK` | `crates/ingest/src/lock.rs` | a task whose work panics unlocks while it unwinds (R4) | `lock::a_task_that_panics_releases_the_lock_and_its_caller_sees_the_panic` |
| `S47002-A-STOPPED-TASK-KEEPS-THE-LOCK` | `crates/ingest/src/lock.rs` | a task its runtime drops keeps the lock until the process exits (R3) | `lock::a_task_its_runtime_drops_keeps_the_lock_until_the_process_ends` |
| `S47003-THE-HELD-LOCK-RUNS-IN-ITS-OWN-TASK` | `crates/ingest/src/lock.rs` | the work runs in its own task, so a caller that stops waiting leaves it to end (R2) | `lock::a_held_lock_handed_to_a_task_runs_to_its_end_when_its_caller_stops_waiting` |
| `S47004-THE-CALLER-SEES-THE-WORKS-PANIC` | `crates/ingest/src/lock.rs` | the work's panic resumes in its caller (R4) | `lock::a_task_that_panics_releases_the_lock_and_its_caller_sees_the_panic` |
| `S47005-AN-INSTRUMENT-RUN-HOLDS-THE-LOCK-TO-ITS-END` | `crates/coordination/src/instruments.rs` | `run_held` hands its lock to a task that runs to the end (R2) | `instruments_lock::a_run_whose_caller_went_away_keeps_the_lock_until_its_read_ends` |

Rows S09407, S09408 and S09410 (SPEC-094) stay on `instruments.rs` unchanged; S09410's find, the
acquire, stays byte for byte.
