------------------------- MODULE OneInstrumentReader -------------------------
(***************************************************************************)
(* At most one instrument reads at a time across the job, api and bot      *)
(* processes (#476). The three roles share one lock file,                  *)
(* instruments.lock, under their one state directory. A run takes it       *)
(* exclusively without waiting, reads on a blocking thread that nothing    *)
(* can stop, stores its report and releases. A caller may go away and a    *)
(* process may stop between any two of those steps.                        *)
(*                                                                         *)
(* Modelled claims (item names, never line numbers):                       *)
(*   instruments.rs::step          - the weekly step reads whether the     *)
(*                                   report is due before it takes the     *)
(*                                   lock; a held lock leaves it due.      *)
(*   instruments.rs::run_held      - the acquire is exclusive and never    *)
(*                                   waits; the run, its read, its store   *)
(*                                   and its release follow it.            *)
(*   instruments.rs::run_on_demand - an on-demand run that finds the lock  *)
(*                                   held is answered that a run is in     *)
(*                                   progress and starts nothing.          *)
(*   lock.rs::try_exclusive        - the take, without waiting.            *)
(*   lock.rs::hold_to_end          - the run's own task holds the lock to  *)
(*                                   the end of its work; a task its       *)
(*                                   runtime drops keeps it until the      *)
(*                                   process exits.                        *)
(*   reader.rs::with_copy          - the read runs to its end on a         *)
(*                                   blocking thread.                      *)
(*   insights_routes.rs::run       - the api's on-demand run, awaited      *)
(*                                   inline by the route.                  *)
(*   sync_cycle.rs::run_instruments - the job's sync calls the step.       *)
(*   the request limit in router.rs - drops a waiting handler's future.    *)
(*   the runtime's shutdown in main.rs - drops every async task.           *)
(*                                                                         *)
(* Exclusive, HoldToEnd and KeepOnStop are the cure's three parts: an      *)
(* exclusive acquire, a run that keeps the lock when its caller goes, and  *)
(* a run that keeps it when its process stops. The cure is all three TRUE; *)
(* the code before it is HoldToEnd = FALSE and KeepOnStop = FALSE.         *)
(*                                                                         *)
(* Abstractions: the instruments' readings and reports are not modelled    *)
(* (#142): the property is who reads when, not what a read returns. One    *)
(* weekly instrument stands for each; a week passing is Week. Two api runs *)
(* share the api process; each on-demand run is requested at most once.    *)
(* The job process installs no stop handler, so its run is never stopped   *)
(* apart from its read.                                                    *)
(***************************************************************************)

\* @phx covers crates/coordination/src/instruments.rs anchor=run_held digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/coordination/src/instruments.rs anchor=step digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/coordination/src/instruments.rs anchor=run_on_demand digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/coordination/src/sync_cycle.rs anchor=run_instruments digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/api/src/insights_routes.rs anchor=run digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/ingest/src/lock.rs anchor=try_exclusive digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/ingest/src/lock.rs anchor=hold_to_end digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/ingest/src/reader.rs anchor=with_copy digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #476
\* @phx cites #142
\* @phx property AtMostOneReader ramp=report
\* @phx property ADueWeeklyRunRunsOnceTheLockIsFree ramp=report
\* @phx witness witness/one-reader-needs-the-exclusive-acquire.cfg kills=AtMostOneReader
\* @phx witness witness/the-reader-keeps-the-lock-after-its-caller-goes.cfg kills=AtMostOneReader
\* @phx witness witness/the-reader-keeps-the-lock-until-its-process-exits.cfg kills=AtMostOneReader
\* @phx witness witness/a-due-weekly-run-runs-once-the-lock-is-free.cfg kills=ADueWeeklyRunRunsOnceTheLockIsFree
EXTENDS Naturals, FiniteSets

CONSTANTS NApi,        \* the api runs, all in the api process
          NBot,        \* the bot runs, all in the bot process
          Exclusive,   \* TRUE when the acquire is exclusive
          HoldToEnd,   \* TRUE when a run keeps the lock when its caller goes
          KeepOnStop   \* TRUE when a run keeps the lock when its process stops

W == <<"job", 0>>
ApiRuns == {<<"api", i>> : i \in 1..NApi}
BotRuns == {<<"bot", i>> : i \in 1..NBot}
OnDemand == ApiRuns \cup BotRuns
Runs == {W} \cup OnDemand
Procs == {"api", "bot"}
Proc(r) == r[1]
Busy == {"try", "held", "reading", "storing"}

VARIABLES pc,       \* each run's step
          reading,  \* the runs whose blocking read runs
          holders,  \* the runs whose handle holds the lock
          answer,   \* each on-demand run's answer to its caller
          due,      \* the weekly report is due
          up        \* each long-running process's state

vars == <<pc, reading, holders, answer, due, up>>

\* The weekly run, or an on-demand run whose process is up. The second disjunct names its own
\* domain, since TLC evaluates each disjunct of an action as a branch of its own.
Live(r) == r = W \/ (r \in OnDemand /\ up[Proc(r)] = "up")
Acquirable == holders = {} \/ ~Exclusive
Mine(p) == {r \in OnDemand : Proc(r) = p}

TypeOK == /\ pc \in [Runs -> {"idle", "check", "try", "held", "reading", "storing", "done"}]
          /\ reading \subseteq Runs
          /\ holders \subseteq Runs
          /\ answer \in [OnDemand -> {"none", "waiting", "report", "inProgress", "dropped"}]
          /\ due \in BOOLEAN
          /\ up \in [Procs -> {"up", "stopping", "exited"}]

Init == /\ pc = [r \in Runs |-> "idle"]
        /\ reading = {}
        /\ holders = {}
        /\ answer = [r \in OnDemand |-> "none"]
        /\ due = TRUE
        /\ up = [p \in Procs |-> "up"]

\* A week passes: the weekly report is due again.
Week == /\ ~due
        /\ due' = TRUE
        /\ UNCHANGED <<pc, reading, holders, answer, up>>

\* sync_cycle.rs::run_instruments: the job's sync starts instruments.rs::step.
Sync == /\ pc[W] = "idle"
        /\ pc' = [pc EXCEPT ![W] = "check"]
        /\ UNCHANGED <<reading, holders, answer, due, up>>

\* instruments.rs::step: the due read, before the lock.
Check == /\ pc[W] = "check"
         /\ pc' = [pc EXCEPT ![W] = IF due THEN "try" ELSE "idle"]
         /\ UNCHANGED <<reading, holders, answer, due, up>>

\* insights_routes.rs::run and instruments.rs::run_on_demand: an on-demand request.
Request(r) == /\ r \in OnDemand
              /\ pc[r] = "idle"
              /\ up[Proc(r)] = "up"
              /\ pc' = [pc EXCEPT ![r] = "try"]
              /\ answer' = [answer EXCEPT ![r] = "waiting"]
              /\ UNCHANGED <<reading, holders, due, up>>

\* instruments.rs::run_held through lock.rs::try_exclusive: the take, without waiting. A
\* refused weekly run goes back to idle and stays due; a refused on-demand run is answered
\* that a run is in progress.
TryLock(r) == /\ pc[r] = "try"
              /\ Live(r)
              /\ IF Acquirable
                    THEN /\ holders' = holders \cup {r}
                         /\ pc' = [pc EXCEPT ![r] = "held"]
                         /\ UNCHANGED answer
                    ELSE /\ UNCHANGED holders
                         /\ IF r = W
                               THEN /\ pc' = [pc EXCEPT ![r] = "idle"]
                                    /\ UNCHANGED answer
                               ELSE /\ pc' = [pc EXCEPT ![r] = "done"]
                                    /\ answer' = [answer EXCEPT ![r] = "inProgress"]
              /\ UNCHANGED <<reading, due, up>>

\* reader.rs::with_copy: the read starts on the offload's blocking thread.
StartRead(r) == /\ pc[r] = "held"
                /\ Live(r)
                /\ pc' = [pc EXCEPT ![r] = "reading"]
                /\ reading' = reading \cup {r}
                /\ UNCHANGED <<holders, answer, due, up>>

\* reader.rs::with_copy: a blocking read ends on its own, whoever still waits for it; the
\* run moves on only if it is still reading.
ReadEnds(r) == /\ r \in reading
               /\ reading' = reading \ {r}
               /\ pc' = IF pc[r] = "reading" THEN [pc EXCEPT ![r] = "storing"] ELSE pc
               /\ UNCHANGED <<holders, answer, due, up>>

\* instruments.rs::run_held and lock.rs::hold_to_end: the store, then the release; a waiting
\* caller gets the report.
Store(r) == /\ pc[r] = "storing"
            /\ Live(r)
            /\ due' = FALSE
            /\ holders' = holders \ {r}
            /\ pc' = [pc EXCEPT ![r] = IF r = W THEN "idle" ELSE "done"]
            /\ answer' = IF r \in OnDemand /\ answer[r] = "waiting"
                            THEN [answer EXCEPT ![r] = "report"]
                            ELSE answer
            /\ UNCHANGED <<reading, up>>

\* The request limit in router.rs, or a client gone: the caller of an on-demand run goes
\* away while its process is up. lock.rs::hold_to_end leaves the run to its end; without it
\* the run ends, and its lock goes with it unless it is kept.
Cancel(r) == /\ r \in OnDemand
             /\ answer[r] = "waiting"
             /\ pc[r] \in Busy
             /\ up[Proc(r)] = "up"
             /\ answer' = [answer EXCEPT ![r] = "dropped"]
             /\ pc' = IF pc[r] # "try" /\ HoldToEnd THEN pc ELSE [pc EXCEPT ![r] = "done"]
             /\ holders' = IF pc[r] = "try" \/ HoldToEnd \/ KeepOnStop
                              THEN holders
                              ELSE holders \ {r}
             /\ UNCHANGED <<reading, due, up>>

\* The runtime's shutdown in main.rs: the process stops, and every busy run of it ends while
\* its read runs on; lock.rs::hold_to_end keeps the lock until the process exits.
Stop(p) == /\ up[p] = "up"
           /\ up' = [up EXCEPT ![p] = "stopping"]
           /\ pc' = [r \in Runs |-> IF r \in Mine(p) /\ pc[r] \in Busy THEN "done" ELSE pc[r]]
           /\ answer' = [r \in OnDemand |->
                           IF r \in Mine(p) /\ pc[r] \in Busy /\ answer[r] = "waiting"
                              THEN "dropped"
                              ELSE answer[r]]
           /\ holders' = IF KeepOnStop THEN holders ELSE holders \ Mine(p)
           /\ reading' = reading
           /\ UNCHANGED due

\* The process exits: the operating system closes its lock file, and its reads end with it.
Exit(p) == /\ up[p] = "stopping"
           /\ up' = [up EXCEPT ![p] = "exited"]
           /\ reading' = reading \ Mine(p)
           /\ holders' = holders \ Mine(p)
           /\ UNCHANGED <<pc, answer, due>>

Next == \/ Week
        \/ Sync
        \/ Check
        \/ \E r \in Runs : TryLock(r) \/ StartRead(r) \/ ReadEnds(r) \/ Store(r)
        \/ \E r \in OnDemand : Request(r) \/ Cancel(r)
        \/ \E p \in Procs : Stop(p) \/ Exit(p)

\* No fairness here: the liveness states its own. The weekly run always has a move, so the
\* model never deadlocks.
Spec == Init /\ [][Next]_vars

\* ADR-094's consequence, "at most one instrument reads at a time on the host", and SPEC-094
\* R8, "so no two instruments read at once on the host": at most one instrument run holds the
\* lock and reads at any time.
AtMostOneReader == \A r1, r2 \in reading : r1 = r2

RunsFair == \A r \in Runs : WF_vars(TryLock(r)) /\ WF_vars(StartRead(r))
                            /\ WF_vars(ReadEnds(r)) /\ WF_vars(Store(r))
ExitFair == \A p \in Procs : WF_vars(Exit(p))

\* A due weekly run eventually runs once the lock is free. A stop and a cancel are never
\* fair: nothing obliges either to happen.
ADueWeeklyRunRunsOnceTheLockIsFree ==
    (WF_vars(Sync) /\ WF_vars(Check) /\ RunsFair /\ ExitFair) => (due ~> ~due)

=============================================================================
