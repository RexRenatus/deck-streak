---------------------------- MODULE UndoOwnAnswer ----------------------------
\* @phx covers crates/engine-core/src/undo_answer.rs anchor=judge digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/engine-core/src/dispatch.rs anchor=run_undo digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/web-engine/src/wasm.rs anchor=undo digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/web-engine/src/wasm.rs anchor=undo_offer digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/web-engine/src/wasm.rs anchor=rate digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #714
\* @phx property AnUndoRevertsOnlyTheOfferedAnswer ramp=report
\* @phx property AnUndoRunsOnlyOnAConfirm ramp=report
\* @phx witness witness/an-undo-reverts-only-the-offered-answer.cfg kills=AnUndoRevertsOnlyTheOfferedAnswer
\* @phx witness witness/an-undo-runs-only-on-a-confirm.cfg kills=AnUndoRunsOnlyOnAConfirm
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

EXTENDS Integers, Sequences

CONSTANTS NCards, MaxSteps, Checked, OnConfirm

Cards == 1..NCards
Steps == 1..MaxSteps
Kinds == {"answer", "bury", "flag"}
Named == [card : Cards, step : Steps]
NONE == [card |-> 0, step |-> 0]
Op == [kind : Kinds, card : Cards, synced : BOOLEAN, step : Steps]
Undone == [kind : Kinds, card : Cards, synced : BOOLEAN, step : Steps, by : Named \cup {NONE}]

VARIABLES ops, counter, record, offers, confirmed, reverted

vars == <<ops, counter, record, offers, confirmed, reverted>>

TypeOK ==
    /\ \E n \in 0..MaxSteps : ops \in [1..n -> Op]
    /\ counter \in 0..MaxSteps
    /\ record \in Named \cup {NONE}
    /\ offers \in SUBSET Named
    /\ confirmed \in SUBSET Named
    /\ reverted \in SUBSET Undone

Init ==
    /\ ops = <<>>
    /\ counter = 0
    /\ record = NONE
    /\ offers = {}
    /\ confirmed = {}
    /\ reverted = {}

\* An op begins a step, and joins the queue.
Begin(k, c) ==
    /\ counter < MaxSteps
    /\ counter' = counter + 1
    /\ ops' = Append(ops, [kind |-> k, card |-> c, synced |-> FALSE, step |-> counter + 1])

\* The review's answer, then its record: the card, and the engine's last step after the answer
\* (`wasm.rs::rate`).
Answer(c) ==
    /\ Begin("answer", c)
    /\ record' = [card |-> c, step |-> counter + 1]
    /\ UNCHANGED <<offers, confirmed, reverted>>

\* The op the queue ends with, the one Undo (3,8) reverts.
Newest == ops[Len(ops)]

\* Any other op on a card: it begins a step, and leaves the record.
OtherOp(c) ==
    /\ \E k \in {"bury", "flag"} : Begin(k, c)
    /\ UNCHANGED <<record, offers, confirmed, reverted>>

\* A sync sends every row, so every op is marked synced.
Sync ==
    /\ \E i \in DOMAIN ops : ~ops[i].synced
    /\ ops' = [i \in DOMAIN ops |-> [ops[i] EXCEPT !.synced = TRUE]]
    /\ UNCHANGED <<counter, record, offers, confirmed, reverted>>

\* Opening or closing the collection forgets the record.
Forget ==
    /\ record # NONE
    /\ record' = NONE
    /\ UNCHANGED <<ops, counter, offers, confirmed, reverted>>

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

\* A confirmation arrives for an offer the record no longer names.
StaleConfirm(o) ==
    /\ OnConfirm
    /\ o \notin confirmed
    /\ o # record
    /\ confirmed' = confirmed \cup {o}
    /\ Write(o)
    /\ UNCHANGED <<counter, offers>>

\* Every step is spent, so the run may end.
Finished ==
    /\ counter = MaxSteps
    /\ UNCHANGED vars

Next ==
    \/ \E c \in Cards : Answer(c) \/ OtherOp(c)
    \/ Sync
    \/ Forget
    \/ Offer
    \/ \E o \in offers : Confirm(o) \/ StaleConfirm(o)
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

=============================================================================
