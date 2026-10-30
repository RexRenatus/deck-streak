-- @phx covers crates/sync/src/merge.rs anchor=merge digest=sha256:5c675e6e5f7c91bd68c920182aaebac7c93db15da467b37780c1a9d398e75ec0
-- @phx covers crates/sync/src/merge.rs anchor=pick digest=sha256:3f3dee3d3b7c4b55599a393ab2058c991d75f9a5eafa85ea636580eeab6bdf34
-- @phx cites #15
-- @phx vectors formal/vectors/field-merge.jsonl
-- @phx theorem merge_comm ramp=report
-- @phx witness a_field_won_by_arrival_order kills=merge_comm
-- @phx theorem merge_assoc ramp=report
-- @phx witness a_merge_that_compares_clocks_in_a_window kills=merge_assoc
-- @phx theorem merge_idem ramp=report
-- @phx witness a_merge_that_restamps_the_winner kills=merge_idem
-- @phx theorem apply_perm_invariant ramp=report
-- @phx witness an_apply_that_keeps_the_last_arrival kills=apply_perm_invariant

import Formal.SyncOrder

/-!
# Formal.FieldMerge

A note field's register, written ahead of its code (SPEC-504, ADR-522). The register holds the
winning edit or nothing; `merge` keeps the greater edit by key (and by value, which two distinct
operations never share a key for), and `apply` folds a field's edits into the register. The claims:
`merge` is commutative, associative and idempotent, so `apply` gives one register for every arrival
order of the same edits. Each is refuted, at a concrete input, of the defect its witness names: a
register won by arrival order, a comparison that only orders clocks within a window, a merge that
re-stamps its winner, and an apply that keeps the last edit to arrive. The sync core's `merge` and
`pick` (SPEC-509) are the covered code; its `apply` folds as the port's does, and its test reads
`formal/vectors/field-merge.jsonl` byte-equal.
-/

namespace Formal.FieldMerge

open Formal.SyncOrder

/-- A field's register: the winning edit, or none yet. -/
abbrev Reg := Option Edit

/-- The greater of two edits. -/
def pick (a b : Edit) : Edit := if Edit.lt a b then b else a

/-- Merge two registers: the greater edit wins. -/
def merge : Reg → Reg → Reg
  | none, r => r
  | r, none => r
  | some a, some b => some (pick a b)

/-- Fold a field's edits into the register. -/
def apply (m : Reg → Reg → Reg) (l : List Edit) : Reg :=
  l.foldr (fun e r => m (some e) r) none

def Comm (m : Reg → Reg → Reg) : Prop := ∀ a b, m a b = m b a
def Assoc (m : Reg → Reg → Reg) : Prop := ∀ a b c, m (m a b) c = m a (m b c)
def Idem (m : Reg → Reg → Reg) : Prop := ∀ a, m a a = a
def ApplyPermInvariant (m : Reg → Reg → Reg) : Prop :=
  ∀ l1 l2 : List Edit, l1.Perm l2 → apply m l1 = apply m l2

theorem pick_comm (a b : Edit) : pick a b = pick b a := by
  by_cases hab : a = b
  · subst hab
    rfl
  · rcases Edit.lt_total hab with h | h
    · simp [pick, h, Edit.lt_asymm h]
    · simp [pick, h, Edit.lt_asymm h]

theorem pick_ge_left (a b : Edit) : Edit.lt (pick a b) a = false := by
  unfold pick
  split
  · next h => exact Edit.lt_asymm h
  · exact Edit.lt_irrefl a

theorem pick_ge_right (a b : Edit) : Edit.lt (pick a b) b = false := by
  unfold pick
  split
  · exact Edit.lt_irrefl b
  · next h => simpa using h

theorem pick_mem (a b : Edit) : pick a b = a ∨ pick a b = b := by
  unfold pick
  split <;> simp

theorem nlt_trans {x y z : Edit} (h1 : Edit.lt x y = false) (h2 : Edit.lt y z = false) :
    Edit.lt x z = false := by
  cases hxz : Edit.lt x z
  · rfl
  · by_cases hyz : y = z
    · subst hyz
      simp_all
    · rcases Edit.lt_total hyz with h | h
      · simp_all
      · have := Edit.lt_trans hxz h
        simp_all

theorem eq_of_nlt {x y : Edit} (h1 : Edit.lt x y = false) (h2 : Edit.lt y x = false) : x = y := by
  by_cases h : x = y
  · exact h
  · rcases Edit.lt_total h with h' | h' <;> simp_all

/-- Both groupings give the one edit of the three that none of them is greater than. -/
theorem pick_assoc (a b c : Edit) : pick (pick a b) c = pick a (pick b c) := by
  have la : Edit.lt (pick (pick a b) c) a = false := nlt_trans (pick_ge_left _ _) (pick_ge_left _ _)
  have lb : Edit.lt (pick (pick a b) c) b = false :=
    nlt_trans (pick_ge_left _ _) (pick_ge_right _ _)
  have lc : Edit.lt (pick (pick a b) c) c = false := pick_ge_right _ _
  have ra : Edit.lt (pick a (pick b c)) a = false := pick_ge_left _ _
  have rb : Edit.lt (pick a (pick b c)) b = false :=
    nlt_trans (pick_ge_right _ _) (pick_ge_left _ _)
  have rc : Edit.lt (pick a (pick b c)) c = false :=
    nlt_trans (pick_ge_right _ _) (pick_ge_right _ _)
  apply eq_of_nlt
  · rcases pick_mem a (pick b c) with h | h
    · rw [h]
      exact la
    · rw [h]
      rcases pick_mem b c with h' | h'
      · rw [h']
        exact lb
      · rw [h']
        exact lc
  · rcases pick_mem (pick a b) c with h | h
    · rw [h]
      rcases pick_mem a b with h' | h'
      · rw [h']
        exact ra
      · rw [h']
        exact rb
    · rw [h]
      exact rc

theorem merge_comm : Comm merge := by
  intro a b
  cases a <;> cases b <;> simp [merge, pick_comm]

theorem merge_assoc : Assoc merge := by
  intro a b c
  cases a <;> cases b <;> cases c <;> simp [merge, pick_assoc]

theorem merge_idem : Idem merge := by
  intro a
  cases a <;> simp [merge, pick, Edit.lt_irrefl]

theorem apply_perm_invariant : ApplyPermInvariant merge := by
  intro l1 l2 p
  induction p with
  | nil => rfl
  | cons x _ ih => simp only [apply, List.foldr] at *; rw [ih]
  | swap x y l =>
    simp only [apply, List.foldr]
    rw [← merge_assoc (some y) (some x), merge_comm (some y) (some x),
      merge_assoc (some x) (some y)]
  | trans _ _ ih12 ih23 => rw [ih12, ih23]

/-- The defect: the register takes the later argument, the edit that arrived last. -/
def mergeArrival : Reg → Reg → Reg
  | a, none => a
  | _, some b => some b

/-- The defect: clocks are ordered only when the later one is within one tick. -/
def ltWindow (a b : Edit) : Bool := a.hlc < b.hlc && b.hlc < a.hlc + 2
def mergeWindow : Reg → Reg → Reg
  | none, r => r
  | r, none => r
  | some a, some b => some (if ltWindow a b then b else a)

/-- The defect: the winner is re-stamped with the next clock. -/
def mergeRestamp : Reg → Reg → Reg
  | none, r => r
  | r, none => r
  | some a, some b => some { pick a b with hlc := (pick a b).hlc + 1 }

theorem a_field_won_by_arrival_order : ¬ Comm mergeArrival := by
  intro h
  have := h (some ⟨1, 0, 0, 0⟩) (some ⟨2, 0, 0, 0⟩)
  simp [mergeArrival] at this

theorem a_merge_that_compares_clocks_in_a_window : ¬ Assoc mergeWindow := by
  intro h
  have := h (some ⟨0, 0, 0, 0⟩) (some ⟨1, 0, 0, 0⟩) (some ⟨2, 0, 0, 0⟩)
  simp [mergeWindow, ltWindow] at this

theorem a_merge_that_restamps_the_winner : ¬ Idem mergeRestamp := by
  intro h
  have := h (some ⟨1, 0, 0, 0⟩)
  simp [mergeRestamp, pick, Edit.lt_irrefl] at this

theorem an_apply_that_keeps_the_last_arrival : ¬ ApplyPermInvariant mergeArrival := by
  intro h
  have := h [⟨1, 0, 0, 0⟩, ⟨2, 0, 0, 0⟩] [⟨2, 0, 0, 0⟩, ⟨1, 0, 0, 0⟩] (List.Perm.swap _ _ _)
  simp [apply, mergeArrival] at this

end Formal.FieldMerge
