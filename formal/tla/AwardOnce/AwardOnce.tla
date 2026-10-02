---------------------------- MODULE AwardOnce ----------------------------
\* @phx covers crates/coordination/src/recompute/mod.rs anchor=run digest=sha256:c51dc67a3988bbeb8c46a953acdaba06669275e0ba405edb1146de880ae815fd
\* @phx covers crates/coordination/src/recompute/mod.rs anchor=offer_owed digest=sha256:0d418cb9fe6912d5780f32d0e837891e9927590bd6cc55b3aaf883ebfdf5bc00
\* @phx covers crates/coordination/src/recompute/badges.rs anchor=evaluate digest=sha256:20660e5d08a54f50f60824125958cba6cf65c5750af0a7468ecbad0e26ffd548
\* @phx covers crates/coordination/src/recompute/badges.rs anchor=offer_badges digest=sha256:cbf19ec6758776f6a64ffd60c45d775f30003963a3a1d2c273571016df1d3b48
\* @phx covers crates/coordination/src/recompute/records.rs anchor=evaluate digest=sha256:5aa0de349398bcf888dfaadd1437dbe027b2e4d82ba3ec530c3ad5f7bab08218
\* @phx covers crates/coordination/src/recompute/records.rs anchor=upsert digest=sha256:0702ef24ebd40778731306415bead309ef68954c2c858320eb32ed78b2f1bc4e
\* @phx covers crates/coordination/src/recompute/records.rs anchor=offer_records digest=sha256:94d4c086e62de5ee1bc65547fa613409b603a137fb3e5e24a0e70c82bac9243c
\* @phx covers crates/coordination/src/sync_cycle.rs anchor=sync_cycle digest=sha256:b8158a6a2359166e7b55a5dbbb15189d50cdbeebd84c20a0805067046c31dc6c
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
\* @phx witness witness/a-same-day-beat-keeps-the-seeds-mark.cfg kills=NoSilentLoss
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
\*
\* Re-read against the code that implements design "mark" with Recheck (SPEC-073 part B):
\* - Pre and DrainSkip: recompute/mod.rs::run calls mod.rs::offer_owed before each settled day's
\*   write, before the current day's write, and after the revisit write commits; offer_owed logs a
\*   failed offer and never fails the fold.
\* - DrainOffer and Offer: badges.rs::offer_badges and records.rs::offer_records hand each row
\*   whose mark is unset to the router (AwardOffers::offer, then Router's Celebrate port, which
\*   calls router.rs::route).
\* - DrainMiss and OfferMiss: the router's Err arm in offer_badges and offer_records logs and
\*   continues, and nothing is marked.
\* - DrainMark and Clear: the mark is a write of its own, UPDATE ... AND celebrated_at IS NULL,
\*   keyed by badge and tier, or by record kind AND study day, so it marks only the offered item
\*   and only while the row still holds it.
\* - Commit (award): badges.rs::evaluate awards inside the fold's BEGIN IMMEDIATE write, its mark
\*   unset; the progression award port's unique key answers AlreadyAwarded (UniqueKey).
\* - Commit (record, Recheck): records.rs::upsert re-reads the kind's row inside the day's write,
\*   names a row of another day whose mark is unset ("celebration not sent"), then upserts; a row
\*   of the same day keeps its mark, so a best that climbs all day is one award, unless the row is
\*   the seed (its previous equals its value, R12, and every detected record's previous is below
\*   its value), which a beat of its day replaces with its mark unset (KeepSeedMark = FALSE).
\* - Seed (records.rs::evaluate, SPEC-073 R12, fix round 1): with no record stored, the first
\*   detection writes the window's best for the evaluation's own day with the mark already set; it
\*   is no award, so it owes nothing, and Seeded reads it as the row no award wrote.
\* - Router(d) and OnceKey: ledger.rs::claim. Crash: between any two of those transactions.
\* Abstractions, each a stuttering of the model's variables:
\* - the offers read the unmarked rows on a reader, then route; DrainOffer reads and routes in one
\*   step. A row replaced between the read and the route is offered under its own day's key, its
\*   mark write then matches nothing, and the replacing write has named it: no property moves;
\* - the seed writes every kind's best at once; one row stands for the kinds, as for the records;
\* - the backfill and revisit writes run no badge or record step (mod.rs's
\*   Evaluation::runs_today_only_rules is Settle and Current only), so the backfill's write with
\*   no offers before it awards nothing, and the offers after the revisit write are the Offer that
\*   follows the current day's Commit.
(***************************************************************************)
EXTENDS Naturals

CONSTANTS Design, UniqueKey, OnceKey, Drain, Recheck, KeepSeedMark, MaxCrash, MaxMiss

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

\* the row holds the seed: a best no award wrote, whose previous equals its value (R12)
Seeded == row # 0 /\ awards[row] = 0

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
               \* records.rs::upsert's CASE: a row of the same day keeps its mark unless it is
               \* the seed; KeepSeedMark is the CASE before fix round 1, which kept the seed's too
               /\ due' = IF row = d /\ (~Seeded \/ KeepSeedMark) THEN due ELSE (Design = "mark")
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

\* the first detection, with no record stored: the window's best for the evaluation's own day,
\* written with its mark set, in the day's own write; it is no award
Seed(e) ==
    /\ pc[e] = "write"
    /\ Design = "mark"
    /\ row = 0
    /\ row' = DayOf[e]
    /\ due' = FALSE
    /\ pc' = [pc EXCEPT ![e] = "committed"]
    /\ UNCHANGED <<awards, sent, named, fresh, off, crashes, misses>>

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
          \/ Seed(e) \/ Commit(e) \/ Offer(e) \/ OfferMiss(e) \/ Clear(e) \/ Crash(e)
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
