-- @phx covers crates/habits/src/writing.rs anchor=writing_day_xp digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/habits/src/writing.rs anchor=all_confirmed_days digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx vectors formal/vectors/habit-writing.jsonl
-- @phx cites #94, #93
-- @phx theorem the_writing_day_pays_each_confirmed_course_and_the_bonus_only_over_a_full_set ramp=report
-- @phx witness the_predecessors_subset_rule_pays_the_bonus_over_no_writing_course kills=the_writing_day_pays_each_confirmed_course_and_the_bonus_only_over_a_full_set
-- @phx theorem no_day_is_all_confirmed_over_no_writing_course ramp=report
-- @phx witness an_empty_writing_set_confirms_every_day kills=no_day_is_all_confirmed_over_no_writing_course

/-!
# Formal.HabitWriting

A study day's writing XP (#94, SPEC-078 R7 and R8), `writing_day_xp` and `all_confirmed_days` in
`crates/habits/src/writing.rs`, as total functions over any course type with decidable equality
and epoch days as `Int`.

**The claim.** For every writing set, study day and set of confirmations:
- `the_writing_day_pays_each_confirmed_course_and_the_bonus_only_over_a_full_set`: each writing
  course pays 75 XP on the day when it is confirmed on it and 0 otherwise, in the writing set's
  order and for no other code, and `write:all` pays 100 exactly when the writing set is not empty
  and every course in it is confirmed on the day;
- `no_day_is_all_confirmed_over_no_writing_course`: a day is one of `all_confirmed_days` exactly
  when the writing set is not empty and every course in it is confirmed on that day. `write:all`
  and the writing streak both read this one helper, so its guard decides both.

**What is ported.** The code reads the confirmations into a set of (course, day) pairs and returns
a set of days; the port keeps the rows as a list and answers with a list, and every claim is about
membership, which a set and a list of the same elements share. `writing_day_xp` pays a course when
its pair is confirmed, and pays `write:all` when the day is one of `all_confirmed_days`, as the code
does. The amounts are `WRITING_XP_PER_DAY` and `WRITING_XP_ALL_THREE_BONUS`; the vectors carry both
to the Rust test, which fails if either moves.

**The witnesses** port the predecessor at 27ee2bc, a commit that ran: `habits.py:writing_day_xp`,
whose `set(codes) <= set(done_codes)` pays the bonus over an empty writing set, and
`habits.py:_writing_days_all`, whose `need <= got` holds for every day with a row when `need` is
empty, as Python's `all([])` does.
-/

namespace Formal.HabitWriting

/-- `WRITING_XP_PER_DAY`: what a confirmed writing course pays on one study day. -/
def writingXpPerDay : Nat := 75

/-- `WRITING_XP_ALL_THREE_BONUS`: what `write:all` pays on a day every writing course is
confirmed. -/
def writingXpAllThreeBonus : Nat := 100

section Port

variable {α : Type} [DecidableEq α]

/-- `all_confirmed_days`: no day over an empty writing set; otherwise each confirmed row's day on
which every writing course is confirmed. -/
def allConfirmedDays (writing : List α) (rows : List (α × Int)) : List Int :=
  if writing.isEmpty then []
  else (rows.map Prod.snd).filter fun day => writing.all fun code => decide ((code, day) ∈ rows)

/-- `writing_day_xp`: each writing course and its XP on the day, then the day's `write:all`. -/
def writingDayXp (writing : List α) (day : Int) (rows : List (α × Int)) :
    List (α × Nat) × Nat :=
  let courses := writing.map fun code =>
    (code, if (code, day) ∈ rows then writingXpPerDay else 0)
  let all := if day ∈ allConfirmedDays writing rows then writingXpAllThreeBonus else 0
  (courses, all)

/-- The predecessor's `_writing_days_all`: every day with a row on which the confirmed codes
include every writing code, which an empty writing set always passes. -/
def allConfirmedDaysPredecessor (writing : List α) (rows : List (α × Int)) : List Int :=
  (rows.map Prod.snd).filter fun day => writing.all fun code => decide ((code, day) ∈ rows)

/-- The predecessor's `writing_day_xp`: the same per-course grants, and the bonus when the
writing codes are a subset of the day's confirmed codes, an empty set included. -/
def writingDayXpPredecessor (writing : List α) (day : Int) (rows : List (α × Int)) :
    List (α × Nat) × Nat :=
  (writing.map fun code => (code, if (code, day) ∈ rows then writingXpPerDay else 0),
    if ∀ code ∈ writing, (code, day) ∈ rows then writingXpAllThreeBonus else 0)

/-- The claim, stated once: each course pays when it is confirmed, and the bonus is paid exactly
over a non-empty, fully confirmed writing set. -/
def PaysEachCourseAndTheBonusOnlyOverAFullSet
    (f : List α → Int → List (α × Int) → List (α × Nat) × Nat) : Prop :=
  ∀ writing day rows,
    (f writing day rows).1 =
        writing.map (fun code => (code, if (code, day) ∈ rows then writingXpPerDay else 0)) ∧
      (f writing day rows).2 =
        if writing ≠ [] ∧ ∀ code ∈ writing, (code, day) ∈ rows then writingXpAllThreeBonus
        else 0

/-- The claim, stated once: a day is all-confirmed exactly when the writing set is not empty and
every course in it is confirmed on that day. -/
def AllConfirmedExactly (f : List α → List (α × Int) → List Int) : Prop :=
  ∀ writing rows day, day ∈ f writing rows ↔ writing ≠ [] ∧ ∀ code ∈ writing, (code, day) ∈ rows

theorem no_day_is_all_confirmed_over_no_writing_course :
    AllConfirmedExactly (allConfirmedDays (α := α)) := by
  intro writing rows day
  cases writing with
  | nil => simp [allConfirmedDays]
  | cons c cs =>
    simp only [allConfirmedDays, List.isEmpty_cons, Bool.false_eq_true, ↓reduceIte,
      List.mem_filter, List.mem_map, List.all_eq_true, decide_eq_true_eq]
    constructor
    · rintro ⟨_, hall⟩
      exact ⟨List.cons_ne_nil c cs, hall⟩
    · rintro ⟨_, hall⟩
      exact ⟨⟨(c, day), hall c (List.mem_cons_self), rfl⟩, hall⟩

theorem the_writing_day_pays_each_confirmed_course_and_the_bonus_only_over_a_full_set :
    PaysEachCourseAndTheBonusOnlyOverAFullSet (writingDayXp (α := α)) := by
  intro writing day rows
  refine ⟨rfl, ?_⟩
  have h := no_day_is_all_confirmed_over_no_writing_course writing rows day
  simp only [writingDayXp, h]

end Port

/-- One confirmation, of course 0 on epoch day 0. -/
def oneRow : List (Nat × Int) := [(0, 0)]

theorem the_predecessors_subset_rule_pays_the_bonus_over_no_writing_course :
    ¬ PaysEachCourseAndTheBonusOnlyOverAFullSet (writingDayXpPredecessor (α := Nat)) := by
  intro h
  have h1 := (h [] 0 []).2
  revert h1
  decide

theorem an_empty_writing_set_confirms_every_day :
    ¬ AllConfirmedExactly (allConfirmedDaysPredecessor (α := Nat)) := by
  intro h
  have h1 := h [] oneRow 0
  revert h1
  decide

end Formal.HabitWriting
