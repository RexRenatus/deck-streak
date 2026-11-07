------------------------- MODULE AnswerOnTheShownState -------------------------
\* @phx covers crates/engine-core/src/dispatch.rs anchor=run_answer digest=sha256:e6b7ab332fee33fec2200576fb150ffe2e10800f25ffee4a5c6091b04102c857
\* @phx covers crates/engine-core/src/answer.rs anchor=checked digest=sha256:3e51786073908ab8ca69f25949b2f31f74c60357f8afbe31bbb4d448bd76f499
\* @phx covers crates/engine-core/src/answer.rs anchor=answer_request digest=sha256:afa913d06fa456f1f5af0776a8d5a8f77e0643345182a6dd87835dce0b44575b
\* @phx cites #760
\* @phx property AnAnswerCarriesTheStateItWasShown ramp=report
\* @phx property NoAnswerIsRecordedOnAChangedCard ramp=report
\* @phx witness witness/an-answer-carries-the-state-it-was-shown.cfg kills=AnAnswerCarriesTheStateItWasShown
\* @phx witness witness/no-answer-is-recorded-on-a-changed-card.cfg kills=NoAnswerIsRecordedOnAChangedCard
\*
\* An owner's answer is recorded on the state its card was shown with, or refused (SPEC-460,
\* ADR-477). A press shows the card with the states the queue gives it, the adapter writes the
\* answer's request from those states, and the core's one door hands the checked request to the
\* engine, which compares the card's current scheduling state with the request's and records the
\* answer only when they are equal. Between a press's show and its answer, another caller can
\* change the card: another press's answer, a gesture's write such as a due date, or a sync.
\*
\* What the model abstracts, and why:
\* - `card` is the card's scheduling state as the engine computes it from the card's row at
\*   answer time. `States` are those computed states; a write that leaves the computed state equal
\*   (a bury or a flag of a new or review card, ADR-477 D1) is no step here.
\* - Each press shows, sends and is answered once. `shown` and `sent` are the press's shown state
\*   and the current state its request carries; before its show they hold the card's first state
\*   and no action or property reads them.
\* - The engine's comparison and its write are one action. Its backend lock was not read
\*   (ADR-477, "What would make this wrong"): if another caller's write can land between the two,
\*   `Engine` splits into a check and a write.
\* - `recorded` and `refused` keep each answer the engine judged, with the state its press was
\*   shown, the state its request sent and the card's state at the engine's step.
\* - `ReadAtAnswer` is ADR-376 D3's rejected option, the core reading the engine's own state at
\*   answer time and sending it in place of the shown one (row S46001's mutant). `EngineCompares`
\*   switched off is an engine that writes without the comparison.

EXTENDS Naturals

CONSTANTS Presses, States, MaxChanges, ReadAtAnswer, EngineCompares

VARIABLES card, pc, shown, sent, recorded, refused, changes

vars == <<card, pc, shown, sent, recorded, refused, changes>>

Places == {"idle", "shown", "sent", "done"}

Judged == [press : Presses, shown : States, sent : States, at : States]

TypeOK ==
    /\ card \in States
    /\ pc \in [Presses -> Places]
    /\ shown \in [Presses -> States]
    /\ sent \in [Presses -> States]
    /\ recorded \subseteq Judged
    /\ refused \subseteq Judged
    /\ changes \in 0..MaxChanges

Init ==
    /\ card \in States
    /\ pc = [p \in Presses |-> "idle"]
    /\ shown = [p \in Presses |-> card]
    /\ sent = [p \in Presses |-> card]
    /\ recorded = {}
    /\ refused = {}
    /\ changes = 0

\* The queue shows the card (SchedulerService.GetQueuedCards) and the adapter keeps the states it
\* was shown with: the native app passes them back to `Engine::answer`, the web `rate` reads them
\* from `SHOWN`, and the test keeps them.
Show(p) ==
    /\ pc[p] = "idle"
    /\ shown' = [shown EXCEPT ![p] = card]
    /\ pc' = [pc EXCEPT ![p] = "shown"]
    /\ UNCHANGED <<card, sent, recorded, refused, changes>>

\* The core's door: `answer.rs::answer_request` writes the request's current state from the shown
\* states, and `dispatch.rs::run_answer` runs `answer.rs::checked`, which compares only the card
\* and the rating and encodes the request again, so the current state reaches the engine as it was
\* written. With `ReadAtAnswer`, the door sends the card's state at this step instead.
Send(p) ==
    /\ pc[p] = "shown"
    /\ sent' = [sent EXCEPT ![p] = IF ReadAtAnswer THEN card ELSE shown[p]]
    /\ pc' = [pc EXCEPT ![p] = "sent"]
    /\ UNCHANGED <<card, shown, recorded, refused, changes>>

\* The engine's check and write, one action: the engine's `answer_card_inner`
\* (`rslib/src/scheduler/answering/mod.rs`) computes the card's current state, refuses unless it
\* equals the request's (`card was modified`), and only then applies the answer, which moves the
\* card to another state. Without `EngineCompares` it records whatever was sent.
Engine(p) ==
    /\ pc[p] = "sent"
    /\ pc' = [pc EXCEPT ![p] = "done"]
    /\ LET judged == [press |-> p, shown |-> shown[p], sent |-> sent[p], at |-> card]
       IN IF ~EngineCompares \/ card = sent[p]
          THEN /\ recorded' = recorded \cup {judged}
               /\ card' \in States \ {card}
               /\ refused' = refused
          ELSE /\ refused' = refused \cup {judged}
               /\ UNCHANGED <<card, recorded>>
    /\ UNCHANGED <<shown, sent, changes>>

\* Another caller between a show and its answer: another device's answer, a gesture's write such
\* as `ExemptWrite::SetDueDate`, or a sync, at most `MaxChanges` times.
Change ==
    /\ changes < MaxChanges
    /\ card' \in States \ {card}
    /\ changes' = changes + 1
    /\ UNCHANGED <<pc, shown, sent, recorded, refused>>

\* Every press answered or refused: the run has ended.
Finished ==
    /\ \A p \in Presses : pc[p] = "done"
    /\ UNCHANGED vars

Next ==
    \/ \E p \in Presses : Show(p) \/ Send(p) \/ Engine(p)
    \/ Change
    \/ Finished

Spec == Init /\ [][Next]_vars

\* ADR-376 D3: the core sends the engine the state the card was shown with, and reads nothing from
\* the collection at answer time.
AnAnswerCarriesTheStateItWasShown == \A r \in recorded : r.sent = r.shown

\* ADR-477 D1: the engine records an answer only on the card's state its request carries, so no
\* answer is recorded on a card that changed after it was shown.
NoAnswerIsRecordedOnAChangedCard == \A r \in recorded : r.at = r.sent

=============================================================================
