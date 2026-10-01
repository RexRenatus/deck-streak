-- @phx covers crates/streaks/src/lapse.rs anchor=open_lapse digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx cites #472, #446
-- @phx theorem at_most_one_step_per_day ramp=report
-- @phx theorem never_overflows ramp=report
-- @phx theorem answers_the_rule ramp=report
-- @phx witness a_walk_past_the_first_day_breaks_the_step_bound kills=at_most_one_step_per_day
-- @phx witness an_unchecked_step_overflows_at_the_smallest_day kills=never_overflows
-- @phx witness an_unguarded_walk_answers_at_the_smallest_day kills=answers_the_rule
-- @phx vectors formal/vectors/open-lapse.jsonl

/-!
# Formal.OpenLapse

The open lapse walk, `open_lapse` in `crates/streaks/src/lapse.rs`, as a total function of today,
the window's review counts, the skip days and the threshold (#472).

**The claim.** For every today and every window the day type admits:
- `at_most_one_step_per_day`: the walk answers after at most one step per day from the window's
  first day to today;
- `never_overflows`: no step back overflows, the smallest day included;
- `answers_the_rule`: the answer is the rule's. Walk back from today to the window's first day
  while a day holds no study review; a skip day neither counts nor ends the run. When the run holds
  at least the threshold's number of silent days, the answer is its earliest silent day, and
  otherwise none. The one exception: a run that reaches the smallest day the type admits answers
  none, as the loop the range replaced did.

**What is ported.** The day is `Int64`, the type `StudyDay` wraps (`pub struct StudyDay(i64);` in
`crates/kernel/src/study_day.rs`), so the smallest day is `Int64.minValue` and the bound is the
type's own. The window is the map's entries in key order, a list here; its first day is the least
key. The range `(window_start..=today).rev()` is ported as core's `RangeInclusive::next_back`: an
empty range answers nothing, a range whose start is below its end steps its end back by one, and a
range whose start equals its end yields that day and is exhausted. The count saturates at
`u32::MAX`, as `saturating_add` does. Rust's loop has no fuel; the port takes `2^64 + 1` steps of
fuel for structural recursion, more than any window holds, and `answers_the_rule` proves the walk
never runs out of it.

**What is not ported.** `anchor_beyond_the_walk` in the same file, a separate function.

**The rule** is stated over `Int`, apart from the port: `runStart` is the day after the latest
study review in the window up to today, or the window's first day, and `silentDays` and
`earliestSilent` read the run from there to today.

**The witnesses** are ports that each break one claim at one input: a walk whose range starts the
day before the window's first day (the mutant `window_start - 1`), a walk that steps back from a
day without a check (the `while` loop's step before it was checked), and a walk without the guard
at the smallest day (the mutant `if false`).
-/

namespace Formal.OpenLapse

/-- The window: each study day's count of qualifying reviews, in key order. -/
abbrev Counts := List (Int64 × UInt32)

/-- The window's first day, its least key; `None` for an empty window, as `keys().next()?`. -/
def windowStart : Counts → Option Int64
  | [] => none
  | (key, _) :: rest =>
    match windowStart rest with
    | none => some key
    | some least => some (if key ≤ least then key else least)

/-- A day's count; a day with no entry counts zero, as `copied().unwrap_or(0)` reads it. -/
def reviews (counts : Counts) (day : Int64) : UInt32 := (counts.lookup day).getD 0

/-- `RangeInclusive<i64>`: its start, its end and whether it is exhausted. -/
structure Days where
  start : Int64
  stop : Int64
  exhausted : Bool

/-- `Step::backward_checked(day, 1)`: the day before, if the type holds one. -/
def stepBack (day : Int64) : Option Int64 :=
  if day = Int64.minValue then none else some (day - 1)

/-- One `next_back`: nothing left, the next day with the rest, or a step that overflowed. -/
inductive Back where
  | done
  | next (day : Int64) (rest : Days)
  | overflow

/-- `RangeInclusive::next_back`, branch for branch. -/
def nextBack (days : Days) : Back :=
  if days.exhausted || !decide (days.start ≤ days.stop) then Back.done
  else if days.start < days.stop then
    match stepBack days.stop with
    | none => Back.overflow
    | some before => Back.next days.stop { days with stop := before }
  else Back.next days.stop { days with exhausted := true }

/-- `u32::saturating_add`. -/
def saturatingAdd (a b : UInt32) : UInt32 :=
  if a.toNat + b.toNat ≤ 4294967295 then a + b else 4294967295

/-- The walk's two variables, `silent` and `first_silent`. -/
structure Tally where
  silent : UInt32
  firstSilent : Option Int64

/-- One day's count: a day that is not a skip day is silent and becomes the run's first. -/
def tally (skips : List Int64) (day : Int64) (t : Tally) : Tally :=
  if !skips.contains day then ⟨saturatingAdd t.silent 1, some day⟩ else t

/-- The last `if`: the run's first silent day once the run holds at least the threshold. -/
def answerOf (threshold : UInt32) (t : Tally) : Option Int64 :=
  if t.silent ≥ threshold then t.firstSilent else none

/-- How a walk ends: with an answer, by an overflowing step, or out of fuel. -/
inductive Outcome where
  | answer (day : Option Int64)
  | overflow
  | outOfFuel
  deriving DecidableEq, Repr

/-- A walk's outcome and the number of days it read. -/
structure Run where
  outcome : Outcome
  steps : Nat

/-- The loop: one step per day it reads, newest first. -/
def walk (counts : Counts) (skips : List Int64) (threshold : UInt32) :
    Nat → Days → Tally → Nat → Run
  | 0, _, _, steps => ⟨Outcome.outOfFuel, steps⟩
  | fuel + 1, days, t, steps =>
    match nextBack days with
    | Back.done => ⟨Outcome.answer (answerOf threshold t), steps⟩
    | Back.overflow => ⟨Outcome.overflow, steps⟩
    | Back.next day rest =>
      if reviews counts day > 0 then ⟨Outcome.answer (answerOf threshold t), steps + 1⟩
      else walk counts skips threshold fuel rest (tally skips day t) (steps + 1)

/-- More steps than any window of `Int64` days holds. -/
def fuel : Nat := 2 ^ 64 + 1

/-- `open_lapse`. -/
def openLapse (today : Int64) (counts : Counts) (skips : List Int64) (threshold : UInt32) : Run :=
  match windowStart counts with
  | none => ⟨Outcome.answer none, 0⟩
  | some start => walk counts skips threshold fuel ⟨start, today, false⟩ ⟨0, none⟩ 0

/-! ## The rule -/

/-- Whether a day is a skip day. -/
def isSkip (skips : List Int64) (day : Int) : Bool := skips.any fun s => s.toInt == day

/-- The silent days among the `n` days from `lo` up. -/
def silentDays (skips : List Int64) (lo : Int) : Nat → Nat
  | 0 => 0
  | n + 1 => (if isSkip skips lo then 0 else 1) + silentDays skips (lo + 1) n

/-- The earliest silent day among the `n` days from `lo` up. -/
def earliestSilent (skips : List Int64) (lo : Int) : Nat → Option Int
  | 0 => none
  | n + 1 => if isSkip skips lo then earliestSilent skips (lo + 1) n else some lo

/-- The latest day of a list that satisfies `p`. -/
def latest (p : Int64 → Bool) : List Int64 → Option Int64
  | [] => none
  | day :: rest =>
    match latest p rest with
    | none => if p day then some day else none
    | some later => if p day && later < day then some day else some later

/-- A day of the window, up to today, that holds a study review. -/
def reviewedIn (counts : Counts) (start today day : Int64) : Bool :=
  decide (start ≤ day ∧ day ≤ today ∧ 0 < reviews counts day)

/-- The run's earliest day: after the latest study review up to today, or the window's first. -/
def runStart (counts : Counts) (start today : Int64) : Int :=
  match latest (reviewedIn counts start today) (counts.map Prod.fst) with
  | some day => day.toInt + 1
  | none => start.toInt

/-- The rule from the run's earliest day, with the exception at the smallest day. -/
def ruleFrom (today : Int64) (skips : List Int64) (threshold : UInt32) (lo : Int) :
    Option Int64 :=
  if lo = Int64.minValue.toInt then none
  else if threshold.toNat ≤ silentDays skips lo (today.toInt + 1 - lo).toNat then
    (earliestSilent skips lo (today.toInt + 1 - lo).toNat).map Int64.ofInt
  else none

/-- The rule's lapse day. -/
def rule (today : Int64) (counts : Counts) (skips : List Int64) (threshold : UInt32) :
    Option Int64 :=
  match windowStart counts with
  | none => none
  | some start => ruleFrom today skips threshold (runStart counts start today)

/-- The days from the window's first day to today. -/
def daysInWindow (today : Int64) (counts : Counts) : Nat :=
  match windowStart counts with
  | none => 0
  | some start => (today.toInt + 1 - start.toInt).toNat

/-- A walk's signature. -/
abbrev Port := Int64 → Counts → List Int64 → UInt32 → Run

/-- T1: at most one step per day from the window's first day to today. -/
def StepBound (f : Port) : Prop :=
  ∀ today counts skips threshold,
    (f today counts skips threshold).steps ≤ daysInWindow today counts

/-- T2: no step back overflows, the smallest day included. -/
def NoOverflow (f : Port) : Prop :=
  ∀ today counts skips threshold, (f today counts skips threshold).outcome ≠ Outcome.overflow

/-- T3: the answer is the rule's. -/
def AnswersTheRule (f : Port) : Prop :=
  ∀ today counts skips threshold,
    (f today counts skips threshold).outcome = Outcome.answer (rule today counts skips threshold)

/-! ## The proofs -/

theorem toInt_sub_one (a : Int64) (h : a ≠ Int64.minValue) : (a - 1).toInt = a.toInt - 1 := by
  have hne : a.toInt ≠ -2 ^ 63 := by
    intro he; apply h; apply Int64.toInt_inj.mp; rw [he, Int64.toInt_minValue]
  have h1 := Int64.le_toInt a
  have h2 := Int64.toInt_lt a
  rw [Int64.toInt_sub, Int64.toInt_one]
  apply Int.bmod_eq_of_le <;> omega

theorem ne_minValue_of_lt {a b : Int64} (h : a.toInt < b.toInt) : b ≠ Int64.minValue := by
  intro hb; subst hb; have := Int64.le_toInt a; rw [Int64.toInt_minValue] at h; omega

theorem nextBack_exhausted (s c : Int64) : nextBack ⟨s, c, true⟩ = Back.done := by
  simp [nextBack]

theorem nextBack_empty {s c : Int64} (h : c.toInt < s.toInt) :
    nextBack ⟨s, c, false⟩ = Back.done := by
  have : ¬ s ≤ c := by rw [Int64.le_iff_toInt_le]; omega
  simp [nextBack, this]

theorem nextBack_last (s : Int64) : nextBack ⟨s, s, false⟩ = Back.next s ⟨s, s, true⟩ := by
  simp [nextBack]

theorem nextBack_iter {s c : Int64} (h : s.toInt < c.toInt) :
    nextBack ⟨s, c, false⟩ = Back.next c ⟨s, c - 1, false⟩ := by
  have hle : s ≤ c := by rw [Int64.le_iff_toInt_le]; omega
  have hlt : s < c := by rw [Int64.lt_iff_toInt_lt]; omega
  simp [nextBack, hle, hlt, stepBack, ne_minValue_of_lt h]

theorem saturatingAdd_one (a : UInt32) :
    (saturatingAdd a 1).toNat = min (a.toNat + 1) 4294967295 := by
  have ha := UInt32.toNat_lt a
  unfold saturatingAdd
  split
  · rename_i h; simp at h; rw [UInt32.toNat_add]; simp; omega
  · rename_i h; simp at h; simp; omega

theorem contains_eq_isSkip (skips : List Int64) (d : Int64) :
    skips.contains d = isSkip skips d.toInt := by
  apply Bool.eq_iff_iff.mpr
  rw [List.contains_iff_mem, isSkip, List.any_eq_true]
  constructor
  · intro h; exact ⟨d, h, by simp⟩
  · rintro ⟨s, hs, he⟩
    have : s = d := Int64.toInt_inj.mp (by simpa using he)
    exact this ▸ hs

/-- The walk's variables after it has counted the `n` days from `lo` up. -/
def Tallied (skips : List Int64) (lo : Int) (n : Nat) (t : Tally) : Prop :=
  t.silent.toNat = min (silentDays skips lo n) 4294967295 ∧
    t.firstSilent = (earliestSilent skips lo n).map Int64.ofInt

theorem tallied_zero (skips : List Int64) (lo : Int) : Tallied skips lo 0 ⟨0, none⟩ := by
  simp [Tallied, silentDays, earliestSilent]

theorem tallied_step (skips : List Int64) (d : Int64) (n : Nat) (t : Tally)
    (h : Tallied skips (d.toInt + 1) n t) : Tallied skips d.toInt (n + 1) (tally skips d t) := by
  obtain ⟨hs, hf⟩ := h
  unfold tally Tallied
  rw [contains_eq_isSkip]
  cases hk : isSkip skips d.toInt
  · simp only [Bool.not_false, ite_true, silentDays, earliestSilent, hk]
    refine ⟨?_, ?_⟩
    · rw [saturatingAdd_one, hs]; simp; omega
    · simp
  · simp only [Bool.not_true, Bool.false_eq_true, ite_false, silentDays, earliestSilent, hk,
      ite_true]
    exact ⟨by rw [hs]; simp, hf⟩

theorem answerOf_tallied (skips : List Int64) (threshold : UInt32) (lo : Int) (n : Nat)
    (t : Tally) (h : Tallied skips lo n t) :
    answerOf threshold t =
      if threshold.toNat ≤ silentDays skips lo n then
        (earliestSilent skips lo n).map Int64.ofInt
      else none := by
  obtain ⟨hs, hf⟩ := h
  have ht := UInt32.toNat_lt threshold
  unfold answerOf
  have : (t.silent ≥ threshold) ↔ threshold.toNat ≤ silentDays skips lo n := by
    rw [ge_iff_le, UInt32.le_iff_toNat_le, hs]; omega
  by_cases hc : threshold.toNat ≤ silentDays skips lo n
  · simp [this.mpr hc, hc, hf]
  · have : ¬ (t.silent ≥ threshold) := fun h' => hc (this.mp h')
    simp [this, hc]

theorem mem_keys_of_reviews {counts : Counts} {d : Int64} (h : 0 < reviews counts d) :
    d ∈ counts.map Prod.fst := by
  unfold reviews at h
  cases hl : counts.lookup d with
  | none => rw [hl] at h; simp at h
  | some v =>
    obtain ⟨l₁, l₂, he, _⟩ := List.lookup_eq_some_iff.mp hl
    rw [he]; simp

theorem latest_some {p : Int64 → Bool} :
    ∀ {l : List Int64} {m : Int64}, latest p l = some m →
      m ∈ l ∧ p m = true ∧ ∀ d ∈ l, p d = true → d.toInt ≤ m.toInt
  | [], m, h => by simp [latest] at h
  | d :: rest, m, h => by
    unfold latest at h
    cases hr : latest p rest with
    | none =>
      rw [hr] at h
      have hnone : ∀ e ∈ rest, p e = false := by
        intro e he
        cases hp : p e
        · rfl
        · exact absurd hr (latest_ne_none he hp)
      by_cases hd : p d = true
      · simp [hd] at h; subst h
        refine ⟨by simp, hd, ?_⟩
        intro e he hpe
        simp at he
        rcases he with he | he
        · subst he; exact Int.le_refl _
        · simp [hnone e he] at hpe
      · simp [hd] at h
    | some later =>
      rw [hr] at h
      obtain ⟨hm, hpl, hmax⟩ := latest_some hr
      by_cases hc : (p d && decide (later < d)) = true
      · simp [hc] at h; subst h
        simp at hc
        obtain ⟨hpd, hlt⟩ := hc
        rw [Int64.lt_iff_toInt_lt] at hlt
        refine ⟨by simp, hpd, ?_⟩
        intro e he hpe
        simp at he
        rcases he with he | he
        · subst he; exact Int.le_refl _
        · have := hmax e he hpe; omega
      · simp [hc] at h; subst h
        refine ⟨by simp [hm], hpl, ?_⟩
        intro e he hpe
        simp at he
        rcases he with he | he
        · subst he
          simp [hpe, Int64.lt_iff_toInt_lt] at hc
          exact hc
        · exact hmax e he hpe
where
  latest_ne_none : ∀ {l : List Int64} {e : Int64}, e ∈ l → p e = true → latest p l ≠ none
    | [], e, he, _ => by simp at he
    | d :: rest, e, he, hp => by
      unfold latest
      cases hr : latest p rest with
      | none =>
        simp at he
        rcases he with he | he
        · subst he; simp [hp]
        · exact absurd hr (latest_ne_none he hp)
      | some later => simp; split <;> simp

theorem latest_none {p : Int64 → Bool} :
    ∀ {l : List Int64}, (∀ d ∈ l, p d = false) → latest p l = none
  | [], _ => rfl
  | d :: rest, h => by
    unfold latest
    rw [latest_none (fun e he => h e (by simp [he]))]
    simp [h d (by simp)]

theorem runStart_reviewed {counts : Counts} {start today cur : Int64}
    (hs : start.toInt ≤ cur.toInt) (ht : cur.toInt ≤ today.toInt)
    (hr : 0 < reviews counts cur)
    (hq : ∀ d : Int64, cur.toInt < d.toInt → d.toInt ≤ today.toInt → reviews counts d = 0) :
    runStart counts start today = cur.toInt + 1 := by
  unfold runStart
  have hp : reviewedIn counts start today cur = true := by
    simp [reviewedIn, Int64.le_iff_toInt_le, hs, ht, hr]
  cases hl : latest (reviewedIn counts start today) (counts.map Prod.fst) with
  | none => exact absurd hl (latest_some.latest_ne_none (mem_keys_of_reviews hr) hp)
  | some m =>
    obtain ⟨_, hpm, hmax⟩ := latest_some hl
    have hcm := hmax cur (mem_keys_of_reviews hr) hp
    simp [reviewedIn, Int64.le_iff_toInt_le] at hpm
    obtain ⟨_, hmt, hmr⟩ := hpm
    have : m.toInt = cur.toInt := by
      by_cases hlt : cur.toInt < m.toInt
      · have := hq m hlt hmt; rw [this] at hmr; simp at hmr
      · omega
    simp [this]

theorem runStart_none {counts : Counts} {start today : Int64}
    (hq : ∀ d : Int64, start.toInt ≤ d.toInt → d.toInt ≤ today.toInt → reviews counts d = 0) :
    runStart counts start today = start.toInt := by
  unfold runStart
  rw [latest_none]
  intro d _
  simp only [reviewedIn, Int64.le_iff_toInt_le, decide_eq_false_iff_not]
  rintro ⟨h1, h2, h3⟩
  rw [hq d h1 h2] at h3
  simp at h3

theorem minValue_lt_succ (d : Int64) : d.toInt + 1 ≠ Int64.minValue.toInt := by
  have := Int64.le_toInt d; rw [Int64.toInt_minValue]; omega

theorem at_most_one_step_per_day : StepBound openLapse := by
  sorry

theorem never_overflows : NoOverflow openLapse := by
  sorry

theorem answers_the_rule : AnswersTheRule openLapse := by
  sorry

/-! ## The witnesses -/

/-- A walk whose range starts the day before the window's first day. -/
def openLapsePastTheFirstDay (today : Int64) (counts : Counts) (skips : List Int64)
    (threshold : UInt32) : Run :=
  match windowStart counts with
  | none => ⟨Outcome.answer none, 0⟩
  | some start => walk counts skips threshold fuel ⟨start - 1, today, false⟩ ⟨0, none⟩ 0

/-- A loop that reads a day, then steps back from it with no check. -/
def walkWithAnUncheckedStep (counts : Counts) (skips : List Int64) (threshold : UInt32)
    (start : Int64) : Nat → Int64 → Tally → Nat → Run
  | 0, _, _, steps => ⟨Outcome.outOfFuel, steps⟩
  | fuel + 1, day, t, steps =>
    if day < start then ⟨Outcome.answer (answerOf threshold t), steps⟩
    else if reviews counts day > 0 then ⟨Outcome.answer (answerOf threshold t), steps + 1⟩
    else
      match stepBack day with
      | none => ⟨Outcome.overflow, steps + 1⟩
      | some before =>
        walkWithAnUncheckedStep counts skips threshold start fuel before (tally skips day t)
          (steps + 1)

/-- The walk with a step that is not checked. -/
def openLapseWithAnUncheckedStep (today : Int64) (counts : Counts) (skips : List Int64)
    (threshold : UInt32) : Run :=
  match windowStart counts with
  | none => ⟨Outcome.answer none, 0⟩
  | some start => walkWithAnUncheckedStep counts skips threshold start fuel today ⟨0, none⟩ 0

/-- The range walk without the guard at the smallest day. -/
def walkWithoutTheGuard (counts : Counts) (skips : List Int64) (threshold : UInt32) :
    Nat → Days → Tally → Nat → Run
  | 0, _, _, steps => ⟨Outcome.outOfFuel, steps⟩
  | fuel + 1, days, t, steps =>
    match nextBack days with
    | Back.done => ⟨Outcome.answer (answerOf threshold t), steps⟩
    | Back.overflow => ⟨Outcome.overflow, steps⟩
    | Back.next day rest =>
      if reviews counts day > 0 then ⟨Outcome.answer (answerOf threshold t), steps + 1⟩
      else walkWithoutTheGuard counts skips threshold fuel rest (tally skips day t) (steps + 1)

/-- `open_lapse` without the guard at the smallest day. -/
def openLapseWithoutTheGuard (today : Int64) (counts : Counts) (skips : List Int64)
    (threshold : UInt32) : Run :=
  match windowStart counts with
  | none => ⟨Outcome.answer none, 0⟩
  | some start => walkWithoutTheGuard counts skips threshold fuel ⟨start, today, false⟩ ⟨0, none⟩ 0

/-- A walk past the window's first day takes more steps than the window has days. -/
theorem a_walk_past_the_first_day_breaks_the_step_bound :
    ¬ StepBound openLapsePastTheFirstDay := by
  intro h
  exact absurd (h 0 [(0, 0)] [] 1) (by decide +kernel)

/-- A step back from a day with no check overflows at the smallest day. -/
theorem an_unchecked_step_overflows_at_the_smallest_day :
    ¬ NoOverflow openLapseWithAnUncheckedStep := by
  intro h
  exact h Int64.minValue [(Int64.minValue, 0)] [] 1 (by decide +kernel)

/-- A walk without the guard answers a lapse at the smallest day, where the rule answers none. -/
theorem an_unguarded_walk_answers_at_the_smallest_day :
    ¬ AnswersTheRule openLapseWithoutTheGuard := by
  intro h
  exact absurd (h Int64.minValue [(Int64.minValue, 0)] [] 1) (by decide +kernel)

end Formal.OpenLapse
