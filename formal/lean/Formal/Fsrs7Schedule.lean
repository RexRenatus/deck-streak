-- @phx covers crates/engine-core/src/replay.rs anchor=scheduled digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/engine-core/src/replay.rs anchor=day_of digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/engine-core/src/replay.rs anchor=stock_fields digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx cites #823
-- @phx theorem the_interval_is_a_day_or_more_and_never_past_the_maximum ramp=report
-- @phx witness an_interval_with_no_floor_reads_zero_days kills=the_interval_is_a_day_or_more_and_never_past_the_maximum
-- @phx theorem the_due_is_the_last_review_day_plus_the_interval ramp=report
-- @phx witness a_day_counted_by_truncation_puts_an_earlier_instant_in_today kills=the_due_is_the_last_review_day_plus_the_interval
-- @phx theorem writing_the_stock_fields_again_changes_nothing ramp=report
-- @phx witness a_due_counted_from_the_cards_own_due_moves_at_each_write kills=writing_the_stock_fields_again_changes_nothing
-- @phx vectors formal/vectors/fsrs7-schedule.jsonl

/-!
# Formal.Fsrs7Schedule

The FSRS-7 replay's schedule and its stock-field write in the engine core (#823):
`scheduled`, `day_of` and `CardReplay::stock_fields` in `crates/engine-core/src/replay.rs`.

**The claims.**
- `the_interval_is_a_day_or_more_and_never_past_the_maximum`: for every `f32` the scheduler can
  return, NaN and both infinities included, and every maximum, the schedule's interval is at least
  1 and at most the larger of 1 and the maximum, a NaN reads 1, and over finite values the
  interval never falls as the scheduler's interval grows.
- `the_due_is_the_last_review_day_plus_the_interval`: the engine day of an instant is today's
  count plus the whole days between the instant and the instant today began, by the floor, for
  every instant, one before the epoch or after today included; and a schedule, when the due fits
  the engine's `i32`, is that interval due that many days after its last review's engine day, so
  never on or before that day.
- `writing_the_stock_fields_again_changes_nothing`: the write sets the memory state, and a review
  card's interval and due, from the replay alone, so a second write of the same replay leaves the
  card as the first left it, and every other field is the card's own.

**What is ported.** An `f32` is NaN, an infinity, or a finite value held as the exact rational it
denotes; every finite `f32` is a rational, so a claim over every rational covers every finite
`f32`. `f32::round` rounds half away from zero and is exact, and `as u32` saturates: NaN and every
value below 0 read 0, and every value past `u32::MAX` reads it. The engine's integers are `Int`:
`div_euclid` by a positive divisor is Lean's `/` on `Int`. The card is its memory state, interval
and due, and one field standing for every other field, which the write never reads.

**What is not ported.** The overflow of `i64` arithmetic inside `day_of`: its operands are an
instant in milliseconds and the engine's next rollover in seconds, each far inside the type.
Rust's `f32::round` and saturating cast are taken from the language reference, not re-derived.
-/

namespace Formal.Fsrs7Schedule

/-- An `f32` as the code reads it: NaN, an infinity, or a finite value as its exact rational. -/
inductive F32 where
  | nan
  | negInf
  | posInf
  | fin (q : Rat)
  deriving DecidableEq, Repr

/-- `u32::MAX`. -/
def u32Max : Nat := 4294967295

/-- `i32::MIN` and `i32::MAX`. -/
def i32Min : Int := -2147483648
def i32Max : Int := 2147483647

/-- `f32::round` on a finite value: half away from zero. -/
def roundHalfAway (q : Rat) : Int :=
  if 0 ≤ q then (q + 1 / 2).floor else -((-q + 1 / 2).floor)

/-- `interval.round() as u32`: NaN reads 0, and the cast saturates at both ends. -/
def roundedU32 : F32 → Nat
  | .nan => 0
  | .negInf => 0
  | .posInf => u32Max
  | .fin q =>
    let i := roundHalfAway q
    if i ≤ 0 then 0 else min i.toNat u32Max

/-- `rounded.min(max_ivl).max(1)` in `scheduled`. -/
def ivl (interval : F32) (maxIvl : Nat) : Nat :=
  max (min (roundedU32 interval) maxIvl) 1

/-- The engine day the caller read: `EngineDay`'s days elapsed and next rollover, in seconds. -/
structure EngineDay where
  daysElapsed : Nat
  nextDayAt : Int
  deriving DecidableEq, Repr

/-- `day_of`: the engine day of the instant `instant`, in milliseconds. -/
def dayOf (day : EngineDay) (instant : Int) : Int :=
  let began := day.nextDayAt - 86400
  (day.daysElapsed : Int) + (instant / 1000 - began) / 86400

/-- `scheduled`: a review card's interval and due, or `none` when the due is past `i32`. -/
def scheduled (interval : F32) (lastReview : Int) (day : EngineDay) (maxIvl : Nat) :
    Option (Nat × Int) :=
  let i := ivl interval maxIvl
  let due := dayOf day lastReview + i
  if i32Min ≤ due ∧ due ≤ i32Max then some (i, due) else none

/-- One replayed card's stock values: its stability, its difficulty and a review card's
schedule. -/
structure CardReplay where
  stability : F32
  difficulty : F32
  schedule : Option (Nat × Int)
  deriving DecidableEq, Repr

/-- The engine's card as the write reads it: the memory state, the interval, the due, and one
field standing for every other. -/
structure Card where
  memory : Option (F32 × F32)
  interval : Nat
  due : Int
  rest : Nat
  deriving DecidableEq, Repr

/-- `CardReplay::stock_fields`. -/
def stockFields (r : CardReplay) (c : Card) : Card :=
  let c := { c with memory := some (r.stability, r.difficulty) }
  match r.schedule with
  | some (i, due) => { c with interval := i, due := due }
  | none => c

/-! ## The claims -/

/-- The interval claim over an interval function. -/
def IntervalRule (f : F32 → Nat → Nat) : Prop :=
  (∀ x m, 1 ≤ f x m ∧ f x m ≤ max m 1) ∧
  (∀ m, f .nan m = 1) ∧
  (∀ a b m, a ≤ b → f (.fin a) m ≤ f (.fin b) m)

/-- The day and due claim over a day function. -/
def DueRule (d : EngineDay → Int → Int) : Prop :=
  (∀ (day : EngineDay) (k instant : Int),
      (day.nextDayAt - 86400 + 86400 * k) * 1000 ≤ instant →
      instant < (day.nextDayAt - 86400 + 86400 * (k + 1)) * 1000 →
      d day instant = day.daysElapsed + k) ∧
  (∀ x last day m i due, scheduled x last day m = some (i, due) →
      i = ivl x m ∧ due = d day last + i ∧ d day last < due) ∧
  (∀ x last day m, i32Min ≤ d day last + ivl x m → d day last + ivl x m ≤ i32Max →
      scheduled x last day m = some (ivl x m, d day last + ivl x m))

/-- The write claim over a write function. -/
def WriteRule (w : CardReplay → Card → Card) : Prop :=
  ∀ r c, w r (w r c) = w r c ∧ (w r c).rest = c.rest ∧
    (w r c).memory = some (r.stability, r.difficulty) ∧
    (r.schedule = none → (w r c).interval = c.interval ∧ (w r c).due = c.due) ∧
    (∀ i due, r.schedule = some (i, due) → (w r c).interval = i ∧ (w r c).due = due)

/-! ## The proofs -/

theorem floor_mono {a b : Rat} (h : a ≤ b) : a.floor ≤ b.floor := by
  have h1 := Rat.floor_le a
  have h2 := Rat.lt_floor_add_one b
  have h3 : ((a.floor : Int) : Rat) < ((b.floor + 1 : Int) : Rat) := by grind
  have h4 : a.floor < b.floor + 1 := Rat.intCast_lt_intCast.mp h3
  omega

theorem floor_nonneg {a : Rat} (h : 0 ≤ a) : 0 ≤ a.floor := by
  have h2 := Rat.lt_floor_add_one a
  have h3 : ((0 : Int) : Rat) < ((a.floor + 1 : Int) : Rat) := by grind
  have h4 : (0 : Int) < a.floor + 1 := Rat.intCast_lt_intCast.mp h3
  omega

theorem roundHalfAway_mono {a b : Rat} (h : a ≤ b) : roundHalfAway a ≤ roundHalfAway b := by
  unfold roundHalfAway
  by_cases ha : 0 ≤ a <;> by_cases hb : 0 ≤ b <;> simp only [ha, hb, ↓reduceIte]
  · exact floor_mono (by grind)
  · exact absurd (by grind : (0 : Rat) ≤ b) hb
  · have h1 := floor_nonneg (a := -a + 1 / 2) (by grind)
    have h2 := floor_nonneg (a := b + 1 / 2) (by grind)
    omega
  · have := floor_mono (a := -b + 1 / 2) (b := -a + 1 / 2) (by grind)
    omega

theorem roundedU32_mono {a b : Rat} (h : a ≤ b) :
    roundedU32 (.fin a) ≤ roundedU32 (.fin b) := by
  have := roundHalfAway_mono h
  simp only [roundedU32]
  split <;> split <;> omega

theorem interval_rule : IntervalRule ivl := by
  refine ⟨?_, ?_, ?_⟩
  · intro x m
    simp only [ivl]
    omega
  · intro m
    simp [ivl, roundedU32]
  · intro a b m h
    have := roundedU32_mono h
    simp only [ivl]
    omega

theorem the_interval_is_a_day_or_more_and_never_past_the_maximum : IntervalRule ivl :=
  interval_rule

theorem day_rule (day : EngineDay) (k instant : Int)
    (h1 : (day.nextDayAt - 86400 + 86400 * k) * 1000 ≤ instant)
    (h2 : instant < (day.nextDayAt - 86400 + 86400 * (k + 1)) * 1000) :
    dayOf day instant = day.daysElapsed + k := by
  simp only [dayOf]
  omega

theorem the_due_is_the_last_review_day_plus_the_interval : DueRule dayOf := by
  refine ⟨day_rule, ?_, ?_⟩
  · intro x last day m i due h
    simp only [scheduled] at h
    split at h
    · simp only [Option.some.injEq, Prod.mk.injEq] at h
      have hge := (interval_rule.1 x m).1
      omega
    · exact absurd h (by simp)
  · intro x last day m h1 h2
    simp only [scheduled]
    simp [h1, h2]

theorem writing_the_stock_fields_again_changes_nothing : WriteRule stockFields := by
  intro r c
  rcases r with ⟨s, d, sch⟩
  rcases sch with _ | ⟨i, due⟩ <;> simp [stockFields]

/-! ## The witnesses

Each mutant changes one step of the code, and each breaks its claim at one input. -/

/-- The interval with no floor: `rounded.min(max_ivl)`. -/
def ivlWithNoFloor (interval : F32) (maxIvl : Nat) : Nat :=
  min (roundedU32 interval) maxIvl

/-- A first Again's interval at 0.9, about 3.8e-5 days, rounds to 0, and with no floor it is a
schedule of 0 days. -/
theorem an_interval_with_no_floor_reads_zero_days : ¬ IntervalRule ivlWithNoFloor := by
  intro h
  have := (h.1 (.fin (38275626 / 1000000000000)) 36500).1
  revert this
  decide +kernel

/-- The day counted by truncating division, Rust's `/` on `i64` in place of `div_euclid`. -/
def dayOfByTruncation (day : EngineDay) (instant : Int) : Int :=
  let began := day.nextDayAt - 86400
  (day.daysElapsed : Int) + Int.tdiv (Int.tdiv instant 1000 - began) 86400

/-- A review one second before today began reads as today's under truncation: day 10 began at
86400, and the instant 86399000 lies in day 9. -/
theorem a_day_counted_by_truncation_puts_an_earlier_instant_in_today :
    ¬ DueRule dayOfByTruncation := by
  intro h
  have := h.1 ⟨10, 172800⟩ (-1) 86399000 (by decide) (by decide)
  revert this
  decide

/-- The write that counts the due from the card's own due. -/
def stockFieldsFromTheCardsDue (r : CardReplay) (c : Card) : Card :=
  let c := { c with memory := some (r.stability, r.difficulty) }
  match r.schedule with
  | some (i, _) => { c with interval := i, due := c.due + i }
  | none => c

/-- A replay with an interval of 5 written twice moves a card due on day 0 to day 5, then 10. -/
theorem a_due_counted_from_the_cards_own_due_moves_at_each_write :
    ¬ WriteRule stockFieldsFromTheCardsDue := by
  intro h
  have := (h ⟨.fin 4, .fin 5, some (5, 5)⟩ ⟨none, 0, 0, 0⟩).1
  revert this
  decide +kernel

end Formal.Fsrs7Schedule
