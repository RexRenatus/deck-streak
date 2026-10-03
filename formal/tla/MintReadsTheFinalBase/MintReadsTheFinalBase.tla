------------------------- MODULE MintReadsTheFinalBase -------------------------
\* @phx covers crates/coordination/src/recompute/mod.rs anchor=Phase digest=sha256:f899318ac1f66e431ae82839be6e5165a0bede2d76122b5175e892af576cbfcb
\* @phx covers crates/coordination/src/recompute/mod.rs anchor=PHASES digest=sha256:7f21e3de566b544029e0c3ff78dd7940425c51120333fed2127384f698290be8
\* @phx covers crates/coordination/src/recompute/mod.rs anchor=register digest=sha256:c4ea8f4bef00e1f3b17ad8d80af338db958189a32fcd6f623cf8b18b71fb03a2
\* @phx covers crates/coordination/src/recompute/mod.rs anchor=run digest=sha256:7a383ba01872f4dde46ccb5220fa492de1f89498adf71774cae868e3e0b7acbb
\* @phx covers crates/economy/src/wallet.rs anchor=settle_mint_on digest=sha256:f527db16a0f325855b0278d2ffa535cd1ba5b2428a5b876138ce606fc4ee0633
\* @phx covers crates/progression/src/settle.rs anchor=settle digest=sha256:9872f0859a95fb7f19be47343abdc7227cdde7e7d0ea913a7372c7a402b47dbe
\* @phx covers crates/coordination/src/recompute/mint.rs anchor=phase digest=sha256:a0fb8d223f853cd70d7107472d703c985558145bb83d2203a6ff4063844156e1
\* @phx covers crates/coordination/src/recompute/mint.rs anchor=evaluate digest=sha256:27aadb0475e92829e5a94f133713d8ea336074638fb394d8b4a4932b1f7dc5e4
\* @phx cites #106
\* @phx property SettledMintEqualsItsFinalBase ramp=report
\* @phx property SettledDayMintNeverFalls ramp=report
\* @phx property CurrentMintFollowsItsBase ramp=report
\* @phx witness witness/a-mint-read-before-its-days-base-is-written.cfg kills=SettledMintEqualsItsFinalBase
\* @phx witness witness/a-mint-written-in-a-transaction-of-its-own.cfg kills=SettledMintEqualsItsFinalBase
\* @phx witness witness/a-backfill-that-writes-no-mint.cfg kills=SettledMintEqualsItsFinalBase
\* @phx witness witness/a-mint-from-the-cycles-own-reading.cfg kills=SettledDayMintNeverFalls
\* @phx witness witness/a-current-mint-read-before-its-base-is-written.cfg kills=CurrentMintFollowsItsBase
(***************************************************************************)
\* The fold's coin mint (#106, SPEC-082 R4): two cycles, the scheduled one and the owner's
\* recompute, each running SPEC-071's fold over one database while the study day turns over.
\*
\* Each write of mod.rs::run is ONE action, and a Crash may end a cycle between any two of them:
\* - Backfill: the write that reads the settle cursor and, when none is held, rolls every past day
\*   before the closing one up historically (SPEC-071 R17);
\* - Settle: one write per owed day, oldest first, which moves the cursor with it; the loop stops at
\*   the closing day, or at the first owed day the cycle's sync did not start after (StopSettling);
\* - Current: the current study day, open;
\* - Revisit: every past day backfilled or at or before the last settled one, closed, in one write.
\* db.rs::write opens each with BEGIN IMMEDIATE, so no two writes interleave inside one.
\* Re-read of mod.rs::run on 2026-10-02, against #311's fix (ADR-313): Settle's write reads the
\* cursor again first and, for a day it does not owe, writes nothing and goes on from the owed day
\* (Settle's first arm); the first write, the current day's write and the revisit are unchanged.
\*
\* Inside one write a day runs its steps in the phases' order (mod.rs::Phase, PHASES, register).
\* Phase 2 settles the day's base XP; phase 5 writes only consistency and Ascendant, the sources
\* the base leaves out (economy.json day_base_excludes), so the base is final once phase 4 ends and
\* the model folds phases 2 to 5 into the write's one base update (phase 3's relight grant, relight:<day>, is counted in the base; phases 4 and 5 move
\* no counted row). The awards' offers between the writes (mod.rs::offer_owed) mark celebrations only and write no base, held flag or mint, so they are
\* stutters. Phase 6, the mint (mint.rs::phase, CoinMint), reads
\* the base in the same write: mint.rs::evaluate sums the day's rows on the fold's write as phase 5
\* sums them, for every evaluation the fold makes, opens a savepoint on that write and no connection
\* of its own, and settles mint(base) through wallet.rs::settle_mint_on, closed unless the day is
\* the current one.
\*
\* base is a day's base XP as its rows hold it; held, whether those rows are held closed. A cycle's
\* reading of a day's reviews is any value: its snapshot was taken at some earlier time, and reviews
\* arrive late for past days and can be removed from any day. Mint(b) stands for E1's rule, the
\* min of the cap and b divided by the divisor, which its golden proves; the model asks only which
\* base the mint reads, so the divisor and the cap are small.
\* The floor's room in settle_mint_on's open lowering never binds here: E1b writes no debit, so
\* every movement is a mint and the balance covers any held mint (WalletFloor's FloorHolds owns
\* the floor).
\*
\* MintAt places the read: "phase6" is the built step; "phase1" a step registered before the base
\* is written; "job" a mint in a write of its own after the fold (ADR's rejected design b);
\* "reading" a mint taken from the cycle's own reading of the reviews instead of the day's rows.
\* MintsBackfill = FALSE is a mint step that skips the backfill's evaluation.
(***************************************************************************)
EXTENDS Integers, FiniteSets

CONSTANTS MintAt, MintsBackfill, NDays, MaxBase, Divisor, Cap, Runs

Cycles == {"scheduled", "owner"}
Days == 1..NDays
Bases == 0..MaxBase
Places == {"done", "w1", "settle", "current", "revisit", "job"}

Max(a, b) == IF a >= b THEN a ELSE b
\* economy rules.rs::mint_for_base_xp, at a small divisor and cap
Mint(b) == IF b \div Divisor < Cap THEN b \div Divisor ELSE Cap

VARIABLES today, base, held, mint, cursor, fell, pc, now, cur, bf, owed, last, runs

vars == <<today, base, held, mint, cursor, fell, pc, now, cur, bf, owed, last, runs>>

TypeOK ==
    /\ today \in Days
    /\ base \in [Days -> Bases]
    /\ held \in [Days -> BOOLEAN]
    /\ mint \in [Days -> 0..Cap]
    /\ cursor \in 0..NDays
    /\ fell \in BOOLEAN
    /\ pc \in [Cycles -> Places]
    /\ now \in [Cycles -> Days]
    /\ cur \in [Cycles -> 0..NDays]
    /\ bf \in [Cycles -> BOOLEAN]
    /\ owed \in [Cycles -> 1..NDays + 1]
    /\ last \in [Cycles -> 0..NDays]
    /\ runs \in [Cycles -> 0..Runs]

Init ==
    /\ today = NDays - 1
    /\ base = [d \in Days |-> 0]
    /\ held = [d \in Days |-> FALSE]
    /\ mint = [d \in Days |-> 0]
    /\ cursor = 0
    /\ fell = FALSE
    /\ pc = [c \in Cycles |-> "done"]
    /\ now = [c \in Cycles |-> NDays - 1]
    /\ cur = [c \in Cycles |-> 0]
    /\ bf = [c \in Cycles |-> FALSE]
    /\ owed = [c \in Cycles |-> 1]
    /\ last = [c \in Cycles |-> 0]
    /\ runs = [c \in Cycles |-> 0]

\* progression settle.rs::settle under the recompute's cause: a row held closed, or settled closed,
\* is only raised (held.amount.max(request.amount)); an open row is replaced
\* Re-read of settle.rs::settle for SPEC-078 (2026-10-03): its registry test moved from the nine
\* names to settle.rs::is_derived, which also admits `read:<code>` and `readgoal:<code>` for a
\* valid course code, and still refuses before any write. It moves none of base, held or mint on
\* any path: a stuttering step. SPEC-078's habit step (recompute/habits.rs, phase 4) settles those
\* counted sources inside the same write, before phase 6, so it is part of the write's one base
\* update above.
Settled(d, closed, r) == IF held[d] \/ closed THEN Max(base[d], r) ELSE r

\* wallet.rs::settle_mint_on: a closed settle, or a raise, keeps held.max(amount); an open lowering
\* follows the amount
MintSettled(d, closed, a) == IF closed \/ a >= mint[d] THEN Max(mint[d], a) ELSE a

\* the amount the mint step settles, by where it reads the base
MintRead(d, closed, r) ==
    CASE MintAt = "phase6" -> Mint(Settled(d, closed, r))
      [] MintAt = "phase1" -> Mint(base[d])
      [] MintAt = "reading" -> Mint(r)
      [] OTHER -> 0

\* the day's steps for every day of D, read as r, inside one write
Evaluate(D, closed, r, mints) ==
    /\ base' = [d \in Days |-> IF d \in D THEN Settled(d, closed, r[d]) ELSE base[d]]
    /\ held' = [d \in Days |-> held[d] \/ (d \in D /\ closed)]
    /\ mint' = [d \in Days |->
                  IF d \in D /\ mints THEN MintSettled(d, closed, MintRead(d, closed, r[d]))
                  ELSE mint[d]]
    /\ fell' = (fell \/ \E d \in Days : held[d] /\ mint'[d] < mint[d])

FoldMints == MintAt # "job"

Start(c) ==
    /\ pc[c] = "done"
    /\ runs[c] < Runs
    /\ pc' = [pc EXCEPT ![c] = "w1"]
    /\ now' = [now EXCEPT ![c] = today]
    /\ last' = [last EXCEPT ![c] = 0]
    /\ runs' = [runs EXCEPT ![c] = @ + 1]
    /\ UNCHANGED <<today, base, held, mint, cursor, fell, cur, bf, owed>>

\* mod.rs::run (1): the write that reads the cursor; with none held, the first recompute rolls
\* every past day before the closing one up, closed
Backfill(c) ==
    /\ pc[c] = "w1"
    /\ LET closing == now[c] - 1
           D == IF cursor = 0 THEN {d \in Days : d < closing} ELSE {}
       IN /\ \E r \in [D -> Bases] : Evaluate(D, TRUE, r, FoldMints /\ MintsBackfill)
          /\ owed' = [owed EXCEPT ![c] = IF cursor = 0 THEN closing ELSE cursor + 1]
    /\ cur' = [cur EXCEPT ![c] = cursor]
    /\ bf' = [bf EXCEPT ![c] = (cursor = 0)]
    /\ pc' = [pc EXCEPT ![c] = "settle"]
    /\ UNCHANGED <<today, cursor, now, last, runs>>

\* mod.rs::run (2): one owed day, closed, in a write of its own that marks it settled; the cursor
\* is the latest settled day (rollup.rs settle_cursor reads max(study_day)). The write reads the
\* cursor again first (#311, ADR-313): the day owed is the day after it or, with no cursor, the
\* cycle's own day; a write for any other day is rolled back before any step runs, so it moves no
\* base, held flag, mint or cursor, and the loop goes on from the owed day
Settle(c) ==
    /\ pc[c] = "settle"
    /\ owed[c] <= now[c] - 1
    /\ LET d == owed[c]
           due == IF cursor = 0 THEN d ELSE cursor + 1 IN
       IF d # due
          THEN /\ owed' = [owed EXCEPT ![c] = due]
               /\ UNCHANGED <<base, held, mint, cursor, fell, last>>
          ELSE /\ \E r \in [{d} -> Bases] : Evaluate({d}, TRUE, r, FoldMints)
               /\ cursor' = Max(cursor, d)
               /\ last' = [last EXCEPT ![c] = d]
               /\ owed' = [owed EXCEPT ![c] = d + 1]
    /\ UNCHANGED <<today, pc, now, cur, bf, runs>>

\* the settle loop ends at the closing day, or at the first owed day the cycle's sync did not start
\* after: no write
StopSettling(c) ==
    /\ pc[c] = "settle"
    /\ pc' = [pc EXCEPT ![c] = "current"]
    /\ UNCHANGED <<today, base, held, mint, cursor, fell, now, cur, bf, owed, last, runs>>

\* mod.rs::run (3): the cycle's current study day, open
Current(c) ==
    /\ pc[c] = "current"
    /\ \E r \in [{now[c]} -> Bases] : Evaluate({now[c]}, FALSE, r, FoldMints)
    /\ pc' = [pc EXCEPT ![c] = "revisit"]
    /\ UNCHANGED <<today, cursor, now, cur, bf, owed, last, runs>>

\* mod.rs::run (4): every past day the cycle backfilled, or at or before the last day it settled
\* (else the cursor it read), closed, in one write
Revisit(c) ==
    /\ pc[c] = "revisit"
    /\ LET through == IF last[c] > 0 THEN last[c] ELSE cur[c]
           D == {d \in Days : d < now[c] /\ ((bf[c] /\ d < now[c] - 1) \/ d <= through)}
       IN \E r \in [D -> Bases] : Evaluate(D, TRUE, r, FoldMints)
    /\ pc' = [pc EXCEPT ![c] = IF MintAt = "job" THEN "job" ELSE "done"]
    /\ UNCHANGED <<today, cursor, now, cur, bf, owed, last, runs>>

\* the rejected design: a mint of its own after the fold, one write over the cycle's days, each
\* read from the base its rows hold, closed before the cycle's current day
MintJob(c) ==
    /\ pc[c] = "job"
    /\ mint' = [d \in Days |->
                  IF d <= now[c] THEN MintSettled(d, d < now[c], Mint(base[d])) ELSE mint[d]]
    /\ fell' = (fell \/ \E d \in Days : held[d] /\ mint'[d] < mint[d])
    /\ pc' = [pc EXCEPT ![c] = "done"]
    /\ UNCHANGED <<today, base, held, cursor, now, cur, bf, owed, last, runs>>

\* a cycle ends between two of its writes: what it committed stays
Crash(c) ==
    /\ pc[c] \in Places \ {"done"}
    /\ pc' = [pc EXCEPT ![c] = "done"]
    /\ UNCHANGED <<today, base, held, mint, cursor, fell, now, cur, bf, owed, last, runs>>

\* the study day turns over
Rollover ==
    /\ today < NDays
    /\ today' = today + 1
    /\ UNCHANGED <<base, held, mint, cursor, fell, pc, now, cur, bf, owed, last, runs>>

\* every run of both cycles has ended on the last day
Finished ==
    /\ today = NDays
    /\ \A c \in Cycles : pc[c] = "done" /\ runs[c] = Runs
    /\ UNCHANGED vars

Next ==
    \/ \E c \in Cycles :
          \/ Start(c)
          \/ Backfill(c)
          \/ Settle(c)
          \/ StopSettling(c)
          \/ Current(c)
          \/ Revisit(c)
          \/ MintJob(c)
          \/ Crash(c)
    \/ Rollover
    \/ Finished

Spec == Init /\ [][Next]_vars

\* SPEC-082 R4 and A7: every day the fold has written closed (backfilled, settled or revisited) holds
\* the mint of the base its closed rows hold, so the mint follows each raise of its final base
SettledMintEqualsItsFinalBase == \A d \in Days : held[d] => mint[d] = Mint(base[d])

\* SPEC-082 R4: a settled day's mint is raised by a later recompute and never lowered, whichever of
\* the two cycles writes it last (composes with WalletFloor's SettledMintNeverFalls)
SettledDayMintNeverFalls == ~fell

\* SPEC-082 R4: the current study day's mint follows its base at each recompute
CurrentMintFollowsItsBase == mint[today] = Mint(base[today])
================================================================================
