---------------------------- MODULE FoldSettlesOnce ----------------------------
\* @phx covers crates/coordination/src/recompute/mod.rs anchor=run digest=sha256:7a383ba01872f4dde46ccb5220fa492de1f89498adf71774cae868e3e0b7acbb
\* @phx covers crates/analytics/src/rollup.rs anchor=settle_cursor digest=sha256:2571fa2f5d4ae18a4571c59f3f6911ba61a864f2bb2cee248288ec9a189976c1
\* @phx covers crates/analytics/src/rollup.rs anchor=record_settled digest=sha256:41ddfd54e6f21de292b44e7c58a0157155b5b3a40ff3d50a4c7cb68805695b2c
\* @phx covers crates/coordination/src/sync_cycle.rs anchor=sync_cycle digest=sha256:b8158a6a2359166e7b55a5dbbb15189d50cdbeebd84c20a0805067046c31dc6c
\* @phx covers crates/kernel/src/db.rs anchor=write digest=sha256:c3d700eda268a6f46c7eea0aabcd2f62d8fc0aeffbe03d1cd1e8437a88bdb916
\* @phx cites #311
\* @phx property SettleOnce ramp=report
\* @phx property SettleOldestFirst ramp=report
\* @phx witness witness/settle-once-without-the-re-read.cfg kills=SettleOnce
\* @phx witness witness/oldest-first-without-the-re-read.cfg kills=SettleOldestFirst
\* @phx property SettleInTurn ramp=report
\* @phx witness witness/in-turn-without-the-re-read.cfg kills=SettleInTurn
(***************************************************************************)
\* The recompute's fold (SPEC-071 R15 to R17, ADR-071) run by two cycles over one ledger: the
\* scheduled cycle and the owner's recompute. Each reaches the fold after its own successful sync,
\* and either can reach it before the other has cleared the rescore mark, so their folds overlap.
\* Each cycle runs its fold up to MaxRuns times; a run's now is read when it starts (Start), and
\* its sync started that study day or, across the rollover, the day before (FoldInput::synced_in).
\*
\* Each action is ONE transaction of the code, so a Crash may fall between any two of them:
\* - db.rs::write opens every write with BEGIN IMMEDIATE, so two folds' writes never interleave
\*   inside one transaction, on one connection or two, in one process or two;
\* - Read is the fold's first write (mod.rs::run step 1): it reads the cursor
\*   (rollup.rs::settle_cursor, the last settled day); with none, it rolls up the past study days
\*   in the historical form and owes the most recently closed day; otherwise it owes the day after
\*   the cursor;
\* - Settle is one owed day's write (mod.rs::run step 2): the day's steps and its settled_at
\*   (rollup.rs::record_settled), which moves the cursor in the same write (R16). The day is owed
\*   only while the run's sync started after its close (R15). Recheck: the write re-reads the cursor
\*   inside its own BEGIN IMMEDIATE, and the day owed is the day after it or, with no cursor, the
\*   run's own day; a write for any other day writes nothing, and the run goes on from the owed day
\*   (ADR-313, mod.rs::run as #311 built it). With Recheck = FALSE the write trusts the cursor the
\*   run read in its first write, as mod.rs::run did before #311;
\* - Finish is the loop's end: the current day's write, the revisit's write and the offers touch
\*   neither the cursor nor settled_at, so each is a stuttering step of this model.
\*
\* Abstractions, each a stuttering of the model's variables:
\* - the rescore mark (sync_cycle.rs::sync_cycle runs the fold after the gate's check and clears the
\*   mark after the fold) admits a fold; the fold never reads the mark. Any cycle may start a run at
\*   any time here, a superset of what the mark admits, so both cycles reaching the fold before
\*   either clears the mark is a behaviour of this model;
\* - the backfill's historical rows and the current day's row are not settles: the days before the
\*   first settled day are outside the settle set (SPEC-071's amendment of 2026-10-02, #311), and no
\*   variable here holds a row;
\* - the offers between writes (mod.rs::offer_owed) run in transactions of their own and touch
\*   neither the cursor nor settled_at (AwardOnce models them).
\*
\* Re-read of mod.rs::run on 2026-10-02, against #311's fix (ADR-313): the owed loop's write now
\* reads rollup.rs::settle_cursor first; `owed != day` rolls the write back and sets the loop's day
\* to the owed one, which is Settle's Recheck arm (next' = owed, no settle). The first write, the
\* current day's write and the revisit are unchanged, so Read and Finish stand as they were.
(***************************************************************************)
EXTENDS Naturals

CONSTANTS Recheck, MaxDay, MaxRuns, MaxCrash

Cycles == {"scheduled", "owner"}
Days == 1..MaxDay

VARIABLES today, cursor, settles, backward, gap, pc, closed, sync, next, runs, crashes

vars == <<today, cursor, settles, backward, gap, pc, closed, sync, next, runs, crashes>>

Max(a, b) == IF a >= b THEN a ELSE b

TypeOK ==
    /\ today \in Days
    /\ cursor \in 0..MaxDay
    /\ settles \in [Days -> 0..(2 * MaxRuns)]
    /\ backward \in BOOLEAN
    /\ pc \in [Cycles -> {"idle", "read", "settle"}]
    /\ closed \in [Cycles -> 0..MaxDay]
    /\ sync \in [Cycles -> 0..MaxDay]
    /\ next \in [Cycles -> 0..(MaxDay + 1)]
    /\ runs \in [Cycles -> 0..MaxRuns]
    /\ crashes \in 0..MaxCrash
    /\ gap \in BOOLEAN

Init ==
    /\ today = 1
    /\ cursor = 0
    /\ settles = [d \in Days |-> 0]
    /\ backward = FALSE
    /\ gap = FALSE
    /\ pc = [c \in Cycles |-> "idle"]
    /\ closed = [c \in Cycles |-> 0]
    /\ sync = [c \in Cycles |-> 0]
    /\ next = [c \in Cycles |-> 0]
    /\ runs = [c \in Cycles |-> 0]
    /\ crashes = 0

\* the study day rolls over; a day may pass with no successful sync and no fold
Tick ==
    /\ today < MaxDay
    /\ today' = today + 1
    /\ UNCHANGED <<cursor, settles, backward, gap, pc, closed, sync, next, runs, crashes>>

\* a cycle's sync succeeded and its fold starts: the run's now is read here
Start(c) ==
    /\ pc[c] = "idle"
    /\ runs[c] < MaxRuns
    /\ today >= 2
    /\ runs' = [runs EXCEPT ![c] = runs[c] + 1]
    /\ closed' = [closed EXCEPT ![c] = today - 1]
    /\ \E s \in {today - 1, today} : sync' = [sync EXCEPT ![c] = s]
    /\ pc' = [pc EXCEPT ![c] = "read"]
    /\ UNCHANGED <<today, cursor, settles, backward, gap, next, crashes>>

\* the fold's first write: the cursor read once, and the first owed day
Read(c) ==
    /\ pc[c] = "read"
    /\ next' = [next EXCEPT ![c] = IF cursor = 0 THEN closed[c] ELSE cursor + 1]
    /\ pc' = [pc EXCEPT ![c] = "settle"]
    /\ UNCHANGED <<today, cursor, settles, backward, gap, closed, sync, runs, crashes>>

\* the run's loop condition: the day has closed, and the run's sync started after its close
Owed(c) == next[c] <= closed[c] /\ next[c] < sync[c]

\* one owed day's write, BEGIN IMMEDIATE: the day's steps and its settled_at, or, with Recheck,
\* nothing when the day is not the one the cursor re-read inside the write owes (the day after it,
\* or with no cursor the run's own day), and the run goes on from the owed day
Settle(c) ==
    /\ pc[c] = "settle"
    /\ Owed(c)
    /\ LET d == next[c]
           owed == IF cursor = 0 THEN d ELSE cursor + 1 IN
       IF Recheck /\ d # owed
          THEN /\ next' = [next EXCEPT ![c] = owed]
               /\ UNCHANGED <<cursor, settles, backward, gap>>
          ELSE /\ settles' = [settles EXCEPT ![d] = settles[d] + 1]
               /\ backward' = (backward \/ d < cursor)
               /\ gap' = (gap \/ (cursor # 0 /\ d > cursor + 1))
               /\ cursor' = Max(cursor, d)
               /\ next' = [next EXCEPT ![c] = d + 1]
    /\ UNCHANGED <<today, pc, closed, sync, runs, crashes>>

\* the loop is done: the current day, the revisit and the offers, then the run ends
Finish(c) ==
    /\ pc[c] = "settle"
    /\ ~Owed(c)
    /\ pc' = [pc EXCEPT ![c] = "idle"]
    /\ UNCHANGED <<today, cursor, settles, backward, gap, closed, sync, next, runs, crashes>>

\* the process dies between two transactions; a later run of the cycle is a new process
Crash(c) ==
    /\ pc[c] \in {"read", "settle"}
    /\ crashes < MaxCrash
    /\ crashes' = crashes + 1
    /\ pc' = [pc EXCEPT ![c] = "idle"]
    /\ UNCHANGED <<today, cursor, settles, backward, gap, closed, sync, next, runs>>

Quiescent == \A c \in Cycles : pc[c] = "idle"

Finished == Quiescent /\ UNCHANGED vars

Next ==
    \/ Tick
    \/ \E c \in Cycles : Start(c) \/ Read(c) \/ Settle(c) \/ Finish(c) \/ Crash(c)
    \/ Finished

Spec == Init /\ [][Next]_vars

\* R16: a recompute never settles a day twice, whichever cycles overlap (#311)
SettleOnce == \A d \in Days : settles[d] <= 1

\* R15: the closed days are settled oldest first: no day is settled after a later one
SettleOldestFirst == ~backward

\* R15, R16: after the first settled day each closed day is settled in turn: no settle lands past the
\* day after the cursor, so no day between two settled days is skipped (#311, ADR-313); the days
\* before the first settled day are outside it (SPEC-071's amendment of 2026-10-02)
SettleInTurn == ~gap
=============================================================================
