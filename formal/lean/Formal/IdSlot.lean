-- @phx covers crates/sync/src/ids.rs anchor=next_id digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx vectors formal/vectors/id-slot.jsonl
-- @phx cites #15
-- @phx theorem next_id_residue ramp=report
-- @phx witness a_slot_added_without_its_residue kills=next_id_residue
-- @phx theorem next_id_above_floor ramp=report
-- @phx witness an_allocator_without_the_floor kills=next_id_above_floor
-- @phx theorem next_id_at_least_now ramp=report
-- @phx witness an_allocator_without_the_instant kills=next_id_at_least_now
-- @phx theorem slots_disjoint ramp=report
-- @phx witness an_allocator_that_ignores_the_slot kills=slots_disjoint

/-!
# Formal.IdSlot

The one id allocator (SPEC-509, ADR-527): `nextId` ports `next_id` in `crates/sync/src/ids.rs`.
An id is the least value above the floor (the largest id the store has made or seen) and at or
above the instant, whose residue mod 64 is the allocating slot's; when that value would pass the
largest 64-bit signed integer, there is none. The writer `Formal/IdSlotVectors.lean` prints the
port's answers to `formal/vectors/id-slot.jsonl`, which the crate's test reads byte-equal.

Each claim is a `Prop` over an allocator, proved of `nextId` and refuted, at a concrete input, of
the defect its witness names:
- `Residue`: every id holds its slot's residue (`nextIdSlotUnreduced` refutes it).
- `AboveFloor`: every id is above the floor (`nextIdWithoutFloor`).
- `AtLeastNow`: every id is at or above the instant (`nextIdWithoutNow`).
- `SlotsDisjoint`: two slots never make the same id, whatever their floors and instants
  (`nextIdIgnoresSlot`).
-/

namespace Formal.IdSlot

/-- The largest value a 64-bit signed integer holds. -/
def idMax : Int := 9223372036854775807

/-- The least id above `floor` and at or above `now` whose residue mod 64 is `slot`, or none
when it would pass `idMax`. -/
def nextId (floor now slot : Int) : Option Int :=
  if idMax < floor + 1 then none
  else
    let base := max (floor + 1) now
    let id := base + (slot - base % 64) % 64
    if idMax < id then none else some id

/-- Every id an allocator makes for a slot below 64 holds that slot's residue. -/
def Residue (f : Int → Int → Int → Option Int) : Prop :=
  ∀ floor now slot id, 0 ≤ slot → slot < 64 → f floor now slot = some id → id % 64 = slot

/-- Every id an allocator makes is above the floor. -/
def AboveFloor (f : Int → Int → Int → Option Int) : Prop :=
  ∀ floor now slot id, 0 ≤ slot → slot < 64 → f floor now slot = some id → floor < id

/-- Every id an allocator makes is at or above the instant. -/
def AtLeastNow (f : Int → Int → Int → Option Int) : Prop :=
  ∀ floor now slot id, 0 ≤ slot → slot < 64 → f floor now slot = some id → now ≤ id

/-- Two slots never make the same id, whatever floors and instants they allocate at. -/
def SlotsDisjoint (f : Int → Int → Int → Option Int) : Prop :=
  ∀ floor₁ now₁ slot₁ floor₂ now₂ slot₂ id₁ id₂,
    0 ≤ slot₁ → slot₁ < 64 → 0 ≤ slot₂ → slot₂ < 64 → slot₁ ≠ slot₂ →
    f floor₁ now₁ slot₁ = some id₁ → f floor₂ now₂ slot₂ = some id₂ → id₁ ≠ id₂

/-- An id `nextId` makes is the base, the floor's successor raised to the instant, moved up to
the slot's residue. -/
theorem nextId_some {floor now slot id : Int} (h : nextId floor now slot = some id) :
    id = max (floor + 1) now + (slot - max (floor + 1) now % 64) % 64 := by
  revert h
  unfold nextId
  split
  · intro h
    cases h
  · dsimp only
    split
    · intro h
      cases h
    · intro h
      exact (Option.some.inj h).symm

theorem next_id_residue : Residue nextId := by
  intro floor now slot id h0 h64 h
  have := nextId_some h
  omega

theorem next_id_above_floor : AboveFloor nextId := by
  intro floor now slot id h0 h64 h
  have := nextId_some h
  omega

theorem next_id_at_least_now : AtLeastNow nextId := by
  intro floor now slot id h0 h64 h
  have := nextId_some h
  omega

theorem slots_disjoint : SlotsDisjoint nextId := by
  intro floor₁ now₁ slot₁ floor₂ now₂ slot₂ id₁ id₂ h0₁ h64₁ h0₂ h64₂ hne h₁ h₂ heq
  have r₁ := next_id_residue floor₁ now₁ slot₁ id₁ h0₁ h64₁ h₁
  have r₂ := next_id_residue floor₂ now₂ slot₂ id₂ h0₂ h64₂ h₂
  rw [heq] at r₁
  omega

/-- The defect: the slot added to the base as it stands, never reduced mod 64. -/
def nextIdSlotUnreduced (floor now slot : Int) : Option Int :=
  if idMax < floor + 1 then none
  else
    let base := max (floor + 1) now
    if idMax < base + slot then none else some (base + slot)

/-- The defect: no floor; the id is taken from the instant alone. -/
def nextIdWithoutFloor (_floor now slot : Int) : Option Int :=
  let id := now + (slot - now % 64) % 64
  if idMax < id then none else some id

/-- The defect: no instant; the id is taken from the floor alone. -/
def nextIdWithoutNow (floor _now slot : Int) : Option Int :=
  if idMax < floor + 1 then none
  else
    let id := floor + 1 + (slot - (floor + 1) % 64) % 64
    if idMax < id then none else some id

/-- The defect: the slot is never read, so every slot takes slot 0's id. -/
def nextIdIgnoresSlot (floor now _slot : Int) : Option Int :=
  nextId floor now 0

theorem a_slot_added_without_its_residue : ¬ Residue nextIdSlotUnreduced := by
  intro h
  have := h 0 0 1 2 (by omega) (by omega) (by simp [nextIdSlotUnreduced, idMax] <;> omega)
  omega

theorem an_allocator_without_the_floor : ¬ AboveFloor nextIdWithoutFloor := by
  intro h
  have := h 10 0 0 0 (by omega) (by omega) (by simp [nextIdWithoutFloor, idMax] <;> omega)
  omega

theorem an_allocator_without_the_instant : ¬ AtLeastNow nextIdWithoutNow := by
  intro h
  have := h 0 100 1 1 (by omega) (by omega) (by simp [nextIdWithoutNow, idMax] <;> omega)
  omega

theorem an_allocator_that_ignores_the_slot : ¬ SlotsDisjoint nextIdIgnoresSlot := by
  intro h
  have := h 0 0 1 0 0 2 64 64 (by omega) (by omega) (by omega) (by omega) (by omega)
    (by simp [nextIdIgnoresSlot, nextId, idMax] <;> omega)
    (by simp [nextIdIgnoresSlot, nextId, idMax] <;> omega)
  exact this rfl

end Formal.IdSlot
