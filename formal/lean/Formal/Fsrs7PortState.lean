-- @phx covers crates/engine-core/src/replay.rs anchor=fold digest=sha256:122756ece51ecabd1976ac9faf79cfcef9f1d26f2de8eeaf65480c9c8dfdb9c8
-- @phx covers crates/engine-core/src/replay.rs anchor=new_state digest=sha256:35a48ac904f8d50abd0d8125ef0a74c55aed72f2957f9b33babd1349b095a4a9
-- @phx covers crates/engine-core/src/replay.rs anchor=same_model digest=sha256:324176ec32f62da3764f9e5c512e07f26c95327905f3f816214c0053eedb99ea
-- @phx covers crates/engine-core/src/scheduler.rs anchor=check_event digest=sha256:2d0a5b91be534d46902e5856a2b462914874917958b7352f44a3e50cb25a53cb
-- @phx cites #823
-- @phx theorem a_refused_event_leaves_the_state_unchanged ramp=report
-- @phx witness a_fold_that_writes_before_it_checks_keeps_a_refused_event kills=a_refused_event_leaves_the_state_unchanged
-- @phx theorem the_folded_keys_strictly_increase ramp=report
-- @phx witness a_check_that_admits_an_equal_key_folds_it_twice kills=the_folded_keys_strictly_increase

/-!
# Formal.Fsrs7PortState

The FSRS-7 implementation of the engine core's scheduling port where it has state (#823, Q1 and
Q3): `Fsrs7`'s `new_state`, `fold` and `same_model` in `crates/engine-core/src/replay.rs`, and the
port's `check_event` in `crates/engine-core/src/scheduler.rs`.

**The claims.** For every parameter vector and every event:
- `a_refused_event_leaves_the_state_unchanged`: a fold that refuses its event, for a state made
  under other parameters, a key not after the last, a grade outside 1 to 4 or an answer time past
  the cap, returns the state it was given.
- `the_folded_keys_strictly_increase`: in every state a model's folds reach from its new state,
  the folded events' keys strictly increase, read as the signed 64-bit integers they are.

**What is ported.** The state's events, last key and parameter bits, and each fold's checks in
the code's order, with every write after them. A 64-bit key is its bit pattern, below 2^64, read
signed as Rust's `i64` comparison reads it. The state's memo of replayed memory states is reset by
every fold and read only through `replayed`, so it is left out: two states with the same events,
last key and parameters replay alike.
-/

namespace Formal.Fsrs7PortState

/-- `ANSWER_MS_CAP`. -/
def answerMsCap : Nat := 3600000

/-- A review event: card, note, deck and preset as `i64`, the grade as `u8`, the answer time as
`u32`, the key as `i64` and the study day as `i32`, each as its bit pattern. -/
structure Event where
  card : Fin (2 ^ 64)
  note : Fin (2 ^ 64)
  deck : Fin (2 ^ 64)
  preset : Fin (2 ^ 64)
  grade : Fin (2 ^ 8)
  answerMs : Fin (2 ^ 32)
  key : Fin (2 ^ 64)
  studyDay : Fin (2 ^ 32)
  deriving DecidableEq, Repr

/-- An `i64` bit pattern read signed. -/
def signed (k : Fin (2 ^ 64)) : Int :=
  if k.val < 2 ^ 63 then k.val else (k.val : Int) - 2 ^ 64

/-- `FoldError`. -/
inductive FoldError where
  | otherModel
  | keyNotAfter (last key : Fin (2 ^ 64))
  | gradeOutOfRange (grade : Fin (2 ^ 8))
  | answerTooLong (answerMs : Fin (2 ^ 32))
  deriving DecidableEq, Repr

/-- A fold's answer: `Ok(())` or the error. -/
inductive Folded where
  | ok
  | err (e : FoldError)
  deriving DecidableEq, Repr

/-- `check_event`, refusing a key at or before the last (`strict`) or only one before it. -/
def checkEventWith (strict : Bool) (last : Option (Fin (2 ^ 64))) (ev : Event) : Folded :=
  match last with
  | some l =>
    if (if strict then signed ev.key ≤ signed l else signed ev.key < signed l) then
      .err (.keyNotAfter l ev.key)
    else if ev.grade.val < 1 ∨ 4 < ev.grade.val then .err (.gradeOutOfRange ev.grade)
    else if answerMsCap < ev.answerMs.val then .err (.answerTooLong ev.answerMs)
    else .ok
  | none =>
    if ev.grade.val < 1 ∨ 4 < ev.grade.val then .err (.gradeOutOfRange ev.grade)
    else if answerMsCap < ev.answerMs.val then .err (.answerTooLong ev.answerMs)
    else .ok

/-- `Fsrs7`: the bits of the parameters the model was made under. -/
structure Model where
  parameters : List (Fin (2 ^ 32))
  deriving DecidableEq, Repr

/-- `Fsrs7State`, its memo left out. -/
structure State where
  events : List Event
  last : Option (Fin (2 ^ 64))
  parameters : List (Fin (2 ^ 32))
  deriving DecidableEq, Repr

/-- `new_state`. -/
def newState (f : Model) : State := ⟨[], none, f.parameters⟩

/-- `same_model`. -/
def sameModel (f : Model) (st : State) : Folded :=
  if st.parameters = f.parameters then .ok else .err .otherModel

/-- `fold` on `&mut st`: the state after it, and its answer. With `checkFirst` the checks come
before the writes, as the code has them; without, the event is written first. -/
def foldWith (checkFirst strict : Bool) (f : Model) (st : State) (ev : Event) : State × Folded :=
  let pushed : State := ⟨st.events ++ [ev], some ev.key, st.parameters⟩
  let checked :=
    match sameModel f st with
    | .ok => checkEventWith strict st.last ev
    | .err e => .err e
  match checked with
  | .ok => (pushed, .ok)
  | .err e => (if checkFirst then st else pushed, .err e)

/-- The code's fold. -/
def fold : Model → State → Event → State × Folded := foldWith true true

/-- The states a model's folds reach from its new state. -/
inductive Reachable (fd : Model → State → Event → State × Folded) (f : Model) : State → Prop
  | new : Reachable fd f (newState f)
  | step {st st' : State} {ev : Event} : Reachable fd f st → fd f st ev = (st', .ok) →
      Reachable fd f st'

/-! ## The claims -/

/-- The refusal claim over a fold. -/
def RefusalRule (fd : Model → State → Event → State × Folded) : Prop :=
  ∀ f st ev st' e, fd f st ev = (st', .err e) → st' = st

/-- The key claim over a fold. -/
def KeysRule (fd : Model → State → Event → State × Folded) : Prop :=
  ∀ f st, Reachable fd f st → (st.events.map fun e => signed e.key).Pairwise (· < ·)

/-! ## The proofs -/

theorem a_refused_event_leaves_the_state_unchanged : RefusalRule fold := by
  intro f st ev st' e h
  simp only [fold, foldWith] at h
  split at h
  · simp at h
  · simp only [↓reduceIte, Prod.mk.injEq] at h
    exact h.1.symm

/-- A fold that answers `Ok` writes the event after the last, under the same parameters, and its
key is after the last. -/
theorem fold_ok {f : Model} {st st' : State} {ev : Event} (h : fold f st ev = (st', .ok)) :
    st' = ⟨st.events ++ [ev], some ev.key, st.parameters⟩ ∧
      ∀ l, st.last = some l → signed l < signed ev.key := by
  simp only [fold, foldWith] at h
  split at h
  · rename_i hc
    simp only [Prod.mk.injEq] at h
    refine ⟨h.1.symm, ?_⟩
    intro l hl
    revert hc
    unfold sameModel
    split
    · simp only [hl, checkEventWith, ↓reduceIte]
      split
      · simp
      · rename_i hk
        intro _
        grind
    · simp
  · simp at h

/-- What every reachable state keeps: its keys strictly increase, and each is at or before the
last key. -/
def Keyed (st : State) : Prop :=
  (st.events.map fun e => signed e.key).Pairwise (· < ·) ∧
    ∀ e ∈ st.events, ∃ l, st.last = some l ∧ signed e.key ≤ signed l

theorem reachable_keyed {f : Model} {st : State} (h : Reachable fold f st) : Keyed st := by
  induction h with
  | new => exact ⟨List.Pairwise.nil, fun e he => by simp [newState] at he⟩
  | step _ hf ih =>
    obtain ⟨e1, e2⟩ := fold_ok hf
    subst e1
    obtain ⟨p, b⟩ := ih
    refine ⟨?_, ?_⟩
    · simp only [List.map_append, List.map_cons, List.map_nil, List.pairwise_append,
        List.pairwise_singleton, List.mem_singleton, true_and]
      refine ⟨p, ?_⟩
      intro a ha b' hb
      subst hb
      simp only [List.mem_map] at ha
      obtain ⟨x, hx, rfl⟩ := ha
      obtain ⟨l, hl, hle⟩ := b x hx
      have := e2 l hl
      grind
    · intro x hx
      refine ⟨_, rfl, ?_⟩
      simp only [List.mem_append, List.mem_singleton] at hx
      rcases hx with hx | rfl
      · obtain ⟨l, hl, hle⟩ := b x hx
        have := e2 l hl
        grind
      · exact Int.le_refl _

theorem the_folded_keys_strictly_increase : KeysRule fold := by
  intro f st h
  exact (reachable_keyed h).1

/-! ## The witnesses -/

/-- A fold that writes the event before it checks it. -/
def foldWritingFirst : Model → State → Event → State × Folded := foldWith false true

/-- An event of grade 0 at key 5. -/
def gradeZero : Event := ⟨0, 0, 0, 0, 0, 0, 5, 0⟩

/-- A grade-0 event folded into a new state is refused, and the refused event stays written. -/
theorem a_fold_that_writes_before_it_checks_keeps_a_refused_event :
    ¬ RefusalRule foldWritingFirst := by
  intro h
  have := h ⟨[]⟩ (newState ⟨[]⟩) gradeZero _ _ rfl
  revert this
  decide +kernel

/-- A fold whose check refuses only a key before the last. -/
def foldAdmittingAnEqualKey : Model → State → Event → State × Folded := foldWith true false

/-- A Good event at key 5. -/
def goodAtFive : Event := ⟨0, 0, 0, 0, 3, 0, 5, 0⟩

/-- The same event folded twice: both are kept, at equal keys. -/
theorem a_check_that_admits_an_equal_key_folds_it_twice :
    ¬ KeysRule foldAdmittingAnEqualKey := by
  intro h
  have r1 : Reachable foldAdmittingAnEqualKey ⟨[]⟩ ⟨[goodAtFive], some 5, []⟩ :=
    Reachable.step (ev := goodAtFive) Reachable.new (by decide +kernel)
  have r2 : Reachable foldAdmittingAnEqualKey ⟨[]⟩ ⟨[goodAtFive, goodAtFive], some 5, []⟩ :=
    Reachable.step (ev := goodAtFive) r1 (by decide +kernel)
  have := h _ _ r2
  revert this
  decide +kernel

end Formal.Fsrs7PortState
