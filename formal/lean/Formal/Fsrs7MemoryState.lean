-- @phx covers crates/fsrs7/src/replay.rs anchor=replay digest=sha256:672f1a84f5825d9e752bde61903bc327a8839a1145a6f3424f95db667ce03999
-- @phx cites #823
-- @phx theorem a_replayed_state_lies_in_the_schedulers_bounds ramp=report
-- @phx witness a_replay_with_no_finiteness_check_returns_a_state_outside_the_bounds kills=a_replayed_state_lies_in_the_schedulers_bounds
-- @phx theorem the_batch_replays_each_card_as_alone ramp=report
-- @phx witness a_batch_keyed_on_the_starting_states_alone_gives_every_card_the_first_cards_state kills=the_batch_replays_each_card_as_alone
-- @phx theorem a_review_reads_through_its_clamped_rating_and_gap ramp=report
-- @phx witness a_replay_that_reads_the_raw_gap_tells_a_negative_gap_from_zero kills=a_review_reads_through_its_clamped_rating_and_gap
-- @phx vectors formal/vectors/fsrs7-memory-state.jsonl

/-!
# Formal.Fsrs7MemoryState

The FSRS-7 replay of card histories into memory states (#823): `replay` in
`crates/fsrs7/src/replay.rs`, which runs each history through the pinned scheduler crate's
`FSRS::memory_state`, card by card, or through its `memory_state_batch`.

**The claims.** For every value the pinned crate's per-review arithmetic can produce, NaN and both
infinities included, and every list of histories with no empty history:
- `a_replayed_state_lies_in_the_schedulers_bounds`: every state a replay returns has its
  stability and its fast stability in the crate's range of 0.0001 to 36500 days and its
  difficulty in 1 to 10, all finite. A non-finite state is refused, never returned.
- `the_batch_replays_each_card_as_alone`: the batch method returns exactly what the card-by-card
  method returns, the refusal included, so its shortcut for a list of equal histories changes no
  card's state.
- `a_review_reads_through_its_clamped_rating_and_gap`: a rating past 4 replays as 4, and a gap
  below zero, or NaN, replays as a gap of zero, for both methods; a rating of 0 keeps its own
  meaning.

**What is ported.** The pinned revision's `FSRS::memory_state`, `memory_state_batch`,
`forward_reviews`, `init_state_from_first_review`, `step` and `validate_state`, branch for branch,
at the place each clamps its values, compares a rating with 0, or reads a gap through
`max(0.0)`. An `f32` is NaN, an infinity or a finite value as its exact rational; `f32::clamp`
passes NaN through and holds an infinity at its bound, and `f32::max(x, 0.0)` reads NaN as 0.
Two histories compare as Rust's `PartialEq` compares them: a NaN gap equals nothing.

**What is not ported, and why.** The arithmetic between the clamps: the forgetting curve, the
stability and difficulty updates, and the fast stability's product with 0.8. Each is a parameter
of the claims (`Ops`), so every claim holds for every value they compute, finite or not, and so
rests on no fact about `powf`, `exp` or `ln`. The crate's source is not in this tree, so no cover
names it: a re-pin of the crate does not stale this entry.
-/

namespace Formal.Fsrs7MemoryState

/-- An `f32` as the code reads it: NaN, an infinity, or a finite value as its exact rational. -/
inductive F32 where
  | nan
  | negInf
  | posInf
  | fin (q : Rat)
  deriving DecidableEq, Repr

/-- `f32::clamp(lo, hi)` for `lo ≤ hi`: NaN stays NaN, and an infinity is held at its bound. -/
def F32.clamp (x : F32) (lo hi : Rat) : F32 :=
  match x with
  | .nan => .nan
  | .negInf => .fin lo
  | .posInf => .fin hi
  | .fin q => if q < lo then .fin lo else if hi < q then .fin hi else .fin q

/-- `f32::max(x, 0.0)`: NaN reads 0, as IEEE `maxNum` gives the other operand. -/
def F32.maxZero (x : F32) : F32 :=
  match x with
  | .nan => .fin 0
  | .negInf => .fin 0
  | .posInf => .posInf
  | .fin q => .fin (max q 0)

/-- `f32::is_finite`. -/
def F32.finite : F32 → Bool
  | .fin _ => true
  | _ => false

/-- IEEE equality, Rust's `==` on `f32`: NaN equals nothing. -/
def F32.ieeeEq : F32 → F32 → Bool
  | .fin a, .fin b => a == b
  | .posInf, .posInf => true
  | .negInf, .negInf => true
  | _, _ => false

/-- The crate's bounds: `S_MIN`, `S_MAX`, `D_MIN` and `D_MAX`. -/
def sMin : Rat := 1 / 10000
def sMax : Rat := 36500
def dMin : Rat := 1
def dMax : Rat := 10

/-- A memory state: the stability, the difficulty and the fast stability. -/
structure State where
  s : F32
  d : F32
  sf : F32
  deriving DecidableEq, Repr

/-- `MemoryState`'s `PartialEq`, field by field over IEEE equality. -/
def State.ieeeEq (a b : State) : Bool :=
  a.s.ieeeEq b.s && a.d.ieeeEq b.d && a.sf.ieeeEq b.sf

/-- A review: its rating, a `u32`, and its gap in days, an `f32`. -/
structure Review where
  rating : Nat
  gap : F32
  deriving DecidableEq, Repr

/-- `FSRSReview`'s `PartialEq`. -/
def Review.ieeeEq (a b : Review) : Bool :=
  a.rating == b.rating && a.gap.ieeeEq b.gap

/-- `FSRSItem`'s `PartialEq`: equal lengths, equal reviews in order. -/
def itemEq : List Review → List Review → Bool
  | [], [] => true
  | a :: as, b :: bs => a.ieeeEq b && itemEq as bs
  | _, _ => false

/-- `Option<MemoryState>`'s `PartialEq`. -/
def startEq : Option State → Option State → Bool
  | none, none => true
  | some a, some b => a.ieeeEq b
  | _, _ => false

/-- `rating.clamp(1, 4)`. -/
def clampRating (r : Nat) : Nat := min (max r 1) 4

/-- The pinned crate's arithmetic between its clamps, left abstract: the first review's stability
(`w[rating - 1]`) and difficulty, the fast stability's product with 0.8, and the raw slow
stability, difficulty and fast stability of `fsrs7_next_state_scalar` before each one's last
clamp, read at the clamped last state, the gap through `max(0.0)` and the clamped rating. -/
structure Ops where
  initS : Nat → F32
  initD : Nat → F32
  fast : F32 → F32
  slow : State → F32 → Nat → F32
  diff : State → F32 → Nat → F32
  fastNext : State → F32 → Nat → F32

/-- `init_state_from_first_review`. -/
def initState (o : Ops) (r : Review) : State :=
  if r.rating = 0 then ⟨.fin sMin, .fin dMin, .fin sMin⟩
  else
    let rr := clampRating r.rating
    let s := (o.initS rr).clamp sMin sMax
    ⟨s, (o.initD rr).clamp dMin dMax, (o.fast s).clamp sMin sMax⟩

/-- `step`, with the gap read through `readGap`: `max(0.0)` in the crate. -/
def stepWith (readGap : F32 → F32) (o : Ops) (gap : F32) (rating : Nat) (state : State)
    (nth : Nat) : State :=
  let last : State := ⟨state.s.clamp sMin sMax, state.d.clamp dMin dMax, state.sf.clamp sMin sMax⟩
  let g := readGap gap
  let rr := clampRating rating
  let next0 : State :=
    ⟨(o.slow last g rr).clamp sMin sMax, (o.diff last g rr).clamp dMin dMax,
      (o.fastNext last g rr).clamp sMin sMax⟩
  let next : State :=
    if nth = 0 ∧ state.s.ieeeEq (.fin 0) then
      let s := (o.initS rr).clamp sMin sMax
      ⟨s, (o.initD rr).clamp dMin dMax, (o.fast s).clamp sMin sMax⟩
    else next0
  if rating = 0 then last else next

/-- The loop of `forward_reviews` from review index `n`. -/
def walkWith (readGap : F32 → F32) (o : Ops) : State → Nat → List Review → State
  | st, _, [] => st
  | st, n, r :: rs => walkWith readGap o (stepWith readGap o r.gap r.rating st n) (n + 1) rs

/-- `forward_reviews`. -/
def forwardWith (readGap : F32 → F32) (o : Ops) (item : List Review) (start : Option State) :
    State :=
  match start, item with
  | some st, _ => walkWith readGap o st 0 item
  | none, [] => ⟨.fin 0, .fin 0, .fin 0⟩
  | none, r :: rs => walkWith readGap o (initState o r) 1 rs

/-- `validate_state` for FSRS-7: all three values finite. -/
def State.finite (st : State) : Bool := st.s.finite && st.d.finite && st.sf.finite

/-- `FSRS::memory_state`, with or without its `validate_state`. -/
def memoryStateWith (readGap : F32 → F32) (validated : Bool) (o : Ops) (item : List Review)
    (start : Option State) : Option State :=
  let st := forwardWith readGap o item start
  if !validated || st.finite then some st else none

/-- `memory_state_batch`, its shortcut keyed on the histories and the starting states, or on the
starting states alone. -/
def batchWith (readGap : F32 → F32) (validated keyOnItems : Bool) (o : Ops)
    (items : List (List Review)) (starts : List (Option State)) : Option (List State) :=
  if items.isEmpty then some []
  else if items.length ≠ starts.length then none
  else
    match items, starts with
    | i0 :: _, s0 :: _ =>
      if (!keyOnItems || items.all (itemEq · i0)) && starts.all (startEq · s0) then
        (memoryStateWith readGap validated o i0 s0).map (List.replicate items.length)
      else (items.zip starts).mapM fun p => memoryStateWith readGap validated o p.1 p.2
    | _, _ => none

/-- `Method`. -/
inductive Method where
  | single
  | batch
  deriving DecidableEq, Repr

/-- `replay`: each history with no starting state, card by card or in one batch. -/
def replayWith (readGap : F32 → F32) (validated keyOnItems : Bool) (o : Ops) :
    Method → List (List Review) → Option (List State)
  | .single, items => items.mapM fun item => memoryStateWith readGap validated o item none
  | .batch, items =>
    batchWith readGap validated keyOnItems o items (List.replicate items.length none)

/-- The code: the gap through `max(0.0)`, `validate_state`, and the shortcut keyed on both. -/
def replay : Ops → Method → List (List Review) → Option (List State) :=
  replayWith F32.maxZero true true

/-! ## The claims -/

/-- A state in the crate's bounds, every value finite. -/
def Bounded (st : State) : Prop :=
  ∃ s d sf, st = ⟨.fin s, .fin d, .fin sf⟩ ∧ sMin ≤ s ∧ s ≤ sMax ∧ dMin ≤ d ∧ d ≤ dMax ∧
    sMin ≤ sf ∧ sf ≤ sMax

/-- The bounds claim over a replay. -/
def BoundsRule (rep : Ops → Method → List (List Review) → Option (List State)) : Prop :=
  ∀ o m items states, rep o m items = some states → (∀ i ∈ items, i ≠ []) →
    ∀ st ∈ states, Bounded st

/-- The batch claim over a replay. -/
def BatchRule (rep : Ops → Method → List (List Review) → Option (List State)) : Prop :=
  ∀ o items, rep o .batch items = rep o .single items

/-- A review as the crate reads it: a rating past 4 as 4, and the gap through `max(0.0)`. -/
def Review.normal (r : Review) : Review :=
  ⟨if r.rating = 0 then 0 else min r.rating 4, r.gap.maxZero⟩

/-- The reading claim over a replay. -/
def ReadingRule (rep : Ops → Method → List (List Review) → Option (List State)) : Prop :=
  ∀ o m items, rep o m items = rep o m (items.map (·.map Review.normal))

/-! ## The proofs -/

/-- A value a clamp returns is NaN or a finite value inside the bounds. -/
def InRange (x : F32) (lo hi : Rat) : Prop :=
  x = .nan ∨ ∃ q, x = .fin q ∧ lo ≤ q ∧ q ≤ hi

theorem clamp_inRange (x : F32) {lo hi : Rat} (h : lo ≤ hi) : InRange (x.clamp lo hi) lo hi := by
  unfold InRange F32.clamp
  rcases x with _ | _ | _ | q
  · exact Or.inl rfl
  · exact Or.inr ⟨lo, rfl, Rat.le_refl, h⟩
  · exact Or.inr ⟨hi, rfl, h, Rat.le_refl⟩
  · by_cases h1 : q < lo
    · simp only [h1, ↓reduceIte]; exact Or.inr ⟨lo, rfl, Rat.le_refl, h⟩
    · by_cases h2 : hi < q
      · simp only [h1, h2, ↓reduceIte]; exact Or.inr ⟨hi, rfl, h, Rat.le_refl⟩
      · simp only [h1, h2, ↓reduceIte]; exact Or.inr ⟨q, rfl, by grind, by grind⟩

/-- Every value of the state is NaN or inside its bounds. -/
def Shaped (st : State) : Prop :=
  InRange st.s sMin sMax ∧ InRange st.d dMin dMax ∧ InRange st.sf sMin sMax

theorem sMin_le : sMin ≤ sMax := by decide +kernel
theorem dMin_le : dMin ≤ dMax := by decide +kernel

theorem initState_shaped (o : Ops) (r : Review) : Shaped (initState o r) := by
  unfold initState
  split
  · refine ⟨Or.inr ⟨sMin, rfl, Rat.le_refl, sMin_le⟩, Or.inr ⟨dMin, rfl, Rat.le_refl, dMin_le⟩,
      Or.inr ⟨sMin, rfl, Rat.le_refl, sMin_le⟩⟩
  · exact ⟨clamp_inRange _ sMin_le, clamp_inRange _ dMin_le, clamp_inRange _ sMin_le⟩

theorem step_shaped (readGap : F32 → F32) (o : Ops) (gap : F32) (rating : Nat) (st : State)
    (nth : Nat) : Shaped (stepWith readGap o gap rating st nth) := by
  unfold stepWith
  simp only []
  split
  · exact ⟨clamp_inRange _ sMin_le, clamp_inRange _ dMin_le, clamp_inRange _ sMin_le⟩
  · split
    · exact ⟨clamp_inRange _ sMin_le, clamp_inRange _ dMin_le, clamp_inRange _ sMin_le⟩
    · exact ⟨clamp_inRange _ sMin_le, clamp_inRange _ dMin_le, clamp_inRange _ sMin_le⟩

theorem walk_shaped (readGap : F32 → F32) (o : Ops) (st : State) (n : Nat) (rs : List Review)
    (h : Shaped st) : Shaped (walkWith readGap o st n rs) := by
  induction rs generalizing st n with
  | nil => exact h
  | cons r rs ih => exact ih _ _ (step_shaped readGap o r.gap r.rating st n)

theorem forward_shaped (readGap : F32 → F32) (o : Ops) (item : List Review) (h : item ≠ []) :
    Shaped (forwardWith readGap o item none) := by
  rcases item with _ | ⟨r, rs⟩
  · exact absurd rfl h
  · exact walk_shaped readGap o _ 1 rs (initState_shaped o r)

theorem inRange_finite {x : F32} {lo hi : Rat} (h : InRange x lo hi) (hf : x.finite = true) :
    ∃ q, x = .fin q ∧ lo ≤ q ∧ q ≤ hi := by
  rcases h with h | h
  · subst h; simp [F32.finite] at hf
  · exact h

theorem shaped_bounded {st : State} (h : Shaped st) (hf : st.finite = true) : Bounded st := by
  simp only [State.finite, Bool.and_eq_true] at hf
  obtain ⟨⟨hs, hd⟩, hsf⟩ := hf
  obtain ⟨s, es, s1, s2⟩ := inRange_finite h.1 hs
  obtain ⟨d, ed, d1, d2⟩ := inRange_finite h.2.1 hd
  obtain ⟨sf, esf, sf1, sf2⟩ := inRange_finite h.2.2 hsf
  rcases st with ⟨a, b, c⟩
  simp only at es ed esf
  subst es ed esf
  exact ⟨s, d, sf, rfl, s1, s2, d1, d2, sf1, sf2⟩

theorem memoryState_bounded (o : Ops) (item : List Review) (h : item ≠ []) (st : State)
    (hm : memoryStateWith F32.maxZero true o item none = some st) : Bounded st := by
  simp only [memoryStateWith, Bool.not_true, Bool.false_or] at hm
  split at hm
  · simp only [Option.some.injEq] at hm
    subst hm
    exact shaped_bounded (forward_shaped _ o item h) (by assumption)
  · exact absurd hm (by simp)

theorem mapM_some {α β : Type} (f : α → Option β) (xs : List α) (ys : List β)
    (h : xs.mapM f = some ys) : ∀ y ∈ ys, ∃ x ∈ xs, f x = some y := by
  induction xs generalizing ys with
  | nil =>
    simp at h
    subst h
    simp
  | cons x xs ih =>
    simp only [List.mapM_cons, Option.bind_eq_bind] at h
    cases hx : f x with
    | none => simp [hx] at h
    | some b =>
      cases hxs : xs.mapM f with
      | none => simp [hx, hxs] at h
      | some bs =>
        simp [hx, hxs] at h
        subst h
        intro y hy
        simp only [List.mem_cons] at hy
        rcases hy with hy | hy
        · exact ⟨x, by simp, by rw [hx, hy]⟩
        · obtain ⟨x', hx', e⟩ := ih bs hxs y hy
          exact ⟨x', by simp [hx'], e⟩

theorem itemEq_eq : ∀ (a b : List Review), itemEq a b = true → a = b
  | [], [], _ => rfl
  | [], _ :: _, h => by simp [itemEq] at h
  | _ :: _, [], h => by simp [itemEq] at h
  | x :: xs, y :: ys, h => by
    simp only [itemEq, Bool.and_eq_true] at h
    obtain ⟨h1, h2⟩ := h
    have e := itemEq_eq xs ys h2
    simp only [Review.ieeeEq, Bool.and_eq_true, beq_iff_eq] at h1
    obtain ⟨hr, hg⟩ := h1
    rcases x with ⟨xr, xg⟩
    rcases y with ⟨yr, yg⟩
    simp only at hr hg
    subst hr e
    rcases xg with _ | _ | _ | a <;> rcases yg with _ | _ | _ | b <;>
      simp_all [F32.ieeeEq]

theorem mapM_replicate {α β : Type} (f : α → Option β) (x : α) :
    ∀ (ys : List α), (∀ y ∈ ys, y = x) →
      (x :: ys).mapM f = (f x).map (List.replicate (ys.length + 1))
  | [], _ => by cases hfx : f x <;> simp [hfx]
  | y :: ys, h => by
    have hy : y = x := h y (by simp)
    have ih := mapM_replicate f x ys (fun z hz => h z (by simp [hz]))
    subst hy
    rw [List.mapM_cons, ih]
    cases f y <;> simp [List.replicate_succ]

theorem mapM_zip_none {β : Type} (f : List Review → Option State → Option β) :
    ∀ (items : List (List Review)),
      (items.zip (List.replicate items.length none)).mapM (fun p => f p.1 p.2) =
        items.mapM (fun i => f i none)
  | [] => by simp
  | i :: is => by
    simp only [List.length_cons, List.replicate_succ, List.zip_cons_cons, List.mapM_cons]
    rw [mapM_zip_none f is]

theorem batch_single (readGap : F32 → F32) (validated : Bool) (o : Ops)
    (items : List (List Review)) :
    replayWith readGap validated true o .batch items =
      replayWith readGap validated true o .single items := by
  simp only [replayWith, batchWith]
  rcases items with _ | ⟨i0, rest⟩
  · simp
  · simp only [List.isEmpty_cons, Bool.false_eq_true, ↓reduceIte, List.length_replicate,
      ne_eq, not_true_eq_false, List.length_cons, List.replicate_succ, Bool.not_true,
      Bool.false_or]
    have hstarts : (none :: List.replicate rest.length none).all (startEq · none) = true := by
      simp only [List.all_cons, List.all_eq_true, Bool.and_eq_true]
      refine ⟨rfl, ?_⟩
      intro x hx
      simp only [List.mem_replicate] at hx
      rw [hx.2]
      rfl
    rw [hstarts, Bool.and_true]
    split
    · rename_i hall
      have heq : ∀ y ∈ i0 :: rest, y = i0 := by
        intro y hy
        exact itemEq_eq y i0 (List.all_eq_true.mp hall y hy)
      rw [mapM_replicate _ i0 rest (fun y hy => heq y (by simp [hy]))]
    · have := mapM_zip_none (fun i s => memoryStateWith readGap validated o i s) (i0 :: rest)
      simp only [List.length_cons, List.replicate_succ] at this
      exact this

theorem the_batch_replays_each_card_as_alone : BatchRule replay := by
  intro o items
  exact batch_single F32.maxZero true o items

theorem a_replayed_state_lies_in_the_schedulers_bounds : BoundsRule replay := by
  intro o m items states h hne st hst
  have hs : items.mapM (fun item => memoryStateWith F32.maxZero true o item none) =
      some states := by
    cases m with
    | single => exact h
    | batch =>
      have := batch_single F32.maxZero true o items
      simp only [replay] at h
      rw [this] at h
      exact h
  obtain ⟨item, hi, e⟩ := mapM_some _ items states hs st hst
  exact memoryState_bounded o item (hne item hi) st e

theorem clampRating_min (r : Nat) (_h : r ≠ 0) : clampRating (min r 4) = clampRating r := by
  simp only [clampRating]
  omega

theorem max_max_zero (q : Rat) : max (max q 0) 0 = max q 0 := by
  by_cases h : q ≤ 0
  · have e : max q 0 = 0 := by rw [Rat.max_def]; simp [h]
    rw [e, Rat.max_def]
    simp
  · have e : max q 0 = q := by rw [Rat.max_def]; simp [h]
    rw [e, Rat.max_def]
    simp [h]

theorem maxZero_maxZero (g : F32) : g.maxZero.maxZero = g.maxZero := by
  rcases g with _ | _ | _ | q
  · rfl
  · simp [F32.maxZero, Rat.max_def]
  · rfl
  · simp only [F32.maxZero, max_max_zero]

theorem step_normal (o : Ops) (r : Review) (st : State) (n : Nat) :
    stepWith F32.maxZero o r.normal.gap r.normal.rating st n =
      stepWith F32.maxZero o r.gap r.rating st n := by
  simp only [Review.normal]
  by_cases h : r.rating = 0
  · simp [stepWith, h]
  · have hm : min r.rating 4 ≠ 0 := by omega
    unfold stepWith
    simp only [h, hm, ↓reduceIte, clampRating_min r.rating h, maxZero_maxZero]

theorem walk_normal (o : Ops) (st : State) (n : Nat) (rs : List Review) :
    walkWith F32.maxZero o st n (rs.map Review.normal) = walkWith F32.maxZero o st n rs := by
  induction rs generalizing st n with
  | nil => rfl
  | cons r rs ih =>
    simp only [List.map_cons, walkWith]
    rw [step_normal o r st n, ih]

theorem initState_normal (o : Ops) (r : Review) : initState o r.normal = initState o r := by
  simp only [Review.normal]
  by_cases h : r.rating = 0
  · simp [initState, h]
  · have hm : min r.rating 4 ≠ 0 := by omega
    unfold initState
    simp only [h, hm, ↓reduceIte, clampRating_min r.rating h]

theorem forward_normal (o : Ops) (item : List Review) :
    forwardWith F32.maxZero o (item.map Review.normal) none = forwardWith F32.maxZero o item none := by
  rcases item with _ | ⟨r, rs⟩
  · rfl
  · simp only [forwardWith, List.map_cons]
    rw [walk_normal, initState_normal]

theorem memoryState_normal (o : Ops) (item : List Review) :
    memoryStateWith F32.maxZero true o (item.map Review.normal) none =
      memoryStateWith F32.maxZero true o item none := by
  simp only [memoryStateWith, forward_normal]

theorem a_review_reads_through_its_clamped_rating_and_gap : ReadingRule replay := by
  intro o m items
  have single : ∀ (xs : List (List Review)),
      xs.mapM (fun item => memoryStateWith F32.maxZero true o item none) =
        (xs.map (·.map Review.normal)).mapM
          (fun item => memoryStateWith F32.maxZero true o item none) := by
    intro xs
    induction xs with
    | nil => rfl
    | cons x xs ih => simp only [List.map_cons, List.mapM_cons, memoryState_normal, ih]
  cases m with
  | single => exact single items
  | batch =>
    simp only [replay]
    rw [batch_single, batch_single]
    exact single items

/-! ## The witnesses -/

/-- A replay that returns `forward_reviews`'s state with no `validate_state`. -/
def replayWithNoFinitenessCheck : Ops → Method → List (List Review) → Option (List State) :=
  replayWith F32.maxZero false true

/-- Arithmetic whose first-review stability is NaN, as `w[0]` NaN would make it. -/
def nanOps : Ops :=
  ⟨fun _ => .nan, fun _ => .fin 5, fun s => s, fun _ _ _ => .nan, fun _ _ _ => .fin 5,
    fun _ _ _ => .nan⟩

/-- A one-review history whose first stability is NaN is returned, not refused. -/
theorem a_replay_with_no_finiteness_check_returns_a_state_outside_the_bounds :
    ¬ BoundsRule replayWithNoFinitenessCheck := by
  intro h
  have := h nanOps .single [[⟨3, .fin 0⟩]] [⟨.nan, .fin 5, .nan⟩] (by decide +kernel)
    (by simp) ⟨.nan, .fin 5, .nan⟩ (by simp)
  obtain ⟨s, d, sf, e, _⟩ := this
  simp at e

/-- A batch whose shortcut reads only the starting states, which `replay` always passes as equal. -/
def replayKeyedOnTheStartingStates : Ops → Method → List (List Review) → Option (List State) :=
  replayWith F32.maxZero true false

/-- Arithmetic whose first-review stability is the rating. -/
def ratingOps : Ops :=
  ⟨fun r => .fin r, fun _ => .fin 5, fun s => s, fun _ _ _ => .fin 1, fun _ _ _ => .fin 5,
    fun _ _ _ => .fin 1⟩

/-- An Again card and an Easy card, each one review: the keyed batch gives both the Again
card's state. -/
theorem a_batch_keyed_on_the_starting_states_alone_gives_every_card_the_first_cards_state :
    ¬ BatchRule replayKeyedOnTheStartingStates := by
  intro h
  have := h ratingOps [[⟨1, .fin 0⟩], [⟨4, .fin 0⟩]]
  revert this
  decide +kernel

/-- A replay that hands the arithmetic the raw gap. -/
def replayReadingTheRawGap : Ops → Method → List (List Review) → Option (List State) :=
  replayWith id true true

/-- Arithmetic that is NaN at a negative gap, finite elsewhere. -/
def gapOps : Ops :=
  ⟨fun _ => .fin 1, fun _ => .fin 5, fun s => s,
    fun _ g _ => match g with
      | .fin q => if q < 0 then .nan else .fin 1
      | _ => .fin 1,
    fun _ _ _ => .fin 5, fun _ _ _ => .fin 1⟩

/-- A Good review four days before the one it follows: read raw, it is refused, and read as the
crate reads it, at a gap of zero, it is not. -/
theorem a_replay_that_reads_the_raw_gap_tells_a_negative_gap_from_zero :
    ¬ ReadingRule replayReadingTheRawGap := by
  intro h
  have := h gapOps .single [[⟨3, .fin 0⟩, ⟨3, .fin (-4)⟩]]
  revert this
  decide +kernel

end Formal.Fsrs7MemoryState
