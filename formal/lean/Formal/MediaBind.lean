-- @phx covers crates/media/src/bind.rs anchor=step digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers crates/media/src/bind.rs anchor=release_step digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx vectors formal/vectors/media-bind.jsonl
-- @phx cites #822
-- @phx theorem bind_keeps_a_bound_hash ramp=report
-- @phx witness an_overwriting_bind_rebinds_a_bound_name kills=bind_keeps_a_bound_hash
-- @phx theorem bind_records_its_outcome ramp=report
-- @phx witness a_bind_that_records_nothing_leaves_its_name_unbound kills=bind_records_its_outcome
-- @phx theorem bind_is_idempotent ramp=report
-- @phx witness a_bind_that_refuses_its_repeat_is_not_idempotent kills=bind_is_idempotent
-- @phx theorem revival_only_for_the_same_hash ramp=report
-- @phx witness a_revival_by_any_hash_rebinds_a_released_name kills=revival_only_for_the_same_hash
-- @phx theorem a_mismatched_claim_is_refused ramp=report
-- @phx witness a_bind_blind_to_its_claim_admits_a_mismatch kills=a_mismatched_claim_is_refused
-- @phx theorem release_keeps_the_hash ramp=report
-- @phx witness a_release_that_forgets_lets_a_name_rebind kills=release_keeps_the_hash

/-!
# Formal.MediaBind

The media store's write-once bindings (SPEC-513 R3 and A3; ADR-531 D3): `step` and
`release_step` in `crates/media/src/bind.rs`, the machine over one name's slot.

**The claim.** For every slot, every claim and every hash:
- `bind_keeps_a_bound_hash`: a bound name, live or released, keeps its hash through any bind.
- `bind_records_its_outcome`: an admitted bind leaves the name live at the hash it was given, and
  a refused bind leaves the slot as it was.
- `bind_is_idempotent`: an admitted bind repeated with the same claim and hash changes nothing and
  says so.
- `revival_only_for_the_same_hash`: a released name is admitted again only for its own hash.
- `a_mismatched_claim_is_refused`: a certified name's claim of another hash is refused, and the
  slot is left as it was.
- `release_keeps_the_hash`: a release leaves the name bound, released, to the hash it had.

**What is ported.** A hash is a natural number: the machine reads only whether two hashes are
equal. A claim is the hash a certified name carries, none for an imported name. The Rust
`Result` of a bind, `Bound` or `BindRefusal`, is one outcome here; a release's likewise.

**What is not ported.** The hash itself (SHA-256, the `sha2` crate's), the name's form, and the
map from names to slots (`Bindings`, which applies the machine to the slot of the name's text).
The vectors tie the port to the Rust code: `crates/media/tests/formal_vectors_bind.rs` answers
every vector with `step` and `release_step`.

**Each witness** is the machine with one rule broken: a bind that overwrites a bound name, a bind
that admits without recording, a bind that refuses its own repeat, a revival by any hash, a bind
that ignores its claim, and a release that forgets the name.
-/

namespace Formal.MediaBind

/-- `BindState`. -/
inductive BindState where
  | live
  | released
  deriving DecidableEq, Repr

/-- `Binding`: the hash a name is bound to, and its state. -/
structure Binding where
  hash : Nat
  state : BindState
  deriving DecidableEq, Repr

/-- `Bound` and `BindRefusal`, one outcome. -/
inductive Outcome where
  | new
  | unchanged
  | revived
  | mismatch
  | conflict
  deriving DecidableEq, Repr

/-- Whether an outcome admits the bind: `Ok` in Rust. -/
def Outcome.admitted : Outcome → Bool
  | .new => true
  | .unchanged => true
  | .revived => true
  | .mismatch => false
  | .conflict => false

/-- A bind over one name's slot: the slot, the claimed hash and the file's hash. -/
abbrev StepFn := Option Binding → Option Nat → Nat → Option Binding × Outcome

/-- `step`: a claim of another hash is refused; an unbound name is bound live; a bound name is
admitted again only for its own hash, unchanged when live and revived when released; any other
bind is a conflict. A refusal returns the slot as it was. -/
def step (slot : Option Binding) (claimed : Option Nat) (hash : Nat) : Option Binding × Outcome :=
  if claimed.any (fun c => c != hash) then (slot, .mismatch)
  else
    match slot with
    | none => (some ⟨hash, .live⟩, .new)
    | some b =>
      if b.hash = hash then
        match b.state with
        | .live => (slot, .unchanged)
        | .released => (some ⟨hash, .live⟩, .revived)
      else (slot, .conflict)

/-- `Release` and `ReleaseRefusal`, one outcome. -/
inductive ReleaseOutcome where
  | released
  | alreadyReleased
  | unbound
  deriving DecidableEq, Repr

/-- `release_step`: a live binding is released and keeps its hash; a released one is left as it
was; an unbound name is refused. -/
def releaseStep : Option Binding → Option Binding × ReleaseOutcome
  | none => (none, .unbound)
  | some b =>
    match b.state with
    | .live => (some ⟨b.hash, .released⟩, .released)
    | .released => (some b, .alreadyReleased)

/-- A bound name keeps its hash through any bind. -/
def KeepsABoundHash (f : StepFn) : Prop :=
  ∀ (b : Binding) (claimed : Option Nat) (hash : Nat),
    (f (some b) claimed hash).1.map Binding.hash = some b.hash

/-- An admitted bind leaves the name live at its hash; a refused one leaves the slot as it was. -/
def RecordsItsOutcome (f : StepFn) : Prop :=
  ∀ (slot : Option Binding) (claimed : Option Nat) (hash : Nat),
    ((f slot claimed hash).2.admitted = true → (f slot claimed hash).1 = some ⟨hash, .live⟩) ∧
    ((f slot claimed hash).2.admitted = false → (f slot claimed hash).1 = slot)

/-- An admitted bind, repeated, changes nothing and says so. -/
def Idempotent (f : StepFn) : Prop :=
  ∀ (slot : Option Binding) (claimed : Option Nat) (hash : Nat),
    (f slot claimed hash).2.admitted = true →
      f (f slot claimed hash).1 claimed hash = ((f slot claimed hash).1, .unchanged)

/-- A released name is admitted again only for its own hash. -/
def RevivesOnlyItsOwnHash (f : StepFn) : Prop :=
  ∀ (old : Nat) (claimed : Option Nat) (hash : Nat),
    (f (some ⟨old, .released⟩) claimed hash).2.admitted = true → hash = old

/-- A claim of another hash is refused, and the slot is left as it was. -/
def RefusesAMismatch (f : StepFn) : Prop :=
  ∀ (slot : Option Binding) (claim hash : Nat), claim ≠ hash →
    f slot (some claim) hash = (slot, .mismatch)

/-- A release leaves the name bound, released, to its hash. -/
def KeepsTheHashOnRelease (g : Option Binding → Option Binding × ReleaseOutcome) : Prop :=
  ∀ (b : Binding), (g (some b)).1 = some ⟨b.hash, .released⟩

/-- Every case of `step`, as one statement about its slot and outcome. -/
theorem step_cases (slot : Option Binding) (claimed : Option Nat) (hash : Nat) :
    (claimed.any (fun c => c != hash) = true ∧ step slot claimed hash = (slot, .mismatch)) ∨
    (claimed.any (fun c => c != hash) = false ∧ slot = none ∧
      step slot claimed hash = (some ⟨hash, .live⟩, .new)) ∨
    (claimed.any (fun c => c != hash) = false ∧ slot = some ⟨hash, .live⟩ ∧
      step slot claimed hash = (slot, .unchanged)) ∨
    (claimed.any (fun c => c != hash) = false ∧ slot = some ⟨hash, .released⟩ ∧
      step slot claimed hash = (some ⟨hash, .live⟩, .revived)) ∨
    (claimed.any (fun c => c != hash) = false ∧ (∃ b : Binding, slot = some b ∧ b.hash ≠ hash) ∧
      step slot claimed hash = (slot, .conflict)) := by
  cases hc : claimed.any (fun c => c != hash)
  · cases slot with
    | none => exact Or.inr (Or.inl ⟨rfl, rfl, by simp [step, hc]⟩)
    | some b =>
      cases b with
      | mk old state =>
        by_cases he : old = hash
        · subst he
          cases state with
          | live => exact Or.inr (Or.inr (Or.inl ⟨rfl, rfl, by simp [step, hc]⟩))
          | released => exact Or.inr (Or.inr (Or.inr (Or.inl ⟨rfl, rfl, by simp [step, hc]⟩)))
        · exact Or.inr (Or.inr (Or.inr (Or.inr
            ⟨rfl, ⟨⟨old, state⟩, rfl, he⟩, by simp [step, hc, he]⟩)))
  · exact Or.inl ⟨rfl, by simp [step, hc]⟩

theorem bind_keeps_a_bound_hash : KeepsABoundHash step := by
  intro b claimed hash
  rcases step_cases (some b) claimed hash with
    ⟨_, h⟩ | ⟨_, h, _⟩ | ⟨_, h, e⟩ | ⟨_, h, e⟩ | ⟨_, _, e⟩
  · rw [h]; rfl
  · cases h
  · rw [e]; rfl
  · cases h; rw [e]; rfl
  · rw [e]; rfl

theorem bind_records_its_outcome : RecordsItsOutcome step := by
  intro slot claimed hash
  rcases step_cases slot claimed hash with
    ⟨_, e⟩ | ⟨_, h, e⟩ | ⟨_, h, e⟩ | ⟨_, h, e⟩ | ⟨_, _, e⟩
  · rw [e]; exact ⟨fun h => (by cases h), fun _ => rfl⟩
  · rw [e]; exact ⟨fun _ => rfl, fun h => (by cases h)⟩
  · rw [e]; exact ⟨fun _ => h, fun h => (by cases h)⟩
  · rw [e]; exact ⟨fun _ => rfl, fun h => (by cases h)⟩
  · rw [e]; exact ⟨fun h => (by cases h), fun _ => rfl⟩

theorem bind_is_idempotent : Idempotent step := by
  intro slot claimed hash ok
  rcases step_cases slot claimed hash with
    ⟨_, e⟩ | ⟨hc, _, e⟩ | ⟨hc, h, e⟩ | ⟨hc, _, e⟩ | ⟨_, _, e⟩
  · rw [e] at ok; cases ok
  · rw [e]; simp [step, hc]
  · rw [e, h]; simp [step, hc]
  · rw [e]; simp [step, hc]
  · rw [e] at ok; cases ok

theorem revival_only_for_the_same_hash : RevivesOnlyItsOwnHash step := by
  intro old claimed hash ok
  rcases step_cases (some ⟨old, .released⟩) claimed hash with
    ⟨_, e⟩ | ⟨_, h, _⟩ | ⟨_, h, _⟩ | ⟨_, h, _⟩ | ⟨_, _, e⟩
  · rw [e] at ok; cases ok
  · cases h
  · cases h
  · cases h; rfl
  · rw [e] at ok; cases ok

theorem a_mismatched_claim_is_refused : RefusesAMismatch step := by
  intro slot claim hash ne
  have hc : (some claim).any (fun c => c != hash) = true := by simp [ne]
  simp [step, hc]

theorem release_keeps_the_hash : KeepsTheHashOnRelease releaseStep := by
  intro b
  cases b with
  | mk hash state => cases state <;> rfl

/-- The wrong variant: a bind of a bound name to another hash overwrites it. -/
def overwritingStep (slot : Option Binding) (claimed : Option Nat) (hash : Nat) :
    Option Binding × Outcome :=
  match step slot claimed hash with
  | (_, .conflict) => (some ⟨hash, .live⟩, .new)
  | other => other

theorem an_overwriting_bind_rebinds_a_bound_name : ¬ KeepsABoundHash overwritingStep := by
  intro h
  have := h ⟨0, .live⟩ none 1
  revert this
  decide

/-- The wrong variant: a bind of an unbound name says it is new and records nothing. -/
def recordlessStep (slot : Option Binding) (claimed : Option Nat) (hash : Nat) :
    Option Binding × Outcome :=
  match step slot claimed hash with
  | (_, .new) => (slot, .new)
  | other => other

theorem a_bind_that_records_nothing_leaves_its_name_unbound :
    ¬ RecordsItsOutcome recordlessStep := by
  intro h
  have := (h none none 0).1 rfl
  revert this
  decide

/-- The wrong variant: a bind of a live name to its own hash is refused as a conflict. -/
def repeatRefusingStep (slot : Option Binding) (claimed : Option Nat) (hash : Nat) :
    Option Binding × Outcome :=
  match step slot claimed hash with
  | (s, .unchanged) => (s, .conflict)
  | other => other

theorem a_bind_that_refuses_its_repeat_is_not_idempotent : ¬ Idempotent repeatRefusingStep := by
  intro h
  have := h none none 0 rfl
  revert this
  decide

/-- The wrong variant: a released name is revived by any hash. -/
def anyRevivalStep (slot : Option Binding) (claimed : Option Nat) (hash : Nat) :
    Option Binding × Outcome :=
  match slot, step slot claimed hash with
  | some ⟨_, .released⟩, (_, .conflict) => (some ⟨hash, .live⟩, .revived)
  | _, other => other

theorem a_revival_by_any_hash_rebinds_a_released_name :
    ¬ RevivesOnlyItsOwnHash anyRevivalStep := by
  intro h
  have := h 0 none 1 rfl
  revert this
  decide

/-- The wrong variant: the claim is not read. -/
def claimBlindStep (slot : Option Binding) (_claimed : Option Nat) (hash : Nat) :
    Option Binding × Outcome :=
  step slot none hash

theorem a_bind_blind_to_its_claim_admits_a_mismatch : ¬ RefusesAMismatch claimBlindStep := by
  intro h
  have := h none 0 1 (by decide)
  revert this
  decide

/-- The wrong variant: a release forgets the name, so any hash may bind it again. -/
def forgettingRelease : Option Binding → Option Binding × ReleaseOutcome
  | none => (none, .unbound)
  | some b =>
    match b.state with
    | .live => (none, .released)
    | .released => (some b, .alreadyReleased)

theorem a_release_that_forgets_lets_a_name_rebind :
    ¬ KeepsTheHashOnRelease forgettingRelease := by
  intro h
  have := h ⟨0, .live⟩
  revert this
  decide

end Formal.MediaBind
