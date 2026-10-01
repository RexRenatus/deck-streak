-- @phx cites #76
-- @phx theorem pick_is_least ramp=report
-- @phx witness pick_last_violates kills=pick_is_least
-- @phx theorem ties_go_to_the_earlier_ladder ramp=report
-- @phx witness pick_later_on_tie_violates kills=ties_go_to_the_earlier_ladder
-- @phx theorem all_complete_is_the_top_review_rung ramp=report
-- @phx witness no_top_rung_violates kills=all_complete_is_the_top_review_rung

/-! The next milestone is chosen by a pure rule. A candidate is the nearest unreached rung of one
ladder; its remaining fraction is `(rung - value) / rung`, compared by cross-multiplication so the
rule is exact over the rationals. The ladders are folded in the tie order: reviews, streak, then
mature cards. -/

namespace Formal.NextMilestone

/-- A ladder's nearest unreached rung: the ladder's place in the tie order, the rung, and the count
measured against it. -/
structure Cand where
  idx : Nat
  rung : Nat
  value : Nat
deriving DecidableEq

/-- `a` has strictly the smaller remaining fraction. -/
def Smaller (a b : Cand) : Prop :=
  (a.rung - a.value) * b.rung < (b.rung - b.value) * a.rung

instance (a b : Cand) : Decidable (Smaller a b) := by unfold Smaller; infer_instance

/-- The earlier candidate keeps a tie; the later one wins only when strictly smaller. -/
def pick (a b : Option Cand) : Option Cand :=
  match a, b with
  | none, x => x
  | x, none => x
  | some x, some y => if Smaller y x then some y else some x

/-- Pick over the three ladders in the tie order. -/
def nextMilestone (c1 c2 c3 : Option Cand) : Option Cand := pick (pick c1 c2) c3

/-- With all three ladders complete the answer is the top review rung (100%). -/
def milestone (c1 c2 c3 : Option Cand) (top : Cand) : Option Cand :=
  some ((nextMilestone c1 c2 c3).getD top)

/-- The wrong variant: a tie goes to the later ladder. -/
def pickLater (a b : Option Cand) : Option Cand :=
  match a, b with
  | none, x => x
  | x, none => x
  | some x, some y => if Smaller x y then some x else some y

/-- The wrong variant: the last present ladder wins, whatever its fraction. -/
def pickLast (a b : Option Cand) : Option Cand :=
  match b with
  | none => a
  | some _ => b

/-- The wrong variant for completion: three complete ladders yield nothing to show. -/
def milestoneNoTop (c1 c2 c3 : Option Cand) (_top : Cand) : Option Cand :=
  nextMilestone c1 c2 c3

theorem smaller_trans (a b c : Cand) (ha : 0 < a.rung) (_hb : 0 < b.rung) (hc : 0 < c.rung) :
    Smaller a b → Smaller b c → Smaller a c := by
  unfold Smaller
  intro h1 h2
  have e1 := Nat.mul_lt_mul_of_pos_right h1 hc
  have e2 := Nat.mul_lt_mul_of_pos_right h2 ha
  have e3 : (a.rung - a.value) * c.rung * b.rung < (c.rung - c.value) * a.rung * b.rung := by
    calc (a.rung - a.value) * c.rung * b.rung
        = (a.rung - a.value) * b.rung * c.rung := by rw [Nat.mul_right_comm]
      _ < (b.rung - b.value) * a.rung * c.rung := e1
      _ = (b.rung - b.value) * c.rung * a.rung := by rw [Nat.mul_right_comm]
      _ < (c.rung - c.value) * b.rung * a.rung := e2
      _ = (c.rung - c.value) * a.rung * b.rung := by rw [Nat.mul_right_comm]
  exact Nat.lt_of_mul_lt_mul_right e3

theorem pick_none_right (x : Option Cand) : pick x none = x := by
  cases x <;> rfl

theorem pick_none_left (x : Option Cand) : pick none x = x := by
  cases x <;> rfl

theorem pick_from (a b : Option Cand) (s : Cand) (h : pick a b = some s) :
    a = some s ∨ b = some s := by
  cases a with
  | none => rw [pick_none_left] at h; exact Or.inr h
  | some x =>
    cases b with
    | none => rw [pick_none_right] at h; exact Or.inl h
    | some y =>
      by_cases hs : Smaller y x
      · simp [pick, hs] at h; subst h; simp
      · simp [pick, hs] at h; subst h; simp

theorem pick_none (a b : Option Cand) (h : pick a b = none) : a = none ∧ b = none := by
  cases a with
  | none => rw [pick_none_left] at h; exact ⟨rfl, h⟩
  | some x =>
    cases b with
    | none => rw [pick_none_right] at h; simp at h
    | some y => by_cases hs : Smaller y x <;> simp [pick, hs] at h

theorem pick_min (a b : Option Cand) (s : Cand) (h : pick a b = some s) :
    (∀ x, a = some x → ¬ Smaller x s) ∧ (∀ y, b = some y → ¬ Smaller y s) := by
  cases a with
  | none =>
    rw [pick_none_left] at h; subst h
    exact ⟨by simp, fun y hy => by simp at hy; subst hy; simp [Smaller]⟩
  | some x =>
    cases b with
    | none =>
      rw [pick_none_right] at h; simp at h; subst h
      exact ⟨fun z hz => by simp at hz; subst hz; simp [Smaller], by simp⟩
    | some y =>
      by_cases hs : Smaller y x
      · simp [pick, hs] at h; subst h
        refine ⟨fun z hz hz2 => ?_, fun z hz => ?_⟩
        · simp at hz; subst hz; unfold Smaller at hs hz2; omega
        · simp at hz; subst hz; simp [Smaller]
      · simp [pick, hs] at h; subst h
        exact ⟨fun z hz => by simp at hz; subst hz; simp [Smaller], fun z hz => by
          simp at hz; subst hz; exact hs⟩

/-- The claim, stated once: the three-way pick is minimal. -/
def Least (f : Option Cand → Option Cand → Option Cand) : Prop :=
  ∀ (a b c : Option Cand) (s : Cand), f (f a b) c = some s →
    (∀ x, a = some x → 0 < x.rung) → (∀ x, b = some x → 0 < x.rung) →
    (∀ x, c = some x → 0 < x.rung) →
    (∀ x, a = some x → ¬ Smaller x s) ∧ (∀ x, b = some x → ¬ Smaller x s) ∧
      (∀ x, c = some x → ¬ Smaller x s)

/-- A tie keeps the earlier ladder. -/
def TieToEarlier (f : Option Cand → Option Cand → Option Cand) : Prop :=
  ∀ x y : Cand, ¬ Smaller y x → ¬ Smaller x y → f (some x) (some y) = some x

/-- Three complete ladders answer the top rung, never nothing. -/
def CompleteIsTop (g : Option Cand → Option Cand → Option Cand → Cand → Option Cand) : Prop :=
  ∀ top : Cand, g none none none top = some top

theorem pick_is_least : Least pick := by
  intro a b c s h ha hb hc
  cases c with
  | none =>
    rw [pick_none_right] at h
    have hm := pick_min a b s h
    exact ⟨hm.1, hm.2, by simp⟩
  | some y =>
    have hy := hc y rfl
    cases hab : pick a b with
    | none =>
      obtain ⟨rfl, rfl⟩ := pick_none a b hab
      rw [hab, pick_none_left] at h
      simp at h; subst h
      exact ⟨by simp, by simp, fun z hz hz2 => by
        simp at hz; subst hz; exact absurd hz2 (by simp [Smaller])⟩
    | some x =>
      rw [hab] at h
      have hx : 0 < x.rung := by
        rcases pick_from a b x hab with h1 | h1
        · exact ha x h1
        · exact hb x h1
      have hm := pick_min a b x hab
      by_cases hs : Smaller y x
      · simp [pick, hs] at h; subst h
        refine ⟨fun z hz hz2 => ?_, fun z hz hz2 => ?_, fun z hz hz2 => ?_⟩
        · exact hm.1 z hz (smaller_trans z y x (ha z hz) hy hx hz2 hs)
        · exact hm.2 z hz (smaller_trans z y x (hb z hz) hy hx hz2 hs)
        · simp at hz; subst hz; exact absurd hz2 (by simp [Smaller])
      · simp [pick, hs] at h; subst h
        exact ⟨hm.1, hm.2, fun z hz hz2 => by simp at hz; subst hz; exact hs hz2⟩

theorem ties_go_to_the_earlier_ladder : TieToEarlier pick := by
  intro x y h1 _
  simp [pick, h1]

theorem all_complete_is_the_top_review_rung : CompleteIsTop milestone := by
  intro top
  simp [milestone, nextMilestone, pick]

theorem pick_last_violates : ¬ Least pickLast := by
  intro h
  have := h (some ⟨0, 2, 1⟩) (some ⟨1, 10, 0⟩) none ⟨1, 10, 0⟩ (by simp [pickLast])
    (by simp) (by simp) (by simp)
  exact this.1 ⟨0, 2, 1⟩ rfl (by simp [Smaller])

theorem pick_later_on_tie_violates : ¬ TieToEarlier pickLater := by
  intro h
  have := h ⟨0, 2, 1⟩ ⟨1, 2, 1⟩ (by simp [Smaller]) (by simp [Smaller])
  simp [pickLater, Smaller] at this

theorem no_top_rung_violates : ¬ CompleteIsTop milestoneNoTop := by
  intro h
  have := h ⟨0, 100, 0⟩
  simp [milestoneNoTop, nextMilestone, pick] at this

end Formal.NextMilestone
