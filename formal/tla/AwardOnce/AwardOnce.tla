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
\* per kind: day 2's beat replaces day 1's row.
\*
\* Each action is ONE transaction of the code, so a Crash may fall between any two of them:
\* - the router runs in transactions of its own, never inside a caller's write: router.rs::route
\*   opens its own write on its own pooled connection (db.rs::write is BEGIN IMMEDIATE), so an
\*   offer (DrainOffer, Offer) is an action apart from the day's write (Commit) and apart from the
\*   mark (DrainMark, Clear);
\* - a router call may come back with no answer (an error): it may or may not have claimed and sent
\*   (a bot send commits its claim before the call, router.rs::send_bot), and nothing is marked
\*   (DrainMiss, OfferMiss, at most MaxMiss of them);
\* - the mark is its own write, and it re-reads the row: it marks only the item that was offered and
\*   only while the row still holds it.
\*
\* Design "literal": only the evaluation whose insert wrote the row offers the celebration, and
\* AlreadyAwarded raises none. Design "mark": the award's row carries an unset mark written in the
\* same write; before each day's write the fold offers every unmarked award and marks each one the
\* router answered (Drain), and the evaluation offers its own award after its write. Recheck: the
\* day's write re-reads the row inside its own BEGIN IMMEDIATE, and when it replaces another day's
\* row whose mark is still unset it names that record in a log line (kind, day, celebration not
\* sent) before it writes.
(***************************************************************************)
EXTENDS Naturals

CONSTANTS Design, UniqueKey, OnceKey, Drain, Recheck, MaxCrash, MaxMiss

Evals == {1, 2, 3}
Days == {1, 2}
DayOf == [e \in Evals |-> IF e = 3 THEN 2 ELSE 1]

VARIABLES awards, sent, row, due, named, pc, fresh, off, crashes, misses

vars == <<awards, sent, row, due, named, pc, fresh, off, crashes, misses>>

TypeOK ==
    /\ awards \in [Days -> 0..2]
    /\ sent \in [Days -> 0..3]
    /\ row \in 0..2
    /\ due \in BOOLEAN
    /\ named \in [Days -> BOOLEAN]
    /\ pc \in [Evals -> {"idle", "drain", "drained", "write", "committed", "routed", "done", "dead"}]
    /\ fresh \in [Evals -> BOOLEAN]
    /\ off \in [Evals -> 0..2]
    /\ crashes \in 0..MaxCrash
    /\ misses \in 0..MaxMiss

Init ==
    /\ awards = [d \in Days |-> 0]
    /\ sent = [d \in Days |-> 0]
    /\ row = 0
    /\ due = FALSE
    /\ named = [d \in Days |-> FALSE]
    /\ pc = [e \in Evals |-> "idle"]
    /\ fresh = [e \in Evals |-> FALSE]
    /\ off = [e \in Evals |-> 0]
    /\ crashes = 0
    /\ misses = 0

\* the router's claim of the once-ever key (ledger.rs::claim): a second offer of a key collapses
Router(d) == [sent EXCEPT ![d] = IF OnceKey THEN (IF sent[d] = 0 THEN 1 ELSE sent[d]) ELSE sent[d] + 1]

\* day 2 settles after day 1's evaluation has ended
Startable(e) == e # 3 \/ pc[1] \in {"done", "dead"}

\* an unmarked award is on the row
Pending == due /\ row # 0

Pre(e) ==
    /\ pc[e] = "idle"
    /\ Startable(e)
    /\ pc' = [pc EXCEPT ![e] = IF Design = "mark" /\ Drain THEN "drain" ELSE "write"]
    /\ UNCHANGED <<awards, sent, row, due, named, fresh, off, crashes, misses>>

\* the fold's offers before the day's write: the router answered the pending award
DrainOffer(e) ==
    /\ pc[e] = "drain"
    /\ Pending
    /\ sent' = Router(row)
    /\ off' = [off EXCEPT ![e] = row]
    /\ pc' = [pc EXCEPT ![e] = "drained"]
    /\ UNCHANGED <<awards, row, due, named, fresh, crashes, misses>>

\* the router came back with no answer: the item stays unmarked, and the fold goes on to its write
DrainMiss(e) ==
    /\ pc[e] = "drain"
    /\ Pending
    /\ misses < MaxMiss
    /\ misses' = misses + 1
    /\ \E s \in {sent, Router(row)} : sent' = s
    /\ pc' = [pc EXCEPT ![e] = "write"]
    /\ UNCHANGED <<awards, row, due, named, fresh, off, crashes>>

\* nothing is pending: the fold goes on to its write
DrainSkip(e) ==
    /\ pc[e] = "drain"
    /\ ~Pending
    /\ pc' = [pc EXCEPT ![e] = "write"]
    /\ UNCHANGED <<awards, sent, row, due, named, fresh, off, crashes, misses>>

\* the mark's own write: only the offered item, and only while the row still holds it
DrainMark(e) ==
    /\ pc[e] = "drained"
    /\ due' = IF row = off[e] THEN FALSE ELSE due
    /\ pc' = [pc EXCEPT ![e] = "write"]
    /\ UNCHANGED <<awards, sent, row, named, fresh, off, crashes, misses>>

\* the day's write: one BEGIN IMMEDIATE that re-reads the row, then the award row and its unset mark
Commit(e) ==
    /\ pc[e] = "write"
    /\ LET d == DayOf[e] IN
       IF UniqueKey /\ awards[d] > 0
          THEN /\ UNCHANGED <<awards, row, due, named>>
               /\ fresh' = [fresh EXCEPT ![e] = FALSE]
          ELSE /\ awards' = [awards EXCEPT ![d] = awards[d] + 1]
               /\ named' = IF Design = "mark" /\ Recheck /\ due /\ row # 0 /\ row # d
                              THEN [named EXCEPT ![row] = TRUE]
                              ELSE named
               /\ row' = d
               /\ due' = (Design = "mark")
               /\ fresh' = [fresh EXCEPT ![e] = TRUE]
    /\ pc' = [pc EXCEPT ![e] = "committed"]
    /\ UNCHANGED <<sent, off, crashes, misses>>

\* the hand-off to the router after the commit (the next offers of the fold, or the last ones)
Offer(e) ==
    /\ pc[e] = "committed"
    /\ IF Design = "mark"
          THEN IF Pending
                  THEN /\ sent' = Router(row)
                       /\ off' = [off EXCEPT ![e] = row]
                       /\ pc' = [pc EXCEPT ![e] = "routed"]
                  ELSE /\ pc' = [pc EXCEPT ![e] = "done"]
                       /\ UNCHANGED <<sent, off>>
          ELSE IF fresh[e]
                  THEN /\ sent' = Router(DayOf[e])
                       /\ off' = [off EXCEPT ![e] = DayOf[e]]
                       /\ pc' = [pc EXCEPT ![e] = "routed"]
                  ELSE /\ pc' = [pc EXCEPT ![e] = "done"]
                       /\ UNCHANGED <<sent, off>>
    /\ UNCHANGED <<awards, row, due, named, fresh, crashes, misses>>

\* the router came back with no answer to the offer after the commit: nothing is marked
OfferMiss(e) ==
    /\ pc[e] = "committed"
    /\ Design = "mark"
    /\ Pending
    /\ misses < MaxMiss
    /\ misses' = misses + 1
    /\ \E s \in {sent, Router(row)} : sent' = s
    /\ pc' = [pc EXCEPT ![e] = "done"]
    /\ UNCHANGED <<awards, row, due, named, fresh, off, crashes>>

\* the mark of the offered item, set in its own write once the router has answered
Clear(e) ==
    /\ pc[e] = "routed"
    /\ due' = IF Design = "mark" /\ row = off[e] THEN FALSE ELSE due
    /\ pc' = [pc EXCEPT ![e] = "done"]
    /\ UNCHANGED <<awards, sent, row, named, fresh, off, crashes, misses>>

\* the process dies between any two transactions
Crash(e) ==
    /\ pc[e] \in {"drain", "drained", "write", "committed", "routed"}
    /\ crashes < MaxCrash
    /\ crashes' = crashes + 1
    /\ pc' = [pc EXCEPT ![e] = "dead"]
    /\ UNCHANGED <<awards, sent, row, due, named, fresh, off, misses>>

Quiescent == \A e \in Evals : pc[e] \in {"done", "dead"}

Finished == Quiescent /\ UNCHANGED vars

Next ==
    \/ \E e \in Evals :
          \/ Pre(e) \/ DrainOffer(e) \/ DrainMiss(e) \/ DrainSkip(e) \/ DrainMark(e)
          \/ Commit(e) \/ Offer(e) \/ OfferMiss(e) \/ Clear(e) \/ Crash(e)
    \/ Finished

Spec == Init /\ [][Next]_vars

\* one row per key and tier, per record kind and day (R1, R3, R9 to R11)
AwardOnce == \A d \in Days : awards[d] <= 1

\* at most one celebration per dedupe key
CelebrateAtMostOnce == \A d \in Days : sent[d] <= 1

\* an award is celebrated, or its celebration is still due on the row that holds it, or a record
\* superseded before it could be offered is named: never dropped silently
NoSilentLoss ==
    Quiescent => \A d \in Days : awards[d] > 0 => (sent[d] > 0 \/ (row = d /\ due) \/ named[d])
=============================================================================
