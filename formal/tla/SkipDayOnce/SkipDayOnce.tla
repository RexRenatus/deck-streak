---------------------------- MODULE SkipDayOnce ----------------------------
\* @phx covers crates/ingest/src/skip.rs anchor=begin digest=sha256:0363a7b54992969173ddb6f01a37173e803066bad4bb30479179a720777e12b5
\* @phx covers crates/ingest/src/skip.rs anchor=latest_undoable digest=sha256:f6d4e84bbfdb88a6894172e053242c37cd861cf263d2c158f2feaeccf5a9a2c9
\* @phx covers crates/ingest/src/skip.rs anchor=records_on digest=sha256:53306e5a71d58ec3f2d16252a54c5fe04c462f628a6ec66d5df735f702028d2d
\* @phx covers crates/ingest/src/skip.rs anchor=settle_applied_on digest=sha256:96b6af6dd6de41484246f2f3941696426b58216e05bc52fb6aaa37312fea3de4
\* @phx covers crates/ingest/src/skip.rs anchor=mark_undone_on digest=sha256:e0b7e7f4960a51ec999bad57b0697441235c38383fdf5bbdbcf267df67dd658a
\* @phx covers crates/coordination/src/skip/mod.rs anchor=settle_applied digest=sha256:7df3e27a07ba3c716aef05c9eb92ce04d5e1b727eaf8543da0f6eb0c41445faa
\* @phx covers crates/coordination/src/skip/mod.rs anchor=settle_undone digest=sha256:61aa16597ae2e193cf7a29d4606a0def0d356b2d28672b102eadb08705dbb350
\* @phx cites #108
\* @phx property AtMostOneActivePerDay ramp=report
\* @phx property ChargedOnlyWhenApplied ramp=report
\* @phx property ChargedAtMostOncePerSkip ramp=report
\* @phx property RefundOnlyWhenUndoneAtMostOnce ramp=report
\* @phx property UndoRefusesWhilePending ramp=report
\* @phx witness witness/a-day-key-read-outside-the-write.cfg kills=AtMostOneActivePerDay
\* @phx witness witness/a-charge-taken-before-the-outcome.cfg kills=ChargedOnlyWhenApplied
\* @phx witness witness/a-charge-keyed-to-the-settlements-own-day.cfg kills=ChargedAtMostOncePerSkip
\* @phx witness witness/a-refund-keyed-to-the-retrys-own-day.cfg kills=RefundOnlyWhenUndoneAtMostOnce
\* @phx witness witness/an-undo-that-reads-no-pending-take.cfg kills=UndoRefusesWhilePending
(***************************************************************************)
\* The skip day's record, its charge and its refund (#108, SPEC-083 R2, R5, R8, R9 and section 10's
\* T3 and T4): the confirms (the bot's and the API's, NConfirms of them in all), the take's own
\* settlement, the start-up settlement and the undo, over one skip_days table and the coin ledger's
\* two skip sources, across study days the clock moves through.
\*
\* Each write of the code is ONE transaction through the kernel's BEGIN IMMEDIATE, which takes
\* SQLite's write lock at its first statement, so the model takes each as one atomic step:
\* - a confirm is skip.rs::begin: INSERT ... ON CONFLICT on the migration's partial key, one skip a
\*   study day `pending` or `applied` and not undone, so the day's key is read inside the write;
\* - a settlement is skip/mod.rs::settle_applied: in one write it reads the row, takes the
\*   floor-clipped debit keyed (the skip's own study day, skip_tariff, the skip's id) unless the row
\*   is failed or undone, and settles a pending row applied. A retry reaches the same key, and the
\*   ledger's unique key writes nothing on that conflict (wallet.rs::insert);
\* - the undo's settlement is skip/mod.rs::settle_undone: in one write it marks the skip undone at
\*   the undo's instant when it is applied and not undone, reads what the skip paid, and credits it
\*   keyed (the study day the row records the undo on, skip_tariff_refund, the skip's id).
\* The undo's decision is skip.rs::latest_undoable: on the reader, it refuses while the study day's
\* take is pending, else picks the latest applied skip not undone by (study day, id). It is a read
\* outside any write, so the model takes it as its own step before the undo's settlement.
\*
\* Abstractions. A take's write outcome (E4b) is a fate drawn at the confirm: the take's own
\* settlement and the start-up settlement both settle the row by it, and either may also never run
\* (R25 leaves a row pending). Each settler runs at most once a row, so a row sees at most two
\* settlements, on any study day. The undo's settlement runs once and may be retried once. Every
\* price is positive and every debit pays something: a price of 0 asks the wallet for 0, which
\* writes no movement, and a debit the empty wallet clips to 0 is refunded as 0, so both only
\* remove movements, and every property bounds movements. The price itself and the amount paid
\* are lean/SkipTariff's.
\*
\* Switches: each is TRUE in the code as built, and its FALSE arm is the defect its witness names.
(***************************************************************************)
EXTENDS Integers, FiniteSets

CONSTANTS KeyInWrite, ChargeAfterOutcome, ChargeOnSkipDay, RefundOnUndoDay, UndoReadsPending,
          NConfirms, NDays

Confirms == 1..NConfirms
Days == 1..NDays
\* one row a confirm at most, so the ids are the confirms' count
Ids == 1..NConfirms
Settlers == {"take", "startup"}
Fates == {"applied", "failed"}
States == {"pending", "applied", "failed"}

RowType == [day : Days, state : States, undone : BOOLEAN, undoneDay : 0..NDays, fate : Fates]
NoRow == [day |-> 0, state |-> "none", undone |-> FALSE, undoneDay |-> 0, fate |-> "none"]

VARIABLES today, rows, nrows, charged, refunded, cpc, cday, settled, upc, target, pastPending

vars == <<today, rows, nrows, charged, refunded, cpc, cday, settled, upc, target, pastPending>>

Exists(i) == i <= nrows

\* skip.rs::SkipRecord::covers_its_day and the migration's key: pending or applied, not undone
Active(i) == Exists(i) /\ rows[i].state \in {"pending", "applied"} /\ ~rows[i].undone

ActiveOn(d) == {i \in Ids : Active(i) /\ rows[i].day = d}

PendingOn(d) == \E i \in Ids : Active(i) /\ rows[i].day = d /\ rows[i].state = "pending"

\* the coin ledger's movements of one skip, by its reference
Charges(i) == {k \in charged : k[2] = i}
Refunds(i) == {k \in refunded : k[2] = i}

TypeOK ==
    /\ today \in Days
    /\ nrows \in 0..NConfirms
    /\ rows \in [Ids -> RowType \cup {NoRow}]
    /\ charged \subseteq Days \X Ids
    /\ refunded \subseteq Days \X Ids
    /\ cpc \in [Confirms -> {"idle", "checked", "done"}]
    /\ cday \in [Confirms -> 0..NDays]
    /\ settled \in [Settlers \X Ids -> BOOLEAN]
    /\ upc \in {"idle", "decided", "settled", "done"}
    /\ target \in 0..NConfirms
    /\ pastPending \in BOOLEAN

Init ==
    /\ today = 1
    /\ rows = [i \in Ids |-> NoRow]
    /\ nrows = 0
    /\ charged = {}
    /\ refunded = {}
    /\ cpc = [c \in Confirms |-> "idle"]
    /\ cday = [c \in Confirms |-> 0]
    /\ settled = [x \in Settlers \X Ids |-> FALSE]
    /\ upc = "idle"
    /\ target = 0
    /\ pastPending = FALSE

\* skip.rs::begin writes a pending row on day d. The defect ChargeAfterOutcome = FALSE takes the
\* tariff here too, before the take's outcome is known.
Insert(d, f) ==
    LET i == nrows + 1
    IN /\ nrows' = i
       /\ rows' = [rows EXCEPT ![i] = [day |-> d, state |-> "pending", undone |-> FALSE,
                                       undoneDay |-> 0, fate |-> f]]
       /\ charged' = IF ChargeAfterOutcome THEN charged ELSE charged \cup {<<d, i>>}

\* skip.rs::begin as built: the day's key read and the insert in one write
Confirm(c) ==
    /\ KeyInWrite
    /\ cpc[c] = "idle"
    /\ cpc' = [cpc EXCEPT ![c] = "done"]
    /\ cday' = [cday EXCEPT ![c] = today]
    /\ IF ActiveOn(today) = {}
       THEN \E f \in Fates : Insert(today, f)
       ELSE UNCHANGED <<rows, nrows, charged>>
    /\ UNCHANGED <<today, refunded, settled, upc, target, pastPending>>

\* the defect KeyInWrite = FALSE: the day's key read on the reader, outside the write ...
Check(c) ==
    /\ ~KeyInWrite
    /\ cpc[c] = "idle"
    /\ cday' = [cday EXCEPT ![c] = today]
    /\ cpc' = [cpc EXCEPT ![c] = IF ActiveOn(today) = {} THEN "checked" ELSE "done"]
    /\ UNCHANGED <<today, rows, nrows, charged, refunded, settled, upc, target, pastPending>>

\* ... and the insert, in a later write that trusts it
InsertChecked(c) ==
    /\ cpc[c] = "checked"
    /\ cpc' = [cpc EXCEPT ![c] = "done"]
    /\ \E f \in Fates : Insert(cday[c], f)
    /\ UNCHANGED <<today, refunded, cday, settled, upc, target, pastPending>>

\* T3 and R-f: the charge is keyed to the skip's own study day; the defect keys it to the day the
\* settlement runs
ChargeDay(r) == IF ChargeOnSkipDay THEN r.day ELSE today

\* skip/mod.rs::settle_applied, by the take's own task or by the start-up settlement, once each
Settle(s, i) ==
    /\ Exists(i)
    /\ ~settled[<<s, i>>]
    /\ settled' = [settled EXCEPT ![<<s, i>>] = TRUE]
    /\ LET r == rows[i]
       IN IF r.fate = "failed"
          THEN /\ rows' = IF r.state = "pending" THEN [rows EXCEPT ![i].state = "failed"] ELSE rows
               /\ UNCHANGED charged
          ELSE IF r.state = "failed" \/ r.undone
               THEN UNCHANGED <<rows, charged>>
               ELSE /\ charged' = charged \cup {<<ChargeDay(r), i>>}
                    /\ rows' = [rows EXCEPT ![i].state = "applied"]
    /\ UNCHANGED <<today, nrows, refunded, cpc, cday, upc, target, pastPending>>

\* the latest applied skip not undone, by (study day, id)
Latest(S) == CHOOSE i \in S : \A j \in S : rows[j].day < rows[i].day \/ (rows[j].day = rows[i].day /\ j <= i)

\* skip.rs::latest_undoable on the reader: refuses while the study day's take is pending; the
\* defect UndoReadsPending = FALSE never reads that row. pastPending is history: an undo went
\* on while its study day's take was pending.
Decide ==
    /\ upc = "idle"
    /\ LET pending == PendingOn(today)
           cands == {i \in Ids : Active(i) /\ rows[i].state = "applied"}
       IN IF (UndoReadsPending /\ pending) \/ cands = {}
          THEN /\ upc' = "done"
               /\ UNCHANGED <<target, pastPending>>
          ELSE /\ target' = Latest(cands)
               /\ upc' = "decided"
               /\ pastPending' = (pastPending \/ pending)
    /\ UNCHANGED <<today, rows, nrows, charged, refunded, cpc, cday, settled>>

\* T4 and R-f: the refund is keyed to the study day the row records the undo on; the defect keys it
\* to the day the settlement runs
RefundDay(r) == IF RefundOnUndoDay THEN r.undoneDay ELSE today

\* skip/mod.rs::settle_undone: once, then once more as a retry, each on any study day
SettleUndo ==
    /\ upc \in {"decided", "settled"}
    /\ upc' = IF upc = "decided" THEN "settled" ELSE "done"
    /\ LET i == target
           r == rows[i]
           marked == r.state = "applied" /\ ~r.undone
           r2 == IF marked THEN [r EXCEPT !.undone = TRUE, !.undoneDay = today] ELSE r
       IN /\ rows' = [rows EXCEPT ![i] = r2]
          /\ refunded' = IF r2.undone /\ Charges(i) # {}
                         THEN refunded \cup {<<RefundDay(r2), i>>}
                         ELSE refunded
    /\ UNCHANGED <<today, nrows, charged, cpc, cday, settled, target, pastPending>>

\* the clock moves to the next study day
Tick ==
    /\ today < NDays
    /\ today' = today + 1
    /\ UNCHANGED <<rows, nrows, charged, refunded, cpc, cday, settled, upc, target, pastPending>>

\* every actor has run, every row is settled by both settlers, and the clock is at its last day
Finished ==
    /\ today = NDays
    /\ \A c \in Confirms : cpc[c] = "done"
    /\ upc = "done"
    /\ \A s \in Settlers : \A i \in Ids : Exists(i) => settled[<<s, i>>]
    /\ UNCHANGED vars

Next ==
    \/ \E c \in Confirms : Confirm(c) \/ Check(c) \/ InsertChecked(c)
    \/ \E s \in Settlers, i \in Ids : Settle(s, i)
    \/ Decide
    \/ SettleUndo
    \/ Tick
    \/ Finished

Spec == Init /\ [][Next]_vars

\* R2 and the migration's key (008301): "one skip a study day that is `pending` or `applied` and not
\* undone ... two concurrent takes write one row whatever the callers do"
AtMostOneActivePerDay == \A d \in Days : Cardinality(ActiveOn(d)) <= 1

\* R8: the charge is "made only when the take's write is `applied`"
ChargedOnlyWhenApplied == \A k \in charged : rows[k[2]].state = "applied"

\* T3: "a settlement on any later day finds the same key and charges once"
ChargedAtMostOncePerSkip == \A i \in Ids : Cardinality(Charges(i)) <= 1

\* R9 and T4: undo refunds "only when the undo is accepted", and "a retry on a later day credits
\* once"
RefundOnlyWhenUndoneAtMostOnce ==
    \A i \in Ids : /\ Cardinality(Refunds(i)) <= 1
                   /\ Refunds(i) # {} => rows[i].undone

\* R5 and A43: "while the study day's take is `pending`, an undo ... refuses before any request or
\* write"
UndoRefusesWhilePending == ~pastPending
=============================================================================
