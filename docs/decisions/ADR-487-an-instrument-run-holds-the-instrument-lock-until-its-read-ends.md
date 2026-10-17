---
status: "accepted"
decision-makers: "the DeckStreak architect"
---

# An instrument run holds the instrument lock until its read ends

## Context and Problem Statement

ADR-094 decides that the instruments run weekly after the sync or on demand, one at a time on the
host, and SPEC-094 R8 holds it with one exclusive file lock taken without waiting,
`<state directory>/instruments.lock`. ADR-094's consequence reads "at most one instrument reads at
a time on the host" (ADR-094 line 58), and SPEC-094 R8 says "so no two instruments read at once on
the host". Issue #476 asks for a model of that lock across the job, api and bot roles: at most one
instrument run holds the lock and reads at any time, a run that finds it held starts nothing, and a
due weekly run eventually runs once the lock is free.

Read at dev `36c4783a` (every citation below is `path:line` at that commit), the claim is false.
The lock is released when the future that holds it is dropped, while the read it guards runs on
to its end on a blocking thread that nothing can stop. A request the api's request limit ends, a
client that goes away, or an api or bot process that stops therefore frees the lock while its read
still runs, and the job role's weekly step, in its own process, can then take the lock and read
beside it. This record decides how the lock is held, models it, and cures it.

## Decision Drivers

- ADR-094's claim holds across the three role processes, not only inside one.
- The acquire stays as it is: exclusive, without waiting, a held lock starting nothing (row
  S09410 keeps its find byte for byte).
- No dependency change: `crates/coordination/Cargo.toml` and `Cargo.lock` stay untouched.
- Each half of the cure is shown to be needed by a model witness of its own.
- The tests decide by a gate, never by a timing race, and no red can hang.

## Decision

### D1. The mechanism, as measured at `36c4783a`

- **One lock file for the three roles.** `LOCK_FILE = "instruments.lock"`
  (`crates/coordination/src/instruments.rs:23`); the lock is
  `CollectionLock::new(state_directory.join(LOCK_FILE))` (`instruments.rs:253`), and the state
  directory is `$STATE_DIRECTORY` (`crates/daemon/src/wiring.rs:87-88`, `:115`;
  `Instruments::new(.., state.path(), ..)` at `wiring.rs:1833-1835`). The api, bot and job units
  name the same `StateDirectory=` (`deploy/systemd/deck-streak-api.service:19`,
  `deploy/systemd/deck-streak-bot.service:22`, `deploy/systemd/deck-streak-job@.service:20`).
- **Three kinds of process.** `deckstreakd api` (`deck-streak-api.service:16`) and `bot`
  (`deck-streak-bot.service:19`) run long; `job %i` (`deck-streak-job@.service:17`) is
  `Type=oneshot` (`:16`), started by `deck-streak-job@sync.timer` and `deck-streak-job@sync.path`.
  Two runs can share one process: two api requests run in the api process at once.
- **The acquire.** `run_held` (`instruments.rs:332-368`) takes the lock by
  `self.lock.try_exclusive().await` (`:337`): `try_exclusive` (`crates/ingest/src/lock.rs:53-70`)
  opens the file read, write and create without truncating, on `spawn_blocking`, and calls
  `File::try_lock`; `WouldBlock` is `Ok(None)`, which `run_held` answers `InProgress` (`:339`),
  and any other error is `Lock` (`:340`). The lock is per open file description, so two runs in one
  process exclude each other too (SPEC-094 A15, `crates/coordination/tests/instruments_step.rs:180-217`,
  holds it through a second handle).
- **The read.** `runner.run(today)` (`instruments.rs:342`, a failure becoming the failure envelope
  at `:344-347`) reaches `Frame::run` (`instruments.rs:179-190`), then `StructureSource::read`
  (`wiring.rs:1800-1803`), `read_structure` (`crates/ingest/src/structure.rs:127`) and `with_copy`
  (`crates/ingest/src/reader.rs:381-414`), whose read "runs to its end on the offload's blocking
  thread" (its doc, `reader.rs:381-384`; `offload.run(..)` at `:398-409`), through `Offload::run`
  (`crates/kernel/src/offload.rs:53-80`, `spawn_blocking` at `:64`). The offload's worker bound
  (`offload.rs:21`) is per process, so it serialises nothing across the roles.
- **The release.** After the store (`instruments.rs:350`), `held.release()` unlocks with a warning
  on failure (`:351-353`), before the store's error returns (`:354`). `Held` also unlocks when it
  is dropped (`lock.rs:111-118`; `Held::release` at `lock.rs:107-109`). So an early return, an
  error and a panic all release, and so does a drop of the future that owns `held`.
- **The weekly step.** `step` (`instruments.rs:284-312`) reads whether the report is due
  (`:293-298`) before `run_held` takes the lock (`:302`); a refusal leaves the instrument due and
  stores nothing (`:305-308`). `sync_cycle.rs::run_instruments` calls it
  (`crates/coordination/src/sync_cycle.rs:376-381`); only the job role's `sync` builds the
  instruments (`crates/daemon/src/role_job.rs:95-96`), from its scheduled cycle and its
  owner-request path (`role_job.rs:113-116`, `:176-188`, `:258`).
- **The api's on-demand run.** `POST /api/insights/{id}/run` (`crates/api/src/insights_routes.rs:31`,
  `:54`, `:98-121`) awaits the run inline (`:110`) and answers 409 `run_in_progress` on a held lock
  (`:113`), through `LateInstruments::run` (`wiring.rs:1913-1915`), `Instruments::run`
  (`instruments.rs:425-427`) and `run_on_demand` (`:320-329`). Every route sits under
  `TimeoutLayer::with_status_code(408, REQUEST_TIMEOUT)` (`crates/api/src/router.rs:320-323`, over
  the insights routes merged at `router.rs:279-280`; `REQUEST_TIMEOUT` at `router.rs:79`), whose
  timeout drops the handler's future, and under `CatchPanicLayer` (`router.rs:319`).
- **The bot's on-demand run.** The bot builds the instruments (`crates/daemon/src/role_bot.rs:117`)
  and hands them to its commands (`role_bot.rs:134`; `crates/bot/src/commands.rs:468-469`,
  `:523-531`); no bot command calls `run` at `36c4783a` (only `crates/bot/tests/commands.rs:713`
  and `:717` call `instruments()`). SPEC-094 R8 names a later command, so the model keeps a bot run.
- **The stops.** `ShutdownSignal::install` runs in `crates/daemon/src/role_api.rs:191` (its select
  at `:249-251`), `role_bot.rs:94` (`:146`) and `role_mcp.rs:80`; the job role installs none, so a
  stop ends its process at once. `main.rs:111-112` ends the runtime with
  `runtime.shutdown_timeout(EXIT_GRACE)` (`main.rs:37`), which drops every async task and leaves a
  started blocking task running. The api drains, then closes the database (`role_api.rs:247-265`).
- **The finding.** A dropped caller, or a stopping api or bot runtime, drops `held` and unlocks
  while its read runs on its blocking thread; the job process's weekly step can then take the lock
  and read beside it. ADR-094's "at most one instrument reads at a time on the host" is false at
  `36c4783a`. The due read before the lock is safe: its worst case is a redundant run after the
  other one, never two readers at once.

### D2. The model

A new TLA+ entry, `formal/tla/OneInstrumentReader/`, on the repository's default entry budget:
`OneInstrumentReader.tla`, `MCOneInstrumentReader.cfg` (`NApi = 2`, `NBot = 1`), `MCLiveness.cfg`
(`NApi = 1`, `NBot = 1`) and four witnesses.

- **Runs and processes.** Run ids are tuples of one type with integer bounds:
  `W == <<"job", 0>>`, `ApiRuns == {<<"api", i>> : i \in 1..NApi}`,
  `BotRuns == {<<"bot", i>> : i \in 1..NBot}`, `OnDemand == ApiRuns \cup BotRuns`,
  `Runs == {W} \cup OnDemand`, `Procs == {"api", "bot"}`, `Proc(r) == r[1]`,
  `Live(r) == r = W \/ up[Proc(r)] = "up"`. Two api runs share the api process.
- **Constants.** `NApi`, `NBot`, and three switches: `Exclusive` (the acquire is exclusive),
  `HoldToEnd` (a run keeps the lock when its caller goes) and `KeepOnStop` (a run keeps the lock
  when its process stops). The cure is all three TRUE; the code at `36c4783a` is
  `HoldToEnd = FALSE` and `KeepOnStop = FALSE`.
- **Variables.** `pc` per run (`idle`, `check`, `try`, `held`, `reading`, `storing`, `done`);
  `reading`, the runs whose blocking read runs; `holders`, the runs whose handle holds the lock;
  `answer` per on-demand run (`none`, `waiting`, `report`, `inProgress`, `dropped`); `due`, the
  weekly report's due state (`Init` TRUE); `up` per process (`up`, `stopping`, `exited`).
- **Actions, in the code's order.** `Week` (a week passes: `~due` to `due`); `Sync` (the job starts
  its step); `Check` (the due read, before the lock); `Request(r)` (an on-demand request, its
  answer `waiting`); `TryLock(r)`, which takes the lock when
  `Acquirable == holders = {} \/ ~Exclusive`, and otherwise sends the weekly run back to `idle`
  (it stays due) or ends an on-demand run as `done` with the answer `inProgress`; `StartRead(r)`;
  `ReadEnds(r)`, always enabled for a run in `reading`, because a blocking read ends on its own
  (the run moves to `storing` only if it was still `reading`); `Store(r)`, which clears `due`,
  releases, and answers a waiting caller `report`; `Cancel(r)`, the caller of an on-demand run going
  away while its process is up (the answer becomes `dropped`; the run ends unless it holds to its
  end, `pc' = IF pc[r] # "try" /\ HoldToEnd THEN pc ELSE [pc EXCEPT ![r] = "done"]`, and the lock
  goes with it unless `pc[r] = "try" \/ HoldToEnd \/ KeepOnStop`); `Stop(p)` (each busy run of `p`
  ends, a waiting answer becomes `dropped`, and the lock goes unless `KeepOnStop`); `Exit(p)` (the
  process ends: its reads and its holders go together). The weekly run always has a move, so the
  model never deadlocks.
- **The invariant**, in the issue's words: at most one instrument run holds the lock and reads at
  any time, `AtMostOneReader == \A r1, r2 \in reading : r1 = r2`.
- **The liveness**: a due weekly run eventually runs once the lock is free,
  `ADueWeeklyRunRunsOnceTheLockIsFree == (WF_vars(Sync) /\ WF_vars(Check) /\ RunsFair /\ ExitFair) => (due ~> ~due)`,
  with `RunsFair == \A r \in Runs : WF_vars(TryLock(r)) /\ WF_vars(StartRead(r)) /\ WF_vars(ReadEnds(r)) /\ WF_vars(Store(r))`
  and `ExitFair == \A p \in Procs : WF_vars(Exit(p))`. The fairness sits inside the property; a
  stop and a cancel are never fair, since nothing obliges either to happen.
- **The witnesses**, each at the model's constants with one switch changed, each a property the
  checker must report:
  `witness/one-reader-needs-the-exclusive-acquire.cfg` (`Exclusive = FALSE`; the issue's own;
  breaks `AtMostOneReader`), `witness/the-reader-keeps-the-lock-after-its-caller-goes.cfg`
  (`HoldToEnd = FALSE`, `KeepOnStop = FALSE`, the code at `36c4783a`; breaks `AtMostOneReader`),
  `witness/the-reader-keeps-the-lock-until-its-process-exits.cfg` (`HoldToEnd = TRUE`,
  `KeepOnStop = FALSE`, a task alone; breaks `AtMostOneReader`) and
  `witness/a-due-weekly-run-runs-once-the-lock-is-free.cfg` (`NApi = 1`, `NBot = 1`,
  `HoldToEnd = FALSE`, `KeepOnStop = TRUE`, a guard alone, which keeps a dropped run's lock in a
  live process for ever; breaks the liveness).
- **The covers.** `run_held`, `step` and `run_on_demand` in `crates/coordination/src/instruments.rs`;
  `run_instruments` in `crates/coordination/src/sync_cycle.rs`; `run` in
  `crates/api/src/insights_routes.rs`; `try_exclusive` and `hold_to_end` in
  `crates/ingest/src/lock.rs`; `with_copy` in `crates/ingest/src/reader.rs`. The entry cites #476
  and #142, and both properties report.
- **The situations the model must reach**, each checked as a trap that must be violated: a weekly
  run that reads after an on-demand run stored (the redundant sequential run); an on-demand run
  answered `inProgress`; a dropped caller whose read runs and holds the lock; a stopping process
  whose read runs and holds the lock; a weekly run trying a held lock while due. `AtMostOneReader`
  also holds with one more actor (`NApi = 3`, `NBot = 2`).

### D3. The code fails the invariant; the cure is two files

Today's code fails `AtMostOneReader` (D1's finding, witness two) and meets the liveness. The
smallest cure changes two files and no manifest:

- **`crates/ingest/src/lock.rs` gains `Held::hold_to_end`.**
  `pub async fn hold_to_end<T, F>(self, work: F) -> io::Result<(T, io::Result<()>)>`, where
  `F: Future<Output = T> + Send + 'static` and `T: Send + 'static`, runs `work` in its own task
  that owns the held lock: `let task = async move { let mut kept = ToEnd(Some(self)); let value = work.await; (value, kept.release()) };`,
  then `match tokio::spawn(task).await { Ok(ended) => Ok(ended), Err(error) => match error.try_into_panic() { Ok(panic) => std::panic::resume_unwind(panic), Err(error) => Err(io::Error::other(error)), }, }`.
  A private `struct ToEnd(Option<Held>)` has `fn release(&mut self) -> io::Result<()> { self.0.take().map_or(Ok(()), Held::release) }`
  and a `Drop` that, when it still holds the lock, drops it while a panic unwinds (the unlock) and
  otherwise forgets it (`std::mem::forget`), so the descriptor stays open and the lock stays held
  until the process exits. The module doc gains one sentence on that release at the process's exit.
  A dropped caller only detaches the task, which runs its work to the end and releases; a task its
  runtime drops at a stop keeps the lock until the process ends, when the operating system closes
  the descriptor and the blocking read ends with the process.
- **`crates/coordination/src/instruments.rs`: `run_held` hands its lock and its run to it.**
  `run_held(&self, runner: &Arc<dyn InstrumentRunner>, today: i64)` keeps its acquire
  (`instruments.rs:337-341`) byte for byte, then clones the runner, the store and the clock into
  `let run = async move { .. }`, which runs the read, builds the failure envelope on an error, reads
  the clock and stores, returning `(envelope, failed, at, stored)`; then
  `let ended = held.hold_to_end(run).await;` and
  `let ((envelope, failed, at, stored), released) = ended.map_err(OnDemandRefusal::Lock)?;`, a
  warning on `released`'s error as today, and the rest unchanged. Its callers (`:302`, `:326`) pass
  the runner's `Arc`, not `.as_ref()`.

### D4. One delivery

The model, its witnesses, the cure, its tests and its rows land in one pull request, which closes
#476.

### D5. Tests and rows

- **Red first**, in a new `crates/coordination/tests/instruments_lock.rs`, each compiling at the
  base and failing there by its assertion: A1, a run whose caller went away keeps the lock until its
  read ends; A2, that run still stores its report and frees the lock; A3, a run its runtime dropped
  keeps the lock until the process ends. Each reads a gated blocking read (a gate the test opens,
  with a bounded wait and an open-on-drop guard, so no red hangs), never a timing race.
- **Not red**, in `crates/ingest/tests/lock.rs`, because they call the added `Held::hold_to_end`
  and cannot compile at the base: A4, a held lock handed to a task runs to its end when its caller
  stops waiting; A5, a task that panics releases the lock and its caller sees the panic; A6, a task
  its runtime drops keeps the lock until the process ends. Their reds are rows S47001 to S47004.
- **Rows**, in `scripts/mutation-rows.d/S47000-S47099.json`: S47001 to S47004 on
  `crates/ingest/src/lock.rs`, each killed by an ingest test, and S47005 on
  `crates/coordination/src/instruments.rs`, killed by A1. Each killer lives in the crate it mutates.
- **The model** is judged by the repository's formal check: each witness reported, both properties
  clean, the covers stamped after the last edit.

### D6. Not in this decision

The instruments' own readings and reports (#142); the weekly report's content (#130); the sync that
schedules the step (#142, ADR-094); and the collection lock's same release on a dropped caller in
`with_copy` (`reader.rs:395`, `:411-413`), for every other api and bot ingest read and for an
instrument read at a runtime stop (#957).

## D1 was chosen against

- Treating a run as one atomic take, read and release: the read outlives its caller on a blocking thread, so an atomic step hides the race this record finds.
- One run per role: two api requests run in the api process at once, so a role is not a run.

## D2 was chosen against

- The read inside the lock step: folding the read into the acquire hides the window between a release and the read's end.
- A horizon invariant for the liveness: the due state cycles week by week, so only a leads-to states that a due run eventually runs.
- One witness: one witness would not show that each half of the cure is needed; two witnesses break the invariant one half at a time, and a third breaks the liveness with the guard alone.
- Model values for the run ids: the entry's bounds take integer constants, so the ids are tuples bounded by `NApi` and `NBot`.

## D3 was chosen against

- One blocking closure that owns the guard: the closure is ingest's `with_copy`, two ports below `run_held` behind `InstrumentRunner` and `ReadSource`, and a closure that held an offload permit while its read waited for another would deadlock at the offload's default bound.
- Spawning the task in coordination: coordination has no tokio dependency outside its tests, and its `Cargo.toml` and `Cargo.lock` would change.
- Shielding only the api route: the bot's commands and every stop also reach `run_held`, so a route-only cure leaves the race open.
- The task alone: a stop drops the task and unlocks while the read runs (witness three).
- The guard alone: a dropped caller in a live process would keep the lock for ever, and the weekly run would never run (the liveness witness).
- A longer request limit: any limit can be passed, and a stop still drops the run.
- A waiting lock: ADR-094 rejected it, since a held lock starts nothing and a weekly turn stays due.

## D4 was chosen against

- The model first and the cure after: the model at the base's code would report a violation on dev between the two parts, its covers would be stamped twice, and the cure is two files no open change touches.

## D5 was chosen against

- Cancelling by a time limit in the tests: a limit races the read, while a gate the test opens decides the same order every run.
- Killers in coordination for the rows on `lock.rs`: a row's killer runs in the crate it mutates, so the rows on `lock.rs` are killed by ingest's tests.

## D6 was chosen against

- Curing the collection lock in `with_copy` here: it guards every other api and bot ingest read, not only the instruments, so its cure is its own decision (#957).
- Modelling the instruments' readings and reports: the property is about who reads when, not what a read returns (#142).

## Consequences

- Good, because ADR-094's "at most one instrument reads at a time on the host" holds across the three role processes again.
- Good, because the acquire is unchanged: a held lock still starts nothing, an on-demand request is still answered that a run is in progress, and a weekly turn still stays due.
- Good, because no manifest, lockfile or formal configuration changes.
- Bad, because a run whose caller went away still runs to its end and stores its report, though nobody awaits it; the caller already had its answer.
- Bad, because a stopping api or bot process keeps the lock until it exits, so the weekly step defers its instrument to the next sync's step for that short window.
- Neutral, because the job process installs no stop handler and ends at once on a stop, so its lock and its read end together.

## Confirmation

SPEC-470 A1 to A6 decide the cure, and its formal section checks the model: each witness reported,
both properties clean, the covers stamped. Rows S47001 to S47005 are proved by hand, each mutant
killed by its named test, and are read by name in CI.

## What would make this wrong

- A process that lives on after its runtime stops: a forgotten lock would then stay held until that process exits, and every later run would be refused.
- A caller that aborts the spawned task, not only drops its handle: an abort drops the task's guard outside a panic and keeps the lock until the process exits.
- A role that gives each process its own state directory: the three roles would then lock three files, and the model's one lock would be wrong.
- A blocking read that can be stopped: the read would then end with its caller, and holding the lock to its end would be needless.

## More Information

ADR-094 and SPEC-094 R8 (the instruments' frame and its lock), SPEC-022 R7 (the collection lock's
pattern), SPEC-020 (the offload), SPEC-470 (this record's requirements and criteria), #476, #142,
#130 and #957.
