-- @phx covers crates/fsrs7/src/convert.rs anchor=histories digest=sha256:4789e62c1752f5d5c2d19aa3e3fd6b3d0a27a36746c590917f17cc71fa8df0b0
-- @phx covers crates/fsrs7/src/convert.rs anchor=kept digest=sha256:8f8a3c47f4c0cb225ff0ed2057520b53f2481544d840a00ce3ca17ca6a0bdbed
-- @phx covers crates/fsrs7/src/convert.rs anchor=reset digest=sha256:2a6fffea3f2ef16160b233c11ce83ceab78b23e4943762bc087e711128ed81a5
-- @phx covers crates/fsrs7/src/convert.rs anchor=RESET_KIND digest=sha256:675e879a6a7a66f0e5b0a1efb9665605fccb00f8fc6b0e25c96d9f1cd5435185
-- @phx covers crates/fsrs7/src/convert.rs anchor=DROPPED_KINDS digest=sha256:fb1c40c92aa7cbb8c62b9e31439abf4b05781e08c983efb6e588cfc2cc0db22a
-- @phx cites #641
-- @phx theorem the_selection_keeps_the_rated_reviews_after_the_last_reset ramp=report
-- @phx witness merge_base_keeps_a_review_before_its_reset kills=the_selection_keeps_the_rated_reviews_after_the_last_reset
-- @phx vectors formal/vectors/replay-history.jsonl

/-!
# Formal.ReplayHistory

The history selection of the FSRS-7 replay, `histories` in `crates/fsrs7/src/convert.rs`, as a
total function of a set of review-log rows (#641, SPEC-386 R2 to R4 and R15).

**The claim.** For every row set whose ids are distinct, as the review log's primary key makes
them, `the_selection_keeps_the_rated_reviews_after_the_last_reset` holds of the port:
- any arrival order of the rows gives the same histories;
- each history's ratings are, up to order, the ratings of its card's rows that are rated reviews
  (ease not 0, kind neither 4 nor 5) and come after every reset of the card (kind 4 with the factor
  0, the engine's own Forget row): the reset cut and the drops;
- each history's first review has the distance 0, and no review has a negative distance.

**What is ported.** The row's integers: its card and id as `Int` (both `i64`), its ease, kind and
factor as `Nat`. The constants `RESET_KIND` and `DROPPED_KINDS`, the predicates `reset` and `kept`,
and `histories` branch for branch: a `BTreeMap` keyed by card is the sorted list of distinct cards
(`cids`) with each card's rows in arrival order; `sort_by_key` by id is a stable insertion sort
(`sortById`); `rposition` is the index of the last row the predicate admits; the slice from it is
`List.drop`, then `filter(kept)`; `last()?` is `getLast?`, and a card whose selection is empty gets
no history, as `filter_map` drops it. A review's distance is kept in milliseconds: `days`, the
division into a fractional number of days, is not ported, and the vectors' reader divides.

**What is not ported.** The replay over the model's floating-point arithmetic, which is the pinned
revision's own code, held by its own test values (SPEC-386 A5), and `days`.

**The rule** is stated apart from the port: `isReset`, `isReview` and `counted` read a card's rows
with the literals 4, 5 and 0, never the port's constants.

**The witness** ports the merge-base's `histories` (`crates/fsrs7/src/convert.rs:43` at the merge
base, with `kept` at `:73`): it filters the kept rows, groups and sorts them, and cuts at no reset.
On one card reviewed, reset and reviewed again it keeps the review from before the reset.
-/

namespace Formal.ReplayHistory

/-- `RevlogRow`: one review-log row, as the engine's table holds it. -/
structure RevlogRow where
  cid : Int
  id : Int
  ease : Nat
  kind : Nat
  factor : Nat
  deriving DecidableEq, Repr

/-- `RESET_KIND`: the engine's manual entry, which its Forget writes with the factor 0. -/
def RESET_KIND : Nat := 4

/-- `DROPPED_KINDS`: a manual entry (4) and a reschedule (5). -/
def DROPPED_KINDS : List Nat := [4, 5]

/-- `reset`: the engine's Forget row, a manual entry with the factor 0. -/
def reset (row : RevlogRow) : Bool := row.kind == RESET_KIND && row.factor == 0

/-- `kept`: a rated review of the card's memory. -/
def kept (row : RevlogRow) : Bool := row.ease != 0 && !DROPPED_KINDS.contains row.kind

/-- One kept review: its rating, and its distance in milliseconds from the review before it. -/
structure Review where
  rating : Nat
  delta : Int
  deriving DecidableEq, Repr

/-- `CardHistory`: the card, the id of its last kept review, and its kept reviews in id order. -/
structure CardHistory where
  cid : Int
  lastId : Int
  reviews : List Review
  deriving DecidableEq, Repr

/-- One step of `sort_by_key(|row| row.id)`: a row goes before the first row whose id is not below
its own, so rows of equal id keep their arrival order. -/
def insertById (row : RevlogRow) : List RevlogRow → List RevlogRow
  | [] => [row]
  | first :: rest => if row.id ≤ first.id then row :: first :: rest else first :: insertById row rest

/-- `sort_by_key(|row| row.id)`, a stable sort. -/
def sortById (rows : List RevlogRow) : List RevlogRow := rows.foldr insertById []

/-- `Iterator::rposition`: the index of the last row the predicate admits. -/
def rposition (p : RevlogRow → Bool) : List RevlogRow → Option Nat
  | [] => none
  | row :: rest =>
    match rposition p rest with
    | some index => some (index + 1)
    | none => if p row then some 0 else none

/-- One key of the `BTreeMap`: the cards stay sorted and distinct. -/
def insertCid (cid : Int) : List Int → List Int
  | [] => [cid]
  | first :: rest =>
    if cid < first then cid :: first :: rest
    else if cid = first then first :: rest
    else first :: insertCid cid rest

/-- The `BTreeMap`'s keys: every card the rows name, in order, once. -/
def cids (rows : List RevlogRow) : List Int := rows.foldr (fun row keys => insertCid row.cid keys) []

/-- The map from a card's kept rows to its reviews: the first at the distance 0, each later one at
its distance from the one before (`previous.map_or(0.0, ...)`). -/
def reviewsOf : Option Int → List RevlogRow → List Review
  | _, [] => []
  | previous, row :: rest =>
    ⟨row.ease, match previous with
      | none => 0
      | some before => row.id - before⟩ :: reviewsOf (some row.id) rest

/-- One card's history: its rows sorted by id, cut at the last reset, the kept rows, and the last
kept row's id; none when no row is kept. -/
def history (cid : Int) (rows : List RevlogRow) : Option CardHistory :=
  let sorted := sortById rows
  let since := (rposition reset sorted).getD 0
  let selected := (sorted.drop since).filter kept
  match selected.getLast? with
  | none => none
  | some last => some ⟨cid, last.id, reviewsOf none selected⟩

/-- `histories`: each card's history, the cards in id order. -/
def histories (rows : List RevlogRow) : List CardHistory :=
  (cids rows).filterMap fun cid => history cid (rows.filter (·.cid == cid))

/-! ## The rule -/

/-- A reset, by the rule: kind 4 with the factor 0. -/
def isReset (row : RevlogRow) : Bool := row.kind == 4 && row.factor == 0

/-- A rated review, by the rule: ease not 0, and kind neither 4 nor 5. -/
def isReview (row : RevlogRow) : Bool := row.ease != 0 && row.kind != 4 && row.kind != 5

/-- A card's rows. -/
def card (rows : List RevlogRow) (cid : Int) : List RevlogRow := rows.filter (·.cid == cid)

/-- A row the rule counts: a rated review later than every reset of its card. -/
def counted (cardRows : List RevlogRow) (row : RevlogRow) : Bool :=
  isReview row && cardRows.all fun other => !isReset other || decide (other.id < row.id)

/-- The claim, stated once over the selection it judges. -/
def Claim (select : List RevlogRow → List CardHistory) : Prop :=
  ∀ rows : List RevlogRow, (rows.map (·.id)).Nodup →
    (∀ rows' : List RevlogRow, rows'.Perm rows → select rows' = select rows) ∧
    ∀ h ∈ select rows,
      (h.reviews.map (·.rating)).Perm
          (((card rows h.cid).filter (counted (card rows h.cid))).map (·.ease)) ∧
        h.reviews.head?.map (·.delta) = some 0 ∧
        ∀ review ∈ h.reviews, 0 ≤ review.delta

/-! ## The sort -/

theorem insertById_perm (row : RevlogRow) :
    ∀ rows : List RevlogRow, (insertById row rows).Perm (row :: rows)
  | [] => List.Perm.refl _
  | first :: rest => by
    unfold insertById
    split
    · exact List.Perm.refl _
    · exact ((insertById_perm row rest).cons first).trans (List.Perm.swap row first rest)

theorem sortById_perm : ∀ rows : List RevlogRow, (sortById rows).Perm rows
  | [] => List.Perm.refl _
  | row :: rest =>
    (insertById_perm row (sortById rest)).trans ((sortById_perm rest).cons row)

/-- Rows in id order. -/
abbrev ByIdLe (rows : List RevlogRow) : Prop := rows.Pairwise fun a b => a.id ≤ b.id

theorem insertById_sorted (row : RevlogRow) :
    ∀ rows : List RevlogRow, ByIdLe rows → ByIdLe (insertById row rows)
  | [], _ => by simp [insertById, ByIdLe]
  | first :: rest, sorted => by
    unfold insertById
    rw [ByIdLe, List.pairwise_cons] at sorted
    split
    · next le =>
      rw [ByIdLe, List.pairwise_cons, List.pairwise_cons]
      refine ⟨?_, sorted.1, sorted.2⟩
      intro other mem
      rcases List.mem_cons.1 mem with rfl | mem
      · exact le
      · exact Int.le_trans le (sorted.1 other mem)
    · next gt =>
      rw [ByIdLe, List.pairwise_cons]
      refine ⟨?_, insertById_sorted row rest sorted.2⟩
      intro other mem
      rcases List.mem_cons.1 ((insertById_perm row rest).mem_iff.1 mem) with rfl | mem
      · omega
      · exact sorted.1 other mem

theorem sortById_sorted : ∀ rows : List RevlogRow, ByIdLe (sortById rows)
  | [] => List.Pairwise.nil
  | row :: rest => insertById_sorted row (sortById rest) (sortById_sorted rest)

theorem insertById_comm (a b : RevlogRow) (distinct : a.id ≠ b.id) :
    ∀ rows : List RevlogRow,
      insertById a (insertById b rows) = insertById b (insertById a rows) := by
  intro rows
  induction rows with
  | nil => grind [insertById]
  | cons first rest ih => grind [insertById]

theorem nodup_ids_cons {row : RevlogRow} {rows : List RevlogRow}
    (nodup : ((row :: rows).map (·.id)).Nodup) : (rows.map (·.id)).Nodup :=
  (List.nodup_cons.1 (by simpa using nodup)).2

theorem sortById_eq_of_perm {rows₁ rows₂ : List RevlogRow} (perm : rows₁.Perm rows₂) :
    (rows₁.map (·.id)).Nodup → sortById rows₁ = sortById rows₂ := by
  induction perm with
  | nil => intro _; rfl
  | cons row _ ih =>
    intro nodup
    show insertById row (sortById _) = insertById row (sortById _)
    rw [ih (nodup_ids_cons nodup)]
  | swap a b rest =>
    intro nodup
    show insertById b (insertById a (sortById rest)) = insertById a (insertById b (sortById rest))
    have distinct : b.id ≠ a.id := by
      intro same
      simp [List.map_cons, List.nodup_cons, same] at nodup
    exact insertById_comm b a distinct (sortById rest)
  | trans first second ih₁ ih₂ =>
    intro nodup
    rw [ih₁ nodup, ih₂ ((first.map (·.id)).nodup_iff.1 nodup)]

/-! ## The cards -/

theorem insertCid_comm (a b : Int) :
    ∀ keys : List Int, insertCid a (insertCid b keys) = insertCid b (insertCid a keys) := by
  intro keys
  induction keys with
  | nil => grind [insertCid]
  | cons first rest ih => grind [insertCid]

theorem cids_eq_of_perm {rows₁ rows₂ : List RevlogRow} (perm : rows₁.Perm rows₂) :
    cids rows₁ = cids rows₂ := by
  induction perm with
  | nil => rfl
  | cons row _ ih =>
    show insertCid row.cid (cids _) = insertCid row.cid (cids _)
    rw [ih]
  | swap a b rest =>
    show insertCid b.cid (insertCid a.cid (cids rest)) = insertCid a.cid (insertCid b.cid (cids rest))
    exact (insertCid_comm b.cid a.cid (cids rest))
  | trans _ _ ih₁ ih₂ => rw [ih₁, ih₂]

theorem history_eq_of_sort {cid : Int} {rows₁ rows₂ : List RevlogRow}
    (same : sortById rows₁ = sortById rows₂) : history cid rows₁ = history cid rows₂ := by
  simp only [history, same]

theorem nodup_ids_filter (p : RevlogRow → Bool) {rows : List RevlogRow}
    (nodup : (rows.map (·.id)).Nodup) : ((rows.filter p).map (·.id)).Nodup :=
  List.Nodup.sublist ((List.filter_sublist).map _) nodup

theorem histories_eq_of_perm {rows₁ rows₂ : List RevlogRow} (perm : rows₁.Perm rows₂)
    (nodup : (rows₁.map (·.id)).Nodup) : histories rows₁ = histories rows₂ := by
  unfold histories
  rw [cids_eq_of_perm perm]
  congr 1
  funext cid
  exact history_eq_of_sort
    (sortById_eq_of_perm (perm.filter _) (nodup_ids_filter _ nodup))

/-! ## The cut -/

theorem rposition_some_admits {p : RevlogRow → Bool} :
    ∀ {rows : List RevlogRow} {index : Nat}, rposition p rows = some index →
      ∃ row ∈ rows, p row = true
  | [], _, found => by simp [rposition] at found
  | row :: rest, index, found => by
    cases inner : rposition p rest with
    | some later =>
      obtain ⟨other, mem, admits⟩ := rposition_some_admits inner
      exact ⟨other, List.mem_cons_of_mem _ mem, admits⟩
    | none =>
      simp only [rposition, inner] at found
      split at found
      · exact ⟨row, List.mem_cons_self, by assumption⟩
      · simp at found

theorem rposition_none_refuses {p : RevlogRow → Bool} :
    ∀ {rows : List RevlogRow}, rposition p rows = none → ∀ row ∈ rows, p row = false
  | [], _ => by simp
  | row :: rest, found => by
    simp only [rposition] at found
    split at found
    · simp at found
    · next inner =>
      split at found
      · simp at found
      · next refused =>
        intro other mem
        rcases List.mem_cons.1 mem with rfl | mem
        · simpa using refused
        · exact rposition_none_refuses inner other mem

theorem reset_iff (row : RevlogRow) : reset row = isReset row := by
  simp [reset, isReset, RESET_KIND]

theorem kept_iff (row : RevlogRow) : kept row = isReview row := by
  simp only [kept, isReview, DROPPED_KINDS]
  grind

theorem cut (sorted : List RevlogRow) (strict : sorted.Pairwise fun a b => a.id < b.id) :
    (sorted.drop ((rposition reset sorted).getD 0)).filter kept =
      sorted.filter (counted sorted) := by
  induction sorted with
  | nil => simp [rposition]
  | cons first rest ih =>
    rw [List.pairwise_cons] at strict
    cases found : rposition reset rest with
    | some index =>
      have outer : rposition reset (first :: rest) = some (index + 1) := by
        simp [rposition, found]
      rw [outer, Option.getD_some, List.drop_succ_cons]
      have inner := ih strict.2
      rw [found, Option.getD_some] at inner
      rw [inner, List.filter_cons]
      obtain ⟨later, mem, isLater⟩ := rposition_some_admits found
      have notFirst : counted (first :: rest) first = false := by
        have lt := strict.1 later mem
        simp only [counted, List.all_cons, Bool.and_eq_false_iff]
        right; right
        simp only [List.all_eq_false]
        exact ⟨later, mem, by rw [reset_iff] at isLater; simp [isLater]; omega⟩
      rw [notFirst]
      simp only [Bool.false_eq_true, ite_false]
      apply List.filter_congr
      intro row mem
      have lt := strict.1 row mem
      simp [counted, List.all_cons, lt]
    | none =>
      have none : ∀ row ∈ rest, isReset row = false := by
        intro row mem
        rw [← reset_iff]; exact rposition_none_refuses found row mem
      have start : (rposition reset (first :: rest)).getD 0 = 0 := by
        simp only [rposition, found]
        split <;> rfl
      rw [start, List.drop_zero]
      apply List.filter_congr
      intro row mem
      rw [kept_iff]
      simp only [counted]
      cases review : isReview row
      · rfl
      · simp only [Bool.true_and]
        symm
        rw [List.all_eq_true]
        intro other otherMem
        rcases List.mem_cons.1 otherMem with rfl | otherMem
        · rcases List.mem_cons.1 mem with rfl | rowMem
          · have notReset : isReset row = false := by
              simp only [isReview, isReset] at review ⊢
              grind
            simp [notReset]
          · have := strict.1 row rowMem
            simp [this]
        · simp [none other otherMem]

/-! ## The reviews -/

theorem reviewsOf_ratings : ∀ (previous : Option Int) (rows : List RevlogRow),
    (reviewsOf previous rows).map (·.rating) = rows.map (·.ease)
  | _, [] => rfl
  | previous, row :: rest => by simp [reviewsOf, reviewsOf_ratings (some row.id) rest]

theorem reviewsOf_nonneg : ∀ (previous : Option Int) (rows : List RevlogRow), ByIdLe rows →
    (∀ before, previous = some before → ∀ row ∈ rows, before ≤ row.id) →
    ∀ review ∈ reviewsOf previous rows, 0 ≤ review.delta
  | _, [], _, _ => by simp [reviewsOf]
  | previous, row :: rest, sorted, below => by
    rw [ByIdLe, List.pairwise_cons] at sorted
    intro review mem
    simp only [reviewsOf, List.mem_cons] at mem
    rcases mem with rfl | mem
    · cases previous with
      | none => simp
      | some before =>
        have := below before rfl row List.mem_cons_self
        simp only
        omega
    · exact reviewsOf_nonneg (some row.id) rest sorted.2
        (fun before same later mem => by cases same; exact sorted.1 later mem) review mem

theorem all_eq_of_perm {p : RevlogRow → Bool} {rows₁ rows₂ : List RevlogRow}
    (perm : rows₁.Perm rows₂) : rows₁.all p = rows₂.all p := by
  apply Bool.eq_iff_iff.2
  simp only [List.all_eq_true]
  exact ⟨fun h row mem => h row (perm.mem_iff.2 mem), fun h row mem => h row (perm.mem_iff.1 mem)⟩

theorem counted_eq_of_perm {rows₁ rows₂ : List RevlogRow} (perm : rows₁.Perm rows₂) :
    counted rows₁ = counted rows₂ := by
  funext row
  simp only [counted, all_eq_of_perm perm]

theorem strict_of_sorted {rows : List RevlogRow} (sorted : ByIdLe rows)
    (nodup : (rows.map (·.id)).Nodup) : rows.Pairwise fun a b => a.id < b.id := by
  have distinct : rows.Pairwise fun a b => a.id ≠ b.id := List.pairwise_map.1 nodup
  exact (sorted.and distinct).imp fun both => by have := both.1; have := both.2; omega

theorem history_spec {cid : Int} {rows : List RevlogRow} {h : CardHistory}
    (found : history cid rows = some h) (nodup : (rows.map (·.id)).Nodup) :
    h.cid = cid ∧
      (h.reviews.map (·.rating)).Perm ((rows.filter (counted rows)).map (·.ease)) ∧
      h.reviews.head?.map (·.delta) = some 0 ∧
      ∀ review ∈ h.reviews, 0 ≤ review.delta := by
  have perm := sortById_perm rows
  have sorted := sortById_sorted rows
  have strict := strict_of_sorted sorted ((perm.map (·.id)).nodup_iff.2 nodup)
  have selection := cut (sortById rows) strict
  rw [counted_eq_of_perm perm] at selection
  simp only [history] at found
  rw [selection] at found
  have sub : ((sortById rows).filter (counted rows)).Sublist (sortById rows) :=
    List.filter_sublist
  have ratings : (((sortById rows).filter (counted rows)).map (·.ease)).Perm
      ((rows.filter (counted rows)).map (·.ease)) := (perm.filter _).map _
  generalize (sortById rows).filter (counted rows) = selected at found sub ratings
  split at found
  · simp at found
  · next last heq =>
    injection found with found
    subst found
    refine ⟨rfl, ?_, ?_, ?_⟩
    · simpa only [reviewsOf_ratings] using ratings
    · cases selected with
      | nil => simp at heq
      | cons first rest => simp [reviewsOf]
    · exact reviewsOf_nonneg none selected (sorted.sublist sub) (by simp)

/-- The selection keeps, for each card, exactly its rated reviews after its last reset, from the
distance 0 and never back in time, whatever order the rows arrive in. -/
theorem the_selection_keeps_the_rated_reviews_after_the_last_reset : Claim histories := by
  intro rows nodup
  refine ⟨fun rows' perm => histories_eq_of_perm perm ((perm.map (·.id)).nodup_iff.2 nodup), ?_⟩
  intro h mem
  obtain ⟨cid, -, found⟩ := List.mem_filterMap.1 mem
  obtain ⟨same, ratings, head, nonneg⟩ := history_spec found (nodup_ids_filter _ nodup)
  subst same
  exact ⟨ratings, head, nonneg⟩

/-! ## The witness -/

/-- `histories` at the merge base (`crates/fsrs7/src/convert.rs:43`, `kept` at `:73`): the kept
rows grouped by card in a `BTreeMap` and sorted by id, with no reset cut. Its history named no last
review, so the port's is 0. -/
def historiesAtMergeBase (rows : List RevlogRow) : List CardHistory :=
  let selected := rows.filter kept
  (cids selected).map fun cid =>
    ⟨cid, 0, reviewsOf none (sortById (selected.filter (·.cid == cid)))⟩

/-- One card reviewed, reset by the engine's Forget, and reviewed again. -/
def reviewResetReview : List RevlogRow :=
  [⟨7, 1, 3, 1, 2500⟩, ⟨7, 2, 0, 4, 0⟩, ⟨7, 3, 3, 1, 2500⟩]

theorem merge_base_keeps_a_review_before_its_reset : ¬ Claim historiesAtMergeBase := by
  intro claim
  have judged := (claim reviewResetReview (by decide)).2 ⟨7, 0, [⟨3, 0⟩, ⟨3, 2⟩]⟩ (by decide)
  exact absurd judged.1.length_eq (by decide)

end Formal.ReplayHistory
