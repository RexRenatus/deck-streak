-- @phx covers crates/fsrs7/src/stock.rs anchor=project digest=sha256:1a601a05f3bc33443049813794f0d2eba1e12c7fffb2b7bff4906e1a78d1eb71
-- @phx covers crates/fsrs7/src/stock.rs anchor=DIFFICULTY_MIN digest=sha256:c422caccdcd84138a84c828729330781f1a0ac36af1ab819a0617b28b1863b94
-- @phx covers crates/fsrs7/src/stock.rs anchor=DIFFICULTY_MAX digest=sha256:95738f66444b06fcb6744cc4ac26bdb9bc56c5831b50b8cb55399a0a43cefb85
-- @phx covers crates/fsrs7/src/stock.rs anchor=STOCK_RETENTION digest=sha256:1555dc14a511d6c197e35fefac3d9d82fed48de2c966fb0edef39e6c39458838
-- @phx covers crates/fsrs7/src/stock.rs anchor=stock_stability digest=sha256:9b52910b4a0a17edb9e7a6b932cd73d0186ba1d0826aae86c28c44122b71d02f
-- @phx covers crates/fsrs7/src/stock.rs anchor=STABILITY_MIN digest=sha256:845795996bdc6669a7f9c7e123204234b2e3929d93c025d637217a2d460cb249
-- @phx cites #823
-- @phx theorem the_stock_difficulty_lies_in_the_stock_range ramp=report
-- @phx witness a_difficulty_with_no_clamp_leaves_the_range kills=the_stock_difficulty_lies_in_the_stock_range
-- @phx theorem the_curve_reads_between_its_floor_and_its_ceiling ramp=report
-- @phx witness an_unsquashed_curve_reads_one_at_the_review kills=the_curve_reads_between_its_floor_and_its_ceiling
-- @phx theorem the_curve_never_rises_with_time ramp=report
-- @phx witness a_curve_read_at_a_negative_gap_rises_to_the_review kills=the_curve_never_rises_with_time
-- @phx theorem the_stock_stability_is_at_least_the_floor_and_stored_above_zero ramp=report
-- @phx witness an_unfloored_stability_stores_as_zero kills=the_stock_stability_is_at_least_the_floor_and_stored_above_zero
-- @phx vectors formal/vectors/fsrs7-stock.jsonl

/-!
# Formal.Fsrs7Stock

A replayed FSRS-7 state as the stock fields hold it (#823): `project` in
`crates/fsrs7/src/stock.rs`, and the pinned scheduler's forgetting curve, whose 0.9 point
`project` writes as the stock stability (`STOCK_RETENTION`), held at the revision's least stability
(`stock_stability`, `STABILITY_MIN`).

**The claims.**
- `the_stock_difficulty_lies_in_the_stock_range`: for every `f32` difficulty but NaN, infinities
  included, the stock difficulty is finite and between `DIFFICULTY_MIN` and `DIFFICULTY_MAX`; a
  difficulty already in the range is kept; and NaN passes through. A replayed state is never NaN:
  `Formal.Fsrs7MemoryState` proves every state a replay returns finite.
- `the_curve_reads_between_its_floor_and_its_ceiling`: the pinned curve reads at least 0.00001 and
  at most 0.99999 at every gap, and exactly 0.99999 at a gap of zero or less. It never reads 1:
  the curve's last step maps its mixture onto that band.
- `the_curve_never_rises_with_time`: a later gap never reads a higher retrievability, negative
  gaps included, which read as zero.
- `the_stock_stability_is_at_least_the_floor_and_stored_above_zero`: for every `f32` the 90-percent
  interval can be, NaN and the infinities included, the stock stability is positive infinity or a
  finite value at least `STABILITY_MIN` that the engine's four-place store keeps above zero; and an
  interval at or above the floor is kept as it is.

**What is ported.** The difficulty's `f32::clamp`, with NaN and the infinities. The curve,
`fsrs7_forgetting_curve_scalar_for_state` at the pinned revision: the gap read through
`max(0.0)`, the two components' mixture by their weights, and the final
`mul_add(1.0 - 2e-5, 1e-5)`, over exact rationals. The stock stability's `f32::max` with the floor,
with NaN and the infinities, and the floor as its exact `f32` value.

**What is not ported, and why.** Each component's power law and each weight's value: they are a
parameter of the claims (`Curve`), held to the laws the pinned code gives them (`Laws`): a weight
is positive, since the parameter clipper holds each factor above zero; a component reads 1 at a gap
of zero, since `powf(1.0, y)` is 1; and it lies in 0 to 1 and never rises over a non-negative gap,
since its base is at least 1 and its exponent negative. The curve's `f32` rounding is not ported:
the constants `1e-5` and `1.0 - 2e-5` are read as exact, and the monotonicity of `powf` is taken
from its specification, not from the platform's library. The 0.9 point's arithmetic, the crate's
Newton and bisection search, is not ported: the floor's claim holds for every `f32` it returns.
The engine's store, `(v * 10^4).round() / 10^4`, is the engine's code, not this repository's: it is
stated over exact rationals (`stored`), and its `f32` product is not ported; SPEC-413 A1 reads the
store's rounding on the real `f32`.

**The witnesses** port the merge-base's code. The floor's, `an_unfloored_stability_stores_as_zero`,
ports `project`'s stability at the merge base (`crates/fsrs7/src/stock.rs:36`), the 90-percent
interval written as it is, at the interval a first review rated Again gets under the default
parameters.
-/

namespace Formal.Fsrs7Stock

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

/-- `DIFFICULTY_MIN` and `DIFFICULTY_MAX`. -/
def difficultyMin : Rat := 1
def difficultyMax : Rat := 10

/-- `project`'s difficulty: `state.difficulty.clamp(DIFFICULTY_MIN, DIFFICULTY_MAX)`. -/
def stockDifficulty (d : F32) : F32 := d.clamp difficultyMin difficultyMax

/-- `t.max(0.0)`, the curve's first step, over every `f32`: NaN reads 0, as IEEE `maxNum` gives
the other operand, and so does every gap below zero. Over a finite gap it is `retrievability`'s
`max t 0`. -/
def F32.maxZero (x : F32) : F32 :=
  match x with
  | .nan => .fin 0
  | .negInf => .fin 0
  | .posInf => .posInf
  | .fin q => .fin (max q 0)

/-- The curve's parts at one state: the two weights and the two components' recall by gap. -/
structure Curve where
  w1 : Rat
  w2 : Rat
  r1 : Rat → Rat
  r2 : Rat → Rat

/-- The laws the pinned code gives the parts. -/
structure Laws (c : Curve) : Prop where
  w1_pos : 0 < c.w1
  w2_pos : 0 < c.w2
  r1_zero : c.r1 0 = 1
  r2_zero : c.r2 0 = 1
  r1_unit : ∀ t, 0 ≤ t → 0 ≤ c.r1 t ∧ c.r1 t ≤ 1
  r2_unit : ∀ t, 0 ≤ t → 0 ≤ c.r2 t ∧ c.r2 t ≤ 1
  r1_falls : ∀ a b, 0 ≤ a → a ≤ b → c.r1 b ≤ c.r1 a
  r2_falls : ∀ a b, 0 ≤ a → a ≤ b → c.r2 b ≤ c.r2 a

/-- The curve's floor and ceiling: `mul_add(1.0 - 2e-5, 1e-5)` of 0 and of 1. -/
def floorR : Rat := 1 / 100000
def ceilR : Rat := 1 - 1 / 100000

/-- `fsrs7_forgetting_curve_scalar_for_state`: the gap through `max(0.0)`, the weighted mixture,
then the squash onto the band. -/
def retrievability (c : Curve) (t : Rat) : Rat :=
  let t := max t 0
  let retention := (c.w1 * c.r1 t + c.w2 * c.r2 t) / (c.w1 + c.w2)
  retention * (1 - 2 / 100000) + 1 / 100000

/-- `STABILITY_MIN`, `0.0001` as its `f32`: `13743895 / 2^37`. -/
def stabilityMin : Rat := 13743895 / 137438953472

/-- `f32::max(lo)` for a finite `lo`: NaN and negative infinity read `lo`, as IEEE `maxNum` gives
the other operand; positive infinity stays; and a finite value below `lo` reads `lo`. -/
def F32.atLeast (x : F32) (lo : Rat) : F32 :=
  match x with
  | .nan => .fin lo
  | .negInf => .fin lo
  | .posInf => .posInf
  | .fin q => if q < lo then .fin lo else .fin q

/-- `stock_stability`: `interval.max(STABILITY_MIN)`, the stability `project` writes. -/
def stockStability (interval : F32) : F32 := interval.atLeast stabilityMin

/-- `f32::round`: half away from zero. -/
def roundHalfAway (x : Rat) : Int :=
  if 0 ≤ x then (x + 1 / 2).floor else -((-x + 1 / 2).floor)

/-- The engine's store of a stability, over exact rationals: rounded to four decimal places. -/
def stored (q : Rat) : Rat := (roundHalfAway (q * 10000) : Rat) / 10000

/-! ## The claims -/

/-- The difficulty claim over a difficulty function. -/
def DifficultyRule (f : F32 → F32) : Prop :=
  (∀ x, x ≠ .nan → ∃ q, f x = .fin q ∧ difficultyMin ≤ q ∧ q ≤ difficultyMax) ∧
  (∀ q, difficultyMin ≤ q → q ≤ difficultyMax → f (.fin q) = .fin q) ∧
  f .nan = .nan

/-- The band claim over a curve. -/
def BandRule (r : Curve → Rat → Rat) : Prop :=
  ∀ c, Laws c → ∀ t, floorR ≤ r c t ∧ r c t ≤ ceilR ∧ (t ≤ 0 → r c t = ceilR)

/-- The falling claim over a curve. -/
def FallingRule (r : Curve → Rat → Rat) : Prop :=
  ∀ c, Laws c → ∀ a b, a ≤ b → r c b ≤ r c a

/-- The floor claim over a stability function. -/
def StabilityRule (f : F32 → F32) : Prop :=
  (∀ v, f v = .posInf ∨ ∃ q, f v = .fin q ∧ stabilityMin ≤ q ∧ 0 < stored q) ∧
  (∀ q, stabilityMin ≤ q → f (.fin q) = .fin q)

/-! ## The proofs -/

theorem the_stock_difficulty_lies_in_the_stock_range : DifficultyRule stockDifficulty := by
  refine ⟨?_, ?_, rfl⟩
  · intro x hx
    rcases x with _ | _ | _ | q
    · exact absurd rfl hx
    · exact ⟨difficultyMin, rfl, by decide +kernel, by decide +kernel⟩
    · exact ⟨difficultyMax, rfl, by decide +kernel, by decide +kernel⟩
    · simp only [stockDifficulty, F32.clamp]
      by_cases h1 : q < difficultyMin
      · simp only [h1, ↓reduceIte]
        exact ⟨difficultyMin, rfl, by decide +kernel, by decide +kernel⟩
      · by_cases h2 : difficultyMax < q
        · simp only [h1, h2, ↓reduceIte]
          exact ⟨difficultyMax, rfl, by decide +kernel, by decide +kernel⟩
        · simp only [h1, h2, ↓reduceIte]
          exact ⟨q, rfl, by grind, by grind⟩
  · intro q h1 h2
    have n1 : ¬ q < difficultyMin := by grind
    have n2 : ¬ difficultyMax < q := by grind
    simp only [stockDifficulty, F32.clamp, n1, n2, ↓reduceIte]

theorem max_zero_nonneg (t : Rat) : 0 ≤ max t 0 := by
  rw [Rat.max_def]; split <;> grind

theorem max_zero_mono {a b : Rat} (h : a ≤ b) : max a 0 ≤ max b 0 := by
  rw [Rat.max_def, Rat.max_def]; split <;> split <;> grind

theorem max_zero_of_nonpos {t : Rat} (h : t ≤ 0) : max t 0 = 0 := by
  rw [Rat.max_def]; simp [h]

theorem div_unit {n d : Rat} (hd : 0 < d) (h0 : 0 ≤ n) (h1 : n ≤ d) :
    0 ≤ n / d ∧ n / d ≤ 1 := by
  refine ⟨?_, ?_⟩
  · rw [Rat.div_def]
    exact Rat.mul_nonneg h0 (Rat.le_of_lt (Rat.inv_pos.mpr hd))
  · apply Classical.byContradiction
    intro hc
    have : 1 < n / d := by grind
    have := (Rat.lt_div_iff hd).mp this
    grind

theorem mixture_unit (c : Curve) (l : Laws c) (t : Rat) (ht : 0 ≤ t) :
    0 ≤ (c.w1 * c.r1 t + c.w2 * c.r2 t) / (c.w1 + c.w2) ∧
      (c.w1 * c.r1 t + c.w2 * c.r2 t) / (c.w1 + c.w2) ≤ 1 := by
  have u1 := l.r1_unit t ht
  have u2 := l.r2_unit t ht
  have p1 := l.w1_pos
  have p2 := l.w2_pos
  have a0 := Rat.mul_nonneg (Rat.le_of_lt p1) u1.1
  have b0 := Rat.mul_nonneg (Rat.le_of_lt p2) u2.1
  have a1 : c.w1 * c.r1 t ≤ c.w1 * 1 := Rat.mul_le_mul_of_nonneg_left u1.2 (Rat.le_of_lt p1)
  have b1 : c.w2 * c.r2 t ≤ c.w2 * 1 := Rat.mul_le_mul_of_nonneg_left u2.2 (Rat.le_of_lt p2)
  exact div_unit (by grind) (by grind) (by grind)

theorem the_curve_reads_between_its_floor_and_its_ceiling : BandRule retrievability := by
  intro c l t
  have m := mixture_unit c l (max t 0) (max_zero_nonneg t)
  simp only [retrievability, floorR, ceilR]
  refine ⟨?_, ?_, ?_⟩
  · have := Rat.mul_nonneg m.1 (by decide +kernel : (0 : Rat) ≤ 1 - 2 / 100000)
    grind
  · have := Rat.mul_le_mul_of_nonneg_right m.2 (by decide +kernel : (0 : Rat) ≤ 1 - 2 / 100000)
    grind
  · intro ht
    rw [max_zero_of_nonpos ht, l.r1_zero, l.r2_zero]
    have hd : c.w1 + c.w2 ≠ 0 := by have := l.w1_pos; have := l.w2_pos; grind
    have e : (c.w1 * 1 + c.w2 * 1) / (c.w1 + c.w2) = 1 := by
      rw [Rat.div_def]
      have : c.w1 * 1 + c.w2 * 1 = c.w1 + c.w2 := by grind
      rw [this]
      exact Rat.mul_inv_cancel _ hd
    rw [e]
    grind

theorem the_curve_never_rises_with_time : FallingRule retrievability := by
  intro c l a b hab
  have ha := max_zero_nonneg a
  have hm := max_zero_mono hab
  have f1 := l.r1_falls (max a 0) (max b 0) ha hm
  have f2 := l.r2_falls (max a 0) (max b 0) ha hm
  have g1 : c.w1 * c.r1 (max b 0) ≤ c.w1 * c.r1 (max a 0) :=
    Rat.mul_le_mul_of_nonneg_left f1 (Rat.le_of_lt l.w1_pos)
  have g2 : c.w2 * c.r2 (max b 0) ≤ c.w2 * c.r2 (max a 0) :=
    Rat.mul_le_mul_of_nonneg_left f2 (Rat.le_of_lt l.w2_pos)
  have hd : 0 < c.w1 + c.w2 := by have := l.w1_pos; have := l.w2_pos; grind
  have hn : c.w1 * c.r1 (max b 0) + c.w2 * c.r2 (max b 0) ≤
      c.w1 * c.r1 (max a 0) + c.w2 * c.r2 (max a 0) := by grind
  have hq : (c.w1 * c.r1 (max b 0) + c.w2 * c.r2 (max b 0)) / (c.w1 + c.w2) ≤
      (c.w1 * c.r1 (max a 0) + c.w2 * c.r2 (max a 0)) / (c.w1 + c.w2) := by
    rw [Rat.div_def, Rat.div_def]
    exact Rat.mul_le_mul_of_nonneg_right hn (Rat.le_of_lt (Rat.inv_pos.mpr hd))
  have := Rat.mul_le_mul_of_nonneg_right hq (by decide +kernel : (0 : Rat) ≤ 1 - 2 / 100000)
  simp only [retrievability]
  grind

/-- A stability at or above the floor is stored above zero: its four-place rounding is at least
`0.0001`. -/
theorem stored_pos {q : Rat} (h : stabilityMin ≤ q) : 0 < stored q := by
  have hm : (0 : Rat) < stabilityMin := by decide +kernel
  have h0 : 0 ≤ q * 10000 := Rat.mul_nonneg (by grind) (by decide +kernel)
  have hk : stabilityMin * 10000 ≤ q * 10000 :=
    Rat.mul_le_mul_of_nonneg_right h (by decide +kernel)
  have hc : (1 : Rat) ≤ stabilityMin * 10000 + 1 / 2 := by decide +kernel
  have hf : (1 : Int) ≤ (q * 10000 + 1 / 2).floor := by
    rw [Rat.le_floor_iff]
    have : (((1 : Int) : Rat)) = 1 := rfl
    rw [this]
    grind
  have hr : (1 : Rat) ≤ ((q * 10000 + 1 / 2).floor : Rat) := by exact_mod_cast hf
  simp only [stored, roundHalfAway, h0, ↓reduceIte]
  exact (Rat.lt_div_iff (by decide +kernel)).mpr (by grind)

/-- The stock stability is positive infinity or a value at least the floor that the store keeps
above zero, whatever `f32` the interval is, and an interval at or above the floor is kept. -/
theorem the_stock_stability_is_at_least_the_floor_and_stored_above_zero :
    StabilityRule stockStability := by
  have hm : stabilityMin ≤ stabilityMin := by decide +kernel
  refine ⟨?_, ?_⟩
  · intro v
    rcases v with _ | _ | _ | q
    · exact Or.inr ⟨stabilityMin, rfl, hm, stored_pos hm⟩
    · exact Or.inr ⟨stabilityMin, rfl, hm, stored_pos hm⟩
    · exact Or.inl rfl
    · simp only [stockStability, F32.atLeast]
      by_cases hq : q < stabilityMin
      · simp only [hq, ↓reduceIte]
        exact Or.inr ⟨stabilityMin, rfl, hm, stored_pos hm⟩
      · simp only [hq, ↓reduceIte]
        have hl : stabilityMin ≤ q := by grind
        exact Or.inr ⟨q, rfl, hl, stored_pos hl⟩
  · intro q hq
    have n : ¬ q < stabilityMin := by grind
    simp only [stockStability, F32.atLeast, n, ↓reduceIte]

/-! ## The witnesses -/

/-- The difficulty with no clamp. -/
def stockDifficultyUnclamped (d : F32) : F32 := d

/-- A difficulty of 11 stays 11. -/
theorem a_difficulty_with_no_clamp_leaves_the_range :
    ¬ DifficultyRule stockDifficultyUnclamped := by
  intro h
  obtain ⟨q, e, _, h2⟩ := h.1 (.fin 11) (by decide)
  simp only [stockDifficultyUnclamped, F32.fin.injEq] at e
  subst e
  revert h2
  decide +kernel

/-- The curve with no squash: the mixture alone. -/
def retrievabilityUnsquashed (c : Curve) (t : Rat) : Rat :=
  let t := max t 0
  (c.w1 * c.r1 t + c.w2 * c.r2 t) / (c.w1 + c.w2)

/-- Two components that never fall, at equal weights. -/
def flatCurve : Curve := ⟨1, 1, fun _ => 1, fun _ => 1⟩

theorem flatCurve_laws : Laws flatCurve := by
  have z : (0 : Rat) ≤ 1 := by decide +kernel
  have o : (1 : Rat) ≤ 1 := by decide +kernel
  refine ⟨by decide +kernel, by decide +kernel, rfl, rfl, ?_, ?_, ?_, ?_⟩
  · intro _ _; exact ⟨z, o⟩
  · intro _ _; exact ⟨z, o⟩
  · intro _ _ _ _; exact o
  · intro _ _ _ _; exact o

/-- At the review the unsquashed curve reads 1, past the ceiling. -/
theorem an_unsquashed_curve_reads_one_at_the_review :
    ¬ BandRule retrievabilityUnsquashed := by
  intro h
  have := (h flatCurve flatCurve_laws 0).2.1
  revert this
  decide +kernel

/-- The curve read at the raw gap. -/
def retrievabilityAtTheRawGap (c : Curve) (t : Rat) : Rat :=
  let retention := (c.w1 * c.r1 t + c.w2 * c.r2 t) / (c.w1 + c.w2)
  retention * (1 - 2 / 100000) + 1 / 100000

/-- Components that hold the laws over non-negative gaps and read 0 before the review. -/
def stepCurve : Curve :=
  ⟨1, 1, fun t => if t < 0 then 0 else 1, fun t => if t < 0 then 0 else 1⟩

theorem stepCurve_laws : Laws stepCurve := by
  refine ⟨by decide +kernel, by decide +kernel, by decide +kernel, by decide +kernel, ?_, ?_, ?_, ?_⟩
  · intro t ht
    have : ¬ t < 0 := by grind
    simp only [stepCurve, this, ↓reduceIte]
    exact ⟨by decide +kernel, by decide +kernel⟩
  · intro t ht
    have : ¬ t < 0 := by grind
    simp only [stepCurve, this, ↓reduceIte]
    exact ⟨by decide +kernel, by decide +kernel⟩
  · intro a b ha hab
    have n1 : ¬ a < 0 := by grind
    have n2 : ¬ b < 0 := by grind
    simp only [stepCurve, n1, n2, ↓reduceIte]
    decide +kernel
  · intro a b ha hab
    have n1 : ¬ a < 0 := by grind
    have n2 : ¬ b < 0 := by grind
    simp only [stepCurve, n1, n2, ↓reduceIte]
    decide +kernel

/-- Read raw, a day before the review reads lower than the review itself. -/
theorem a_curve_read_at_a_negative_gap_rises_to_the_review :
    ¬ FallingRule retrievabilityAtTheRawGap := by
  intro h
  have := h stepCurve stepCurve_laws (-1) 0 (by decide +kernel)
  revert this
  decide +kernel

/-- `project`'s stability at the merge base (`crates/fsrs7/src/stock.rs:36`): the revision's
90-percent interval, written as it is. -/
def stockStabilityUnfloored (interval : F32) : F32 := interval

/-- The 90-percent interval of a first review rated Again under the default parameters,
`3.827_562_6e-5` days as its `f32`: `2630281 / 2^36`. -/
def againInterval : Rat := 2630281 / 68719476736

/-- Written as it is, the Again interval is stored as zero. -/
theorem an_unfloored_stability_stores_as_zero : ¬ StabilityRule stockStabilityUnfloored := by
  intro h
  rcases h.1 (.fin againInterval) with e | ⟨q, e, _, h2⟩
  · simp [stockStabilityUnfloored] at e
  · simp only [stockStabilityUnfloored, F32.fin.injEq] at e
    subst e
    revert h2
    decide +kernel

end Formal.Fsrs7Stock
