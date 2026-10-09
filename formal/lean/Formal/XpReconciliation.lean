-- @phx covers crates/progression/src/settle.rs anchor=settle digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/coordination/src/recompute/xp.rs anchor=evaluate digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/xp/src/review_xp.rs anchor=review_xp digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/progression/src/review_xp.rs anchor=review_xp digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx vectors formal/vectors/xp-reconciliation.jsonl
-- @phx cites #755, #639

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

/-- `settle`'s rule over the row it holds, the request and the cause, here WITHOUT the closed-day
arm: every case answers the request, the rule ADR-072 replaced. -/
def settleRule : Option Row → Row → Cause → Row
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

end Formal.XpReconciliation
