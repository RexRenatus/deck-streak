---------------------------- MODULE SkipDayWrite ----------------------------
\* @phx covers crates/ingest/src/skip_write.rs anchor=take digest=sha256:de7de3e104f15af30720a506cd0aab812bfbd9bec30a7ebfc3ee240da9ed680a
\* @phx covers crates/ingest/src/skip_write.rs anchor=restore_check digest=sha256:4a6c45c6c7d8321f08bb84ab9180e95384d1e31efe1a03ef49d80c5fdc4cce40
\* @phx covers crates/ingest/src/skip_write.rs anchor=moved_counts digest=sha256:627202da8c20881c8aa9f6aa9f18408975fd26af7adedaa9830cb18157f69244
\* @phx covers crates/ingest/src/skip.rs anchor=record_prior digest=sha256:c292d9a66e0a6b80e39037eb33bb23bc9250e74cd10fbd7fd7c6fabce9649865
\* @phx covers crates/ingest/src/skip.rs anchor=record_left digest=sha256:90baf127d75873844710bfafdb0f8e2b8c1f86d32e048706355403e323a6c418
\* @phx covers crates/ingest/src/write_class_stop.rs anchor=read_stop digest=sha256:b695c99b28172bfd0ee413ceb777da75dbfc9d6d6774618a56fb623f23df5d51
\* @phx covers crates/ingest/src/write_class_stop.rs anchor=set_by_counts digest=sha256:15578e86520cdcfac315e36f07be6dbe3eda73b4e3bf664866b5010f0068ed2e
\* @phx cites #108
\* @phx property NoUploadOutsidePreview ramp=report
\* @phx property FullSyncDemandWritesNothing ramp=report
\* @phx property RestoreCheckAndPriorStateBeforeChange ramp=report
\* @phx property PendingSettlesByRecordOnly ramp=report
\* @phx property StoppedClassWritesNothing ramp=report
\* @phx witness witness/move-what-the-search-selects.cfg kills=NoUploadOutsidePreview
\* @phx witness witness/resolve-the-demand-by-download.cfg kills=FullSyncDemandWritesNothing
\* @phx witness witness/push-without-restore-check.cfg kills=RestoreCheckAndPriorStateBeforeChange
\* @phx witness witness/reschedule-before-snapshot.cfg kills=RestoreCheckAndPriorStateBeforeChange
\* @phx witness witness/settle-by-due-date.cfg kills=PendingSettlesByRecordOnly
\* @phx witness witness/stop-read-only-at-start.cfg kills=StoppedClassWritesNothing
(***************************************************************************)
\* The skip day's take writing to the owner's server (#108, SPEC-083 R20 to R27 and R34 to R36,
\* ADR-321 D14 to D20), against every other actor that can change what it acts on between its
\* checks and its acts:
\* - the take, skip_write.rs::take, its seventeen steps as one process (pc), on a row its caller
\*   began `pending`, under the exclusive collection lock;
\* - another client, playing the owner's other device on the server, at most MaxClientActs times:
\*   a review that moves a server card (and so its modification time) to a later day or onto the
\*   very day the skip writes, a change that makes a card due today that the preview did not list,
\*   a full upload that makes the server demand a full sync of every other client, and a change of
\*   the configured UTC offset;
\* - the class's stop's setters: the owner at any time, and the take's own counts
\*   (write_class_stop.rs::set_by_counts);
\* - the server, which commits a push only at `finish` and may lose the answer after it commits
\*   (SPEC-083 R25);
\* - the start-up settlement, E4c's compare of a row still `pending` (R26), run once after the take
\*   ended.
\*
\* A card is abstracted to the two facts the properties read: its due (today, a later day, or the
\* day the skip writes) and who last wrote it (nobody, the take's reschedule, another client), the
\* second standing for its modification time: R26's compare reads "the scheduling state and
\* modification time the take recorded after its reschedule (R22), which only the take's push can
\* have put on the server". The private copy is the state the preview listed: SPEC-022's syncer is
\* not modelled, so the take's steps 1 to 4 on it (the stop, the pin, the zone, the offset, the
\* engine's day, the list and its digest) pass or refuse as one step, and Preview is the set of
\* cards due today in it. The take's step 8 counts, and every hook, change nothing the properties
\* read, so they are stuttering steps folded into their neighbours. A count that moves is drawn at
\* step 12 (an engine that wrote more than Set Due Date's own rows). A merge of the take's push
\* with another client's later review is abstracted to the take's card arriving: A34's read-back
\* lists such a card, and no property here reads it.
\*
\* Re-read against the covered code as built (skip_write.rs::take and its steps): day_and_zone and
\* bound_to_preview on the private copy are Start; the working copy and converge_outcome are
\* Converge; day_and_zone on the working copy is Recheck; select, with its answer before any backup
\* when nothing moves, is Select; the counts before are the stutter above; backup and
\* skip_write.rs::restore_check are Backup and Check; skip.rs::record_prior inside reschedule, before
\* the engine's Set Due Date, is Snapshot then Reschedule; the counts after, moved_counts and
\* write_class_stop.rs::set_by_counts are Counts; skip.rs::record_left is Left;
\* write_class_stop.rs::read_stop before the push is step 14; the second write_sync is Push. The
\* read-back after an accepted push writes nothing and is a stutter.
\*
\* Switches: each is FALSE in the code as built, and its TRUE arm is the defect its witness names.
(***************************************************************************)
EXTENDS Integers, FiniteSets

CONSTANTS MoveBySearch, DownloadOnDemand, SkipRestoreCheck, RescheduleFirst, SettleByDue,
          StopReadAtStartOnly, NCards, NPreview, MaxClientActs

Cards == 1..NCards
\* the cards due today in the private copy, which the preview lists and the confirm's digest binds
Preview == 1..NPreview

Dues == {"today", "later", "skipdate"}
Writers == {"none", "take", "other"}
CardType == [due : Dues, by : Writers]

Steps == {"start", "converge", "recheck", "select", "backup", "check", "snapshot", "reschedule",
          "counts", "left", "stop2", "push", "done"}
\* the steps at which the take has not yet read the stop before its push (R36)
BeforeRead == {"start", "converge", "recheck", "select", "backup", "check", "snapshot",
               "reschedule", "counts", "left", "stop2"}

VARIABLES pc, server, serverOffset, fullDemand, wc, wcOffset, moved, checked, snap, left, stop,
          row, acts, metDemand, fullSync, pushCommitted, noneToMove, stopBeforeRead, settled

vars == <<pc, server, serverOffset, fullDemand, wc, wcOffset, moved, checked, snap, left, stop,
          row, acts, metDemand, fullSync, pushCommitted, noneToMove, stopBeforeRead, settled>>

TypeOK ==
    /\ pc \in Steps
    /\ server \in [Cards -> CardType]
    /\ serverOffset \in {"same", "differs"}
    /\ fullDemand \in BOOLEAN
    /\ wc \in [Cards -> CardType]
    /\ wcOffset \in {"same", "differs"}
    /\ moved \subseteq Cards
    /\ checked \in BOOLEAN
    /\ snap \subseteq Cards
    /\ left \subseteq Cards
    /\ stop \in BOOLEAN
    /\ row \in {"pending", "applied", "failed"}
    /\ acts \in 0..MaxClientActs
    /\ metDemand \in BOOLEAN
    /\ fullSync \in BOOLEAN
    /\ pushCommitted \in BOOLEAN
    /\ noneToMove \in BOOLEAN
    /\ stopBeforeRead \in BOOLEAN
    /\ settled \in BOOLEAN

Init ==
    /\ pc = "start"
    /\ server = [c \in Cards |-> [due |-> IF c \in Preview THEN "today" ELSE "later", by |-> "none"]]
    /\ serverOffset = "same"
    /\ fullDemand = FALSE
    /\ wc = server
    /\ wcOffset = "same"
    /\ moved = {}
    /\ checked = FALSE
    /\ snap = {}
    /\ left = {}
    /\ stop = FALSE
    /\ row = "pending"
    /\ acts = 0
    /\ metDemand = FALSE
    /\ fullSync = FALSE
    /\ pushCommitted = FALSE
    /\ noneToMove = FALSE
    /\ stopBeforeRead = FALSE
    /\ settled = FALSE

\* every refusal settles the begun row `failed` with its code and ends the take; the working copy is
\* discarded, so it changes nothing the properties read
Fail ==
    /\ pc' = "done"
    /\ row' = "failed"

TakeVars == <<server, serverOffset, fullDemand, acts, stop, stopBeforeRead, settled>>

\* steps 1 to 4: the stop (writes_stopped), then the pin, the zone, the offset, the engine's day and
\* the digest on the private copy, which pass because the private copy is the one the preview read
Start ==
    /\ pc = "start"
    /\ IF stop
       THEN /\ Fail
            /\ UNCHANGED <<wc, wcOffset, moved, checked, snap, left, metDemand, fullSync,
                           pushCommitted, noneToMove>>
       ELSE /\ pc' = "converge"
            /\ UNCHANGED <<row, wc, wcOffset, moved, checked, snap, left, metDemand, fullSync,
                           pushCommitted, noneToMove>>
    /\ UNCHANGED TakeVars

\* step 5: the working copy converges by one normal sync. A full-sync demand ends the take
\* (full_sync_required, R25); the defect DownloadOnDemand resolves it by a full download, as
\* sync.rs::attempt does, and goes on. With no local change the converge brings the server's state.
Converge ==
    /\ pc = "converge"
    /\ IF fullDemand
       THEN IF DownloadOnDemand
            THEN /\ wc' = server
                 /\ wcOffset' = serverOffset
                 /\ metDemand' = TRUE
                 /\ fullSync' = TRUE
                 /\ fullDemand' = FALSE
                 /\ pc' = "recheck"
                 /\ UNCHANGED row
            ELSE /\ metDemand' = TRUE
                 /\ Fail
                 /\ UNCHANGED <<wc, wcOffset, fullSync, fullDemand>>
       ELSE /\ wc' = server
            /\ wcOffset' = serverOffset
            /\ pc' = "recheck"
            /\ UNCHANGED <<row, metDemand, fullSync, fullDemand>>
    /\ UNCHANGED <<server, serverOffset, acts, stop, stopBeforeRead, settled, moved, checked, snap,
                   left, pushCommitted, noneToMove>>

\* step 6: the offset (and the engine's day) again, on the converged working copy (zone_differs)
Recheck ==
    /\ pc = "recheck"
    /\ IF wcOffset = "differs"
       THEN Fail
       ELSE /\ pc' = "select"
            /\ UNCHANGED row
    /\ UNCHANGED <<wc, wcOffset, moved, checked, snap, left, metDemand, fullSync, pushCommitted,
                   noneToMove>>
    /\ UNCHANGED TakeVars

\* step 7: moved = the previewed cards the wrapped search still selects in the converged working
\* copy (R21); the defect MoveBySearch moves whatever the search selects there. No card to move
\* records the skip with none moved and runs no second sync.
Selected == {c \in Cards : wc[c].due = "today"}
ToMove == IF MoveBySearch THEN Selected ELSE Selected \cap Preview

Select ==
    /\ pc = "select"
    /\ moved' = ToMove
    /\ IF ToMove = {}
       THEN /\ pc' = "done"
            /\ row' = "applied"
            /\ noneToMove' = TRUE
       ELSE /\ pc' = "backup"
            /\ UNCHANGED <<row, noneToMove>>
    /\ UNCHANGED <<wc, wcOffset, checked, snap, left, metDemand, fullSync, pushCommitted>>
    /\ UNCHANGED TakeVars

\* the step after the restore check: step 10, or step 11 under the defect RescheduleFirst
AfterCheck == IF RescheduleFirst THEN "reschedule" ELSE "snapshot"

\* step 9: the backup's partial file (backup_failed), then skip_write.rs::restore_check
\* (backup_check_failed); the defect SkipRestoreCheck goes on with no check
Backup ==
    /\ pc = "backup"
    /\ \/ /\ Fail
       \/ /\ pc' = IF SkipRestoreCheck THEN AfterCheck ELSE "check"
          /\ UNCHANGED row
    /\ UNCHANGED <<wc, wcOffset, moved, checked, snap, left, metDemand, fullSync, pushCommitted,
                   noneToMove>>
    /\ UNCHANGED TakeVars

Check ==
    /\ pc = "check"
    /\ \/ /\ Fail
          /\ UNCHANGED checked
       \/ /\ checked' = TRUE
          /\ pc' = AfterCheck
          /\ UNCHANGED row
    /\ UNCHANGED <<wc, wcOffset, moved, snap, left, metDemand, fullSync, pushCommitted, noneToMove>>
    /\ UNCHANGED TakeVars

\* step 10: skip.rs::record_prior commits each moved card's prior state in one transaction; the
\* defect RescheduleFirst runs step 11 before it
Snapshot ==
    /\ pc = "snapshot"
    /\ snap' = moved
    /\ pc' = IF RescheduleFirst THEN "counts" ELSE "reschedule"
    /\ UNCHANGED <<row, wc, wcOffset, moved, checked, left, metDemand, fullSync, pushCommitted,
                   noneToMove>>
    /\ UNCHANGED TakeVars

\* step 11: the engine's Set Due Date moves exactly the moved cards in the working copy; under the
\* defect RescheduleFirst it runs before step 10, which follows it
Rescheduled == [c \in Cards |-> IF c \in moved THEN [due |-> "skipdate", by |-> "take"] ELSE wc[c]]

Reschedule ==
    /\ pc = "reschedule"
    /\ wc' = Rescheduled
    /\ pc' = IF RescheduleFirst THEN "snapshot" ELSE "counts"
    /\ UNCHANGED <<row, wcOffset, moved, checked, snap, left, metDemand, fullSync, pushCommitted,
                   noneToMove>>
    /\ UNCHANGED TakeVars

\* step 12: skip_write.rs::moved_counts; any count but the review-log rows and the due count that
\* moved sets the class's stop (write_class_stop.rs::set_by_counts) and ends the take
\* (counts_moved), before its push
Counts ==
    /\ pc = "counts"
    /\ \/ /\ Fail
          /\ stop' = TRUE
          /\ stopBeforeRead' = TRUE
       \/ /\ pc' = "left"
          /\ UNCHANGED <<row, stop, stopBeforeRead>>
    /\ UNCHANGED <<wc, wcOffset, moved, checked, snap, left, metDemand, fullSync, pushCommitted,
                   noneToMove>>
    /\ UNCHANGED <<server, serverOffset, fullDemand, acts, settled>>

\* step 13: skip.rs::record_left commits the state each moved card was left in
Left ==
    /\ pc = "left"
    /\ left' = moved
    /\ pc' = "stop2"
    /\ UNCHANGED <<row, wc, wcOffset, moved, checked, snap, metDemand, fullSync, pushCommitted,
                   noneToMove>>
    /\ UNCHANGED TakeVars

\* step 14: the stop read again before the push (writes_stopped); the defect StopReadAtStartOnly
\* reads it only at step 1
Stop2 ==
    /\ pc = "stop2"
    /\ IF stop /\ ~StopReadAtStartOnly
       THEN Fail
       ELSE /\ pc' = "push"
            /\ UNCHANGED row
    /\ UNCHANGED <<wc, wcOffset, moved, checked, snap, left, metDemand, fullSync, pushCommitted,
                   noneToMove>>
    /\ UNCHANGED TakeVars

\* step 15: the push, one normal sync. A full-sync demand ends the take (full_sync_required). Else
\* the server commits at `finish` and answers (applied, by coordination's settlement), commits and
\* loses the answer (the row stays `pending`), fails inside the sync before it commits (`pending`
\* too: the outcome is not known), or never starts (push_failed).
Committed == [c \in Cards |-> IF c \in moved THEN wc[c] ELSE server[c]]

Push ==
    /\ pc = "push"
    /\ pc' = "done"
    /\ IF fullDemand
       THEN /\ metDemand' = TRUE
            /\ row' = "failed"
            /\ UNCHANGED <<server, pushCommitted>>
       ELSE \/ /\ server' = Committed
               /\ pushCommitted' = TRUE
               /\ row' \in {"applied", "pending"}
               /\ UNCHANGED metDemand
            \/ /\ row' \in {"pending", "failed"}
               /\ UNCHANGED <<server, pushCommitted, metDemand>>
    /\ UNCHANGED <<serverOffset, fullDemand, wc, wcOffset, moved, checked, snap, left, stop, acts,
                   fullSync, noneToMove, stopBeforeRead, settled>>

\* another client, on the server
Review(c, d) ==
    /\ acts < MaxClientActs
    /\ server[c].due = "today"
    /\ d \in {"later", "skipdate"}
    /\ server' = [server EXCEPT ![c] = [due |-> d, by |-> "other"]]
    /\ acts' = acts + 1
    /\ UNCHANGED <<pc, serverOffset, fullDemand, wc, wcOffset, moved, checked, snap, left, stop,
                   row, metDemand, fullSync, pushCommitted, noneToMove, stopBeforeRead, settled>>

MakeDue(c) ==
    /\ acts < MaxClientActs
    /\ server[c].due # "today"
    /\ server' = [server EXCEPT ![c] = [due |-> "today", by |-> "other"]]
    /\ acts' = acts + 1
    /\ UNCHANGED <<pc, serverOffset, fullDemand, wc, wcOffset, moved, checked, snap, left, stop,
                   row, metDemand, fullSync, pushCommitted, noneToMove, stopBeforeRead, settled>>

FullUpload ==
    /\ acts < MaxClientActs
    /\ ~fullDemand
    /\ fullDemand' = TRUE
    /\ acts' = acts + 1
    /\ UNCHANGED <<pc, server, serverOffset, wc, wcOffset, moved, checked, snap, left, stop, row,
                   metDemand, fullSync, pushCommitted, noneToMove, stopBeforeRead, settled>>

ChangeOffset ==
    /\ acts < MaxClientActs
    /\ serverOffset = "same"
    /\ serverOffset' = "differs"
    /\ acts' = acts + 1
    /\ UNCHANGED <<pc, server, fullDemand, wc, wcOffset, moved, checked, snap, left, stop, row,
                   metDemand, fullSync, pushCommitted, noneToMove, stopBeforeRead, settled>>

\* the owner sets the class's stop, at any time
OwnerStop ==
    /\ ~stop
    /\ stop' = TRUE
    /\ stopBeforeRead' = (stopBeforeRead \/ pc \in BeforeRead)
    /\ UNCHANGED <<pc, server, serverOffset, fullDemand, wc, wcOffset, moved, checked, snap, left,
                   row, acts, metDemand, fullSync, pushCommitted, noneToMove, settled>>

\* E4c's compare of a row the take left `pending` (R26): `applied` when any recorded card on the
\* server carries what the take recorded after its reschedule, else `failed`; the defect
\* SettleByDue reads the due date alone
Recorded(c) == IF SettleByDue THEN server[c].due = "skipdate" ELSE server[c].by = "take"

Settle ==
    /\ pc = "done"
    /\ row = "pending"
    /\ ~settled
    /\ settled' = TRUE
    /\ row' = IF \E c \in left : Recorded(c) THEN "applied" ELSE "failed"
    /\ UNCHANGED <<pc, server, serverOffset, fullDemand, wc, wcOffset, moved, checked, snap, left,
                   stop, acts, metDemand, fullSync, pushCommitted, noneToMove, stopBeforeRead>>

\* the take has ended and no row is left pending
Finished ==
    /\ pc = "done"
    /\ row # "pending"
    /\ UNCHANGED vars

Next ==
    \/ Start \/ Converge \/ Recheck \/ Select \/ Backup \/ Check \/ Snapshot
    \/ Reschedule \/ Counts \/ Left \/ Stop2 \/ Push
    \/ \E c \in Cards, d \in Dues : Review(c, d)
    \/ \E c \in Cards : MakeDue(c)
    \/ FullUpload \/ ChangeOffset \/ OwnerStop
    \/ Settle
    \/ Finished

Spec == Init /\ [][Next]_vars

\* R21 and R28: "The cards it moves are the previewed cards that the wrapped search (R3) still
\* selects in the converged working copy ... Every other path ... records zero uploads"; only the
\* take's push writes a card the take marks, so every such card was previewed
NoUploadOutsidePreview == \A c \in Cards : server[c].by = "take" => c \in Preview

\* R25: "When the server demands a full or one-way sync at the converge or at the push, the take
\* aborts: it resolves the demand neither by a download nor by an upload"
FullSyncDemandWritesNothing == /\ ~fullSync
                               /\ metDemand => ~pushCommitted

\* R22 and R34: the backup passes its restore check, and each moved card's prior state is
\* committed, "before any card changes"
RestoreCheckAndPriorStateBeforeChange ==
    \A c \in Cards : wc[c].by = "take" => (checked /\ c \in snap)

\* R26: "A due date alone never settles a row `applied`": a row is applied only when the take's
\* push landed, or when it had no card to move
PendingSettlesByRecordOnly == row = "applied" => (pushCommitted \/ noneToMove)

\* R36: "a take ... already running reads it again before its push and ends writing nothing when it
\* is set"
StoppedClassWritesNothing == pushCommitted => ~stopBeforeRead
=============================================================================
