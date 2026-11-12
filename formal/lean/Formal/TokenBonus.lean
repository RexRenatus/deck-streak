-- @phx covers crates/quests/src/tokens.rs anchor=token_bonus_xp digest=sha256:1b2fff0d82a72e38be1e90ba00f500b04da5f4c2ebc142b8111e93f3f2e97a34
-- @phx vectors formal/vectors/token-bonus.jsonl
-- @phx cites #102, #103
-- @phx theorem the_bonus_never_passes_its_cap ramp=report
-- @phx witness a_bonus_with_no_cap_passes_it kills=the_bonus_never_passes_its_cap
-- @phx theorem a_window_with_no_study_review_pays_nothing ramp=report
-- @phx witness a_bonus_paid_for_every_window_pays_an_empty_one kills=a_window_with_no_study_review_pays_nothing
-- @phx theorem the_bonus_is_the_capped_sum_of_the_windows_study_reviews ramp=report
-- @phx witness a_window_closed_at_its_end_counts_one_review_too_many kills=the_bonus_is_the_capped_sum_of_the_windows_study_reviews

/-!
# Formal.TokenBonus

A double-XP token's bonus on one study day (#102, #103), `token_bonus_xp` in
`crates/quests/src/tokens.rs`, as a total function over `Int`.

**The claim.** For every window, study day and list of reviews:
- `the_bonus_never_passes_its_cap`: a bonus, when there is one, is at most 300 XP;
- `a_window_with_no_study_review_pays_nothing`: when no review counts, there is no bonus at all;
- `the_bonus_is_the_capped_sum_of_the_windows_study_reviews`: when a review counts, the bonus is
  the sum of the base XP of every review that counts, capped at 300, and otherwise there is none.
  A review counts when it is a study event (`is_study_event`: its kind is 0 to 3 and its ease is
  at least 1), its instant lies in the window, from its start and before its end, and it falls on
  the study day.

**What is ported.** The code walks the reviews once, keeping whether one counted and the XP so
far, and adds with `i64::saturating_add`; the port keeps both in a fold and adds with the same
saturation at the `i64` bounds. The study day of an instant is the kernel's rule, which the port
takes as an argument, `onDay i`, read "instant `i` falls on the study day": every claim holds for
every such predicate, so for every rule and day. The cap is `TOKEN_BONUS_CAP_XP`; the vectors
carry it to the Rust test, which fails if it moves.

**The witnesses** are ports that each break one claim at one input: a bonus with no cap, for one
review of 301 XP; a bonus paid for every window, for a window with no review; and a window closed
at its end, for one study review at the window's end.
-/

namespace Formal.TokenBonus

/-- `TOKEN_BONUS_CAP_XP`: the most a token's bonus pays on one study day. -/
def tokenBonusCapXp : Int := 300

/-- `i64::MIN`. -/
def i64Min : Int := -9223372036854775808

/-- `i64::MAX`. -/
def i64Max : Int := 9223372036854775807

/-- A review as the bonus reads it: its instant (the review id, in milliseconds), its kind, its
ease and its XP at the base rate. -/
structure Review where
  id : Int
  kind : Int
  ease : Int
  baseXp : Int
  deriving DecidableEq, Repr

/-- `is_study_event` in the ingest reader: a learn, review, relearn or filtered review with an
answer. -/
def isStudyEvent (kind ease : Int) : Bool :=
  decide (0 ≤ kind ∧ kind ≤ 3) && decide (1 ≤ ease)

/-- `i64::saturating_add`. -/
def saturatingAdd (x y : Int) : Int :=
  max i64Min (min i64Max (x + y))

/-- Whether review `r` counts: a study event in the window `[start, stop)`, on the study day. -/
def Counts (start stop : Int) (onDay : Int → Bool) (r : Review) : Bool :=
  isStudyEvent r.kind r.ease && decide (start ≤ r.id ∧ r.id < stop) && onDay r.id

/-- One review of the code's loop: a review that counts sets the flag and adds its XP. -/
def step (start stop : Int) (onDay : Int → Bool) (acc : Bool × Int) (r : Review) : Bool × Int :=
  if Counts start stop onDay r then (true, saturatingAdd acc.2 r.baseXp) else acc

/-- `token_bonus_xp`: the loop, then the capped XP when a review counted, else none. -/
def tokenBonusXp (start stop : Int) (onDay : Int → Bool) (reviews : List Review) :
    Option Int :=
  let acc := reviews.foldl (step start stop onDay) (false, 0)
  if acc.1 then some (min acc.2 tokenBonusCapXp) else none

/-- The XP of the reviews that count, added as the code adds. -/
def countedXp (start stop : Int) (onDay : Int → Bool) (reviews : List Review) : Int :=
  (reviews.filter (Counts start stop onDay)).foldl (fun xp r => saturatingAdd xp r.baseXp) 0

/-- The wrong variant: a bonus with no cap. -/
def tokenBonusXpWithNoCap (start stop : Int) (onDay : Int → Bool) (reviews : List Review) :
    Option Int :=
  let acc := reviews.foldl (step start stop onDay) (false, 0)
  if acc.1 then some acc.2 else none

/-- The wrong variant: a bonus paid for every window, a review counted or not. -/
def tokenBonusXpForEveryWindow (start stop : Int) (onDay : Int → Bool) (reviews : List Review) :
    Option Int :=
  let acc := reviews.foldl (step start stop onDay) (false, 0)
  some (min acc.2 tokenBonusCapXp)

/-- The wrong variant: a window closed at its end, so a review at the end counts. -/
def tokenBonusXpClosedAtItsEnd (start stop : Int) (onDay : Int → Bool)
    (reviews : List Review) : Option Int :=
  tokenBonusXp start (stop + 1) onDay reviews

/-- The claim, stated once: a bonus is at most the cap. -/
def NeverPassesTheCap (bonus : Int → Int → (Int → Bool) → List Review → Option Int) : Prop :=
  ∀ start stop onDay reviews b, bonus start stop onDay reviews = some b → b ≤ tokenBonusCapXp

/-- The claim, stated once: a window where no review counts pays nothing. -/
def EmptyWindowPaysNothing (bonus : Int → Int → (Int → Bool) → List Review → Option Int) :
    Prop :=
  ∀ start stop onDay reviews,
    (∀ r ∈ reviews, Counts start stop onDay r = false) → bonus start stop onDay reviews = none

/-- The claim, stated once: the bonus is the capped XP of the reviews that count, when one does. -/
def IsTheCappedCountedSum (bonus : Int → Int → (Int → Bool) → List Review → Option Int) :
    Prop :=
  ∀ start stop onDay reviews,
    bonus start stop onDay reviews =
      if reviews.any (Counts start stop onDay) then
        some (min (countedXp start stop onDay reviews) tokenBonusCapXp)
      else none

/-- The loop's state after any list: the flag is whether a review counted, and the XP is the
counted XP added onto where it started. -/
theorem foldl_step (start stop : Int) (onDay : Int → Bool) :
    ∀ (reviews : List Review) (w : Bool) (x : Int),
      reviews.foldl (step start stop onDay) (w, x) =
        (w || reviews.any (Counts start stop onDay),
          (reviews.filter (Counts start stop onDay)).foldl
            (fun xp r => saturatingAdd xp r.baseXp) x) := by
  intro reviews
  induction reviews with
  | nil => intro w x; simp
  | cons r rs ih =>
    intro w x
    by_cases h : Counts start stop onDay r = true
    · simp [List.foldl, step, h, ih, List.filter]
    · have hf : Counts start stop onDay r = false := by simpa using h
      simp [List.foldl, step, hf, ih, List.filter]

theorem the_bonus_is_the_capped_sum_of_the_windows_study_reviews :
    IsTheCappedCountedSum tokenBonusXp := by
  intro start stop onDay reviews
  simp only [tokenBonusXp, countedXp, foldl_step]
  simp

theorem the_bonus_never_passes_its_cap : NeverPassesTheCap tokenBonusXp := by
  intro start stop onDay reviews b h
  rw [the_bonus_is_the_capped_sum_of_the_windows_study_reviews] at h
  split at h
  · simp only [Option.some.injEq] at h
    rw [← h]
    exact Int.min_le_right _ _
  · simp at h

theorem a_window_with_no_study_review_pays_nothing :
    EmptyWindowPaysNothing tokenBonusXp := by
  intro start stop onDay reviews hnone
  rw [the_bonus_is_the_capped_sum_of_the_windows_study_reviews]
  have hany : reviews.any (Counts start stop onDay) = false := by
    simpa [List.any_eq_false] using hnone
  simp [hany]

/-- A study review of 301 XP inside a window of `[0, 10)`. -/
def overTheCap : List Review := [⟨5, 1, 3, 301⟩]

/-- A study review at the end of a window of `[0, 10)`. -/
def atTheEnd : List Review := [⟨10, 1, 3, 5⟩]

theorem a_bonus_with_no_cap_passes_it : ¬ NeverPassesTheCap tokenBonusXpWithNoCap := by
  intro h
  have h1 := h 0 10 (fun _ => true) overTheCap 301 (by decide)
  revert h1
  decide

theorem a_bonus_paid_for_every_window_pays_an_empty_one :
    ¬ EmptyWindowPaysNothing tokenBonusXpForEveryWindow := by
  intro h
  have h1 := h 0 10 (fun _ => true) [] (by simp)
  revert h1
  decide

theorem a_window_closed_at_its_end_counts_one_review_too_many :
    ¬ IsTheCappedCountedSum tokenBonusXpClosedAtItsEnd := by
  intro h
  have h1 := h 0 10 (fun _ => true) atTheEnd
  revert h1
  decide

end Formal.TokenBonus
