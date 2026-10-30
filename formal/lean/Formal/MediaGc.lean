-- @phx covers crates/media/src/sweep.rs anchor=fate digest=sha256:7e8a4c2017b74dd5315f0abb5beff9df9bd4c76eb3c950afb69161c790e4c531
-- @phx covers crates/media/src/sweep.rs anchor=in_grace digest=sha256:51406200d7d6364f17f5659ff25f0264940a7d53643a6a77ebfab97b079133ef
-- @phx vectors formal/vectors/media-gc.jsonl
-- @phx cites #822
-- @phx theorem a_kept_file_is_never_deleted ramp=report
-- @phx witness a_sweep_blind_to_grace_deletes_a_young_file kills=a_kept_file_is_never_deleted
-- @phx theorem every_other_file_is_deleted ramp=report
-- @phx witness a_sweep_that_keeps_every_file_deletes_none kills=every_other_file_is_deleted

/-!
# Formal.MediaGc

The media sweep's decision for one file (SPEC-513 R6 and A6; ADR-531 D6): `fate` and `in_grace`
in `crates/media/src/sweep.rs`. `plan_sweep` applies `fate` to every stored file.

**The claim.** For every file's facts and every sweep instant and grace window:
- `a_kept_file_is_never_deleted`: a file a note, template or CSS names, a pinned file, a file
  added within the grace window or after the sweep's instant, and an imported file are never
  deleted.
- `every_other_file_is_deleted`: a file none of those holds for is deleted.

**What is ported.** The instants are integers and the window a natural number, as the Rust code
reads them through `i128`, so no subtraction overflows on either side. Whether a file is named,
pinned or imported is a fact the port is handed; the Rust tests hold how the scan and the name
give them (`crates/media/tests/sweep.rs`, `crates/media/tests/check_golden.rs`).

**The grace window** is stated in the claim itself, not through the port's `inGrace`: a file is
young when the sweep's instant is at or before its added instant, or less than the window after
it.

**Each witness** is the decision with one rule broken: a sweep that never reads the grace window,
and a sweep that keeps every file.
-/

namespace Formal.MediaGc

/-- `Keep`: why a file stays, in the order the sweep asks. `recent` is the Rust `InGrace`. -/
inductive Keep where
  | live
  | pinned
  | recent
  | imported
  deriving DecidableEq, Repr

/-- `Fate`. -/
inductive Fate where
  | keep (reason : Keep)
  | delete
  deriving DecidableEq, Repr

/-- `Facts`. -/
structure Facts where
  referenced : Bool
  pinned : Bool
  imported : Bool
  addedMs : Int
  deriving DecidableEq, Repr

/-- `SweepPolicy`. -/
structure SweepPolicy where
  nowMs : Int
  graceMs : Nat
  deriving DecidableEq, Repr

/-- `in_grace`: added at or after the instant, or less than the window before it. -/
def inGrace (addedMs : Int) (policy : SweepPolicy) : Bool :=
  decide (policy.nowMs ≤ addedMs) || decide (policy.nowMs - addedMs < (policy.graceMs : Int))

/-- `fate`: kept for the first reason that holds, else deleted. -/
def fate (facts : Facts) (policy : SweepPolicy) : Fate :=
  if facts.referenced then .keep .live
  else if facts.pinned then .keep .pinned
  else if inGrace facts.addedMs policy then .keep .recent
  else if facts.imported then .keep .imported
  else .delete

/-- A file is young at an instant: added at or after it, or less than the window before it. -/
def Young (facts : Facts) (policy : SweepPolicy) : Prop :=
  policy.nowMs ≤ facts.addedMs ∨ policy.nowMs - facts.addedMs < (policy.graceMs : Int)

/-- A live, pinned, young or imported file is never deleted. -/
def KeepsWhatItMust (f : Facts → SweepPolicy → Fate) : Prop :=
  ∀ (facts : Facts) (policy : SweepPolicy),
    (facts.referenced = true ∨ facts.pinned = true ∨ Young facts policy ∨
      facts.imported = true) →
      f facts policy ≠ .delete

/-- Every other file is deleted. -/
def DeletesTheRest (f : Facts → SweepPolicy → Fate) : Prop :=
  ∀ (facts : Facts) (policy : SweepPolicy),
    facts.referenced = false → facts.pinned = false → ¬ Young facts policy →
      facts.imported = false → f facts policy = .delete

/-- The port's window is the claim's. -/
theorem inGrace_iff (addedMs : Int) (policy : SweepPolicy) (facts : Facts)
    (h : facts.addedMs = addedMs) : inGrace addedMs policy = true ↔ Young facts policy := by
  subst h
  simp [inGrace, Young]

theorem a_kept_file_is_never_deleted : KeepsWhatItMust fate := by
  intro facts policy held
  have window := inGrace_iff facts.addedMs policy facts rfl
  unfold fate
  cases hr : facts.referenced
  · cases hp : facts.pinned
    · cases hg : inGrace facts.addedMs policy
      · cases hi : facts.imported
        · exfalso
          rcases held with h | h | h | h
          · rw [hr] at h; cases h
          · rw [hp] at h; cases h
          · rw [← window, hg] at h; cases h
          · rw [hi] at h; cases h
        · simp
      · simp
    · simp
  · simp

theorem every_other_file_is_deleted : DeletesTheRest fate := by
  intro facts policy hr hp hy hi
  have window := inGrace_iff facts.addedMs policy facts rfl
  have hg : inGrace facts.addedMs policy = false := by
    cases h : inGrace facts.addedMs policy
    · rfl
    · exact absurd (window.mp h) hy
  simp [fate, hr, hp, hg, hi]

/-- The wrong variant: the sweep never reads the grace window. -/
def graceBlindFate (facts : Facts) (_policy : SweepPolicy) : Fate :=
  if facts.referenced then .keep .live
  else if facts.pinned then .keep .pinned
  else if facts.imported then .keep .imported
  else .delete

theorem a_sweep_blind_to_grace_deletes_a_young_file : ¬ KeepsWhatItMust graceBlindFate := by
  intro h
  exact h ⟨false, false, false, 0⟩ ⟨0, 0⟩ (Or.inr (Or.inr (Or.inl (Or.inl (by decide))))) rfl

/-- The wrong variant: the sweep keeps every file. -/
def keepEveryFate (_facts : Facts) (_policy : SweepPolicy) : Fate :=
  .keep .live

theorem a_sweep_that_keeps_every_file_deletes_none : ¬ DeletesTheRest keepEveryFate := by
  intro h
  have := h ⟨false, false, false, 0⟩ ⟨1, 0⟩ rfl rfl (by simp [Young]) rfl
  cases this

end Formal.MediaGc
