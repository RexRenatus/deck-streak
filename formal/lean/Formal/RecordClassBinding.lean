-- @phx covers scripts/mutation-verdict.py anchor=python_excuses digest=sha256:81d78105ee97ba1584ecd9b67a9db46d2e7d2e53612e5d1c08acd57801a9302e
-- @phx covers scripts/mutation-verdict.py anchor=Excuses digest=sha256:2b92816fa0ec9a189a4522849102ec6b16c94b9defa084439894fb8cca6c040b
-- @phx covers scripts/mutation-verdict.py anchor=judge_python digest=sha256:52ce8581b4657774556f21fae735957a2a98bd729464198b40aa27ac70b38678
-- @phx vectors formal/vectors/record-class-binding.jsonl
-- @phx cites #822, #322
-- @phx theorem each_record_is_judged_in_its_own_class_alone ramp=report
-- @phx witness old_violates kills=each_record_is_judged_in_its_own_class_alone

/-!
# Formal.RecordClassBinding

The mutation verdict's Python record binding, as `judge_python` judges one class (#822, SPEC-437,
ADR-451). `python_excuses` keeps the Python equivalence records whose file `classify` places in
the judged class, and `Excuses.bind` judges each kept record by how many mutants of that class's
population it binds: one is valid, none is STALE, two or more are AMBIGUOUS.

**The claim**, one `Prop` over a binding, `Claim`, over every judged class, every record list,
every record's class and every binding count:
- every record judged is of the judged class, with the verdict its count gives;
- every record of the judged class is judged exactly as many times as it is listed, by its count,
  so a record listed once is judged once.

**What is ported.** `python_excuses`' class test, a record kept when its class is the judged one,
as the two texts compared character by character; `Excuses.bind`'s three arms in their order (one
mutant, then none, then more); and the loop that judges the kept records in their order. A record
is its fragment's index, the class `classify` gives its file, and how many mutants of the judged
class's population `binds` holds it to.

**What is not ported.** `classify` itself, `binds` and the population: each record's class and
count are inputs here. The record's text and its file's text are what the count is read from.

The witness is dev's binding at `b25ed04a`: `judge_python` bound every record against the class's
population (`scripts/mutation-verdict.py:2155`, `scope = None` beside a whole listing) and kept the
valid ones of its class only afterwards (`:2167-2169`). The release run read the scripts record
`scripts/mutation_python.py:745:73`, which binds nothing in the oracle class's population, as
STALE in the oracle class (#822, #322).
-/

namespace Formal.RecordClassBinding

/-- One Python equivalence record as the binding reads it: its index in its fragment, the class
`classify` gives its file, and how many mutants of the judged class's population it binds. -/
structure Record where
  index : Nat
  klass : List Char
  binds : Nat
  deriving DecidableEq, Repr

/-- What `Excuses.bind` makes of one record. -/
inductive Verdict where
  | valid
  | stale
  | ambiguous
  deriving DecidableEq, Repr

/-- Two texts are equal, character by character, as `==` compares two `str`. -/
def same : List Char → List Char → Bool
  | [], [] => true
  | a :: as, b :: bs => a == b && same as bs
  | _, _ => false

/-- `Excuses.bind`'s arms for one record, in its order: one mutant, none, more. -/
def verdictOf (binds : Nat) : Verdict :=
  if binds = 1 then .valid else if binds = 0 then .stale else .ambiguous

/-- `python_excuses`' records for the judged class, in order: those whose class is that class. -/
def kept (klass : List Char) : List Record → List Record
  | [] => []
  | r :: rs => if same r.klass klass then r :: kept klass rs else kept klass rs

/-- `Excuses.bind` over the records it is given: each judged once, by its count, in order. -/
def bindAll : List Record → List (Record × Verdict)
  | [] => []
  | r :: rs => (r, verdictOf r.binds) :: bindAll rs

/-- The binding that ships: `judge_python` passes its class, and the class's records alone are
bound. -/
def binding (klass : List Char) (records : List Record) : List (Record × Verdict) :=
  bindAll (kept klass records)

/-- dev's filter after binding: a valid record is kept only when it is of the class, while a
STALE or AMBIGUOUS one was already a finding when it was bound. -/
def keptAfter (klass : List Char) : List (Record × Verdict) → List (Record × Verdict)
  | [] => []
  | (r, .valid) :: rest =>
    if same r.klass klass then (r, .valid) :: keptAfter klass rest else keptAfter klass rest
  | p :: rest => p :: keptAfter klass rest

/-- dev's binding at the base: every record bound, then filtered by class. -/
def bindingOld (klass : List Char) (records : List Record) : List (Record × Verdict) :=
  keptAfter klass (bindAll records)

/-- How many times `r` is judged with the verdict `v`. -/
def timesJudged (r : Record) (v : Verdict) : List (Record × Verdict) → Nat
  | [] => 0
  | (s, w) :: rest => (if s = r ∧ w = v then 1 else 0) + timesJudged r v rest

/-- How many times `r` is listed. -/
def timesListed (r : Record) : List Record → Nat
  | [] => 0
  | s :: rest => (if s = r then 1 else 0) + timesListed r rest

/-- The claim over a binding: every record judged is of the judged class, with its count's
verdict, and every record of that class is judged as many times as it is listed, by its count. -/
def Claim (bind : List Char → List Record → List (Record × Verdict)) : Prop :=
  ∀ (klass : List Char) (records : List Record),
    (∀ p ∈ bind klass records, same p.1.klass klass = true ∧ p.2 = verdictOf p.1.binds) ∧
      ∀ r ∈ records, same r.klass klass = true →
        timesJudged r (verdictOf r.binds) (bind klass records) = timesListed r records

theorem mem_kept (klass : List Char) :
    ∀ (records : List Record) (r : Record), r ∈ kept klass records →
      same r.klass klass = true ∧ r ∈ records
  | [], _, h => by simp [kept] at h
  | s :: rest, r, h => by
    by_cases hs : same s.klass klass = true
    · simp only [kept, hs, ite_true, List.mem_cons] at h
      rcases h with h | h
      · subst h
        exact ⟨hs, List.mem_cons_self⟩
      · have := mem_kept klass rest r h
        exact ⟨this.1, List.mem_cons_of_mem _ this.2⟩
    · simp only [kept, hs, Bool.false_eq_true, ite_false] at h
      have := mem_kept klass rest r h
      exact ⟨this.1, List.mem_cons_of_mem _ this.2⟩

theorem mem_bindAll :
    ∀ (records : List Record) (p : Record × Verdict), p ∈ bindAll records →
      p.1 ∈ records ∧ p.2 = verdictOf p.1.binds
  | [], _, h => by simp [bindAll] at h
  | s :: rest, p, h => by
    simp only [bindAll, List.mem_cons] at h
    rcases h with h | h
    · subst h
      exact ⟨List.mem_cons_self, rfl⟩
    · have := mem_bindAll rest p h
      exact ⟨List.mem_cons_of_mem _ this.1, this.2⟩

theorem judged_bindAll (r : Record) :
    ∀ records : List Record, timesJudged r (verdictOf r.binds) (bindAll records) =
      timesListed r records
  | [] => rfl
  | s :: rest => by
    simp only [bindAll, timesJudged, timesListed, judged_bindAll r rest]
    by_cases h : s = r
    · subst h
      simp
    · simp [h]

theorem listed_kept (klass : List Char) (r : Record) (hr : same r.klass klass = true) :
    ∀ records : List Record, timesListed r (kept klass records) = timesListed r records
  | [] => rfl
  | s :: rest => by
    by_cases hs : same s.klass klass = true
    · simp only [kept, hs, ite_true, timesListed, listed_kept klass r hr rest]
    · have hne : s ≠ r := fun h => hs (h ▸ hr)
      simp only [kept, hs, Bool.false_eq_true, ite_false, timesListed, hne, ite_false,
        listed_kept klass r hr rest, Nat.zero_add]

/-- The binding that ships judges each record in its own class alone. -/
theorem each_record_is_judged_in_its_own_class_alone : Claim binding := by
  intro klass records
  refine ⟨fun p hp => ?_, fun r _ hr => ?_⟩
  · have hb := mem_bindAll (kept klass records) p hp
    exact ⟨(mem_kept klass records p.1 hb.1).1, hb.2⟩
  · simp only [binding, judged_bindAll, listed_kept klass r hr]

/-! ## The witness: the release run's reading, a scripts record judged in the oracle class -/

/-- A scripts record that binds no mutant of the oracle class's population. -/
def scriptsRecord : Record := ⟨1, "scripts".toList, 0⟩

/-- dev's binding judges the scripts record STALE in the oracle class, so the claim is false of
it. -/
theorem old_violates : ¬ Claim bindingOld := by
  intro h
  have judged := (h "oracle".toList [scriptsRecord]).1 (scriptsRecord, .stale) (by decide +kernel)
  exact absurd judged.1 (by decide +kernel)

end Formal.RecordClassBinding
