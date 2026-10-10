-------------------------- MODULE SensitiveDeckGate --------------------------
\* @phx covers crates/agent/src/duty.rs anchor=decide digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/ingest/src/sensitive.rs anchor=read_marked digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/ingest/src/sensitive.rs anchor=admits digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/readings/src/day_set.rs anchor=hold_back_sensitive digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/coordination/src/readings/resolve.rs anchor=resolve_study_day digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/daemon/src/wiring.rs anchor=judge digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx covers crates/daemon/src/wiring.rs anchor=judge_deck_scope digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
\* @phx cites #751
\* @phx property NoMarkedDeckIsSent ramp=report
\* @phx property AnUnreadableSetSendsNothing ramp=report
\* @phx property NoUnscopedCardIsSent ramp=report
\* @phx witness witness/a-gate-that-trusts-the-selection.cfg kills=NoMarkedDeckIsSent
\* @phx witness witness/an-unreadable-set-admits.cfg kills=AnUnreadableSetSendsNothing
\* @phx witness witness/cards-without-a-scope-are-admitted.cfg kills=NoUnscopedCardIsSent
(***************************************************************************)
\* A deck the learner keeps away from AI reaches no AI duty (#751). The marks are one shared set,
\* the `sensitive_decks` table of migration 038101, and the actors meet over it: the learner, who
\* marks and unmarks a deck, the erase, and two duty runs, each selecting its day set and then
\* asking the deck gate before its runner starts. The gate is a check followed by an act, so the
\* model holds the window between them.
\*
\* The actors, each a step:
\* - Mark, Unmark: sensitive.rs's mark and unmark, each one committed write; marking a marked deck
\*   and unmarking an unmarked one change nothing;
\* - Erase: data_rights.rs's erase deletes every mark in one write. Init is the migration's empty
\*   table: "no row means the deck is not marked, so every deck starts readable" (sensitive.rs
\*   module doc);
\* - Select: resolve.rs::resolve_study_day reads the marks first, through
\*   sensitive.rs::read_marked, and a failed read ends it ("[`ResolveError::Marks`] when the marks
\*   cannot be read ... nothing is recorded then"); day_set.rs::hold_back_sensitive then holds back
\*   "every card ingest's one rule refuses". The run's caller passes its cards with their deck
\*   scope, or with none;
\* - Gate: duty.rs::decide withholds cards that carry no scope ("Cards with no scope cannot be
\*   judged, so the run is withheld unread"), then asks the gate, which reads the marks again at
\*   its judgement (wiring.rs::judge, `self.marks.read_marked().await.ok()`) and judges every card
\*   by sensitive.rs::admits (wiring.rs::judge_deck_scope: "any card that cannot be judged makes
\*   the scope unreadable, else any card kept away makes it kept away, else it is admitted");
\* - Send: the runner starts with the run's cards ("The gate stands before the input gate,
\*   `compose` and the runner", duty.rs::decide).
\*
\* What is abstracted:
\* - two decks, one under the other, and a filtered deck: a card's home is one of the two, and it
\*   sits in its home or in the filtered deck, so admits' ancestor walk, its home deck and its
\*   current deck each decide some card. A deck missing from the tree is not judged, as an
\*   unreadable set is not (judge_deck_scope), and is left out: the Lean entry
\*   formal/lean/Formal/SensitiveDeck.lean proves admits refuses it;
\* - the queue offers every card: a smaller day set sends a subset of what this one sends;
\* - a run's scope is its every card or none: a caller whose scope is narrower than its cards is
\*   SPEC-381's named risk, owed by each later duty's own SPEC;
\* - the input gate, compose and the output gate only withhold more, and are left out;
\* - a read is atomic: it sees the marks as the last committed write left them.
\*
\* A mark that commits between a run's gate read and its runner's start does not stop that run:
\* SPEC-381 names that window, and NoMarkedDeckIsSent judges a run by the marks at its gate read.
\*
\* Three defect switches, each FALSE in a witness: GateRereads (the gate reads the marks at its
\* judgement, not only at the selection), UnreadableRefuses (a failed read refuses, and never reads
\* as no mark) and ScopeCheck (decide withholds cards that carry no scope).
(***************************************************************************)
EXTENDS Naturals, FiniteSets

CONSTANTS GateRereads, UnreadableRefuses, ScopeCheck, NRuns

Runs == 1..NRuns
Decks == {"parent", "child", "filtered"}

\* A card: its home deck, and the deck it sits in now.
Cards == {card \in [home : {"parent", "child"}, current : Decks] :
            card.current \in {card.home, "filtered"}}

\* sensitive.rs::under: the deck is the ancestor, or sits under it.
Under(deck, ancestor) == deck = ancestor \/ (deck = "child" /\ ancestor = "parent")

\* sensitive.rs::admits over a set it could read: a card is kept away when its home deck or its
\* current deck, or an ancestor of either, is marked.
KeptAway(card, m) == \E a \in m : Under(card.home, a) \/ Under(card.current, a)

VARIABLES
    marked,      \* the committed marks
    pc,          \* each run's place: "wait", "picked" (selected), "admitted" (by its gate), "done"
    cards,       \* each run's cards, as its selection left them
    scoped,      \* whether the run's caller passed the cards' deck scope
    selSaw,      \* the marks each run's selection judged by
    gateSaw,     \* history: the committed marks at the gate read of each run its gate admitted
    readFailed,  \* history: a read of the marks failed for the run
    sent         \* the cards each run's runner started with

vars == <<marked, pc, cards, scoped, selSaw, gateSaw, readFailed, sent>>

TypeOK ==
    /\ marked \subseteq Decks
    /\ pc \in [Runs -> {"wait", "picked", "admitted", "done"}]
    /\ cards \in [Runs -> SUBSET Cards]
    /\ scoped \in [Runs -> BOOLEAN]
    /\ selSaw \in [Runs -> SUBSET Decks]
    /\ gateSaw \in [Runs -> SUBSET Decks]
    /\ readFailed \in [Runs -> BOOLEAN]
    /\ sent \in [Runs -> SUBSET Cards]

Init ==
    /\ marked = {}
    /\ pc = [r \in Runs |-> "wait"]
    /\ cards = [r \in Runs |-> {}]
    /\ scoped = [r \in Runs |-> TRUE]
    /\ selSaw = [r \in Runs |-> {}]
    /\ gateSaw = [r \in Runs |-> {}]
    /\ readFailed = [r \in Runs |-> FALSE]
    /\ sent = [r \in Runs |-> {}]

\* sensitive.rs's mark: one committed write.
Mark(d) ==
    /\ marked' = marked \cup {d}
    /\ UNCHANGED <<pc, cards, scoped, selSaw, gateSaw, readFailed, sent>>

\* sensitive.rs's unmark: one committed write.
Unmark(d) ==
    /\ marked' = marked \ {d}
    /\ UNCHANGED <<pc, cards, scoped, selSaw, gateSaw, readFailed, sent>>

\* data_rights.rs's erase: every mark goes in one write.
Erase ==
    /\ marked' = {}
    /\ UNCHANGED <<pc, cards, scoped, selSaw, gateSaw, readFailed, sent>>

\* resolve.rs::resolve_study_day, then day_set.rs::hold_back_sensitive. A failed read ends the
\* resolution with its named error and no run; with the defect it reads as no mark.
Select(r) ==
    /\ pc[r] = "wait"
    /\ \E ok \in BOOLEAN :
         IF ok \/ ~UnreadableRefuses
         THEN LET m == IF ok THEN marked ELSE {}
              IN \E s \in BOOLEAN :
                   /\ cards' = [cards EXCEPT ![r] = {c \in Cards : ~KeptAway(c, m)}]
                   /\ scoped' = [scoped EXCEPT ![r] = s]
                   /\ selSaw' = [selSaw EXCEPT ![r] = m]
                   /\ readFailed' = [readFailed EXCEPT ![r] = ~ok]
                   /\ pc' = [pc EXCEPT ![r] = "picked"]
         ELSE /\ readFailed' = [readFailed EXCEPT ![r] = TRUE]
              /\ pc' = [pc EXCEPT ![r] = "done"]
              /\ UNCHANGED <<cards, scoped, selSaw>>
    /\ UNCHANGED <<marked, gateSaw, sent>>

\* duty.rs::decide: the scope check, then the gate's read and judgement (wiring.rs::judge and
\* judge_deck_scope). A failed read is no set: every card of the scope is not judged, so a
\* scope with a card is unreadable; with the defect the failed read is no mark. With the other
\* defect the gate judges by the selection's marks and reads nothing.
Gate(r) ==
    /\ pc[r] = "picked"
    /\ IF ScopeCheck /\ ~scoped[r] /\ cards[r] # {}
       THEN /\ pc' = [pc EXCEPT ![r] = "done"]
            /\ UNCHANGED <<gateSaw, readFailed>>
       ELSE \E ok \in (IF GateRereads THEN BOOLEAN ELSE {TRUE}) :
              LET m == IF ~GateRereads THEN selSaw[r] ELSE IF ok THEN marked ELSE {}
                  scope == IF scoped[r] THEN cards[r] ELSE {}
                  admitted == IF ~ok /\ UnreadableRefuses
                              THEN scope = {}
                              ELSE \A c \in scope : ~KeptAway(c, m)
              IN /\ gateSaw' = [gateSaw EXCEPT ![r] = IF admitted THEN marked ELSE @]
                 /\ readFailed' = [readFailed EXCEPT ![r] = @ \/ ~ok]
                 /\ pc' = [pc EXCEPT ![r] = IF admitted THEN "admitted" ELSE "done"]
    /\ UNCHANGED <<marked, cards, scoped, selSaw, sent>>

\* The runner starts with the run's cards.
Send(r) ==
    /\ pc[r] = "admitted"
    /\ sent' = [sent EXCEPT ![r] = cards[r]]
    /\ pc' = [pc EXCEPT ![r] = "done"]
    /\ UNCHANGED <<marked, cards, scoped, selSaw, gateSaw, readFailed>>

Next ==
    \/ \E d \in Decks : Mark(d) \/ Unmark(d)
    \/ Erase
    \/ \E r \in Runs : Select(r) \/ Gate(r) \/ Send(r)

Spec == Init /\ [][Next]_vars

\* A deck marked before a run's gate read is never in what that run sends: the gate "reads the marks
\* at every judgement, so a mark made after a day's selection still holds for the run"
\* (wiring.rs::MarksDeckGate doc; wiring.rs::judge).
NoMarkedDeckIsSent == \A r \in Runs : \A c \in sent[r] : ~KeptAway(c, gateSaw[r])

\* A run whose read of the marks failed sends nothing: "A failed read is judged as an unreadable
\* set, which `admits` refuses, so the gate fails closed" (wiring.rs::MarksDeckGate doc), and a
\* failed read at the selection resolves no day (resolve.rs::resolve_study_day).
AnUnreadableSetSendsNothing == \A r \in Runs : readFailed[r] => sent[r] = {}

\* Cards that carry no scope are never sent: "Cards with no scope cannot be judged, so the run is
\* withheld unread" (duty.rs::decide).
NoUnscopedCardIsSent == \A r \in Runs : ~scoped[r] => sent[r] = {}

\* Reachability, beside the witnesses: a run sends cards; the marks move between a run's selection
\* and its gate read, and the run still sends; a mark lands after a run's gate read on a card it
\* sent (the named window); a read fails.
ACardIsSent == \E r \in Runs : sent[r] # {}
TheMarksMoveBeforeTheGate == \E r \in Runs : sent[r] # {} /\ gateSaw[r] # selSaw[r]
AMarkLandsAfterTheGate == \E r \in Runs : \E c \in sent[r] : KeptAway(c, marked)
AReadFails == \E r \in Runs : readFailed[r]

=============================================================================
