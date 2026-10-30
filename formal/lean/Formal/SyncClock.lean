-- @phx covers crates/sync/src/clock.rs anchor=tick digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx cites #15
-- @phx vectors formal/vectors/sync-clock.jsonl
-- @phx theorem tick_gt_seen ramp=report
-- @phx witness a_clock_that_ignores_what_it_saw kills=tick_gt_seen
-- @phx theorem tick_ge_pt ramp=report
-- @phx witness a_clock_that_ignores_the_wall kills=tick_ge_pt
-- @phx theorem run_strict_mono ramp=report
-- @phx witness a_clock_that_can_repeat kills=run_strict_mono
-- @phx theorem causal ramp=report
-- @phx witness a_clock_that_forgets_its_own_last kills=causal

/-!
# Formal.SyncClock

The device's hybrid clock, written ahead of its code (SPEC-504, ADR-522). Before a device records
an operation it takes the next clock: one past the larger of its own last clock and every clock it
has pulled, and never below the wall clock. The sync core's `tick` (SPEC-509) is the covered code,
and its test reads `formal/vectors/sync-clock.jsonl` byte-equal.

Each claim is a `Prop` over a tick function, proved of `tick` and refuted, at a concrete input, of
the defect its witness names:
- `TickGtSeen`: the next clock is past every clock seen (`tickIgnoresSeen` refutes it).
- `TickGePt`: the next clock is at least the wall clock (`tickIgnoresWall`).
- `RunStrictMono`: a device's clocks strictly rise along any run of ticks (`tickCanRepeat`).
- `Causal`: an operation made after a device saw clock `m`, or made one at `m`, is past `m`
  (`tickForgetsLast`).
-/

namespace Formal.SyncClock

/-- The next clock from the device's last clock, the largest clock it has seen and the wall. -/
def tick (last seen pt : Nat) : Nat :=
  max (max last seen + 1) pt

/-- A run of ticks: each step's clock is the next step's last clock. -/
def run (f : Nat → Nat → Nat → Nat) : Nat → List (Nat × Nat) → List Nat
  | _, [] => []
  | last, (seen, pt) :: rest => f last seen pt :: run f (f last seen pt) rest

/-- A list that strictly rises. -/
def Rising : List Nat → Prop
  | a :: b :: rest => a < b ∧ Rising (b :: rest)
  | _ => True

def TickGtSeen (f : Nat → Nat → Nat → Nat) : Prop := ∀ last seen pt, seen < f last seen pt
def TickGePt (f : Nat → Nat → Nat → Nat) : Prop := ∀ last seen pt, pt ≤ f last seen pt
def RunStrictMono (f : Nat → Nat → Nat → Nat) : Prop :=
  ∀ last steps, Rising (last :: run f last steps)
def Causal (f : Nat → Nat → Nat → Nat) : Prop :=
  ∀ last seen pt m, m ≤ max last seen → m < f last seen pt

theorem tick_gt_seen : TickGtSeen tick := by
  intro last seen pt
  simp only [tick]
  omega

theorem tick_ge_pt : TickGePt tick := by
  intro last seen pt
  simp only [tick]
  omega

theorem run_strict_mono : RunStrictMono tick := by
  intro last steps
  induction steps generalizing last with
  | nil => simp [run, Rising]
  | cons s rest ih =>
    obtain ⟨seen, pt⟩ := s
    simp only [run, Rising]
    exact ⟨by simp only [tick]; omega, ih _⟩

theorem causal : Causal tick := by
  intro last seen pt m h
  simp only [tick]
  omega

/-- The defect: a clock that ticks past its own last value only. -/
def tickIgnoresSeen (last _seen pt : Nat) : Nat := max (last + 1) pt
/-- The defect: a clock that never reads the wall. -/
def tickIgnoresWall (last seen _pt : Nat) : Nat := max last seen + 1
/-- The defect: a clock that may repeat its last value. -/
def tickCanRepeat (last seen pt : Nat) : Nat := max (max last seen) pt
/-- The defect: a clock that forgets its own last value. -/
def tickForgetsLast (_last seen pt : Nat) : Nat := max (seen + 1) pt

theorem a_clock_that_ignores_what_it_saw : ¬ TickGtSeen tickIgnoresSeen := by
  intro h
  have := h 0 5 0
  simp [tickIgnoresSeen] at this

theorem a_clock_that_ignores_the_wall : ¬ TickGePt tickIgnoresWall := by
  intro h
  have := h 0 0 10
  simp [tickIgnoresWall] at this

theorem a_clock_that_can_repeat : ¬ RunStrictMono tickCanRepeat := by
  intro h
  have := h 0 [(0, 0)]
  simp [run, Rising, tickCanRepeat] at this

theorem a_clock_that_forgets_its_own_last : ¬ Causal tickForgetsLast := by
  intro h
  have := h 5 0 0 5 (by decide)
  simp [tickForgetsLast] at this

end Formal.SyncClock
