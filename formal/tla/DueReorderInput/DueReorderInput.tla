--------------------------- MODULE DueReorderInput ---------------------------
\* @phx covers crates/web-engine/src/wasm.rs anchor=current_card digest=sha256:605adfade6242520ebc39f34b2852dc699ed1dcb1029ca71522fdef82edf8118
\* @phx covers crates/web-engine/src/wasm.rs anchor=first_queued digest=sha256:066ce8c542e98e05a351d8e1b51ad8d2781edcf69e574faf6fd1dc23150398e0
\* @phx covers crates/web-engine/src/wasm.rs anchor=rate digest=sha256:ec4876c9095cd9dda71a594741e87ecb2ec897e92871e3f696dbd610e8abe7db
\* @phx cites #824
\* @phx property PresentedIsTheStockDueSet ramp=report
\* @phx property NoScoreOrReorderWritesACardField ramp=report
\* @phx property OffIsTheStockOrder ramp=report
\* @phx property ScoresOfAnotherDayOrSnapshotNeverApply ramp=report
\* @phx witness witness/the-presented-queue-is-the-stock-due-set.cfg kills=PresentedIsTheStockDueSet
\* @phx witness witness/the-scorer-writes-no-card-field.cfg kills=NoScoreOrReorderWritesACardField
\* @phx witness witness/the-reorder-writes-no-card-field.cfg kills=NoScoreOrReorderWritesACardField
\* @phx witness witness/off-is-the-stock-order.cfg kills=OffIsTheStockOrder
\* @phx witness witness/scores-apply-only-on-their-own-day-and-snapshot.cfg kills=ScoresOfAnotherDayOrSnapshotNeverApply
\*
\* The reorder-only input of external due-card scores (#824; ADR-339). An external scorer reads a
\* snapshot of the collection, read-only, and publishes scores for the cards due on a study day.
\* The client builds that day's stock queue with the stock scheduler, and the review presents it
\* reordered by the scores when the input is switched on and the scores apply, or in stock order
\* otherwise. Between the scorer's read and its scores' arrival, and between one presentation and
\* the next, the owner answers cards, a lapsed card comes back, a card leaves the queue by another
\* path, the owner switches the input, the study day rolls over and a full sync replaces the
\* collection. Scores arrive late, or never.
\*
\* The model is written ahead of the reorder's code. It covers the items that exist and that the
\* reorder will change or must agree with: `current_card` and `first_queued` in
\* `crates/web-engine/src/wasm.rs` each present the head of the engine's `GetQueuedCards` (13,3),
\* and `rate` answers the shown card. The delivery that writes the reorder adds a cover for its
\* function in the commit that creates it, and re-stamps these.
\*
\* What the model abstracts, and why:
\* - A card is a number in `Cards`, and its stock fields (due date, interval, ease or difficulty,
\*   stability) are one value `fields[c]`. Any write to any of them changes that value, and each
\*   step that changes it adds its actor to `wrote`: the owner's answer, a full sync's download,
\*   the scorer, the reorder. `wrote` records only steps that changed a field.
\* - `stock` is the stock scheduler's queue for today, in its order, with no card twice. Which
\*   cards the scheduler makes due, and in what order, is its own decision and is not under test:
\*   a new study day and a full sync each choose any arrangement of any subset of the cards.
\* - A score is a level in `0..MaxScore`, and 0 is no score. The Lean entry `lean/DueReorder`
\*   proves that an invalid score (not finite, or outside [0, 1]) is read as missing, so the model
\*   starts from the levels the reorder sees. `Reorder` is the rule that entry proves: the scored
\*   cards by score, highest first and stable, then the unscored ones in stock order.
\* - The scorer holds at most one run in flight (`pending`), keyed by the study day and the
\*   snapshot id it read. A new run replaces one in flight. Its scores arrive at any later step,
\*   after a rollover or a full sync included, or never.
\* - The input the client holds is `inbox`: the key the scorer read, ghost when `Keyed` is off,
\*   and the scores. `appliedKey` is a ghost: the key of the scores that ordered the queue now
\*   presented, or `NOKEY` when the queue is in stock order.
\* - A study day is a number that only grows, and a snapshot id is `gen`, which every full sync
\*   advances. Which value the scorer and the client each compute for it is left to the design.
\* - An answer is Good or Again on the presented head. Again puts the card in `relearn`, from which
\*   it may come back into the queue at any position the stock rules pick. A card leaving the queue
\*   by another path (a sibling's bury, a suspend) is `Leave`.
\*
\* The switches each turn one design choice off, and each witness is one of them off:
\* - `Keyed`: the client applies scores only when their study day and snapshot id equal today's.
\*   Off, it is the input ADR-339 decides at the merge-base, "for each card due today, an identifier
\*   and a score", which names neither, so the client cannot tell stale scores from fresh ones.
\* - `Derived`: the presented queue is derived from the stock queue at every step, as
\*   `current_card` derives its head today. Off, the reordered queue is computed when scores arrive
\*   or a session starts, and kept, popping on each answer.
\* - `OffRestores`: switching the input off re-presents the stock order at once. Off, the reordered
\*   queue stays until the next step.
\* - `ScorerReadsOnly` and `ReorderWritesNothing`: the scorer's read and the reorder each write no
\*   card field. Off, a planted scorer writes every card's fields, and a planted reorder moves the
\*   top-scored card's due date to today: ADR-339's refused "model that writes a due date".

EXTENDS Integers, Sequences, FiniteSets

CONSTANTS NCards, MaxDay, MaxGen, MaxScore, Keyed, Derived, OffRestores, ScorerReadsOnly,
          ReorderWritesNothing

ASSUME NCards \in Nat \ {0} /\ MaxDay \in Nat \ {0} /\ MaxGen \in Nat /\ MaxScore \in Nat \ {0}
ASSUME {Keyed, Derived, OffRestores, ScorerReadsOnly, ReorderWritesNothing} \subseteq BOOLEAN

Cards == 1..NCards
Keys == (1..MaxDay) \X (0..MaxGen)
NOKEY == <<0, 0>>
Scores == [Cards -> 0..MaxScore]
Actors == {"owner", "sync", "scorer", "reorder"}

Range(s) == {s[i] : i \in DOMAIN s}
Distinct(s) == \A i, j \in DOMAIN s : i # j => s[i] # s[j]
Arrangements == {s \in UNION {[1..n -> Cards] : n \in 0..NCards} : Distinct(s)}
Remove(s, c) == SelectSeq(s, LAMBDA x : x # c)
Insert(s, c, i) == SubSeq(s, 1, i - 1) \o <<c>> \o SubSeq(s, i, Len(s))

\* The scored cards by score, highest first, each level in stock order; then the unscored ones.
RECURSIVE ByScore(_, _, _)
ByScore(q, sc, s) ==
    IF s = 0 THEN SelectSeq(q, LAMBDA c : sc[c] = 0)
    ELSE SelectSeq(q, LAMBDA c : sc[c] = s) \o ByScore(q, sc, s - 1)
Reorder(q, sc) == ByScore(q, sc, MaxScore)

\* The input applies: switched on, scores held, and (keyed) made for today's day and snapshot.
Applies(o, ib, d, g) == o /\ ib.key # NOKEY /\ (Keyed => ib.key = <<d, g>>)
Shown(q, o, ib, d, g) == IF Applies(o, ib, d, g) THEN Reorder(q, ib.score) ELSE q
KeyOf(o, ib, d, g) == IF Applies(o, ib, d, g) THEN ib.key ELSE NOKEY

VARIABLES day, gen, stock, relearn, on, inbox, pending, presented, appliedKey, fields, wrote

vars == <<day, gen, stock, relearn, on, inbox, pending, presented, appliedKey, fields, wrote>>

TypeOK ==
    /\ day \in 1..MaxDay
    /\ gen \in 0..MaxGen
    /\ stock \in Arrangements
    /\ relearn \subseteq Cards
    /\ on \in BOOLEAN
    /\ inbox \in [key : Keys \cup {NOKEY}, score : Scores]
    /\ pending \in Keys \cup {NOKEY}
    /\ presented \in Arrangements
    /\ appliedKey \in Keys \cup {NOKEY}
    /\ fields \in [Cards -> 0..MaxDay + 1]
    /\ wrote \subseteq Actors

Init ==
    /\ day = 1
    /\ gen = 0
    /\ stock \in Arrangements
    /\ relearn = {}
    /\ on = FALSE
    /\ inbox = [key |-> NOKEY, score |-> [c \in Cards |-> 0]]
    /\ pending = NOKEY
    /\ presented = stock
    /\ appliedKey = NOKEY
    /\ fields = [c \in Cards |-> 0]
    /\ wrote = {}

\* The presented queue, derived from the stock queue and the input as they stand after the step.
Refresh ==
    /\ presented' = Shown(stock', on', inbox', day', gen')
    /\ appliedKey' = KeyOf(on', inbox', day', gen')

Kept == UNCHANGED <<presented, appliedKey>>

Wrote(actor) == wrote' = IF fields' = fields THEN wrote ELSE wrote \cup {actor}

\* The scorer reads the collection's snapshot for today: a read, which writes nothing.
Snapshot ==
    /\ pending' = <<day, gen>>
    /\ fields' = IF ScorerReadsOnly THEN fields ELSE [c \in Cards |-> MaxDay + 1]
    /\ Wrote("scorer")
    /\ UNCHANGED <<day, gen, stock, relearn, on, inbox, presented, appliedKey>>

\* The scores of the run in flight arrive, at any later step, and the review reorders.
Publish(sc) ==
    /\ pending # NOKEY
    /\ inbox' = [key |-> pending, score |-> sc]
    /\ pending' = NOKEY
    /\ UNCHANGED <<day, gen, stock, relearn, on>>
    /\ Refresh
    /\ fields' = IF ~ReorderWritesNothing /\ Applies(on, inbox', day, gen) /\ presented' # <<>>
                 THEN [fields EXCEPT ![Head(presented')] = day]
                 ELSE fields
    /\ Wrote("reorder")

\* The scores of the run in flight never arrive.
Lose ==
    /\ pending # NOKEY
    /\ pending' = NOKEY
    /\ UNCHANGED <<day, gen, stock, relearn, on, inbox, presented, appliedKey, fields, wrote>>

\* The owner answers the presented card (`rate`), and the engine writes its fields.
Answer(g) ==
    /\ presented # <<>>
    /\ stock' = Remove(stock, Head(presented))
    /\ relearn' = IF g = "again" THEN relearn \cup {Head(presented)} ELSE relearn
    /\ fields' = [fields EXCEPT ![Head(presented)] = IF g = "again" THEN day ELSE day + 1]
    /\ Wrote("owner")
    /\ UNCHANGED <<day, gen, on, inbox, pending>>
    /\ IF Derived
       THEN Refresh
       ELSE presented' = Tail(presented) /\ appliedKey' = appliedKey

\* A lapsed card comes back into today's queue, where the stock rules put it.
Reenter(c, i) ==
    /\ c \in relearn
    /\ i \in 1..Len(stock) + 1
    /\ stock' = Insert(stock, c, i)
    /\ relearn' = relearn \ {c}
    /\ UNCHANGED <<day, gen, on, inbox, pending, fields, wrote>>
    /\ IF Derived THEN Refresh ELSE Kept

\* A card leaves today's queue by another path: a sibling's bury, a suspend.
Leave(c) ==
    /\ c \in Range(stock)
    /\ stock' = Remove(stock, c)
    /\ UNCHANGED <<day, gen, relearn, on, inbox, pending, fields, wrote>>
    /\ IF Derived THEN Refresh ELSE Kept

\* The owner switches the input on or off.
Toggle ==
    /\ on' = ~on
    /\ UNCHANGED <<day, gen, stock, relearn, inbox, pending, fields, wrote>>
    /\ IF on /\ ~OffRestores THEN Kept ELSE Refresh

\* The study day rolls over, and the stock scheduler builds the new day's queue.
NewDay ==
    /\ day < MaxDay
    /\ day' = day + 1
    /\ stock' \in Arrangements
    /\ relearn' = {}
    /\ UNCHANGED <<gen, on, inbox, pending, fields, wrote>>
    /\ Refresh

\* A full sync replaces the collection: a new snapshot, new fields, a new stock queue.
FullSync ==
    /\ gen < MaxGen
    /\ gen' = gen + 1
    /\ stock' \in Arrangements
    /\ relearn' = {}
    /\ fields' \in [Cards -> 0..MaxDay + 1]
    /\ Wrote("sync")
    /\ UNCHANGED <<day, on, inbox, pending>>
    /\ Refresh

Next ==
    \/ Snapshot
    \/ \E sc \in Scores : Publish(sc)
    \/ Lose
    \/ \E g \in {"good", "again"} : Answer(g)
    \/ \E c \in Cards, i \in 1..NCards : Reenter(c, i)
    \/ \E c \in Cards : Leave(c)
    \/ Toggle
    \/ NewDay
    \/ FullSync

Spec == Init /\ [][Next]_vars

\* ADR-339: the reorder "never adds a card the stock scheduler did not make due and never removes
\* one it did". The presented queue holds exactly the cards of the stock queue, each once.
PresentedIsTheStockDueSet ==
    Len(presented) = Len(stock) /\ Range(presented) = Range(stock)

\* ADR-339: the reorder "never writes a due date". Only the owner's answer and a full sync's
\* download change a card's fields; the scorer's read and the reorder change none.
NoScoreOrReorderWritesACardField ==
    wrote \subseteq {"owner", "sync"}

\* Switched off, the review presents the stock order, at once.
OffIsTheStockOrder ==
    ~on => presented = stock

\* ADR-339: "missing or stale scores leave the stock order". Scores made for another study day or
\* another snapshot never order the presented queue.
ScoresOfAnotherDayOrSnapshotNeverApply ==
    appliedKey \in {NOKEY, <<day, gen>>}

=============================================================================
