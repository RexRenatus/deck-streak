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
(***************************************************************************)
\* One award (a badge at a tier, or a record of a kind and day) and the two evaluations of one
\* day that may reach it: the closing day at its settle and its replay, or the current day.
\* An evaluation commits its write, and only afterwards hands the celebration to the router
\* (level_up.rs::announce_level_up is called after the recompute, never inside the fold's write).
\* The router claims a once-ever key (ledger.rs::claim), so a second offer collapses.
\* Design "literal": only the evaluation whose insert wrote the row offers the celebration, and
\* AlreadyAwarded raises none. Design "outbox": the award's own row carries a due mark written in
\* the same write; any evaluation that finds the mark offers the celebration, and the mark is
\* cleared after the router answered. Crash may fall after the commit and before the offer, or
\* between the router's answer and the clearing.
(***************************************************************************)
EXTENDS Naturals

CONSTANTS Design, UniqueKey, OnceKey, MaxCrash

Evals == {1, 2}

VARIABLES awards, sent, due, pc, fresh, crashes

vars == <<awards, sent, due, pc, fresh, crashes>>

TypeOK ==
    /\ awards \in 0..2
    /\ sent \in 0..2
    /\ due \in BOOLEAN
    /\ pc \in [Evals -> {"idle", "committed", "routed", "done", "dead"}]
    /\ fresh \in [Evals -> BOOLEAN]
    /\ crashes \in 0..MaxCrash

Init ==
    /\ awards = 0
    /\ sent = 0
    /\ due = FALSE
    /\ pc = [e \in Evals |-> "idle"]
    /\ fresh = [e \in Evals |-> FALSE]
    /\ crashes = 0

\* the day's write: one BEGIN IMMEDIATE, the award row and (outbox) its due mark together
Commit(e) ==
    /\ pc[e] = "idle"
    /\ IF UniqueKey /\ awards > 0
          THEN /\ UNCHANGED <<awards, due>>
               /\ fresh' = [fresh EXCEPT ![e] = FALSE]
          ELSE /\ awards' = awards + 1
               /\ due' = (Design = "outbox")
               /\ fresh' = [fresh EXCEPT ![e] = TRUE]
    /\ pc' = [pc EXCEPT ![e] = "committed"]
    /\ UNCHANGED <<sent, crashes>>

\* the hand-off to the router, after the commit
Offer(e) ==
    /\ pc[e] = "committed"
    /\ IF fresh[e] \/ (Design = "outbox" /\ due)
          THEN /\ sent' = IF OnceKey THEN (IF sent = 0 THEN 1 ELSE sent) ELSE sent + 1
               /\ pc' = [pc EXCEPT ![e] = "routed"]
          ELSE /\ pc' = [pc EXCEPT ![e] = "done"]
               /\ UNCHANGED sent
    /\ UNCHANGED <<awards, due, fresh, crashes>>

\* the mark is cleared once the router has answered
Clear(e) ==
    /\ pc[e] = "routed"
    /\ due' = FALSE
    /\ pc' = [pc EXCEPT ![e] = "done"]
    /\ UNCHANGED <<awards, sent, fresh, crashes>>

\* the process dies after a commit, before the offer or before the clearing
Crash(e) ==
    /\ pc[e] \in {"committed", "routed"}
    /\ crashes < MaxCrash
    /\ crashes' = crashes + 1
    /\ pc' = [pc EXCEPT ![e] = "dead"]
    /\ UNCHANGED <<awards, sent, due, fresh>>

Quiescent == \A e \in Evals : pc[e] \in {"done", "dead"}

Finished == Quiescent /\ UNCHANGED vars

Next == (\E e \in Evals : Commit(e) \/ Offer(e) \/ Clear(e) \/ Crash(e)) \/ Finished

Spec == Init /\ [][Next]_vars

\* one row per key and tier, per record kind and day (R1, R3, R9 to R11)
AwardOnce == awards <= 1

\* at most one celebration per dedupe key
CelebrateAtMostOnce == sent <= 1

\* an award is celebrated, or its celebration is still due for a later evaluation: never dropped
NoSilentLoss == (Quiescent /\ awards > 0) => (sent > 0 \/ due)
=============================================================================
