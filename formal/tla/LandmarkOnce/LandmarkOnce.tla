---------------------------- MODULE LandmarkOnce ----------------------------
\* @phx covers crates/coordination/src/sync_cycle.rs anchor=sync_cycle digest=sha256:b8158a6a2359166e7b55a5dbbb15189d50cdbeebd84c20a0805067046c31dc6c
\* @phx covers crates/notifications/src/router.rs anchor=route digest=sha256:7bcf52fe22d886b3d71dfa0fa8e6dfb1d266a9a6b1662bada5482a2cd0c54dcb
\* @phx covers crates/notifications/src/ledger.rs anchor=claim digest=sha256:0116ef4925de04614d09ac18952c0a0b0f7248fd65f5d4f6ca555448836a6ab7
\* @phx cites #127, #571
\* @phx property FirstRunAtMostOne ramp=report
\* @phx property CelebrateAtMostOnce ramp=report
\* @phx property NoSilentLoss ramp=report
\* @phx witness witness/a-loser-of-the-seed-race-raises-every-due-landmark.cfg kills=FirstRunAtMostOne
\* @phx witness witness/a-first-run-whose-later-offer-raises-every-due-landmark.cfg kills=FirstRunAtMostOne
\* @phx witness witness/a-router-with-no-once-ever-key.cfg kills=CelebrateAtMostOnce
\* @phx witness witness/a-cursor-moved-before-the-router-answered.cfg kills=NoSilentLoss
\* @phx witness witness/a-first-run-that-moves-the-cursor-past-its-unraised-landmarks.cfg kills=NoSilentLoss
\* @phx witness witness/a-cursor-moved-to-a-settle-the-offer-never-read.cfg kills=NoSilentLoss
(***************************************************************************)
\* Four landmarks over four study days: k0 on day 1, k1 and k2 on day 2 (two on one day, k1 first
\* by the (day, key) order), k3 on day 3. The recomputes (runs) start on today = 2 with day 1
\* settled; the environment rolls today over and settles the day before it, between any two steps
\* of any run, since a concurrent recompute's fold may settle a day while another offers.
\*
\* Each action is ONE transaction of the code, so a Crash may fall between any two of them:
\* - Seed: the first offer call of a recompute opens one BEGIN IMMEDIATE write
\*   (notifications::landmark_settings::seed) that reads `landmark_high_water` and
\*   `landmarks_offered_through`, inserts the mark when it is absent (the predecessor's JSON, its
\*   bytes the golden `landmarks_run`'s) and the cursor X := yesterday when it is absent. The run is
\*   a FIRST run exactly when that read saw no mark.
\* - Call: each offer call (recompute/mod.rs::offer_owed, through LandmarkOffers::offer) reads X
\*   and the settle cursor S (analytics::rollup::settle_cursor) and owes, oldest first, every
\*   landmark dated in (X, S] or dated today, less the keys this recompute already had answered. A
\*   first run owes only the first landmark due today, on every call.
\* - Ask: one router call (Router's Celebrate port, router.rs::route in its own write). The switch
\*   off answers nudges_disabled before the claim (Refuse); the once-ever claim answers
\*   already_recorded (OnceKey); otherwise it claims and sends or holds (Keep: one delivery, the
\*   held row being HeldFlush's to send). An Err (Miss) may or may not have claimed and delivered.
\* - Advance: one write of its own that moves X, monotone (UPDATE ... WHERE stored < new), to the S
\*   the call read, or to the day before the oldest owed landmark the router did not answer. A
\*   first run never moves X.
\*
\* Design switches, each FALSE in the chosen design:
\* SplitSeed      the mark's read sits outside its insert, and the run is first only when its own
\*                insert won (ON CONFLICT DO NOTHING affected a row).
\* ForgetCap      a first run's later offer calls owe as a later run does.
\* OnceKey        the router's once-ever claim on (kind, dedupe_key, scope); FALSE claims nothing.
\* EarlyCursor    Advance moves X to S whatever the router answered.
\* FirstAdvances  a first run's Advance moves X too.
\* RereadS        Advance re-reads S in its own write instead of the S the call read.
(***************************************************************************)
EXTENDS Naturals, FiniteSets

CONSTANTS SplitSeed, ForgetCap, OnceKey, EarlyCursor, FirstAdvances, RereadS,
          MaxCrash, MaxMiss, MaxCalls

Runs == {1, 2}
Keys == {"k0", "k1", "k2", "k3"}
Day == [k \in Keys |-> CASE k = "k0" -> 1 [] k = "k1" -> 2 [] k = "k2" -> 2 [] k = "k3" -> 3]
Order == [k \in Keys |-> CASE k = "k0" -> 0 [] k = "k1" -> 1 [] k = "k2" -> 2 [] k = "k3" -> 3]
NDays == 4

Min(S) == CHOOSE x \in S : \A y \in S : x <= y

VARIABLES today, settled, mark, cursorSet, cursor, seedThrough, claimed, deliveries, answered,
          raisedFirst, crashes, misses,
          pc, rToday, first, sawNoMark, calls, readX, readS, owed, got, failed

vars == <<today, settled, mark, cursorSet, cursor, seedThrough, claimed, deliveries, answered,
          raisedFirst, crashes, misses,
          pc, rToday, first, sawNoMark, calls, readX, readS, owed, got, failed>>

global == <<today, settled, mark, cursorSet, cursor, seedThrough, claimed, deliveries, answered,
            raisedFirst, crashes, misses>>

PCs == {"idle", "seed", "seedWrite", "call", "ask", "done", "dead"}

TypeOK ==
    /\ today \in 2..NDays /\ settled \in 1..NDays
    /\ mark \in BOOLEAN /\ cursorSet \in BOOLEAN /\ cursor \in 0..NDays /\ seedThrough \in 0..NDays
    /\ claimed \subseteq Keys /\ answered \subseteq Keys /\ raisedFirst \subseteq Keys
    /\ deliveries \in [Keys -> Nat]
    /\ crashes \in 0..MaxCrash /\ misses \in 0..MaxMiss
    /\ pc \in [Runs -> PCs] /\ rToday \in [Runs -> 0..NDays]
    /\ first \in [Runs -> BOOLEAN] /\ sawNoMark \in [Runs -> BOOLEAN]
    /\ calls \in [Runs -> 0..MaxCalls]
    /\ readX \in [Runs -> 0..NDays] /\ readS \in [Runs -> 0..NDays]
    /\ owed \in [Runs -> SUBSET Keys] /\ got \in [Runs -> SUBSET Keys]
    /\ failed \in [Runs -> SUBSET Keys]

\* The predecessor's mark may be imported before the first run (SPEC-102 section 8, SPEC-141);
\* no cursor exists before part b's first run.
Init ==
    /\ today = 2 /\ settled = 1
    /\ mark \in BOOLEAN
    /\ cursorSet = FALSE /\ cursor = 0 /\ seedThrough = 0
    /\ claimed = {} /\ deliveries = [k \in Keys |-> 0] /\ answered = {}
    /\ raisedFirst = {} /\ crashes = 0 /\ misses = 0
    /\ pc = [r \in Runs |-> "idle"] /\ rToday = [r \in Runs |-> 0]
    /\ first = [r \in Runs |-> FALSE] /\ sawNoMark = [r \in Runs |-> FALSE]
    /\ calls = [r \in Runs |-> 0]
    /\ readX = [r \in Runs |-> 0] /\ readS = [r \in Runs |-> 0]
    /\ owed = [r \in Runs |-> {}] /\ got = [r \in Runs |-> {}] /\ failed = [r \in Runs |-> {}]

\* --- the environment: a day ends, and a fold settles the day before today ---------------------
Rollover ==
    /\ today < NDays
    /\ today' = today + 1
    /\ UNCHANGED <<settled, mark, cursorSet, cursor, seedThrough, claimed, deliveries, answered,
                   raisedFirst, crashes, misses, pc, rToday, first, sawNoMark, calls, readX, readS,
                   owed, got, failed>>

Settle ==
    /\ settled < today - 1
    /\ settled' = settled + 1
    /\ UNCHANGED <<today, mark, cursorSet, cursor, seedThrough, claimed, deliveries, answered,
                   raisedFirst, crashes, misses, pc, rToday, first, sawNoMark, calls, readX, readS,
                   owed, got, failed>>

\* --- a recompute ------------------------------------------------------------------------------
Start(r) ==
    /\ pc[r] = "idle"
    /\ pc' = [pc EXCEPT ![r] = "seed"]
    /\ rToday' = [rToday EXCEPT ![r] = today]
    /\ UNCHANGED <<global, first, sawNoMark, calls, readX, readS, owed, got, failed>>

\* The cursor is inserted only when absent: X := the run's yesterday, so every landmark dated
\* before today is history, as the predecessor's first run treats it.
SeedCursor(r) ==
    IF cursorSet
    THEN UNCHANGED <<cursorSet, cursor, seedThrough>>
    ELSE /\ cursorSet' = TRUE
         /\ cursor' = rToday[r] - 1
         /\ seedThrough' = rToday[r] - 1

\* The chosen design: one write reads the mark and inserts what is absent.
Seed(r) ==
    /\ ~SplitSeed
    /\ pc[r] = "seed"
    /\ sawNoMark' = [sawNoMark EXCEPT ![r] = ~mark]
    /\ first' = [first EXCEPT ![r] = ~mark]
    /\ mark' = TRUE
    /\ SeedCursor(r)
    /\ pc' = [pc EXCEPT ![r] = "call"]
    /\ UNCHANGED <<today, settled, claimed, deliveries, answered, raisedFirst, crashes, misses,
                   rToday, calls, readX, readS, owed, got, failed>>

\* SplitSeed: the read is a transaction of its own ...
SeedRead(r) ==
    /\ SplitSeed
    /\ pc[r] = "seed"
    /\ sawNoMark' = [sawNoMark EXCEPT ![r] = ~mark]
    /\ pc' = [pc EXCEPT ![r] = "seedWrite"]
    /\ UNCHANGED <<global, rToday, first, calls, readX, readS, owed, got, failed>>

\* ... and the run is first only when its own insert of the mark won.
SeedWrite(r) ==
    /\ SplitSeed
    /\ pc[r] = "seedWrite"
    /\ first' = [first EXCEPT ![r] = ~mark]
    /\ mark' = TRUE
    /\ SeedCursor(r)
    /\ pc' = [pc EXCEPT ![r] = "call"]
    /\ UNCHANGED <<today, settled, claimed, deliveries, answered, raisedFirst, crashes, misses,
                   rToday, sawNoMark, calls, readX, readS, owed, got, failed>>

FirstDue(d) == {k \in Keys : Day[k] = d /\ \A j \in Keys : Day[j] = d => Order[k] <= Order[j]}

Capped(r) == first[r] /\ ~(ForgetCap /\ calls[r] > 0)

Owed(r, x, s) ==
    IF Capped(r)
    THEN FirstDue(rToday[r]) \ got[r]
    ELSE {k \in Keys : (x < Day[k] /\ Day[k] <= s) \/ Day[k] = rToday[r]} \ got[r]

Call(r) ==
    /\ pc[r] = "call"
    /\ calls[r] < MaxCalls
    /\ readX' = [readX EXCEPT ![r] = cursor]
    /\ readS' = [readS EXCEPT ![r] = settled]
    /\ owed' = [owed EXCEPT ![r] = Owed(r, cursor, settled)]
    /\ calls' = [calls EXCEPT ![r] = @ + 1]
    /\ pc' = [pc EXCEPT ![r] = "ask"]
    /\ UNCHANGED <<global, rToday, first, sawNoMark, got, failed>>

Raise(r, k) == raisedFirst' = IF sawNoMark[r] THEN raisedFirst \cup {k} ELSE raisedFirst

Answered(r, k) ==
    /\ answered' = answered \cup {k}
    /\ got' = [got EXCEPT ![r] = @ \cup {k}]

\* The switch off: nudges_disabled, before the claim (router.rs::decide's kind switch).
Refuse(r, k) ==
    /\ pc[r] = "ask" /\ k \in owed[r] \ (got[r] \cup failed[r])
    /\ Raise(r, k) /\ Answered(r, k)
    /\ UNCHANGED <<today, settled, mark, cursorSet, cursor, seedThrough, claimed, deliveries,
                   crashes, misses, pc, rToday, first, sawNoMark, calls, readX, readS, owed, failed>>

\* The once-ever claim already held: already_recorded, no second delivery.
Recorded(r, k) ==
    /\ pc[r] = "ask" /\ k \in owed[r] \ (got[r] \cup failed[r])
    /\ OnceKey /\ k \in claimed
    /\ Raise(r, k) /\ Answered(r, k)
    /\ UNCHANGED <<today, settled, mark, cursorSet, cursor, seedThrough, claimed, deliveries,
                   crashes, misses, pc, rToday, first, sawNoMark, calls, readX, readS, owed, failed>>

\* Claimed and sent, or deferred and held for HeldFlush: one delivery either way.
Keep(r, k) ==
    /\ pc[r] = "ask" /\ k \in owed[r] \ (got[r] \cup failed[r])
    /\ ~(OnceKey /\ k \in claimed)
    /\ claimed' = IF OnceKey THEN claimed \cup {k} ELSE claimed
    /\ deliveries' = [deliveries EXCEPT ![k] = @ + 1]
    /\ Raise(r, k) /\ Answered(r, k)
    /\ UNCHANGED <<today, settled, mark, cursorSet, cursor, seedThrough, crashes, misses,
                   pc, rToday, first, sawNoMark, calls, readX, readS, owed, failed>>

\* An Err: the router may have claimed and delivered before it failed, and the caller sees none.
Miss(r, k) ==
    /\ pc[r] = "ask" /\ k \in owed[r] \ (got[r] \cup failed[r])
    /\ misses < MaxMiss
    /\ misses' = misses + 1
    /\ failed' = [failed EXCEPT ![r] = @ \cup {k}]
    /\ Raise(r, k)
    /\ \/ UNCHANGED <<claimed, deliveries>>
       \/ /\ ~(OnceKey /\ k \in claimed)
          /\ claimed' = IF OnceKey THEN claimed \cup {k} ELSE claimed
          /\ deliveries' = [deliveries EXCEPT ![k] = @ + 1]
    /\ UNCHANGED <<today, settled, mark, cursorSet, cursor, seedThrough, answered, crashes,
                   pc, rToday, first, sawNoMark, calls, readX, readS, owed, got>>

Target(r) ==
    LET s == IF RereadS THEN settled ELSE readS[r]
        unanswered == {k \in failed[r] : Day[k] <= s}
    IN IF EarlyCursor THEN s ELSE Min({s} \cup {Day[k] - 1 : k \in unanswered})

Advance(r) ==
    /\ pc[r] = "ask"
    /\ owed[r] \subseteq got[r] \cup failed[r]
    /\ cursor' = IF (~first[r] \/ FirstAdvances) /\ Target(r) > cursor THEN Target(r) ELSE cursor
    /\ failed' = [failed EXCEPT ![r] = {}]
    /\ pc' = [pc EXCEPT ![r] = "call"]
    /\ UNCHANGED <<today, settled, mark, cursorSet, seedThrough, claimed, deliveries, answered,
                   raisedFirst, crashes, misses, rToday, first, sawNoMark, calls, readX, readS,
                   owed, got>>

Finish(r) ==
    /\ pc[r] = "call"
    /\ pc' = [pc EXCEPT ![r] = "done"]
    /\ UNCHANGED <<global, rToday, first, sawNoMark, calls, readX, readS, owed, got, failed>>

Crash(r) ==
    /\ pc[r] \in {"seed", "seedWrite", "call", "ask"}
    /\ crashes < MaxCrash
    /\ crashes' = crashes + 1
    /\ pc' = [pc EXCEPT ![r] = "dead"]
    /\ UNCHANGED <<today, settled, mark, cursorSet, cursor, seedThrough, claimed, deliveries,
                   answered, raisedFirst, misses, rToday, first, sawNoMark, calls, readX, readS,
                   owed, got, failed>>

Quiescent == \A r \in Runs : pc[r] \in {"done", "dead"}

Finished == Quiescent /\ UNCHANGED vars

Next ==
    \/ Rollover \/ Settle
    \/ \E r \in Runs :
        \/ Start(r) \/ Seed(r) \/ SeedRead(r) \/ SeedWrite(r) \/ Call(r) \/ Advance(r)
        \/ Finish(r) \/ Crash(r)
        \/ \E k \in Keys : Refuse(r, k) \/ Recorded(r, k) \/ Keep(r, k) \/ Miss(r, k)
    \/ Finished

Spec == Init /\ [][Next]_vars

-----------------------------------------------------------------------------
\* SPEC-102 R6 and the predecessor's run_landmarks (`current = current[:1]` on the run that found
\* no `landmark_high_water`): the recomputes that saw no mark raise at most one landmark between
\* them, across every offer call.
FirstRunAtMostOne == Cardinality(raisedFirst) <= 1

\* ADR-303 and SPEC-102 R5: a landmark is celebrated at most once, whoever re-offers it.
CelebrateAtMostOnce == \A k \in Keys : deliveries[k] <= 1

\* The cursor passes a landmark only once the router answered it (sent, held or withheld, by a
\* decision row): every landmark dated after the seed and at or before X is answered.
NoSilentLoss ==
    \A k \in Keys : (cursorSet /\ seedThrough < Day[k] /\ Day[k] <= cursor) => k \in answered
=============================================================================
