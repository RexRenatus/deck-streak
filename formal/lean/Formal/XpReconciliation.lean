-- @phx covers crates/progression/src/settle.rs anchor=settle digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/coordination/src/recompute/xp.rs anchor=evaluate digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/xp/src/review_xp.rs anchor=review_xp digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/progression/src/review_xp.rs anchor=review_xp digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx vectors formal/vectors/xp-reconciliation.jsonl
-- @phx cites #755, #639
-- @phx theorem a_closed_row_is_never_lowered_by_a_recompute ramp=report
-- @phx witness a_recompute_that_replaces_a_closed_row_violates kills=a_closed_row_is_never_lowered_by_a_recompute
-- @phx theorem the_close_keeps_the_last_provisional_amount ramp=report
-- @phx witness a_close_that_reads_only_the_held_flag_violates kills=the_close_keeps_the_last_provisional_amount
-- @phx theorem a_days_review_xp_never_falls_as_answers_arrive ramp=report
-- @phx witness a_total_that_wraps_at_the_top_violates kills=a_days_review_xp_never_falls_as_answers_arrive
-- @phx theorem confirmed_xp_is_never_below_the_xp_shown ramp=report
-- @phx witness a_device_that_shows_a_bonus_at_the_grade_violates kills=confirmed_xp_is_never_below_the_xp_shown
-- @phx witness a_shown_total_that_keeps_an_undone_answer_violates kills=confirmed_xp_is_never_below_the_xp_shown

/-! The XP floor (SPEC-389). A study day's shown XP is the device's saturating sum of the XP it
shows for its own answers of the day that are not undone, with no day-level bonus. Its confirmed XP
is the sum of the day's settled rows. At every settle point, every answer the device holds has
synced, and the last write to the day's rows came from facts holding every answer the server holds.

The server confirms XP in one row per study day, source and track, which `settle` decides
(`crates/progression/src/settle.rs`). A fold sums a day's review XP per track
(`crates/coordination/src/recompute/xp.rs`) and settles `reviews` and `reviews_law` with the
Recompute cause, then the day's bonuses. This module ports the settle rule and the per-track sum,
and models the protocol over traces of the device's grades, undos and syncs, other clients'
answers, the folds' reads and writes, and the day's close.

Two premises are stated, never derived. Each answer the device shows XP for shows at most the XP
the server prices it at (`review_xp`, covered by its digest). No event removes an answer from the
server's answers of the day: the model has no such event. -/

namespace Formal.XpReconciliation

/-! ## The settle rule -/

/-- A settled row: the XP it holds and whether the day was over when it was settled. -/
structure Row where
  amount : Nat
  closed : Bool
deriving DecidableEq, Repr

/-- `SettleCause`: the fold's recompute, or the owner's correction. -/
inductive Cause where
  | recompute
  | ownersCorrection
deriving DecidableEq, Repr

/-- `settle`'s rule (`settle.rs:117-122`), branch for branch, over the row it holds, the request
and the cause: a held row under a Recompute, when the held row or the request is closed, holds the
larger amount, closed; every other case answers the request. -/
def settleRule : Option Row → Row → Cause → Row
  | some held, request, .recompute =>
    if held.closed || request.closed then ⟨max held.amount request.amount, true⟩
    else ⟨request.amount, request.closed⟩
  | _, request, _ => ⟨request.amount, request.closed⟩

/-! ## The per-track sum -/

/-- The largest amount a `u32` row holds. -/
def u32Max : Nat := 4294967295

/-- `u32::saturating_add` over naturals that never exceed `u32Max`. -/
def satAdd (a b : Nat) : Nat := min (a + b) u32Max

/-- A track. -/
inductive Track where
  | language
  | law
deriving DecidableEq, Repr

/-- One study-event answer: the track the server files its card under (`none` when the server
does not know the card), the XP the device shows for it, and the XP the server prices it at. -/
structure Answer where
  track : Option Track
  shown : Nat
  price : Nat
deriving DecidableEq, Repr

/-- One answer's step of `evaluate`'s sum (`xp.rs:105-116`): an unknown card counts on the
language track; the law total takes a law answer's XP, the language total every other. -/
def tally (add : Nat → Nat → Nat) (totals : Nat × Nat) (answer : Answer) : Nat × Nat :=
  if answer.track.getD .language = .law then (totals.1, add totals.2 answer.price)
  else (add totals.1 answer.price, totals.2)

/-- `evaluate`'s per-track totals of a day's answers, language then law, each a saturating sum. -/
def trackTotal (answers : List Answer) : Nat × Nat :=
  answers.foldl (tally satAdd) (0, 0)

/-- The device's shown XP of its answers: their shown XP's saturating sum. -/
def shownTotal (answers : List Answer) : Nat :=
  answers.foldl (fun total answer => satAdd total answer.shown) 0

/-! ## The protocol -/

/-- The events of one study day. A fold is named by a number. -/
inductive Event where
  /-- The device grades an answer and shows its XP. -/
  | grade (answer : Answer)
  /-- The device undoes its last unsynced answer. -/
  | undo
  /-- The device syncs every unsynced answer. -/
  | sync
  /-- Another client's answer reaches the server. -/
  | other (answer : Answer)
  /-- A fold reads the server's answers. -/
  | read (fold : Nat)
  /-- A fold writes the day's rows from the answers it read, with a bonus amount. -/
  | write (fold : Nat) (bonus : Nat)
  /-- The day closes. -/
  | close
deriving DecidableEq, Repr

/-- One study day's state. -/
structure State where
  /-- The device's answers not yet synced, oldest first. -/
  unsynced : List Answer
  /-- The device's synced answers, oldest first. -/
  synced : List Answer
  /-- The server's answers of the day, in arrival order. -/
  server : List Answer
  /-- Each fold's last read. -/
  reads : List (Nat × List Answer)
  /-- The `reviews` row. -/
  reviews : Option Row
  /-- The `reviews_law` row. -/
  reviewsLaw : Option Row
  /-- The `score90` row, standing for the day's bonuses. -/
  score90 : Option Row
  /-- Whether the day is over. -/
  closed : Bool
  /-- The facts the last write came from, if any write happened. -/
  lastWrite : Option (List Answer)
deriving DecidableEq, Repr

/-- The day before any event. -/
def init : State := ⟨[], [], [], [], none, none, none, false, none⟩

/-- The three requests a write from `facts` settles: `reviews`, `reviews_law` and `score90`, each
with the day's closed flag. -/
def writeRequests (s : State) (facts : List Answer) (bonus : Nat) : Row × Row × Row :=
  (⟨(trackTotal facts).1, s.closed⟩, ⟨(trackTotal facts).2, s.closed⟩, ⟨bonus, s.closed⟩)

/-- A fold's write from `facts`: each of the three rows settled with the Recompute cause. -/
def written (s : State) (facts : List Answer) (bonus : Nat) : State :=
  { s with
    reviews := some (settleRule s.reviews (writeRequests s facts bonus).1 .recompute)
    reviewsLaw := some (settleRule s.reviewsLaw (writeRequests s facts bonus).2.1 .recompute)
    score90 := some (settleRule s.score90 (writeRequests s facts bonus).2.2 .recompute)
    lastWrite := some facts }

/-- One event. A grade after the close belongs to the next day, so it is the identity here. A
write by a fold that has read nothing is the identity. -/
def step (s : State) : Event → State
  | .grade answer => if s.closed then s else { s with unsynced := s.unsynced ++ [answer] }
  | .undo => { s with unsynced := s.unsynced.dropLast }
  | .sync =>
    { s with unsynced := [], synced := s.synced ++ s.unsynced, server := s.server ++ s.unsynced }
  | .other answer => { s with server := s.server ++ [answer] }
  | .read fold => { s with reads := (fold, s.server) :: s.reads.filter (fun r => r.1 != fold) }
  | .write fold bonus =>
    match s.reads.lookup fold with
    | none => s
    | some facts => written s facts bonus
  | .close => { s with closed := true }

/-- The state after a trace. -/
def run (trace : List Event) : State :=
  trace.foldl step init

/-- A settle point (SPEC-389 R3): no answer is unsynced, and the last write's facts are the
server's answers, none written counting as equal to none held. -/
def settlePoint (s : State) : Prop :=
  s.unsynced = [] ∧ s.lastWrite.getD [] = s.server

instance (s : State) : Decidable (settlePoint s) :=
  inferInstanceAs (Decidable (_ ∧ _))

/-- The XP a row holds, none for no row. -/
def amountOf : Option Row → Nat
  | some row => row.amount
  | none => 0

/-- The day's confirmed XP: the sum of its three rows. -/
def confirmed (s : State) : Nat :=
  amountOf s.reviews + amountOf s.reviewsLaw + amountOf s.score90

/-- The day's shown XP: the device's synced and unsynced answers, none undone. -/
def shownXp (s : State) : Nat :=
  shownTotal (s.synced ++ s.unsynced)

/-! ## T1 and T2: the settle rule's half of the rows' state machine -/

/-- A settle rule's type: the held row, the request and the cause, to the row held after. -/
abbrev Rule := Option Row → Row → Cause → Row

/-- T1's claim: a Recompute over a closed row holds at least the row's amount, and the row stays
closed. -/
def ClosedNeverLowered (rule : Rule) : Prop :=
  ∀ held request, held.closed = true →
    held.amount ≤ (rule (some held) request .recompute).amount ∧
      (rule (some held) request .recompute).closed = true

/-- T2's claim: the close's Recompute over an open row holds at least the row's last provisional
amount, and closes it. -/
def CloseKeepsProvisional (rule : Rule) : Prop :=
  ∀ held request, held.closed = false → request.closed = true →
    held.amount ≤ (rule (some held) request .recompute).amount ∧
      (rule (some held) request .recompute).closed = true

theorem a_closed_row_is_never_lowered_by_a_recompute : ClosedNeverLowered settleRule := by
  intro held request closed
  simp [settleRule, closed, Nat.le_max_left]

theorem the_close_keeps_the_last_provisional_amount : CloseKeepsProvisional settleRule := by
  intro held request _ closing
  simp [settleRule, closing, Nat.le_max_left]

/-- The rule ADR-072 replaced: every settle answers the request. -/
def replacingRule : Rule :=
  fun _ request _ => ⟨request.amount, request.closed⟩

/-- A close that reads only the held row's flag. -/
def heldFlagRule : Rule
  | some held, request, .recompute =>
    if held.closed then ⟨max held.amount request.amount, true⟩
    else ⟨request.amount, request.closed⟩
  | _, request, _ => ⟨request.amount, request.closed⟩

/-- A closed row of 50 under a Recompute request of 30, open: the replacing rule holds 30. -/
theorem a_recompute_that_replaces_a_closed_row_violates : ¬ ClosedNeverLowered replacingRule := by
  intro claim
  have := (claim ⟨50, true⟩ ⟨30, false⟩ rfl).1
  revert this
  decide

/-- An open row of 50 under a closing Recompute request of 30: the rule that reads only the held
flag holds 30. -/
theorem a_close_that_reads_only_the_held_flag_violates :
    ¬ CloseKeepsProvisional heldFlagRule := by
  intro claim
  have := (claim ⟨50, false⟩ ⟨30, true⟩ rfl rfl).1
  revert this
  decide

/-! ## T3: a day's review XP never falls as answers arrive -/

/-- T3's claim, over a fold of the day's answers to its language and law totals: neither total
falls when an answer arrives. -/
def NeverFalls (total : List Answer → Nat × Nat) : Prop :=
  ∀ answers answer, (total answers).1 ≤ (total (answers ++ [answer])).1 ∧
    (total answers).2 ≤ (total (answers ++ [answer])).2

theorem satAdd_le (a b : Nat) : satAdd a b ≤ u32Max :=
  Nat.min_le_right _ _

/-- A saturating add never lowers a total within the maximum. -/
theorem le_satAdd (total price : Nat) (h : total ≤ u32Max) : total ≤ satAdd total price :=
  Nat.le_min.mpr ⟨Nat.le_add_right _ _, h⟩

theorem tally_bounded (totals : Nat × Nat) (answer : Answer) (h1 : totals.1 ≤ u32Max)
    (h2 : totals.2 ≤ u32Max) :
    (tally satAdd totals answer).1 ≤ u32Max ∧ (tally satAdd totals answer).2 ≤ u32Max := by
  unfold tally
  split <;> simp [h1, h2, satAdd_le]

theorem foldl_tally_bounded :
    ∀ (answers : List Answer) (totals : Nat × Nat), totals.1 ≤ u32Max → totals.2 ≤ u32Max →
      (answers.foldl (tally satAdd) totals).1 ≤ u32Max ∧
        (answers.foldl (tally satAdd) totals).2 ≤ u32Max
  | [], _, h1, h2 => ⟨h1, h2⟩
  | answer :: rest, totals, h1, h2 =>
    have h := tally_bounded totals answer h1 h2
    foldl_tally_bounded rest (tally satAdd totals answer) h.1 h.2

theorem trackTotal_bounded (answers : List Answer) :
    (trackTotal answers).1 ≤ u32Max ∧ (trackTotal answers).2 ≤ u32Max :=
  foldl_tally_bounded answers (0, 0) (by decide) (by decide)

theorem a_days_review_xp_never_falls_as_answers_arrive : NeverFalls trackTotal := by
  intro answers answer
  have h := trackTotal_bounded answers
  have last : trackTotal (answers ++ [answer]) = tally satAdd (trackTotal answers) answer := by
    simp [trackTotal, List.foldl_append]
  rw [last]
  generalize trackTotal answers = totals at h ⊢
  obtain ⟨language, law⟩ := totals
  simp only [tally]
  split
  · exact ⟨Nat.le_refl _, le_satAdd _ _ h.2⟩
  · exact ⟨le_satAdd _ _ h.1, Nat.le_refl _⟩

/-- A wrapping add, `u32::wrapping_add`. -/
def wrapAdd (a b : Nat) : Nat := (a + b) % (u32Max + 1)

/-- The per-track fold with a wrapping total. -/
def wrappingTotal (answers : List Answer) : Nat × Nat :=
  answers.foldl (tally wrapAdd) (0, 0)

/-- Answers of 4294967295, then 4294967295 and 1: the wrapping language total falls to 0. -/
theorem a_total_that_wraps_at_the_top_violates : ¬ NeverFalls wrappingTotal := by
  intro claim
  have := (claim [⟨some .language, 4294967295, 4294967295⟩] ⟨some .language, 1, 1⟩).1
  revert this
  decide

/-! ## T4: confirmed XP is never below the XP shown, at every settle point -/

/-- The premise on prices: each answer the device grades shows at most its server price. -/
def ShowsAtMostItsPrice (trace : List Event) : Prop :=
  ∀ answer, Event.grade answer ∈ trace → answer.shown ≤ answer.price

/-- T4's claim, over the device's shown XP as a function of its own log: at every settle point of
every trace whose grades each show at most their price, shown XP is at most confirmed XP. -/
def Floor (shown : List Event → Nat) : Prop :=
  ∀ trace, ShowsAtMostItsPrice trace → settlePoint (run trace) →
    shown trace ≤ confirmed (run trace)

/-- The device's shown XP after a trace: its synced and unsynced answers, none undone. -/
def deviceShown (trace : List Event) : Nat :=
  shownXp (run trace)

/-- A Recompute settle holds at least its request. -/
theorem recompute_holds_its_request (held : Option Row) (request : Row) :
    request.amount ≤ (settleRule held request .recompute).amount := by
  cases held with
  | none => simp [settleRule]
  | some row =>
    simp only [settleRule]
    split <;> simp [Nat.le_max_right]

/-- The language part of an answer's price. -/
def languagePart (answer : Answer) : Nat :=
  if answer.track.getD .language = .law then 0 else answer.price

/-- The law part of an answer's price. -/
def lawPart (answer : Answer) : Nat :=
  if answer.track.getD .language = .law then answer.price else 0

theorem foldl_tally_eq :
    ∀ (answers : List Answer) (totals : Nat × Nat), totals.1 ≤ u32Max → totals.2 ≤ u32Max →
      answers.foldl (tally satAdd) totals =
        (min (totals.1 + (answers.map languagePart).sum) u32Max,
          min (totals.2 + (answers.map lawPart).sum) u32Max)
  | [], totals, h1, h2 => by
    simp only [List.foldl_nil, List.map_nil, List.sum_nil, Nat.add_zero]
    ext <;> simp <;> omega
  | answer :: rest, totals, h1, h2 => by
    have h := tally_bounded totals answer h1 h2
    rw [List.foldl_cons, foldl_tally_eq rest _ h.1 h.2]
    simp only [tally, languagePart, lawPart, List.map_cons, List.sum_cons]
    split <;> simp [satAdd] <;> omega

/-- Each saturating total is the least of its sum and the `u32` maximum. -/
theorem trackTotal_eq (answers : List Answer) :
    trackTotal answers =
      (min (answers.map languagePart).sum u32Max, min (answers.map lawPart).sum u32Max) := by
  rw [trackTotal, foldl_tally_eq answers (0, 0) (by decide) (by decide)]
  simp

theorem foldl_shown_eq :
    ∀ (answers : List Answer) (total : Nat), total ≤ u32Max →
      answers.foldl (fun total answer => satAdd total answer.shown) total =
        min (total + (answers.map (·.shown)).sum) u32Max
  | [], total, h => by simp; omega
  | answer :: rest, total, h => by
    rw [List.foldl_cons, foldl_shown_eq rest _ (satAdd_le _ _)]
    simp [satAdd]
    omega

/-- The device's shown total is the least of its sum and the `u32` maximum. -/
theorem shownTotal_eq (answers : List Answer) :
    shownTotal answers = min (answers.map (·.shown)).sum u32Max := by
  rw [shownTotal, foldl_shown_eq answers 0 (by decide)]
  simp

theorem price_splits :
    ∀ answers : List Answer,
      (answers.map (·.price)).sum = (answers.map languagePart).sum + (answers.map lawPart).sum
  | [] => by simp
  | answer :: rest => by
    simp only [List.map_cons, List.sum_cons, price_splits rest, languagePart, lawPart]
    split <;> omega

theorem sum_le_of_sublist {f : Answer → Nat} :
    ∀ {small large : List Answer}, List.Sublist small large →
      (small.map f).sum ≤ (large.map f).sum
  | _, _, .slnil => by simp
  | _, _, .cons _ h => by
    have := sum_le_of_sublist (f := f) h
    simp only [List.map_cons, List.sum_cons]
    omega
  | _, _, .cons_cons _ h => by
    have := sum_le_of_sublist (f := f) h
    simp only [List.map_cons, List.sum_cons]
    omega

theorem shown_le_price :
    ∀ answers : List Answer, (∀ answer ∈ answers, answer.shown ≤ answer.price) →
      (answers.map (·.shown)).sum ≤ (answers.map (·.price)).sum
  | [], _ => by simp
  | answer :: rest, h => by
    have head := h answer (List.mem_cons_self ..)
    have tail := shown_le_price rest (fun a ha => h a (List.mem_cons_of_mem _ ha))
    simp only [List.map_cons, List.sum_cons]
    omega

/-- What the floor rests on, in every reachable state of a trace whose grades show at most their
price. -/
structure Invariant (s : State) : Prop where
  /-- The synced answers form a sublist of the server's answers. -/
  synced_sublist : List.Sublist s.synced s.server
  /-- Each answer the device holds shows at most its price. -/
  shows_at_most : ∀ answer ∈ s.synced ++ s.unsynced, answer.shown ≤ answer.price
  /-- The rows hold at least the totals of the last write's facts. -/
  rows_hold : ∀ facts, s.lastWrite = some facts →
    (trackTotal facts).1 ≤ amountOf s.reviews ∧ (trackTotal facts).2 ≤ amountOf s.reviewsLaw

theorem init_invariant : Invariant init where
  synced_sublist := List.Sublist.slnil
  shows_at_most := by simp [init]
  rows_hold := by simp [init]

theorem step_invariant (s : State) (event : Event) (hs : Invariant s)
    (price : ∀ answer, event = .grade answer → answer.shown ≤ answer.price) :
    Invariant (step s event) := by
  cases event with
  | grade answer =>
    simp only [step]
    split
    · exact hs
    · exact {
        synced_sublist := hs.synced_sublist
        shows_at_most := by
          intro a ha
          have held : a ∈ s.synced ++ s.unsynced ∨ a = answer := by
            simpa [or_assoc] using ha
          rcases held with ha | ha
          · exact hs.shows_at_most a ha
          · exact ha ▸ price answer rfl
        rows_hold := hs.rows_hold }
  | undo =>
    exact {
      synced_sublist := hs.synced_sublist
      shows_at_most := by
        intro a ha
        have held : a ∈ s.synced ∨ a ∈ s.unsynced.dropLast := by
          simpa [step] using ha
        rcases held with ha | ha
        · exact hs.shows_at_most a (List.mem_append_left _ ha)
        · exact hs.shows_at_most a (List.mem_append_right _ (List.dropLast_subset _ ha))
      rows_hold := hs.rows_hold }
  | sync =>
    exact {
      synced_sublist := hs.synced_sublist.append (List.Sublist.refl _)
      shows_at_most := by
        intro a ha
        exact hs.shows_at_most a (by simpa [step] using ha)
      rows_hold := hs.rows_hold }
  | other answer =>
    exact {
      synced_sublist := hs.synced_sublist.trans (List.sublist_append_left _ _)
      shows_at_most := hs.shows_at_most
      rows_hold := hs.rows_hold }
  | read fold =>
    exact {
      synced_sublist := hs.synced_sublist
      shows_at_most := hs.shows_at_most
      rows_hold := hs.rows_hold }
  | write fold bonus =>
    simp only [step]
    split
    · exact hs
    · rename_i facts _
      exact {
        synced_sublist := hs.synced_sublist
        shows_at_most := hs.shows_at_most
        rows_hold := by
          intro written_facts h
          simp only [written, Option.some.injEq] at h
          subst h
          simp only [written, amountOf, writeRequests]
          exact ⟨recompute_holds_its_request s.reviews ⟨(trackTotal facts).1, s.closed⟩,
            recompute_holds_its_request s.reviewsLaw ⟨(trackTotal facts).2, s.closed⟩⟩ }
  | close =>
    exact {
      synced_sublist := hs.synced_sublist
      shows_at_most := hs.shows_at_most
      rows_hold := hs.rows_hold }

theorem foldl_invariant :
    ∀ (trace : List Event) (s : State), Invariant s → ShowsAtMostItsPrice trace →
      Invariant (trace.foldl step s)
  | [], _, hs, _ => hs
  | event :: rest, s, hs, price =>
    foldl_invariant rest (step s event)
      (step_invariant s event hs fun answer h => price answer (h ▸ List.mem_cons_self ..))
      fun answer h => price answer (List.mem_cons_of_mem _ h)

/-- At a settle point, shown XP is at most confirmed XP. -/
theorem floor_at_a_settle_point (s : State) (hs : Invariant s) (point : settlePoint s) :
    shownXp s ≤ confirmed s := by
  obtain ⟨unsynced, last⟩ := point
  have shown := hs.shows_at_most
  simp only [unsynced, List.append_nil] at shown
  simp only [shownXp, unsynced, List.append_nil, shownTotal_eq, confirmed]
  cases h : s.lastWrite with
  | none =>
    simp only [h, Option.getD_none] at last
    have empty := List.sublist_nil.mp (last ▸ hs.synced_sublist)
    simp [empty]
  | some facts =>
    simp only [h, Option.getD_some] at last
    have rows := hs.rows_hold facts h
    rw [trackTotal_eq, last] at rows
    have bound := shown_le_price s.synced shown
    have within := sum_le_of_sublist (f := (·.price)) hs.synced_sublist
    have split := price_splits s.server
    simp only at rows
    omega

theorem confirmed_xp_is_never_below_the_xp_shown : Floor deviceShown :=
  fun trace price point =>
    floor_at_a_settle_point (run trace) (foldl_invariant trace init init_invariant price) point

/-- G, a language answer shown and priced at 7. -/
def languageSeven : Answer := ⟨some .language, 7, 7⟩

/-- A device that shows the day's bonus of 200 at its first grade. -/
def bonusShown (trace : List Event) : Nat :=
  satAdd (deviceShown trace)
    (if ((run trace).synced ++ (run trace).unsynced).isEmpty then 0 else 200)

/-- The answers a device keeps in its shown total when an undo does not leave it. -/
def keptAnswers : Bool → List Event → List Answer
  | _, [] => []
  | closed, .grade answer :: rest =>
    if closed then keptAnswers closed rest else answer :: keptAnswers closed rest
  | _, .close :: rest => keptAnswers true rest
  | closed, _ :: rest => keptAnswers closed rest

/-- A shown total that keeps an undone answer. -/
def keptShown (trace : List Event) : Nat :=
  shownTotal (keptAnswers false trace)

/-- Trace `GGSR`: the device shows 214, and the day confirms 14. -/
theorem a_device_that_shows_a_bonus_at_the_grade_violates : ¬ Floor bonusShown := by
  intro claim
  have := claim [.grade languageSeven, .grade languageSeven, .sync, .read 0, .write 0 0]
    (by intro answer h; simp only [List.mem_cons, List.mem_nil_iff, or_false] at h
        rcases h with h | h | h | h | h <;> cases h <;> decide)
    (by decide)
  revert this
  decide

/-- Trace `GUR`: the device shows 7, and the day confirms 0. -/
theorem a_shown_total_that_keeps_an_undone_answer_violates : ¬ Floor keptShown := by
  intro claim
  have := claim [.grade languageSeven, .undo, .read 0, .write 0 0]
    (by intro answer h; simp only [List.mem_cons, List.mem_nil_iff, or_false] at h
        rcases h with h | h | h | h <;> cases h <;> decide)
    (by decide)
  revert this
  decide

/-- Why a settle point needs its last clause: a write from facts a fold read before the last
arrival, landing after a write from newer facts, leaves confirmed XP below shown XP while every
answer has synced. -/
example :
    (run [.read 0, .grade languageSeven, .sync, .read 1, .write 1 0, .write 0 0]).unsynced = [] ∧
      confirmed (run [.read 0, .grade languageSeven, .sync, .read 1, .write 1 0, .write 0 0]) <
        shownXp (run [.read 0, .grade languageSeven, .sync, .read 1, .write 1 0, .write 0 0]) ∧
      ¬ settlePoint (run [.read 0, .grade languageSeven, .sync, .read 1, .write 1 0, .write 0 0]) := by
  decide

end Formal.XpReconciliation
