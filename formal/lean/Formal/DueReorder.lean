-- @phx covers crates/engine-core/src/reorder.rs anchor=reorder digest=sha256:53ecc76b432d129c28afe261e3fbd07105d0b8d4196fe6e1a8d38f45f2d99be6
-- @phx covers crates/engine-core/src/reorder.rs anchor=DueScores digest=sha256:119b477dcdc7359de9f6a2e7ed584dcf9cf451769c65be8ad129ca0b0ea3545c
-- @phx cites #824
-- @phx vectors formal/vectors/due-reorder.jsonl
-- @phx theorem the_reorder_is_a_permutation_of_the_stock_queue ramp=report
-- @phx witness stock_order_alone_leaves_a_scored_card_behind_for_the_permutation kills=the_reorder_is_a_permutation_of_the_stock_queue
-- @phx theorem every_entry_leaves_the_reorder_unchanged ramp=report
-- @phx witness stock_order_alone_leaves_a_scored_card_behind_for_the_entries kills=every_entry_leaves_the_reorder_unchanged
-- @phx theorem scored_entries_lead_by_score_then_stock_order ramp=report
-- @phx witness stock_order_alone_leaves_a_scored_card_behind_an_unscored_one kills=scored_entries_lead_by_score_then_stock_order
-- @phx theorem an_unused_input_keeps_the_stock_order_exactly ramp=report
-- @phx witness stock_order_alone_leaves_a_scored_card_behind_for_an_unused_input kills=an_unused_input_keeps_the_stock_order_exactly
-- @phx theorem an_invalid_score_counts_as_missing ramp=report
-- @phx witness stock_order_alone_leaves_a_scored_card_behind_for_an_invalid_score kills=an_invalid_score_counts_as_missing
-- @phx theorem reordering_twice_is_reordering_once ramp=report
-- @phx witness stock_order_alone_leaves_a_scored_card_behind_for_a_second_reorder kills=reordering_twice_is_reordering_once
-- @phx theorem the_order_depends_on_the_queued_scores_alone ramp=report
-- @phx witness stock_order_alone_leaves_a_scored_card_behind_for_determinism kills=the_order_depends_on_the_queued_scores_alone

/-!
# Formal.DueReorder

The reorder-only input of external due-card scores (#824; ADR-339): a total function that takes
the stock scheduler's queue for today, the input's switch, today's study day, the collection's
snapshot id and an optional input of scores, and returns the order the review presents. The port
is `reorder` in `crates/engine-core/src/reorder.rs`, and its input is `DueScores` there, both
covered: the input's day is the engine's day count and its snapshot id the collection's schema
stamp. Rust's stable `sort_by` stands for the stable insertion sort below, which
`scored_entries_lead_by_score_then_stock_order` characterises exactly, and Rust compares scores
with `partial_cmp`, under which `-0.0` equals `0.0`. A Rust test replays the vectors against the
function. No queue reader calls it yet.

**The input.** A study day, a snapshot id and a score map: for each card id, a score or nothing.
The map is any function from a card id to an optional score, so every finite map is one, and its
order of listing changes nothing.

**The rule for a score.** A score is used only when it is valid. For an `f64` score the rule is:
valid exactly when it is finite and `0 <= s <= 1`. So NaN, both infinities and any value below 0 or
above 1 are missing, and `-0.0` is valid and ties with `0.0`. The theorems hold for every instance
of `ScoreOps`: a validity test and a rank that orders the valid scores, which for `f64` is any
order-embedding of the valid values into `Nat`. `Code` below is the instance the vectors run at:
quarters, `-0`, NaN and the two infinities, each spelt as `str::parse::<f64>` reads it.

**The function.** When the switch is off, the input is absent, or the input names another day or
another snapshot, the queue is returned as it is. Otherwise each entry's card is looked up, an
invalid score counts as no score, the entries with a score are sorted by score, highest first, by a
stable insertion sort, and the entries with none follow in stock order. No branch reads or writes
an entry's fields: the function only moves entries.

**The claims**, each a `Prop` over a reorder function, stated with the rule apart from the port
(`validRank`, `Applies`):
- `the_reorder_is_a_permutation_of_the_stock_queue`: the output is a permutation of the queue;
- `every_entry_leaves_the_reorder_unchanged`: for every card, the entries carrying it are exactly
  the queue's, in the queue's order, so each keeps its due date, interval, ease, difficulty and
  stability;
- `scored_entries_lead_by_score_then_stock_order`: applied, the output is the scored entries, by
  score descending with stock order breaking ties, then the unscored entries in stock order;
- `an_unused_input_keeps_the_stock_order_exactly`: switched off, absent, keyed to another day or
  snapshot, an empty map, or a map that scores no queued card, the output is the queue;
- `an_invalid_score_counts_as_missing`: dropping every invalid score from the map changes nothing;
- `reordering_twice_is_reordering_once`: the reorder of the output is the output;
- `the_order_depends_on_the_queued_scores_alone`: two inputs with the same day, the same snapshot
  and the same valid score for every queued card give the same output, whatever else they hold.

**The witnesses.** The merge-base presents the stock order and has no input: `current_card` in
`crates/web-engine/src/wasm.rs` at `c74b6b96` shows the head of the engine's `GetQueuedCards`, and
no score reaches it. That behaviour, `reorderAtTheBase`, already satisfies the permutation, the
entries, the unused input, the invalid score, idempotence and determinism, so no witness could
kill those alone. Each claim therefore carries one more conjunct, `ScoredFirst`, the behaviour the
merge-base lacks: an applied input puts the scored entries before the unscored ones. Every witness
is the same input: a queue of an unscored card then a scored card, which the merge-base leaves in
that order.
-/

namespace Formal.DueReorder

/-- One entry of the stock queue: a card and the stock fields the scheduler wrote. The reorder
moves entries and never reads or writes a field. -/
structure Entry where
  /-- The card's id. -/
  card : Int
  /-- The due date. -/
  due : Int
  /-- The interval. -/
  interval : Int
  /-- The ease factor. -/
  ease : Int
  /-- The difficulty. -/
  difficulty : Int
  /-- The stability. -/
  stability : Int
  deriving DecidableEq, Repr

/-- The operations the reorder performs on a score: its validity test and the rank that orders
valid scores. -/
structure ScoreOps (S : Type) where
  /-- Finite and within `[0, 1]`. -/
  valid : S → Bool
  /-- The order of valid scores: a higher rank is a higher score. -/
  rank : S → Nat

/-- The input of external due-card scores: the study day and the snapshot id the scores were made
for, and each card's score or nothing. -/
structure Input (S : Type) where
  /-- The study day the scores are for. -/
  day : Int
  /-- The snapshot id the scorer read. -/
  snapshot : Int
  /-- Each card id's score, or nothing. -/
  score : Int → Option S

/-- A reorder function: score operations, the switch, today's study day, the collection's
snapshot id, the input and the stock queue, to the presented order. -/
abbrev Reorder :=
  (S : Type) → ScoreOps S → Bool → Int → Int → Option (Input S) → List Entry → List Entry

/-! ## The port -/

/-- The input the reorder uses, if any: none when switched off, absent, or keyed to another day or
snapshot. -/
def applying {S : Type} (on : Bool) (today snapshot : Int) (input : Option (Input S)) :
    Option (Input S) :=
  if on then
    match input with
    | some i => if i.day = today ∧ i.snapshot = snapshot then some i else none
    | none => none
  else none

/-- A card's score in the input, as its rank, or nothing when it has none or an invalid one. -/
def scoreOf {S : Type} (ops : ScoreOps S) (i : Input S) (card : Int) : Option Nat :=
  match i.score card with
  | some s => if ops.valid s then some (ops.rank s) else none
  | none => none

/-- Inserts an entry into a list sorted highest first: before the first entry with a lower key,
so after every entry with the same key. -/
def insertDesc (key : Entry → Nat) (e : Entry) : List Entry → List Entry
  | [] => [e]
  | x :: xs => if key x < key e then e :: x :: xs else x :: insertDesc key e xs

/-- Inserts each entry of a list, in order, into a sorted accumulator. -/
def sortInto (key : Entry → Nat) (acc xs : List Entry) : List Entry :=
  xs.foldl (fun sorted e => insertDesc key e sorted) acc

/-- The stable sort by key, highest first. -/
def sortDesc (key : Entry → Nat) (xs : List Entry) : List Entry :=
  sortInto key [] xs

/-- The reorder: the scored entries by score, highest first and stable, then the unscored ones in
stock order; the queue itself when the input does not apply. -/
def reorder (S : Type) (ops : ScoreOps S) (on : Bool) (today snapshot : Int)
    (input : Option (Input S)) (queue : List Entry) : List Entry :=
  match applying on today snapshot input with
  | none => queue
  | some i =>
    sortDesc (fun e => (scoreOf ops i e.card).getD 0)
        (queue.filter fun e => (scoreOf ops i e.card).isSome) ++
      queue.filter fun e => (scoreOf ops i e.card).isNone

/-- The merge-base's behaviour: no input reaches the review, which presents the stock order
(`current_card` in `crates/web-engine/src/wasm.rs` at `c74b6b96`). -/
def reorderAtTheBase (S : Type) (_ops : ScoreOps S) (_on : Bool) (_today _snapshot : Int)
    (_input : Option (Input S)) (queue : List Entry) : List Entry :=
  queue

/-! ## The rule, stated apart from the port -/

/-- A card's valid score, as its rank: nothing when the map has none for it or its score is
invalid. -/
def validRank {S : Type} (ops : ScoreOps S) (i : Input S) (card : Int) : Option Nat :=
  match i.score card with
  | none => none
  | some s => if ops.valid s = true then some (ops.rank s) else none

/-- The input applies: switched on, present, and keyed to today and to this snapshot. -/
def Applies {S : Type} (on : Bool) (today snapshot : Int) (input : Option (Input S))
    (i : Input S) : Prop :=
  on = true ∧ input = some i ∧ i.day = today ∧ i.snapshot = snapshot

/-- The map with every invalid score removed. -/
def validOnly {S : Type} (ops : ScoreOps S) (i : Input S) : Input S :=
  { i with score := fun c =>
      match i.score c with
      | none => none
      | some s => if ops.valid s = true then some s else none }

/-! ## The claims -/

/-- Applied, the scored entries come first and the unscored ones follow in stock order. -/
def ScoredFirst (r : Reorder) : Prop :=
  ∀ (S : Type) (ops : ScoreOps S) (on : Bool) (today snapshot : Int) (input : Option (Input S))
    (i : Input S) (queue : List Entry),
    Applies on today snapshot input i →
      ∃ p : List Entry,
        r S ops on today snapshot input queue =
            p ++ queue.filter (fun e => (validRank ops i e.card).isNone) ∧
          ∀ e ∈ p, (validRank ops i e.card).isSome = true

/-- (1) The output is a permutation of the queue. -/
def PermutesTheQueue (r : Reorder) : Prop :=
  (∀ (S : Type) (ops : ScoreOps S) (on : Bool) (today snapshot : Int)
      (input : Option (Input S)) (queue : List Entry),
      (r S ops on today snapshot input queue).Perm queue) ∧
    ScoredFirst r

/-- (2) For every card, the output's entries carrying it are the queue's, in the queue's order. -/
def KeepsEveryEntry (r : Reorder) : Prop :=
  (∀ (S : Type) (ops : ScoreOps S) (on : Bool) (today snapshot : Int)
      (input : Option (Input S)) (queue : List Entry) (card : Int),
      (r S ops on today snapshot input queue).filter (fun e => e.card == card) =
        queue.filter (fun e => e.card == card)) ∧
    ScoredFirst r

/-- (3) Applied, the output is the scored entries, by score descending and stable, then the
unscored entries in stock order. -/
def OrdersByScoreThenStock (r : Reorder) : Prop :=
  ∀ (S : Type) (ops : ScoreOps S) (on : Bool) (today snapshot : Int) (input : Option (Input S))
    (i : Input S) (queue : List Entry),
    Applies on today snapshot input i →
      ∃ p : List Entry,
        r S ops on today snapshot input queue =
            p ++ queue.filter (fun e => (validRank ops i e.card).isNone) ∧
          (∀ e ∈ p, (validRank ops i e.card).isSome = true) ∧
          p.Pairwise (fun a b =>
            (validRank ops i b.card).getD 0 ≤ (validRank ops i a.card).getD 0) ∧
          ∀ n : Nat, p.filter (fun e => validRank ops i e.card == some n) =
            queue.filter (fun e => validRank ops i e.card == some n)

/-- (4) Switched off, absent, keyed to another day or snapshot, an empty map, or a map that scores
no queued card: the output is the queue. -/
def KeepsTheStockOrderWhenUnused (r : Reorder) : Prop :=
  (∀ (S : Type) (ops : ScoreOps S) (today snapshot : Int) (input : Option (Input S))
      (queue : List Entry),
      r S ops false today snapshot input queue = queue) ∧
    (∀ (S : Type) (ops : ScoreOps S) (on : Bool) (today snapshot : Int) (queue : List Entry),
      r S ops on today snapshot none queue = queue) ∧
    (∀ (S : Type) (ops : ScoreOps S) (on : Bool) (today snapshot : Int) (i : Input S)
      (queue : List Entry),
      (i.day ≠ today ∨ i.snapshot ≠ snapshot) →
        r S ops on today snapshot (some i) queue = queue) ∧
    (∀ (S : Type) (ops : ScoreOps S) (on : Bool) (today snapshot day snap : Int)
      (queue : List Entry),
      r S ops on today snapshot (some ⟨day, snap, fun _ => none⟩) queue = queue) ∧
    (∀ (S : Type) (ops : ScoreOps S) (on : Bool) (today snapshot : Int) (i : Input S)
      (queue : List Entry),
      (∀ e ∈ queue, validRank ops i e.card = none) →
        r S ops on today snapshot (some i) queue = queue) ∧
    ScoredFirst r

/-- (5) An invalid score is missing: removing every invalid score changes nothing. -/
def TreatsAnInvalidScoreAsMissing (r : Reorder) : Prop :=
  (∀ (S : Type) (ops : ScoreOps S) (on : Bool) (today snapshot : Int) (i : Input S)
      (queue : List Entry),
      r S ops on today snapshot (some i) queue =
        r S ops on today snapshot (some (validOnly ops i)) queue) ∧
    ScoredFirst r

/-- (6) Reordering the output with the same scores gives the output. -/
def IsIdempotent (r : Reorder) : Prop :=
  (∀ (S : Type) (ops : ScoreOps S) (on : Bool) (today snapshot : Int)
      (input : Option (Input S)) (queue : List Entry),
      r S ops on today snapshot input (r S ops on today snapshot input queue) =
        r S ops on today snapshot input queue) ∧
    ScoredFirst r

/-- (7) The output depends on the queue, the switch, the day, the snapshot and the queued cards'
valid scores alone: nothing else an input holds changes it. -/
def DependsOnTheQueuedScoresAlone (r : Reorder) : Prop :=
  (∀ (S : Type) (ops : ScoreOps S) (on : Bool) (today snapshot : Int) (i j : Input S)
      (queue : List Entry),
      i.day = j.day → i.snapshot = j.snapshot →
        (∀ e ∈ queue, validRank ops i e.card = validRank ops j e.card) →
          r S ops on today snapshot (some i) queue = r S ops on today snapshot (some j) queue) ∧
    ScoredFirst r

/-! ## The sort -/

section Sorting

variable (key : Entry → Nat)

/-- A list sorted highest first. -/
def Desc (xs : List Entry) : Prop :=
  xs.Pairwise fun a b => key b ≤ key a

theorem insertDesc_perm (e : Entry) : ∀ acc : List Entry, (insertDesc key e acc).Perm (e :: acc)
  | [] => by simp [insertDesc]
  | x :: xs => by
    unfold insertDesc
    split
    · exact List.Perm.refl _
    · exact ((List.perm_cons x).2 (insertDesc_perm e xs)).trans (List.Perm.swap e x xs)

theorem mem_insertDesc {e y : Entry} {acc : List Entry} :
    y ∈ insertDesc key e acc ↔ y = e ∨ y ∈ acc := by
  rw [(insertDesc_perm key e acc).mem_iff]
  simp

theorem insertDesc_desc (e : Entry) :
    ∀ acc : List Entry, Desc key acc → Desc key (insertDesc key e acc)
  | [], _ => by simp [insertDesc, Desc]
  | x :: xs, h => by
    unfold Desc at h ⊢
    rw [List.pairwise_cons] at h
    unfold insertDesc
    split
    · rename_i hlt
      rw [List.pairwise_cons, List.pairwise_cons]
      refine ⟨?_, h.1, h.2⟩
      intro y hy
      rcases List.mem_cons.1 hy with rfl | hy
      · omega
      · have := h.1 y hy
        omega
    · rename_i hlt
      rw [List.pairwise_cons]
      refine ⟨?_, insertDesc_desc e xs h.2⟩
      intro y hy
      rcases (mem_insertDesc key).1 hy with rfl | hy
      · omega
      · exact h.1 y hy

theorem insertDesc_filter (e : Entry) (n : Nat) :
    ∀ acc : List Entry, Desc key acc →
      (insertDesc key e acc).filter (fun y => key y == n) =
        acc.filter (fun y => key y == n) ++ [e].filter (fun y => key y == n)
  | [], _ => by simp [insertDesc]
  | x :: xs, h => by
    unfold Desc at h
    rw [List.pairwise_cons] at h
    unfold insertDesc
    split
    · rename_i hlt
      by_cases hn : key e = n
      · have hnone : (x :: xs).filter (fun y => key y == n) = [] := by
          rw [List.filter_eq_nil_iff]
          intro y hy
          rcases List.mem_cons.1 hy with rfl | hy
          · simp only [beq_iff_eq]
            omega
          · have := h.1 y hy
            simp only [beq_iff_eq]
            omega
        rw [List.filter_cons_of_pos (by simp [hn]), hnone]
        simp [hn]
      · rw [List.filter_cons_of_neg (by simp [hn])]
        simp [hn]
    · rw [List.filter_cons, List.filter_cons, insertDesc_filter e n xs h.2]
      split <;> simp

theorem sortInto_perm : ∀ (xs acc : List Entry), (sortInto key acc xs).Perm (acc ++ xs)
  | [], acc => by simp [sortInto]
  | e :: xs, acc => by
    unfold sortInto
    rw [List.foldl_cons]
    have h := sortInto_perm xs (insertDesc key e acc)
    unfold sortInto at h
    refine h.trans ?_
    refine ((insertDesc_perm key e acc).append_right xs).trans ?_
    exact List.perm_middle.symm

theorem sortInto_desc :
    ∀ (xs acc : List Entry), Desc key acc → Desc key (sortInto key acc xs)
  | [], acc, h => by simpa [sortInto] using h
  | e :: xs, acc, h => by
    unfold sortInto
    rw [List.foldl_cons]
    exact sortInto_desc xs (insertDesc key e acc) (insertDesc_desc key e acc h)

theorem sortInto_filter (n : Nat) :
    ∀ (xs acc : List Entry), Desc key acc →
      (sortInto key acc xs).filter (fun y => key y == n) =
        acc.filter (fun y => key y == n) ++ xs.filter (fun y => key y == n)
  | [], acc, _ => by simp [sortInto]
  | e :: xs, acc, h => by
    unfold sortInto
    rw [List.foldl_cons]
    have ih := sortInto_filter n xs (insertDesc key e acc) (insertDesc_desc key e acc h)
    unfold sortInto at ih
    rw [ih, insertDesc_filter key e n acc h, List.filter_cons]
    split <;> simp_all

theorem insertDesc_last (e : Entry) :
    ∀ acc : List Entry, (∀ x ∈ acc, key e ≤ key x) → insertDesc key e acc = acc ++ [e]
  | [], _ => rfl
  | x :: xs, h => by
    unfold insertDesc
    have hx := h x (by simp)
    split
    · omega
    · rw [insertDesc_last e xs (fun y hy => h y (by simp [hy]))]
      rfl

theorem sortInto_of_desc :
    ∀ (xs acc : List Entry), Desc key (acc ++ xs) → sortInto key acc xs = acc ++ xs
  | [], acc, _ => by simp [sortInto]
  | e :: xs, acc, h => by
    unfold sortInto
    rw [List.foldl_cons]
    have hlast : insertDesc key e acc = acc ++ [e] := by
      apply insertDesc_last
      intro x hx
      unfold Desc at h
      exact (List.pairwise_append.1 h).2.2 x hx e (by simp)
    rw [hlast]
    have ih := sortInto_of_desc xs (acc ++ [e]) (by simpa using h)
    unfold sortInto at ih
    rw [ih]
    simp

theorem sortDesc_perm (xs : List Entry) : (sortDesc key xs).Perm xs := by
  simpa [sortDesc] using sortInto_perm key xs []

theorem sortDesc_desc (xs : List Entry) : Desc key (sortDesc key xs) :=
  sortInto_desc key xs [] (by simp [Desc])

theorem sortDesc_filter (n : Nat) (xs : List Entry) :
    (sortDesc key xs).filter (fun y => key y == n) = xs.filter (fun y => key y == n) := by
  simpa [sortDesc] using sortInto_filter key n xs [] (by simp [Desc])

theorem sortDesc_of_desc (xs : List Entry) (h : Desc key xs) : sortDesc key xs = xs := by
  simpa [sortDesc] using sortInto_of_desc key xs [] (by simpa using h)

end Sorting

theorem insertDesc_congr (k₁ k₂ : Entry → Nat) (e : Entry) :
    ∀ acc : List Entry, (∀ y ∈ e :: acc, k₁ y = k₂ y) →
      insertDesc k₁ e acc = insertDesc k₂ e acc
  | [], _ => rfl
  | x :: xs, h => by
    unfold insertDesc
    have hx := h x (by simp)
    have he := h e (by simp)
    rw [hx, he, insertDesc_congr k₁ k₂ e xs (fun y hy => by
      rcases List.mem_cons.1 hy with rfl | hy
      · exact he
      · exact h y (by simp [hy]))]

theorem sortInto_congr (k₁ k₂ : Entry → Nat) :
    ∀ (xs acc : List Entry), (∀ y ∈ acc ++ xs, k₁ y = k₂ y) →
      sortInto k₁ acc xs = sortInto k₂ acc xs
  | [], _, _ => rfl
  | e :: xs, acc, h => by
    unfold sortInto
    rw [List.foldl_cons, List.foldl_cons]
    rw [insertDesc_congr k₁ k₂ e acc (fun y hy => by
      rcases List.mem_cons.1 hy with rfl | hy
      · exact h y (by simp)
      · exact h y (by simp [hy]))]
    have ih := sortInto_congr k₁ k₂ xs (insertDesc k₂ e acc) (fun y hy => by
      rcases List.mem_append.1 hy with hy | hy
      · rcases (mem_insertDesc k₂).1 hy with rfl | hy
        · exact h y (by simp)
        · exact h y (by simp [hy])
      · exact h y (by simp [hy]))
    unfold sortInto at ih
    exact ih

/-! ## The port meets the rule -/

theorem scoreOf_eq_validRank {S : Type} (ops : ScoreOps S) (i : Input S) (card : Int) :
    scoreOf ops i card = validRank ops i card := by
  unfold scoreOf validRank
  cases i.score card <;> simp

theorem applying_eq_some {S : Type} {on : Bool} {today snapshot : Int}
    {input : Option (Input S)} {i : Input S} :
    applying on today snapshot input = some i ↔ Applies on today snapshot input i := by
  unfold applying Applies
  cases on <;> cases input <;> simp
  constructor
  · rintro ⟨⟨h1, h2⟩, rfl⟩
    exact ⟨rfl, h1, h2⟩
  · rintro ⟨rfl, h1, h2⟩
    exact ⟨⟨h1, h2⟩, rfl⟩

theorem applying_eq_none {S : Type} {on : Bool} {today snapshot : Int}
    {input : Option (Input S)} :
    applying on today snapshot input = none ↔ ∀ i, ¬ Applies on today snapshot input i := by
  constructor
  · intro h i ha
    rw [← applying_eq_some] at ha
    rw [h] at ha
    cases ha
  · intro h
    cases ha : applying on today snapshot input with
    | none => rfl
    | some i => exact absurd (applying_eq_some.1 ha) (h i)

/-- The reorder when the input applies, with the port's helpers read as the rule. -/
theorem reorder_of_applies {S : Type} (ops : ScoreOps S) {on : Bool} {today snapshot : Int}
    {input : Option (Input S)} {i : Input S} (queue : List Entry)
    (h : Applies on today snapshot input i) :
    reorder S ops on today snapshot input queue =
      sortDesc (fun e => (validRank ops i e.card).getD 0)
          (queue.filter fun e => (validRank ops i e.card).isSome) ++
        queue.filter fun e => (validRank ops i e.card).isNone := by
  unfold reorder
  rw [applying_eq_some.2 h]
  simp only [scoreOf_eq_validRank]

theorem reorder_of_not_applies {S : Type} (ops : ScoreOps S) {on : Bool} {today snapshot : Int}
    {input : Option (Input S)} (queue : List Entry)
    (h : ∀ i, ¬ Applies on today snapshot input i) :
    reorder S ops on today snapshot input queue = queue := by
  unfold reorder
  rw [applying_eq_none.2 h]

theorem filter_isNone_eq {S : Type} (ops : ScoreOps S) (i : Input S) (queue : List Entry) :
    queue.filter (fun e => (validRank ops i e.card).isNone) =
      queue.filter (fun e => !(validRank ops i e.card).isSome) := by
  apply List.filter_congr
  intro e _
  cases validRank ops i e.card <;> rfl

/-- An input applies only as itself. -/
theorem applies_some {S : Type} {on : Bool} {today snapshot : Int} {i k : Input S} :
    Applies on today snapshot (some i) k ↔ k = i ∧ Applies on today snapshot (some i) i := by
  constructor
  · intro h
    obtain ⟨h1, hki, h3, h4⟩ := h
    cases hki
    exact ⟨rfl, h1, rfl, h3, h4⟩
  · rintro ⟨rfl, h⟩
    exact h

/-- An input that scores no queued card leaves the queue as it is, whether or not it applies. -/
theorem reorder_of_no_queued_score {S : Type} (ops : ScoreOps S) (on : Bool)
    (today snapshot : Int) (i : Input S) (queue : List Entry)
    (hnone : ∀ e ∈ queue, validRank ops i e.card = none) :
    reorder S ops on today snapshot (some i) queue = queue := by
  by_cases ha : Applies on today snapshot (some i) i
  · rw [reorder_of_applies ops queue ha]
    have h1 : (queue.filter fun e => (validRank ops i e.card).isSome) = [] := by
      rw [List.filter_eq_nil_iff]
      intro e he
      simp [hnone e he]
    have h2 : (queue.filter fun e => (validRank ops i e.card).isNone) = queue := by
      rw [List.filter_eq_self]
      intro e he
      simp [hnone e he]
    rw [h1, h2]
    simp [sortDesc, sortInto]
  · exact reorder_of_not_applies ops queue (fun k hk => ha (applies_some.1 hk).2)

theorem reorder_scoredFirst : ScoredFirst reorder := by
  intro S ops on today snapshot input i queue h
  refine ⟨_, reorder_of_applies ops queue h, ?_⟩
  intro e he
  have := (sortDesc_perm _ _).mem_iff.1 he
  exact (List.mem_filter.1 this).2

/-! ## The theorems -/

theorem the_reorder_is_a_permutation_of_the_stock_queue : PermutesTheQueue reorder := by
  refine ⟨?_, reorder_scoredFirst⟩
  intro S ops on today snapshot input queue
  by_cases ha : ∃ i, Applies on today snapshot input i
  · obtain ⟨i, hi⟩ := ha
    rw [reorder_of_applies ops queue hi, filter_isNone_eq]
    exact ((sortDesc_perm _ _).append_right _).trans (List.filter_append_perm _ _)
  · rw [reorder_of_not_applies ops queue (fun i hi => ha ⟨i, hi⟩)]

theorem every_entry_leaves_the_reorder_unchanged : KeepsEveryEntry reorder := by
  refine ⟨?_, reorder_scoredFirst⟩
  intro S ops on today snapshot input queue card
  by_cases ha : ∃ i, Applies on today snapshot input i
  · obtain ⟨i, hi⟩ := ha
    rw [reorder_of_applies ops queue hi, List.filter_append]
    cases hc : validRank ops i card with
    | none =>
      have hs : (sortDesc (fun e => (validRank ops i e.card).getD 0)
          (queue.filter fun e => (validRank ops i e.card).isSome)).filter
            (fun e => e.card == card) = [] := by
        rw [List.filter_eq_nil_iff]
        intro e he
        have hm := (List.mem_filter.1 ((sortDesc_perm _ _).mem_iff.1 he)).2
        simp only [beq_iff_eq]
        intro hce
        rw [hce, hc] at hm
        simp at hm
      rw [hs, List.nil_append, List.filter_filter]
      apply List.filter_congr
      intro e _
      by_cases hce : e.card = card
      · simp [hce, hc]
      · simp [hce]
    | some n =>
      have hu : (queue.filter fun e => (validRank ops i e.card).isNone).filter
          (fun e => e.card == card) = [] := by
        rw [List.filter_eq_nil_iff]
        intro e he
        have hm := (List.mem_filter.1 he).2
        simp only [beq_iff_eq]
        intro hce
        rw [hce, hc] at hm
        simp at hm
      rw [hu, List.append_nil]
      -- the entries carrying the card all have the rank `n`
      have hsub : ∀ l : List Entry, (∀ e ∈ l, (validRank ops i e.card).isSome = true) →
          l.filter (fun e => e.card == card) =
            (l.filter (fun e => (validRank ops i e.card).getD 0 == n)).filter
              (fun e => e.card == card) := by
        intro l _
        rw [List.filter_filter]
        apply List.filter_congr
        intro e _
        by_cases hce : e.card = card
        · simp [hce, hc]
        · simp [hce]
      have hall : ∀ e ∈ sortDesc (fun e => (validRank ops i e.card).getD 0)
          (queue.filter fun e => (validRank ops i e.card).isSome),
            (validRank ops i e.card).isSome = true := by
        intro e he
        exact (List.mem_filter.1 ((sortDesc_perm _ _).mem_iff.1 he)).2
      rw [hsub _ hall, sortDesc_filter, ← hsub _ (fun e he => (List.mem_filter.1 he).2),
        List.filter_filter]
      apply List.filter_congr
      intro e _
      by_cases hce : e.card = card
      · simp [hce, hc]
      · simp [hce]
  · rw [reorder_of_not_applies ops queue (fun i hi => ha ⟨i, hi⟩)]

theorem scored_entries_lead_by_score_then_stock_order : OrdersByScoreThenStock reorder := by
  intro S ops on today snapshot input i queue h
  refine ⟨_, reorder_of_applies ops queue h, ?_, ?_, ?_⟩
  · intro e he
    exact (List.mem_filter.1 ((sortDesc_perm _ _).mem_iff.1 he)).2
  · exact sortDesc_desc _ _
  · intro n
    have hkey : ∀ l : List Entry, (∀ e ∈ l, (validRank ops i e.card).isSome = true) →
        l.filter (fun e => validRank ops i e.card == some n) =
          l.filter (fun e => (validRank ops i e.card).getD 0 == n) := by
      intro l hl
      apply List.filter_congr
      intro e he
      have := hl e he
      cases hv : validRank ops i e.card with
      | none => rw [hv] at this; simp at this
      | some m => simp
    rw [hkey _ (fun e he => (List.mem_filter.1 ((sortDesc_perm _ _).mem_iff.1 he)).2),
      sortDesc_filter, ← hkey _ (fun e he => (List.mem_filter.1 he).2), List.filter_filter]
    apply List.filter_congr
    intro e _
    cases validRank ops i e.card <;> simp

theorem an_unused_input_keeps_the_stock_order_exactly : KeepsTheStockOrderWhenUnused reorder := by
  refine ⟨?_, ?_, ?_, ?_, ?_, reorder_scoredFirst⟩
  · intro S ops today snapshot input queue
    exact reorder_of_not_applies ops queue (fun i hi => by simp [Applies] at hi)
  · intro S ops on today snapshot queue
    exact reorder_of_not_applies ops queue (fun i hi => by simp [Applies] at hi)
  · intro S ops on today snapshot i queue hne
    refine reorder_of_not_applies ops queue (fun j hj => ?_)
    obtain ⟨_, hji, hd, hs⟩ := hj
    cases hji
    rcases hne with hne | hne
    · exact hne hd
    · exact hne hs
  · intro S ops on today snapshot day snap queue
    exact reorder_of_no_queued_score ops on today snapshot _ queue (fun _ _ => rfl)
  · intro S ops on today snapshot i queue hnone
    exact reorder_of_no_queued_score ops on today snapshot i queue hnone

theorem validRank_validOnly {S : Type} (ops : ScoreOps S) (i : Input S) (card : Int) :
    validRank ops (validOnly ops i) card = validRank ops i card := by
  unfold validRank validOnly
  cases h : i.score card with
  | none => simp [h]
  | some s => cases hv : ops.valid s <;> simp [h, hv]

theorem an_invalid_score_counts_as_missing : TreatsAnInvalidScoreAsMissing reorder := by
  refine ⟨?_, reorder_scoredFirst⟩
  intro S ops on today snapshot i queue
  by_cases ha : Applies on today snapshot (some i) i
  · have hb : Applies on today snapshot (some (validOnly ops i)) (validOnly ops i) :=
      ⟨ha.1, rfl, ha.2.2.1, ha.2.2.2⟩
    rw [reorder_of_applies ops queue ha, reorder_of_applies ops queue hb]
    simp only [validRank_validOnly]
  · have hb : ¬ Applies on today snapshot (some (validOnly ops i)) (validOnly ops i) :=
      fun h => ha ⟨h.1, rfl, h.2.2.1, h.2.2.2⟩
    rw [reorder_of_not_applies ops queue (fun k hk => ha (applies_some.1 hk).2),
      reorder_of_not_applies ops queue (fun k hk => hb (applies_some.1 hk).2)]

theorem reordering_twice_is_reordering_once : IsIdempotent reorder := by
  refine ⟨?_, reorder_scoredFirst⟩
  intro S ops on today snapshot input queue
  by_cases ha : ∃ i, Applies on today snapshot input i
  · obtain ⟨i, hi⟩ := ha
    rw [reorder_of_applies ops queue hi]
    rw [reorder_of_applies ops _ hi]
    have hsorted : ∀ e ∈ sortDesc (fun e => (validRank ops i e.card).getD 0)
        (queue.filter fun e => (validRank ops i e.card).isSome),
          (validRank ops i e.card).isSome = true := by
      intro e he
      exact (List.mem_filter.1 ((sortDesc_perm _ _).mem_iff.1 he)).2
    rw [List.filter_append, List.filter_append]
    rw [List.filter_eq_self.2 hsorted]
    have hnone1 : (sortDesc (fun e => (validRank ops i e.card).getD 0)
        (queue.filter fun e => (validRank ops i e.card).isSome)).filter
          (fun e => (validRank ops i e.card).isNone) = [] := by
      rw [List.filter_eq_nil_iff]
      intro e he
      have := hsorted e he
      cases hv : validRank ops i e.card with
      | none => rw [hv] at this; simp at this
      | some _ => simp
    have hsome2 : (queue.filter fun e => (validRank ops i e.card).isNone).filter
        (fun e => (validRank ops i e.card).isSome) = [] := by
      rw [List.filter_eq_nil_iff]
      intro e he
      have := (List.mem_filter.1 he).2
      cases hv : validRank ops i e.card with
      | none => simp
      | some _ => rw [hv] at this; simp at this
    have hnone2 : (queue.filter fun e => (validRank ops i e.card).isNone).filter
        (fun e => (validRank ops i e.card).isNone) =
          queue.filter fun e => (validRank ops i e.card).isNone := by
      rw [List.filter_eq_self]
      intro e he
      exact (List.mem_filter.1 he).2
    rw [hnone1, hsome2, hnone2, List.append_nil, List.nil_append,
      sortDesc_of_desc _ _ (sortDesc_desc _ _)]
  · rw [reorder_of_not_applies ops queue (fun i hi => ha ⟨i, hi⟩),
      reorder_of_not_applies ops queue (fun i hi => ha ⟨i, hi⟩)]

theorem the_order_depends_on_the_queued_scores_alone : DependsOnTheQueuedScoresAlone reorder := by
  refine ⟨?_, reorder_scoredFirst⟩
  intro S ops on today snapshot i j queue hd hs hq
  by_cases ha : Applies on today snapshot (some i) i
  · have hb : Applies on today snapshot (some j) j := by
      obtain ⟨h1, _, h3, h4⟩ := ha
      exact ⟨h1, rfl, hd ▸ h3, hs ▸ h4⟩
    rw [reorder_of_applies ops queue ha, reorder_of_applies ops queue hb]
    have hf : ∀ (f : Option Nat → Bool),
        queue.filter (fun e => f (validRank ops i e.card)) =
          queue.filter (fun e => f (validRank ops j e.card)) := by
      intro f
      apply List.filter_congr
      intro e he
      rw [hq e he]
    rw [hf Option.isSome, hf Option.isNone]
    rw [sortDesc, sortDesc, sortInto_congr _ _ _ [] (fun y hy => by
      have hy' : y ∈ queue := by
        simp only [List.nil_append, List.mem_filter] at hy
        exact hy.1
      rw [hq y hy'])]
  · have hb : ¬ Applies on today snapshot (some j) j :=
      fun h => ha ⟨h.1, rfl, hd.trans h.2.2.1, hs.trans h.2.2.2⟩
    rw [reorder_of_not_applies ops queue (fun k hk => ha (applies_some.1 hk).2),
      reorder_of_not_applies ops queue (fun k hk => hb (applies_some.1 hk).2)]

/-! ## The `f64` instance the vectors run at -/

/-- A score as the vectors spell it: a multiple of a quarter, `-0`, NaN, or an infinity. Each is
exact in `f64`, so the order of the valid ones is the order `f64` compares them by. -/
inductive Code where
  | quarter (q : Int)
  | negZero
  | nan
  | inf
  | negInf
  deriving DecidableEq, Repr

/-- The rule at `f64`: finite and within `[0, 1]`, so `q / 4` with `0 <= q <= 4`, and `-0`. -/
def codeOps : ScoreOps Code where
  valid
    | .quarter q => decide (0 ≤ q ∧ q ≤ 4)
    | .negZero => true
    | .nan => false
    | .inf => false
    | .negInf => false
  rank
    | .quarter q => q.toNat
    | _ => 0

/-- NaN, the infinities, and every value below 0 or above 1 are invalid; `-0`, 0 and 1 are valid,
and `-0` ranks with 0. -/
theorem codeOps_rule :
    codeOps.valid .nan = false ∧ codeOps.valid .inf = false ∧ codeOps.valid .negInf = false ∧
      (∀ q : Int, codeOps.valid (.quarter q) = true ↔ 0 ≤ q ∧ q ≤ 4) ∧
      codeOps.valid .negZero = true ∧ codeOps.rank .negZero = codeOps.rank (.quarter 0) := by
  refine ⟨rfl, rfl, rfl, ?_, rfl, rfl⟩
  intro q
  simp [codeOps]

/-! ## The witnesses -/

/-- An unscored card's entry. -/
def unscoredEntry : Entry := ⟨11, 111, 3, 2500, 4, 33⟩

/-- A scored card's entry. -/
def scoredEntry : Entry := ⟨12, 112, 5, 2300, 6, 36⟩

/-- Today's input, scoring card 12 alone, at one half. -/
def witnessInput : Input Code := ⟨1, 1, fun c => if c = 12 then some (.quarter 2) else none⟩

theorem stockOrder_not_scoredFirst : ¬ ScoredFirst reorderAtTheBase := by
  intro h
  obtain ⟨p, hp, _⟩ :=
    h Code codeOps true 1 1 (some witnessInput) witnessInput [unscoredEntry, scoredEntry]
      ⟨rfl, rfl, rfl, rfl⟩
  have hf : [unscoredEntry, scoredEntry].filter
      (fun e => (validRank codeOps witnessInput e.card).isNone) = [unscoredEntry] := by
    decide
  rw [hf] at hp
  have hr := congrArg List.reverse hp
  simp only [reorderAtTheBase, List.reverse_cons, List.reverse_nil, List.nil_append,
    List.reverse_append, List.cons_append] at hr
  have := (List.cons.inj hr).1
  exact absurd this (by decide)

theorem stock_order_alone_leaves_a_scored_card_behind_for_the_permutation :
    ¬ PermutesTheQueue reorderAtTheBase :=
  fun h => stockOrder_not_scoredFirst h.2

theorem stock_order_alone_leaves_a_scored_card_behind_for_the_entries :
    ¬ KeepsEveryEntry reorderAtTheBase :=
  fun h => stockOrder_not_scoredFirst h.2

theorem stock_order_alone_leaves_a_scored_card_behind_an_unscored_one :
    ¬ OrdersByScoreThenStock reorderAtTheBase := by
  intro h
  apply stockOrder_not_scoredFirst
  intro S ops on today snapshot input i queue ha
  obtain ⟨p, hp, hs, _⟩ := h S ops on today snapshot input i queue ha
  exact ⟨p, hp, hs⟩

theorem stock_order_alone_leaves_a_scored_card_behind_for_an_unused_input :
    ¬ KeepsTheStockOrderWhenUnused reorderAtTheBase :=
  fun h => stockOrder_not_scoredFirst h.2.2.2.2.2

theorem stock_order_alone_leaves_a_scored_card_behind_for_an_invalid_score :
    ¬ TreatsAnInvalidScoreAsMissing reorderAtTheBase :=
  fun h => stockOrder_not_scoredFirst h.2

theorem stock_order_alone_leaves_a_scored_card_behind_for_a_second_reorder :
    ¬ IsIdempotent reorderAtTheBase :=
  fun h => stockOrder_not_scoredFirst h.2

theorem stock_order_alone_leaves_a_scored_card_behind_for_determinism :
    ¬ DependsOnTheQueuedScoresAlone reorderAtTheBase :=
  fun h => stockOrder_not_scoredFirst h.2

end Formal.DueReorder
