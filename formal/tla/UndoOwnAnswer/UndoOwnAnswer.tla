---------------------------- MODULE UndoOwnAnswer ----------------------------
\* @phx covers crates/engine-core/src/undo_answer.rs anchor=judge digest=sha256:686f406bd7dc83ab6f6ef1fd2a3c82720ed6ea95094333f1cc51956ee8b85901
\* @phx covers crates/engine-core/src/dispatch.rs anchor=run_undo digest=sha256:29ad324c9103ecea4fe9ce5973690b61fd0f418ed7b6c2da7c296f861525459f
\* @phx covers crates/web-engine/src/wasm.rs anchor=undo digest=sha256:d7bfeff5fb8dbae04a77d0b1c94e919bd30d7c48a07a82c77a88ab8ea753c115
\* @phx covers crates/web-engine/src/wasm.rs anchor=undo_offer digest=sha256:101511962778651131c410f35df7f809a89671660378035518d61e7344488d38
\* @phx covers crates/web-engine/src/wasm.rs anchor=rate digest=sha256:7724033b501870b01a0dd831e264a3ff17cf89c5e7106462d6e3558535a8b8e6
\* @phx covers crates/engine-core/src/undo_change.rs anchor=judge_change digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/engine-core/src/dispatch.rs anchor=run_restore digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/web-engine/src/wasm.rs anchor=bury digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/web-engine/src/wasm.rs anchor=flag digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/web-engine/src/wasm.rs anchor=current_card digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #714
\* @phx cites #753
\* @phx property AnUndoRevertsOnlyTheOfferedAnswer ramp=report
\* @phx property AnUndoRunsOnlyOnAConfirm ramp=report
\* @phx property AnUndoRestoresOnlyTheOfferedChange ramp=report
\* @phx property ARestoreRunsOnlyOnAConfirm ramp=report
\* @phx witness witness/an-undo-reverts-only-the-offered-answer.cfg kills=AnUndoRevertsOnlyTheOfferedAnswer
\* @phx witness witness/an-undo-runs-only-on-a-confirm.cfg kills=AnUndoRunsOnlyOnAConfirm
\* @phx witness witness/an-undo-restores-only-the-offered-change.cfg kills=AnUndoRestoresOnlyTheOfferedChange
\* @phx witness witness/a-restore-runs-only-on-a-confirm.cfg kills=ARestoreRunsOnlyOnAConfirm
\*
\* Undo of the review's own last answer (SPEC-371, ADR-382). The review records its answer when the
\* press is recorded. An undo press asks the engine for an offer, which the core's rule judges
\* against the engine's undo queue, and the page names the card, the answer and the state the card
\* returns to. Only the owner's confirmation of that offer reaches the write, and the rule judges
\* the record again there, inside the core's one door to Undo (3,8). Between the offer and the
\* confirmation, the owner, the review's other controls and a future web sync can each change the
\* queue (#714).
\*
\* What the model abstracts, and why:
\* - The engine's undo queue is `ops`, newest last. Each op is an answer, a bury or a flag, on one
\*   card, with a synced mark and the step it was begun at. An answer's step stands for the id of
\*   the review row it wrote: the row is there while the answer is in the queue, and an undo of the
\*   answer removes it. `counter` is the engine's last step, and every begun op advances it.
\* - The undo label is the last op's kind, so two answers share a label, as the engine's do; their
\*   steps tell them apart.
\* - A bury and a flag stand for every op that is not the review's answer: each begins a step and
\*   carries a label other than an answer's.
\* - A sync marks every op synced and keeps the queue. That is the case in which a write without
\*   the check could still revert a synced answer; a sync that discards the queue leaves the write
\*   nothing to revert. The web engine does not sync today, so the action stands for a future web
\*   sync.
\* - Each call into the web engine is one step: the Worker runs one call at a time, on its one
\*   thread. So `undo`'s rule and `run_undo`'s rule read one state, and the newest review `rate`
\*   reads is always its own answer's.
\* - `record` is `LAST_ANSWER`: the answered card and the engine's last step after the answer, or
\*   NONE. `open` and `close` clear it (Forget); a bury and a flag leave it.
\* - An offer and a confirmation each name a card and a step. `offers` is every offer the engine
\*   made, and `confirmed` every offer a confirmation was sent for. Each offer is confirmed at most
\*   once, because the page leaves its dialog when it sends a confirmation. `reverted` is every op
\*   an undo reverted, with the confirmation that ran it, or NONE for a write that no confirmation
\*   ran.
\* - The page's state machine (`web/app/src/lib/study/review.ts`) is TypeScript, which no cover can
\*   name: its offer and its confirmation are the Offer and Confirm actions, and the write on the
\*   press is the `OnConfirm` switch below.
\* - The card's text, the grade and the state an undo returns the card to are read, and reach no
\*   variable.
\*
\* The switches are the chosen design when both are TRUE: `Checked` is the confirmation's check (the
\* kept answer's card and step, then the rule, in `wasm.rs::undo` and again in
\* `dispatch.rs::run_undo`), and `OnConfirm` is the write waiting for a confirmation of an offer.
\*
\* The action-to-code map, by `file::item`:
\* - Answer(c) -> `wasm.rs::rate`: the answer through the core, then the record of it.
\* - OtherOp(c) -> `wasm.rs::bury`, `wasm.rs::flag`, and every other write that begins a step.
\* - Sync -> a future web sync.
\* - Forget -> `wasm.rs::open`, `wasm.rs::close`.
\* - Offer -> `wasm.rs::undo_offer`, judged by `undo_answer.rs::judge`.
\* - Confirm(o), StaleConfirm(o) -> `wasm.rs::undo`, then `dispatch.rs::run_undo`, for the offer the
\*   record names and for an offer it no longer names.
\*
\* Undo of the review's last bury or flag (SPEC-383, ADR-397, #753). The review keeps one slot: its
\* last answer (`record`) or its last bury or flag (`mark`), never both. A bury and a flag record
\* the change after their write and forget the answer; an answer forgets the change; opening or
\* closing the collection forgets both. An undo press of a change asks for an offer, which the
\* core's change rule judges against the engine's undo queue and the card's mark, and only the
\* owner's confirmation of that offer reaches the write, where the rule judges the change again and
\* the core's one door to Undo (3,8) restores it.
\* - `mark` is `LAST_MARK`: the change's kind, its card and the engine's last step after it, or
\*   NOMARK. `markOffers` and `markConfirmed` are every offer of a change the engine made and every
\*   one a confirmation was sent for; `restored` is every op a restore reverted, with the
\*   confirmation that ran it (or NONE) and the engine's last step when it ran.
\* - The change rule (`undo_change.rs::judge_change`) reads the card's mark: the card still shows
\*   the change and has not synced. The model reads that as the change's own op, unsynced, still in
\*   the queue, with the engine's last step and undo label the change's.
\* - The shipped `OtherOp(c)` stays: it now stands for any write that begins a step and keeps both
\*   slots, a wider set of behaviours than the code's, so a property clean with it is clean without.
\* - The flag's toggle (added, removed or replaced) and the state a bury returns its card to are
\*   read for the dialog, and reach no variable.
\* - `Checked` is also the change's check at the write (`wasm.rs::undo` judging the slot's change,
\*   then `dispatch.rs::run_restore`), and `OnConfirm` the restore waiting for a confirmation.
\*
\* The new actions, by `file::item`:
\* - Bury(c), Flag(c) -> `wasm.rs::bury`, `wasm.rs::flag`: the write, the card's mark read, then
\*   the slot's change; and `wasm.rs::current_card` offers its undo by the slot's kind.
\* - MarkOffer -> `wasm.rs::undo_offer`, judged by `undo_change.rs::judge_change`.
\* - MarkConfirm(o), MarkStaleConfirm(o) -> `wasm.rs::undo`, then `dispatch.rs::run_undo` reading
\*   the record's kind and `dispatch.rs::run_restore`, for the offer the slot names and for an offer
\*   it no longer names.

EXTENDS Integers, Sequences

CONSTANTS NCards, MaxSteps, Checked, OnConfirm

Cards == 1..NCards
Steps == 1..MaxSteps
Kinds == {"answer", "bury", "flag"}
Named == [card : Cards, step : Steps]
NONE == [card |-> 0, step |-> 0]
Op == [kind : Kinds, card : Cards, synced : BOOLEAN, step : Steps]
Undone == [kind : Kinds, card : Cards, synced : BOOLEAN, step : Steps, by : Named \cup {NONE}]
Changes == {"bury", "flag"}
Marked == [kind : Changes, card : Cards, step : Steps]
NOMARK == [kind |-> "none", card |-> 0, step |-> 0]
Restored == [kind : Kinds, card : Cards, synced : BOOLEAN, step : Steps, by : Named \cup {NONE},
             last : 0..MaxSteps]

VARIABLES ops, counter, record, offers, confirmed, reverted, mark, markOffers, markConfirmed, restored

vars == <<ops, counter, record, offers, confirmed, reverted, mark, markOffers, markConfirmed, restored>>

\* The new variables, which every shipped action but Answer and Forget leaves as they are.
markVars == <<mark, markOffers, markConfirmed, restored>>

TypeOK ==
    /\ \E n \in 0..MaxSteps : ops \in [1..n -> Op]
    /\ counter \in 0..MaxSteps
    /\ record \in Named \cup {NONE}
    /\ offers \in SUBSET Named
    /\ confirmed \in SUBSET Named
    /\ reverted \in SUBSET Undone
    /\ mark \in Marked \cup {NOMARK}
    /\ markOffers \in SUBSET Named
    /\ markConfirmed \in SUBSET Named
    /\ restored \in SUBSET Restored

Init ==
    /\ ops = <<>>
    /\ counter = 0
    /\ record = NONE
    /\ offers = {}
    /\ confirmed = {}
    /\ reverted = {}
    /\ mark = NOMARK
    /\ markOffers = {}
    /\ markConfirmed = {}
    /\ restored = {}

\* An op begins a step, and joins the queue.
Begin(k, c) ==
    /\ counter < MaxSteps
    /\ counter' = counter + 1
    /\ ops' = Append(ops, [kind |-> k, card |-> c, synced |-> FALSE, step |-> counter + 1])

\* The review's answer, then its record: the card, and the engine's last step after the answer
\* (`wasm.rs::rate`). The answer is now the review's last action, so the slot forgets any change.
Answer(c) ==
    /\ Begin("answer", c)
    /\ record' = [card |-> c, step |-> counter + 1]
    /\ mark' = NOMARK
    /\ UNCHANGED <<offers, confirmed, reverted, markOffers, markConfirmed, restored>>

\* The op the queue ends with, the one Undo (3,8) reverts.
Newest == ops[Len(ops)]

\* Any other op on a card: it begins a step, and leaves the record.
OtherOp(c) ==
    /\ \E k \in {"bury", "flag"} : Begin(k, c)
    /\ UNCHANGED <<record, offers, confirmed, reverted>>
    /\ UNCHANGED markVars

\* The review's bury or flag (`wasm.rs::bury`, `wasm.rs::flag`): the write begins a step, and the
\* slot then holds the change, its card and the engine's last step after it, and forgets the answer.
Change(k, c) ==
    /\ Begin(k, c)
    /\ mark' = [kind |-> k, card |-> c, step |-> counter + 1]
    /\ record' = NONE
    /\ UNCHANGED <<offers, confirmed, reverted, markOffers, markConfirmed, restored>>

Bury(c) == Change("bury", c)

Flag(c) == Change("flag", c)

\* A sync sends every row, so every op is marked synced.
Sync ==
    /\ \E i \in DOMAIN ops : ~ops[i].synced
    /\ ops' = [i \in DOMAIN ops |-> [ops[i] EXCEPT !.synced = TRUE]]
    /\ UNCHANGED <<counter, record, offers, confirmed, reverted>>
    /\ UNCHANGED markVars

\* Opening or closing the collection forgets the record, and the slot's change with it.
Forget ==
    /\ record # NONE \/ mark # NOMARK
    /\ record' = NONE
    /\ mark' = NOMARK
    /\ UNCHANGED <<ops, counter, offers, confirmed, reverted, markOffers, markConfirmed, restored>>

\* The rule (`undo_answer.rs::judge`): the recorded answer's review row is there, of the card, and
\* has not synced; the queue is not empty; and the engine's last step and undo label are the
\* record's.
Judge(o) ==
    /\ \E i \in DOMAIN ops :
          /\ ops[i].kind = "answer"
          /\ ops[i].step = o.step
          /\ ops[i].card = o.card
          /\ ~ops[i].synced
    /\ ops # <<>>
    /\ counter = o.step
    /\ Newest.kind = "answer"

\* Undo (3,8): the queue's last op is reverted, and the record is forgotten. `by` is the
\* confirmation that ran it, or NONE.
Revert(by) ==
    /\ ops # <<>>
    /\ reverted' = reverted \cup {[kind |-> Newest.kind, card |-> Newest.card,
                                   synced |-> Newest.synced, step |-> Newest.step, by |-> by]}
    /\ ops' = SubSeq(ops, 1, Len(ops) - 1)
    /\ record' = NONE

\* The offer (`wasm.rs::undo_offer`): the rule judges the record against the engine now, and only an
\* admitted record is offered. Without `OnConfirm`, the press is the write.
Offer ==
    /\ record # NONE
    /\ record \notin offers
    /\ Judge(record)
    /\ offers' = offers \cup {record}
    /\ IF OnConfirm
          THEN UNCHANGED <<ops, record, reverted>>
          ELSE Revert(NONE)
    /\ UNCHANGED <<counter, confirmed>>
    /\ UNCHANGED markVars

\* The write a confirmation runs (`wasm.rs::undo`, then `dispatch.rs::run_undo`): the offer is the
\* kept answer's card and step, the rule admits the record, and the gesture's door runs (3,8). A
\* refusal changes nothing. Without `Checked`, (3,8) runs on whatever the queue ends with.
Write(o) ==
    IF Checked
       THEN IF o = record /\ Judge(o)
               THEN Revert(o)
               ELSE UNCHANGED <<ops, record, reverted>>
       ELSE IF ops # <<>>
               THEN Revert(o)
               ELSE UNCHANGED <<ops, record, reverted>>

\* The owner confirms the offer the record names.
Confirm(o) ==
    /\ OnConfirm
    /\ o \notin confirmed
    /\ o = record
    /\ confirmed' = confirmed \cup {o}
    /\ Write(o)
    /\ UNCHANGED <<counter, offers>>
    /\ UNCHANGED markVars

\* A confirmation arrives for an offer the record no longer names.
StaleConfirm(o) ==
    /\ OnConfirm
    /\ o \notin confirmed
    /\ o # record
    /\ confirmed' = confirmed \cup {o}
    /\ Write(o)
    /\ UNCHANGED <<counter, offers>>
    /\ UNCHANGED markVars

\* The card and step the slot's change names, as an offer and a confirmation carry them.
MarkNamed == [card |-> mark.card, step |-> mark.step]

\* The change rule (`undo_change.rs::judge_change`): the change's op is in the queue, of its card
\* and kind, and has not synced, as the card's mark shows; and the engine's last step and undo
\* label are the change's.
JudgeMark(m) ==
    /\ \E i \in DOMAIN ops :
          /\ ops[i].kind = m.kind
          /\ ops[i].step = m.step
          /\ ops[i].card = m.card
          /\ ~ops[i].synced
    /\ ops # <<>>
    /\ counter = m.step
    /\ Newest.kind = m.kind

\* Undo (3,8) through the restore path: the queue's last op is reverted, and the slot is emptied.
\* `by` is the confirmation that ran it, or NONE; `last` is the engine's last step when it ran.
Restore(by) ==
    /\ ops # <<>>
    /\ restored' = restored \cup {[kind |-> Newest.kind, card |-> Newest.card,
                                   synced |-> Newest.synced, step |-> Newest.step, by |-> by,
                                   last |-> counter]}
    /\ ops' = SubSeq(ops, 1, Len(ops) - 1)
    /\ mark' = NOMARK

\* The offer of the slot's change (`wasm.rs::undo_offer`): the change rule judges it against the
\* engine now, and only an admitted change is offered. Without `OnConfirm`, the press is the write.
MarkOffer ==
    /\ mark # NOMARK
    /\ MarkNamed \notin markOffers
    /\ JudgeMark(mark)
    /\ markOffers' = markOffers \cup {MarkNamed}
    /\ IF OnConfirm
          THEN UNCHANGED <<ops, mark, restored>>
          ELSE Restore(NONE)
    /\ UNCHANGED <<counter, record, offers, confirmed, reverted, markConfirmed>>

\* The restore a confirmation runs (`wasm.rs::undo`, then `dispatch.rs::run_restore`): the offer is
\* the slot's change's card and step, the change rule admits it, and the gesture's door runs (3,8).
\* A refusal changes nothing. Without `Checked`, (3,8) runs on whatever the queue ends with.
RestoreWrite(o) ==
    IF Checked
       THEN IF o = MarkNamed /\ JudgeMark(mark)
               THEN Restore(o)
               ELSE UNCHANGED <<ops, mark, restored>>
       ELSE IF ops # <<>>
               THEN Restore(o)
               ELSE UNCHANGED <<ops, mark, restored>>

\* The owner confirms the offer the slot's change names.
MarkConfirm(o) ==
    /\ OnConfirm
    /\ o \notin markConfirmed
    /\ o = MarkNamed
    /\ markConfirmed' = markConfirmed \cup {o}
    /\ RestoreWrite(o)
    /\ UNCHANGED <<counter, record, offers, confirmed, reverted, markOffers>>

\* A confirmation arrives for an offer of a change the slot no longer names.
MarkStaleConfirm(o) ==
    /\ OnConfirm
    /\ o \notin markConfirmed
    /\ o # MarkNamed
    /\ markConfirmed' = markConfirmed \cup {o}
    /\ RestoreWrite(o)
    /\ UNCHANGED <<counter, record, offers, confirmed, reverted, markOffers>>

\* Every step is spent, so the run may end.
Finished ==
    /\ counter = MaxSteps
    /\ UNCHANGED vars

Next ==
    \/ \E c \in Cards : Answer(c) \/ OtherOp(c)
    \/ \E c \in Cards : Bury(c) \/ Flag(c)
    \/ Sync
    \/ Forget
    \/ Offer
    \/ \E o \in offers : Confirm(o) \/ StaleConfirm(o)
    \/ MarkOffer
    \/ \E o \in markOffers : MarkConfirm(o) \/ MarkStaleConfirm(o)
    \/ Finished

Spec == Init /\ [][Next]_vars

\* Whatever an undo reverts is an unsynced answer that an offer named, and the one the confirmation
\* that ran it named: nothing reverted is a bury, a flag or a synced answer ("the confirmation's
\* card and step must be the kept answer's, the core's rule judges the record again against the
\* engine now", `wasm.rs::undo`; "only then does the engine undo", `dispatch.rs::run_undo`; "The
\* first refusal that holds", `undo_answer.rs::judge`).
AnUndoRevertsOnlyTheOfferedAnswer ==
    \A r \in reverted :
        /\ r.kind = "answer"
        /\ ~r.synced
        /\ [card |-> r.card, step |-> r.step] \in offers
        /\ r.by \in {NONE, [card |-> r.card, step |-> r.step]}

\* Nothing is reverted that no confirmation of an offer preceded ("It reads, and writes nothing",
\* `wasm.rs::undo_offer`; "the write runs only through the owner's gesture", `wasm.rs::undo`).
AnUndoRunsOnlyOnAConfirm == \A r \in reverted : r.by \in confirmed

\* Whatever a restore reverts is the bury or the flag an offer of a change named, on its own card,
\* unsynced, with no step begun after it, and the one the confirmation that ran it named ("the
\* core's rule judges the change against the card's mark", `wasm.rs::undo`; "runs (3,8) only when
\* it admits", `dispatch.rs::run_restore`; the refusal order of `undo_change.rs::judge_change`).
AnUndoRestoresOnlyTheOfferedChange ==
    \A r \in restored :
        /\ r.kind \in Changes
        /\ ~r.synced
        /\ r.last = r.step
        /\ [card |-> r.card, step |-> r.step] \in markOffers
        /\ r.by \in {NONE, [card |-> r.card, step |-> r.step]}

\* Nothing is restored that no confirmation of an offer of a change preceded (`wasm.rs::undo_offer`
\* reads and writes nothing; `wasm.rs::undo` restores only through the owner's gesture).
ARestoreRunsOnlyOnAConfirm == \A r \in restored : r.by \in markConfirmed

=============================================================================
