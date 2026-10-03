---------------------------- MODULE BandUpOnce ----------------------------
\* @phx covers crates/coordination/src/recompute/progress.rs anchor=record_progress digest=sha256:12f49fc092311f2dc7d07dcac4f0dd424ead54bb691054e029621c8782c6028e
\* @phx covers crates/coordination/src/recompute/progress.rs anchor=offer_band_ups digest=sha256:7aba5da075cd328a987fb868ee8a25d829cbb3d8f51769a10c14a69951cb95e3
\* @phx covers crates/coordination/src/recompute/mod.rs anchor=offer_owed digest=sha256:0d418cb9fe6912d5780f32d0e837891e9927590bd6cc55b3aaf883ebfdf5bc00
\* @phx covers crates/curriculum/src/store.rs anchor=record_milestone digest=sha256:fc191ed6df00f2838ce98ea85dd80683bab83c687d3a083360d1c70414273107
\* @phx covers crates/curriculum/src/store.rs anchor=owed_band_ups digest=sha256:3402c6e98c23f6013560849496e5383ae6129b6a69a8978f1c35276149ae969c
\* @phx covers crates/curriculum/src/store.rs anchor=mark_band_up digest=sha256:ef165d6add20fa3d3bd43dd208bf4a92e1084aa3542662035d4f4c40ccd207a1
\* @phx covers crates/notifications/src/ledger.rs anchor=claim digest=sha256:0116ef4925de04614d09ac18952c0a0b0f7248fd65f5d4f6ca555448836a6ab7
\* @phx cites #85
\* @phx property BandUpOnce ramp=report
\* @phx property CelebrateAtMostOnce ramp=report
\* @phx property MarkedOnlyWhenAnswered ramp=report
\* @phx property AnsweredIsMarked ramp=report
\* @phx witness witness/a-band-milestone-with-no-unique-key.cfg kills=BandUpOnce
\* @phx witness witness/a-router-with-no-once-ever-key.cfg kills=CelebrateAtMostOnce
\* @phx witness witness/a-band-up-marked-before-the-router-answers.cfg kills=MarkedOnlyWhenAnswered
\* @phx witness witness/an-answered-band-up-left-unmarked.cfg kills=AnsweredIsMarked
(***************************************************************************)
\* One course and one band it reaches, and the recomputes that may reach it. Each recompute runs
\* the fold's offers before its write, its write, then the offers after it; any two recomputes may
\* interleave, which covers a recompute and its replay after a crash as well as two that overlap.
\*
\* Each action is ONE transaction of the code, so a Crash may fall between any two of them:
\* - the day's write (Write) is one BEGIN IMMEDIATE: progress.rs::record_progress compares the
\*   course's current band with the stored one and, for a band-up, writes the milestone with its
\*   mark unset (store.rs::record_milestone, INSERT ... ON CONFLICT DO NOTHING on the key of the
\*   course and the band) and pays its XP in the same write only when the row is new;
\* - the offers (Read, Route, Miss, Mark) run between the fold's writes (mod.rs::offer_owed, through
\*   AwardOffers::offer's third arm, progress.rs::offer_band_ups): they read the owed band-ups on a
\*   reader (store.rs::owed_band_ups, baseline 0 and mark unset), hand each to the router, whose
\*   once-ever dedupe key is ledger.rs::claim, and set the mark in a write of their own only once
\*   the router has answered (store.rs::mark_band_up, UPDATE ... AND celebrated_at IS NULL);
\* - a router call may come back with no answer (an error): it may or may not have sent, and the
\*   band-up is not marked (Miss, at most MaxMiss of them).
\*
\* The switches are the designs a property rules out: UniqueKey FALSE is a milestone with no key on
\* the course and the band; OnceKey FALSE a router with no once-ever key; MarkFirst TRUE a mark
\* written before the router is called; MarkWrite FALSE an offer whose answered band-up is never
\* marked.
\*
\* Abstractions, each a stuttering of the model's variables or a superset of the code's behaviours:
\* - every write may record the band: in the code a later recompute records it again only when the
\*   course dropped below it and climbed back (the golden's reached_again case), and the key is what
\*   holds it to one row then; letting every write try is a superset;
\* - a first sighting writes a silent baseline already marked, which owes nothing and is never read
\*   as owed: it moves none of these variables;
\* - the band badge (phase 7) is awarded marked in the day's write, and its own offers are AwardOnce's;
\* - two courses or two bands share no row, mark or dedupe key, so one course and one band stand for
\*   every pair.
(***************************************************************************)
EXTENDS Naturals

CONSTANTS NEvals, UniqueKey, OnceKey, MarkFirst, MarkWrite, MaxCrash, MaxMiss

Evals == 1..NEvals

VARIABLES recorded, paid, marked, sent, answered, pc, ph, ok, crashes, misses

vars == <<recorded, paid, marked, sent, answered, pc, ph, ok, crashes, misses>>

TypeOK ==
    /\ recorded \in 0..NEvals
    /\ paid \in 0..NEvals
    /\ marked \in BOOLEAN
    /\ sent \in 0..(2 * NEvals)
    /\ answered \in BOOLEAN
    /\ pc \in [Evals -> {"read", "route", "mark", "write", "done", "dead"}]
    /\ ph \in [Evals -> {1, 2}]
    /\ ok \in [Evals -> BOOLEAN]
    /\ crashes \in 0..MaxCrash
    /\ misses \in 0..MaxMiss

Init ==
    /\ recorded = 0
    /\ paid = 0
    /\ marked = FALSE
    /\ sent = 0
    /\ answered = FALSE
    /\ pc = [e \in Evals |-> "read"]
    /\ ph = [e \in Evals |-> 1]
    /\ ok = [e \in Evals |-> FALSE]
    /\ crashes = 0
    /\ misses = 0

\* the band-up is recorded and its mark is unset: what store.rs::owed_band_ups reads
Owed == recorded > 0 /\ ~marked

\* the router's claim of the once-ever key (ledger.rs::claim): a second offer of a key collapses
Router == IF OnceKey THEN (IF sent = 0 THEN 1 ELSE sent) ELSE sent + 1

\* where a recompute goes once its offers are over: its write, after the offers before it, or done
AfterOffers(e) == IF ph[e] = 1 THEN "write" ELSE "done"

\* the offers read the owed band-ups on a reader
Read(e) ==
    /\ pc[e] = "read"
    /\ pc' = [pc EXCEPT ![e] = IF Owed THEN (IF MarkFirst THEN "mark" ELSE "route") ELSE AfterOffers(e)]
    /\ UNCHANGED <<recorded, paid, marked, sent, answered, ph, ok, crashes, misses>>

\* the router answers the offer
Route(e) ==
    /\ pc[e] = "route"
    /\ sent' = Router
    /\ answered' = TRUE
    /\ ok' = [ok EXCEPT ![e] = TRUE]
    /\ pc' = [pc EXCEPT ![e] = IF MarkFirst THEN AfterOffers(e) ELSE "mark"]
    /\ UNCHANGED <<recorded, paid, marked, ph, crashes, misses>>

\* the router comes back with no answer: it may or may not have sent, and nothing is marked
Miss(e) ==
    /\ pc[e] = "route"
    /\ misses < MaxMiss
    /\ misses' = misses + 1
    /\ \E s \in {sent, Router} : sent' = s
    /\ pc' = [pc EXCEPT ![e] = AfterOffers(e)]
    /\ UNCHANGED <<recorded, paid, marked, answered, ph, ok, crashes>>

\* the mark's own write: UPDATE ... AND celebrated_at IS NULL, so a mark set already stays set
Mark(e) ==
    /\ pc[e] = "mark"
    /\ marked' = (marked \/ MarkWrite)
    /\ pc' = [pc EXCEPT ![e] = IF MarkFirst THEN "route" ELSE AfterOffers(e)]
    /\ UNCHANGED <<recorded, paid, sent, answered, ph, ok, crashes, misses>>

\* the day's write: the milestone with its mark unset and its XP, once per key, in one transaction
Write(e) ==
    /\ pc[e] = "write"
    /\ IF UniqueKey /\ recorded > 0
          THEN UNCHANGED <<recorded, paid, marked>>
          ELSE /\ recorded' = recorded + 1
               /\ paid' = paid + 1
               /\ marked' = FALSE
    /\ pc' = [pc EXCEPT ![e] = "read"]
    /\ ph' = [ph EXCEPT ![e] = 2]
    /\ UNCHANGED <<sent, answered, ok, crashes, misses>>

\* the process dies between any two transactions
Crash(e) ==
    /\ pc[e] \in {"read", "route", "mark", "write"}
    /\ crashes < MaxCrash
    /\ crashes' = crashes + 1
    /\ pc' = [pc EXCEPT ![e] = "dead"]
    /\ UNCHANGED <<recorded, paid, marked, sent, answered, ph, ok, misses>>

Quiescent == \A e \in Evals : pc[e] \in {"done", "dead"}

Finished == Quiescent /\ UNCHANGED vars

Next ==
    \/ \E e \in Evals :
          \/ Read(e) \/ Route(e) \/ Miss(e) \/ Mark(e) \/ Write(e) \/ Crash(e)
    \/ Finished

Spec == Init /\ [][Next]_vars

\* one milestone row and one XP grant per course and band (SPEC-077 R7; the migration's key)
BandUpOnce == recorded <= 1 /\ paid <= 1

\* at most one celebration per course and band (SPEC-077 R7; the router's once-ever key)
CelebrateAtMostOnce == sent <= 1

\* a band-up's mark is set only after the router has answered its offer (ADR-303; offer_band_ups)
MarkedOnlyWhenAnswered == marked => answered

\* a recompute that ends after the router answered its offer leaves the band-up marked
AnsweredIsMarked == \A e \in Evals : (pc[e] = "done" /\ ok[e]) => marked
=============================================================================
