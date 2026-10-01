---------------------------- MODULE AwardOnce ----------------------------
\* @phx covers crates/coordination/src/recompute/mod.rs anchor=run digest=sha256:9cd6a034ba6acaaa0686f1aa8def456b21e9128164b1afc52623548b1a62dea1
\* @phx covers crates/coordination/src/sync_cycle.rs anchor=sync_cycle digest=sha256:1b41728ec7e07a7581f9cf9ce88f74f02ad3e59e9a0d402c4ac81a71d128d060
\* @phx covers crates/coordination/src/level_up.rs anchor=announce_level_up digest=sha256:acbb8923e76fbd4ab77949a119c1ad3a87de6f41737e65b89df689875e851dd0
\* @phx covers crates/notifications/src/router.rs anchor=route digest=sha256:7bcf52fe22d886b3d71dfa0fa8e6dfb1d266a9a6b1662bada5482a2cd0c54dcb
\* @phx covers crates/notifications/src/ledger.rs anchor=claim digest=sha256:0116ef4925de04614d09ac18952c0a0b0f7248fd65f5d4f6ca555448836a6ab7
\* @phx cites #74, #75, #76
\* @phx property AwardOnce ramp=report
\* @phx property CelebrateAtMostOnce ramp=report
\* @phx property NoSilentLoss ramp=report
\* @phx witness witness/an-award-with-no-unique-key.cfg kills=AwardOnce
\* @phx witness witness/a-router-with-no-once-ever-key.cfg kills=CelebrateAtMostOnce
\* @phx witness witness/a-celebration-raised-only-by-the-inserting-evaluation.cfg kills=NoSilentLoss
\* @phx witness witness/a-later-day-overwrites-an-unoffered-record.cfg kills=NoSilentLoss
(***************************************************************************)
\* One award per day (a badge at a tier is the same shape with one day), and the evaluations that
\* may reach them: day 1 at its settle (evaluation 1), the replay of day 1 (evaluation 2), and day
\* 2 (evaluation 3), which settles only after evaluation 1 has ended or died. A record is ONE row
\* per kind: day 2's beat overwrites day 1's row.
\* An evaluation commits its write, and only afterwards hands the celebration to the router
\* (level_up.rs::announce_level_up is called after the recompute, never inside the fold's write).
\* The router claims a once-ever key (ledger.rs::claim), so a second offer collapses.
\* Design "literal": only the evaluation whose insert wrote the row offers the celebration, and
\* AlreadyAwarded raises none. Design "outbox": the award's row carries a due mark written in the
\* same write; a write that would overwrite another day's unmarked row first offers that row's pending mark (Drain),
\* then writes, and the evaluation offers again after its write, and the mark is cleared, for its own item only, once the router
\* answered. Crash may fall before the write, after it and before the offer, or between the
\* router's answer and the clearing.
(***************************************************************************)
EXTENDS Naturals

CONSTANTS Design, UniqueKey, OnceKey, OverwriteOffers, MaxCrash

Evals == {1, 2, 3}
Days == {1, 2}
DayOf == [e \in Evals |-> IF e = 3 THEN 2 ELSE 1]

VARIABLES awards, sent, row, due, pc, fresh, off, crashes

vars == <<awards, sent, row, due, pc, fresh, off, crashes>>

TypeOK ==
    /\ awards \in [Days -> 0..2]
    /\ sent \in [Days -> 0..2]
    /\ row \in 0..2
    /\ due \in BOOLEAN
    /\ pc \in [Evals -> {"idle", "ready", "committed", "routed", "done", "dead"}]
    /\ fresh \in [Evals -> BOOLEAN]
    /\ off \in [Evals -> 0..2]
    /\ crashes \in 0..MaxCrash

Init ==
    /\ awards = [d \in Days |-> 0]
    /\ sent = [d \in Days |-> 0]
    /\ row = 0
    /\ due = FALSE
    /\ pc = [e \in Evals |-> "idle"]
    /\ fresh = [e \in Evals |-> FALSE]
    /\ off = [e \in Evals |-> 0]
    /\ crashes = 0

Router(d) == [sent EXCEPT ![d] = IF OnceKey THEN (IF sent[d] = 0 THEN 1 ELSE sent[d]) ELSE sent[d] + 1]

\* day 2 settles after day 1's evaluation has ended
Startable(e) == e # 3 \/ pc[1] \in {"done", "dead"}

Pre(e) ==
    /\ pc[e] = "idle"
    /\ Startable(e)
    /\ pc' = [pc EXCEPT ![e] = "ready"]
    /\ UNCHANGED <<awards, sent, row, due, fresh, off, crashes>>

\* the write would overwrite the record row of another day whose mark is unset (the write's own read)
Blocked(e) == Design = "outbox" /\ OverwriteOffers /\ due /\ row # 0 /\ row # DayOf[e]

\* the step offers that pending mark first and marks it, then retries its write
Drain(e) ==
    /\ pc[e] = "ready"
    /\ Blocked(e)
    /\ sent' = Router(row)
    /\ due' = FALSE
    /\ UNCHANGED <<awards, row, pc, fresh, off, crashes>>

\* the day's write: one BEGIN IMMEDIATE, the award row and (outbox) its due mark together
Commit(e) ==
    /\ pc[e] = "ready"
    /\ ~Blocked(e)
    /\ LET d == DayOf[e] IN
       IF UniqueKey /\ awards[d] > 0
          THEN /\ UNCHANGED <<awards, row, due>>
               /\ fresh' = [fresh EXCEPT ![e] = FALSE]
          ELSE /\ awards' = [awards EXCEPT ![d] = awards[d] + 1]
               /\ row' = d
               /\ due' = (Design = "outbox")
               /\ fresh' = [fresh EXCEPT ![e] = TRUE]
    /\ pc' = [pc EXCEPT ![e] = "committed"]
    /\ UNCHANGED <<sent, off, crashes>>

\* the hand-off to the router, after the commit
Offer(e) ==
    /\ pc[e] = "committed"
    /\ IF Design = "outbox"
          THEN IF due /\ row # 0
                  THEN /\ sent' = Router(row)
                       /\ off' = [off EXCEPT ![e] = row]
                       /\ pc' = [pc EXCEPT ![e] = "routed"]
                  ELSE /\ pc' = [pc EXCEPT ![e] = "done"]
                       /\ UNCHANGED <<sent, off>>
          ELSE IF fresh[e]
                  THEN /\ sent' = Router(DayOf[e])
                       /\ pc' = [pc EXCEPT ![e] = "routed"]
                       /\ UNCHANGED off
                  ELSE /\ pc' = [pc EXCEPT ![e] = "done"]
                       /\ UNCHANGED <<sent, off>>
    /\ UNCHANGED <<awards, row, due, fresh, crashes>>

\* the mark of the offered item is set once the router has answered
Clear(e) ==
    /\ pc[e] = "routed"
    /\ due' = IF Design = "outbox" /\ row = off[e] THEN FALSE ELSE due
    /\ pc' = [pc EXCEPT ![e] = "done"]
    /\ UNCHANGED <<awards, sent, row, fresh, off, crashes>>

\* the process dies before a write, after a commit before the offer, or before the clearing
Crash(e) ==
    /\ pc[e] \in {"ready", "committed", "routed"}
    /\ crashes < MaxCrash
    /\ crashes' = crashes + 1
    /\ pc' = [pc EXCEPT ![e] = "dead"]
    /\ UNCHANGED <<awards, sent, row, due, fresh, off>>

Quiescent == \A e \in Evals : pc[e] \in {"done", "dead"}

Finished == Quiescent /\ UNCHANGED vars

Next == (\E e \in Evals : Pre(e) \/ Drain(e) \/ Commit(e) \/ Offer(e) \/ Clear(e) \/ Crash(e)) \/ Finished

Spec == Init /\ [][Next]_vars

\* one row per key and tier, per record kind and day (R1, R3, R9 to R11)
AwardOnce == \A d \in Days : awards[d] <= 1

\* at most one celebration per dedupe key
CelebrateAtMostOnce == \A d \in Days : sent[d] <= 1

\* an award is celebrated, or its celebration is still due on the row that holds it: never dropped
NoSilentLoss ==
    Quiescent => \A d \in Days : awards[d] > 0 => (sent[d] > 0 \/ (row = d /\ due))
=============================================================================
