-- @phx covers crates/curriculum/src/progress.rs anchor=band_step digest=sha256:e36e7f81009accd4bb06b3ea20f8cdbb59d44de1654869c9b81fa68d6173ef59
-- @phx covers crates/curriculum/src/progress.rs anchor=course_progress digest=sha256:d7ed52f1d7617e92289724bef4baf6c96e69c7335fc31a650aecb20159a4e788
-- @phx covers crates/curriculum/src/progress.rs anchor=parse_unit digest=sha256:fa38ae082b21e74544ac15bfea44a980118150795466f9e82f1e128500711f24
-- @phx covers crates/curriculum/src/law.rs anchor=mastery_pillar digest=sha256:fc81b65561a36ee22789383953b83520a5330cd9567bf7d73a001befe08d2660
-- @phx covers crates/coordination/src/recompute/progress.rs anchor=record_progress digest=sha256:12f49fc092311f2dc7d07dcac4f0dd424ead54bb691054e029621c8782c6028e
-- @phx vectors formal/vectors/road-to-c2.jsonl
-- @phx cites #85
-- @phx theorem the_current_band_is_a1_or_the_end_of_the_run_from_a1 ramp=report
-- @phx witness a_current_band_one_past_the_run_misses_it kills=the_current_band_is_a1_or_the_end_of_the_run_from_a1
-- @phx theorem an_achieved_band_after_a_gap_never_counts ramp=report
-- @phx witness the_last_achieved_band_counts_after_a_gap kills=an_achieved_band_after_a_gap_never_counts
-- @phx theorem a_band_up_is_only_a_later_band_and_a_first_sighting_is_silent ramp=report
-- @phx witness a_band_reached_again_reads_as_a_band_up kills=a_band_up_is_only_a_later_band_and_a_first_sighting_is_silent
-- @phx theorem the_law_pillar_lies_in_its_range_and_never_rises_with_leeches ramp=report
-- @phx witness a_pillar_with_no_penalty_cap_leaves_the_range kills=the_law_pillar_lies_in_its_range_and_never_rises_with_leeches

/-!
# Formal.RoadToC2

Road to C2's pure band rules (#85): the current band `course_progress` reads from its bands, the
band-up decision `band_step` in `crates/curriculum/src/progress.rs`, and the law mastery pillar
`mastery_pillar` in `crates/curriculum/src/law.rs`, as total functions.

**The claim.** For every input:
- `the_current_band_is_a1_or_the_end_of_the_run_from_a1`: the current band is the last band of the
  run of achieved bands that starts at A1, or A1 when A1 is not achieved;
- `an_achieved_band_after_a_gap_never_counts`: once a band is not achieved, no later band moves the
  current band;
- `a_band_up_is_only_a_later_band_and_a_first_sighting_is_silent`: a course with no stored band is a
  first sighting of its current band, never a band-up; and a band-up names the reached band, only
  when both bands have a place in the order and the reached one comes later than the stored one;
- `the_law_pillar_lies_in_its_range_and_never_rises_with_leeches`: the pillar is between 70 and 100
  for a count of 0 or more, 100 for a negative count, and never rises as the count grows.

**What is ported, and how the floats are.**
- `course_progress` computes each band's achieved flag in `f64` (its count is above 0 and its mean
  mastery is at least 80 percent), then walks the bands in order with a `contiguous` flag. The port
  takes the flags as its argument, in the bands' order, and ports the walk; the claims hold for
  every list of flags, so for every float the code computes. A band is its place in `CEFR_BANDS`.
- `band_step` finds each band's place in `CEFR_BANDS` with a closure. The port takes that place
  function as an argument, so the claim holds for every place function, `CEFR_BANDS`'s included.
  Only a band-up is paid: `record_progress` records a first sighting as a silent baseline and grants
  XP only on a band-up whose milestone was written for the first time, so it is covered beside it.
- `mastery_pillar` multiplies the leech count by 3, takes the least of that and 30, subtracts it
  from 100 and clamps the result to 0..100, in `f64`. The port performs the same operations over
  `Int`, and its answer equals the code's at every `i64`: the conversion, the product by 3 and the
  subtraction are exact for counts from 0 to 9, every count of 10 or more meets the cap of 30
  exactly, and every negative count gives a value above 100 that the clamp makes 100. The vectors
  carry the `i64` extremes to the Rust test.
- `parse_unit` reads text; the vectors hold its answers, written by a port over characters, at
  inputs that include a unit beyond `u32` and several units in one name.

**The witnesses** are ports that each break one claim at one input: a current band one past the
run, at A1 alone achieved; the last achieved band, at A1 and B1 achieved around a gap; a band-up
on a band reached again, at A2 stored and reached; and a pillar with no penalty cap, at 20 leeches.
-/

namespace Formal.RoadToC2

/-- `CEFR_BANDS`, in order. -/
def cefrBands : List String := ["A1", "A2", "B1", "B2", "C1", "C2"]

/-- The walk of `course_progress` over the bands' achieved flags from `band` on, with its
`contiguous` flag and the current band so far. -/
def courseProgressFrom : List Bool → Nat → Bool → Nat → Nat
  | [], _, _, current => current
  | achieved :: rest, band, contiguous, current =>
    if contiguous && achieved then courseProgressFrom rest (band + 1) true band
    else courseProgressFrom rest (band + 1) false current

/-- `course_progress`'s current band, as its place in `CEFR_BANDS`, from the bands' achieved flags
in order: the walk starts contiguous at A1. -/
def courseProgress (achieved : List Bool) : Nat :=
  courseProgressFrom achieved 0 true 0

/-- `BandStep`. -/
inductive BandStep where
  | firstSighting (band : String)
  | bandUp (band : String)
  | unchanged
  deriving DecidableEq, Repr

/-- `band_step`, with the place of a band in the order as its argument `place`. -/
def bandStep (place : String → Option Nat) (stored : Option String) (current : String) :
    BandStep :=
  match stored with
  | none => BandStep.firstSighting current
  | some held =>
    match place held, place current with
    | some heldAt, some reachedAt =>
      if reachedAt > heldAt then BandStep.bandUp current else BandStep.unchanged
    | _, _ => BandStep.unchanged

/-- The place of `band` in `bands`, counting from `index`. -/
def placeIn : List String → Nat → String → Option Nat
  | [], _, _ => none
  | candidate :: rest, index, band =>
    if candidate == band then some index else placeIn rest (index + 1) band

/-- The closure `band_step` uses: a band's place in `CEFR_BANDS`. -/
def cefrPlace (band : String) : Option Nat :=
  placeIn cefrBands 0 band

/-- `MASTERY_LEECH_PENALTY`: points one active law leech takes off the pillar. -/
def masteryLeechPenalty : Int := 3

/-- `MASTERY_LEECH_PENALTY_CAP`: the most the leeches take off the pillar. -/
def masteryLeechPenaltyCap : Int := 30

/-- `f64::clamp`: below `lo` is `lo`, above `hi` is `hi`. -/
def clamp (x lo hi : Int) : Int :=
  if x < lo then lo else if x > hi then hi else x

/-- `mastery_pillar`, over `Int`. -/
def masteryPillar (lawLeechActive : Int) : Int :=
  clamp (100 - min (masteryLeechPenalty * lawLeechActive) masteryLeechPenaltyCap) 0 100

/-- `char::is_whitespace`: the code points with Unicode's White_Space property. -/
def isWhitespace (c : Char) : Bool :=
  let n := c.toNat
  (0x09 ≤ n && n ≤ 0x0D) || n == 0x20 || n == 0x85 || n == 0xA0 || n == 0x1680 ||
    (0x2000 ≤ n && n ≤ 0x200A) || n == 0x2028 || n == 0x2029 || n == 0x202F || n == 0x205F ||
    n == 0x3000

/-- `char::is_ascii_digit`. -/
def isAsciiDigit (c : Char) : Bool :=
  '0'.toNat ≤ c.toNat && c.toNat ≤ '9'.toNat

/-- The number ASCII digits spell. -/
def digitValue (digits : List Char) : Nat :=
  digits.foldl (fun value digit => value * 10 + (digit.toNat - '0'.toNat)) 0

/-- The digits after their leading zeros, parsed as a `u32`: `0` for none left, and no unit for a
number past `u32::MAX`. -/
def parseDigits (digits : List Char) : Option Nat :=
  let significant := digits.dropWhile (· == '0')
  if significant.isEmpty then some 0
  else
    let value := digitValue significant
    if value < 2 ^ 32 then some value else none

/-- `parse_unit` over characters: the first `Unit` followed by white space and digits. -/
def parseUnitChars : List Char → Option Nat
  | [] => none
  | 'U' :: 'n' :: 'i' :: 't' :: after =>
    let trimmed := after.dropWhile isWhitespace
    if trimmed.length < after.length then
      let digits := trimmed.takeWhile isAsciiDigit
      if digits.isEmpty then parseUnitChars after else parseDigits digits
    else parseUnitChars after
  | _ :: rest => parseUnitChars rest

/-- `parse_unit`. -/
def parseUnit (deckName : String) : Option Nat :=
  parseUnitChars deckName.toList

/-- The wrong variant: the current band is the first band not achieved, one past the run. -/
def courseProgressOnePast (achieved : List Bool) : Nat :=
  (achieved.takeWhile id).length

/-- The walk with no `contiguous` flag: the last achieved band, after a gap or not. -/
def courseProgressFromLastAchieved : List Bool → Nat → Nat → Nat
  | [], _, current => current
  | achieved :: rest, band, current =>
    if achieved then courseProgressFromLastAchieved rest (band + 1) band
    else courseProgressFromLastAchieved rest (band + 1) current

/-- The wrong variant: the current band is the last achieved band. -/
def courseProgressLastAchieved (achieved : List Bool) : Nat :=
  courseProgressFromLastAchieved achieved 0 0

/-- The wrong variant: a band reached again, at the stored band's own place, reads as a band-up. -/
def bandStepReachedAgain (place : String → Option Nat) (stored : Option String)
    (current : String) : BandStep :=
  match stored with
  | none => BandStep.firstSighting current
  | some held =>
    match place held, place current with
    | some heldAt, some reachedAt =>
      if reachedAt ≥ heldAt then BandStep.bandUp current else BandStep.unchanged
    | _, _ => BandStep.unchanged

/-- The wrong variant: a pillar with no penalty cap. -/
def masteryPillarWithNoCap (lawLeechActive : Int) : Int :=
  clamp (100 - masteryLeechPenalty * lawLeechActive) 0 100

/-- The claim, stated once: the current band is the last band of the achieved run from A1, or A1
when that run is empty (`0 - 1` is `0` in `Nat`). -/
def CurrentIsRunEnd (current : List Bool → Nat) : Prop :=
  ∀ achieved : List Bool, current achieved = (achieved.takeWhile id).length - 1

/-- The claim, stated once: a band that is not achieved ends the run, so nothing after it counts. -/
def GapEndsTheRun (current : List Bool → Nat) : Prop :=
  ∀ before after : List Bool, current (before ++ false :: after) = current before

/-- The claim, stated once: a first sighting is the silent baseline of the current band, and a
band-up names the reached band, later in the order than the stored band. -/
def BandUpOnlyLater (step : (String → Option Nat) → Option String → String → BandStep) : Prop :=
  ∀ (place : String → Option Nat) (stored : Option String) (current : String),
    (stored = none → step place stored current = BandStep.firstSighting current) ∧
      ∀ band : String, step place stored current = BandStep.bandUp band →
        band = current ∧ ∃ held heldAt reachedAt, stored = some held ∧
          place held = some heldAt ∧ place current = some reachedAt ∧ heldAt < reachedAt

/-- The claim, stated once: the pillar lies in 70..100 for a count of 0 or more, is 100 for a
negative count, and never rises as the count grows. -/
def PillarInRangeAndFalling (pillar : Int → Int) : Prop :=
  (∀ n : Int, 0 ≤ n → 70 ≤ pillar n ∧ pillar n ≤ 100) ∧ (∀ n : Int, n < 0 → pillar n = 100) ∧
    ∀ n m : Int, n ≤ m → pillar m ≤ pillar n

/-- A walk that has left the run keeps its current band. -/
theorem courseProgressFrom_after_a_gap (achieved : List Bool) (band current : Nat) :
    courseProgressFrom achieved band false current = current := by
  induction achieved generalizing band with
  | nil => rfl
  | cons head rest ih => simp [courseProgressFrom, ih]

/-- A contiguous walk ends on the last band of the run it starts, or keeps its current band when
the run is empty. -/
theorem courseProgressFrom_contiguous (achieved : List Bool) (band current : Nat) :
    courseProgressFrom achieved band true current =
      if (achieved.takeWhile id).length = 0 then current
      else band + (achieved.takeWhile id).length - 1 := by
  induction achieved generalizing band current with
  | nil => rfl
  | cons head rest ih =>
    cases head with
    | false => simp [courseProgressFrom, courseProgressFrom_after_a_gap]
    | true =>
      simp only [courseProgressFrom, Bool.and_self, ite_true, List.takeWhile_cons, id,
        List.length_cons]
      rw [ih]
      simp only [Nat.add_one_ne_zero, ite_false]
      split <;> omega

/-- The run of `before ++ false :: after` is the run of `before`. -/
theorem takeWhile_gap (before after : List Bool) :
    (before ++ false :: after).takeWhile id = before.takeWhile id := by
  induction before with
  | nil => rfl
  | cons head rest ih => cases head <;> simp [ih]

theorem the_current_band_is_a1_or_the_end_of_the_run_from_a1 :
    CurrentIsRunEnd courseProgress := by
  intro achieved
  simp only [courseProgress, courseProgressFrom_contiguous]
  split <;> omega

theorem an_achieved_band_after_a_gap_never_counts : GapEndsTheRun courseProgress := by
  intro before after
  rw [the_current_band_is_a1_or_the_end_of_the_run_from_a1,
    the_current_band_is_a1_or_the_end_of_the_run_from_a1, takeWhile_gap]

theorem a_band_up_is_only_a_later_band_and_a_first_sighting_is_silent :
    BandUpOnlyLater bandStep := by
  intro place stored current
  refine ⟨fun h => by subst h; rfl, fun band h => ?_⟩
  cases stored with
  | none => simp [bandStep] at h
  | some held =>
    simp only [bandStep] at h
    split at h
    · rename_i heldAt reachedAt hheld hreached
      split at h
      · rename_i hlt
        injection h with h
        exact ⟨h.symm, held, heldAt, reachedAt, rfl, hheld, hreached, hlt⟩
      · contradiction
    · contradiction

/-- The clamp to 0..100 is the greatest of 0 and the least of the value and 100. -/
theorem clamp_to_a_hundred (x : Int) : clamp x 0 100 = max 0 (min x 100) := by
  unfold clamp
  by_cases below : x < 0
  · simp only [below, ite_true]
    omega
  · by_cases above : x > 100
    · simp only [below, above, ite_true, ite_false]
      omega
    · simp only [below, above, ite_false]
      omega

theorem the_law_pillar_lies_in_its_range_and_never_rises_with_leeches :
    PillarInRangeAndFalling masteryPillar := by
  refine ⟨fun n hn => ?_, fun n hn => ?_, fun n m hle => ?_⟩
  all_goals
    simp only [masteryPillar, clamp_to_a_hundred, masteryLeechPenalty, masteryLeechPenaltyCap]
    omega

theorem a_current_band_one_past_the_run_misses_it :
    ¬ CurrentIsRunEnd courseProgressOnePast := by
  intro h
  have := h [true]
  revert this
  decide

theorem the_last_achieved_band_counts_after_a_gap :
    ¬ GapEndsTheRun courseProgressLastAchieved := by
  intro h
  have := h [true] [true]
  revert this
  decide

theorem a_band_reached_again_reads_as_a_band_up :
    ¬ BandUpOnlyLater bandStepReachedAgain := by
  intro h
  let place : String → Option Nat := fun _ => some 1
  have hup : bandStepReachedAgain place (some "A2") "A2" = BandStep.bandUp "A2" := rfl
  obtain ⟨_, held, heldAt, reachedAt, hs, hh, hr, hlt⟩ := (h place (some "A2") "A2").2 "A2" hup
  injection hs with hs
  subst hs
  rw [hh] at hr
  injection hr with hr
  omega

theorem a_pillar_with_no_penalty_cap_leaves_the_range :
    ¬ PillarInRangeAndFalling masteryPillarWithNoCap := by
  intro h
  have := (h.1 20 (by decide)).1
  simp only [masteryPillarWithNoCap, clamp_to_a_hundred, masteryLeechPenalty] at this
  omega

end Formal.RoadToC2
