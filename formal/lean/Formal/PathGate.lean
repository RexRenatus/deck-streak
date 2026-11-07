-- @phx covers scripts/path_gate.py anchor=prefix_of digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers scripts/path_gate.py anchor=reads_entry digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers scripts/path_gate.py anchor=reads digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers scripts/path_gate.py anchor=decide digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx covers scripts/path_gate.py anchor=admit digest=sha256:0000000000000000000000000000000000000000000000000000000000000000
-- @phx vectors formal/vectors/path-gate.jsonl
-- @phx theorem the_gate_skips_exactly_the_changes_no_read_reaches ramp=report
-- @phx witness the_base_runs_a_job_on_a_change_it_never_reads kills=the_gate_skips_exactly_the_changes_no_read_reaches
-- @phx theorem a_skipped_job_passes_only_on_the_gates_own_skip ramp=report
-- @phx witness the_base_refuses_a_job_the_gate_skipped kills=a_skipped_job_passes_only_on_the_gates_own_skip

/-!
# Formal.PathGate

The path gate's decisions (SPEC-634, ADR-658), ported from `scripts/path_gate.py`: which gated Rust
job a pull request into dev may skip, and which results the ci aggregate admits.

**What is ported.** `prefix_of`, `reads_entry`, `reads`, `decide` and `admit`, branch for branch,
over text read as characters. The map is the one the reader admits: `always`, then each job with
its reads, in the file's order. `decide` answers `"run"` or `"skip"`, the two constructors of
`Decision` here, and the vectors print each by the script's word. `decide` is ported as
`decideGate` and `admit` as `admitGate`, since each script name is a tactic's name here.

**What is not ported.** `read_map`, which turns the map's JSON into the admitted map or refuses it
(a key held twice in one object, an empty entry, a directory named without its trailing `/`, a
schema other than `read-map.v1`); a refused map is `none` here. The script's test holds its
refusals, and the vectors carry one map text with a key held twice, which the reader must refuse.

**The claims.** `InMap` states the map's meaning from its schema alone, through no port: a refused
map has no member, a job with no entry reads every path, and otherwise a path is read when an entry
of `always` or of the job's reads reads it (`EntryReads`: `*`, the path itself, or an entry ending
in `/` that begins the path). `ExactGate` ties `reads` to `InMap`, and states that the decision
skips exactly a pull request into dev whose change list is readable and non-empty, on a parsed map,
when no changed path is read. `AdmitExact` states that the admission refuses exactly the gated
jobs whose results do not pass (`Passes`), in order, and passes every other job on with its result.

**The witnesses.** `decide` and `admit` are new, so each old code is the base's behaviour on the
witness input. At the base `ci.yml` has no paths filter (`ci.yml:14-19`) and the `ci` job needs
every job on every event (`ci.yml:1415`), so every job runs on every change: `alwaysRun`, which
runs a job on a change it never reads. The base's aggregate (`ci.yml:1462-1476`) counts one
admission for each mutation leg whose own result is `skipped`, and a skipped result passes only
while admissions remain: `baseAdmit`, which refuses a gated job the gate skipped.
-/

namespace Formal.PathGate

/-- The map as the reader admits it: the `always` entries, then each job with its reads, in the
file's order. The reader refuses a key held twice, so each job is named once. -/
structure ReadMap where
  always : List (List Char)
  jobs : List (List Char × List (List Char))

/-- A run's event as three strings: the event's name, the pull request's base ref and
base-is-dev's release flag. -/
abbrev Event := List Char × List Char × List Char

/-- The decision's two answers, the script's `"run"` and `"skip"`. -/
inductive Decision where
  | runJob
  | skipJob
  deriving DecidableEq, Repr

/-- The jobs the path gate decides for, the script's `GATED`. -/
def gated : List (List Char) :=
  ["rust".toList, "engine".toList, "release".toList, "census".toList]

/-! ## The port -/

/-- `prefix_of`: the entry `d` is a directory prefix, ending in `/`, and the path `q` begins with
it. -/
def prefix_of (d q : List Char) : Bool :=
  d.getLast? == some '/' && d.isPrefixOf q

/-- `reads_entry`: the entry `e` reads the path `q`: `*` reads every path, an exact entry its own
path and a directory prefix every path under it. -/
def reads_entry (e q : List Char) : Bool :=
  e == ['*'] || e == q || prefix_of e q

/-- `reads`: a refused map reads no path, a job with no entry reads every path, and otherwise a
path is read when an entry of `always` or of the job's reads reads it. -/
def reads : Option ReadMap → List Char → List Char → Bool
  | none, _, _ => false
  | some mp, job, q =>
    match mp.jobs.lookup job with
    | none => true
    | some r => (mp.always ++ r).any (fun e => reads_entry e q)

/-- `decide`: `skip` only for a pull request into dev whose change list is readable and
non-empty, on a parsed map, when no changed path is read by the job; `run` on every other input.
The empty change list is tested before the unreadable one, as the script tests them. -/
def decideGate (ev : Event) (job : List Char) (changed : Option (List (List Char)))
    (m : Option ReadMap) : Decision :=
  if ev.1 != "pull_request".toList || ev.2.1 != "dev".toList || ev.2.2 != "false".toList then
    Decision.runJob
  else
    match changed with
    | some [] => Decision.runJob
    | none => Decision.runJob
    | some (q :: qs) =>
      match m with
      | none => Decision.runJob
      | some mp =>
        if (q :: qs).any (fun p => reads (some mp) job p) then Decision.runJob
        else Decision.skipJob

/-- `admit`: each gated job whose result does not pass is refused, in the order of `needs`; a
gated result passes when it is `success`, or `skipped` while the path-gate job succeeded and its
output for the job is `skip`. Every other job is passed on with its result, in order. -/
def admitGate : List (List Char × List Char) → Bool → List (List Char × List Char) →
    List (List Char) × List (List Char × List Char)
  | [], _, _ => ([], [])
  | (job, result) :: needs, gateOk, gateOut =>
    if !(gated.contains job) then
      ((admitGate needs gateOk gateOut).1, (job, result) :: (admitGate needs gateOk gateOut).2)
    else if result == "success".toList then admitGate needs gateOk gateOut
    else if result == "skipped".toList && gateOk && gateOut.lookup job == some "skip".toList then
      admitGate needs gateOk gateOut
    else (job :: (admitGate needs gateOk gateOut).1, (admitGate needs gateOk gateOut).2)

/-! ## The base -/

/-- The base's decision: every job runs on every change (`ci.yml:14-19` has no paths filter, and
`ci.yml:1415` needs every job on every event). -/
def alwaysRun (_ev : Event) (_job : List Char) (_changed : Option (List (List Char)))
    (_m : Option ReadMap) : Decision :=
  Decision.runJob

/-- The mutation legs whose own skip the base's aggregate counts as one admission. -/
def mutationLegs : List (List Char) :=
  ["mutation-rust".toList, "mutation-rows".toList, "mutation-python-release".toList]

/-- The base's loop over every result (`ci.yml:1470-1476`): a skipped result passes while
admissions remain, using one, a success passes, and any other result is refused. -/
def baseLoop : Nat → List (List Char × List Char) → List (List Char)
  | _, [] => []
  | admitted, (job, result) :: needs =>
    if result == "skipped".toList && admitted != 0 then baseLoop (admitted - 1) needs
    else if result == "success".toList then baseLoop admitted needs
    else job :: baseLoop admitted needs

/-- The base's admission in the shape of `admitGate` (`ci.yml:1462-1476`): one admission for each
mutation leg whose own result is `skipped`, its loop over every result, and the gated jobs that
loop refuses; every other job goes on with its result. -/
def baseAdmit (needs : List (List Char × List Char)) (_gateOk : Bool)
    (_gateOut : List (List Char × List Char)) : List (List Char) × List (List Char × List Char) :=
  ((baseLoop (needs.filter (fun p => mutationLegs.contains p.1 && p.2 == "skipped".toList)).length
      needs).filter (fun job => gated.contains job),
    needs.filter (fun p => !(gated.contains p.1)))

/-! ## The claims -/

/-- The entry `e` reads the path `q`, as the schema states it: `*` reads every path, an exact entry
its own path, and an entry ending in `/` every path it begins. -/
def EntryReads (e q : List Char) : Prop :=
  e = ['*'] ∨ e = q ∨ (e.getLast? = some '/' ∧ e <+: q)

/-- The map's meaning, from its schema alone and through no port: a refused map has no member, a
job with no entry reads every path, and otherwise a path is read when an entry of `always` or of
the job's reads reads it. -/
def InMap (m : Option ReadMap) (job q : List Char) : Prop :=
  match m with
  | none => False
  | some mp =>
    match mp.jobs.lookup job with
    | none => True
    | some r => ∃ e ∈ mp.always ++ r, EntryReads e q

/-- A pull request into dev, as the event's three strings name it. -/
def prDev : Event :=
  ("pull_request".toList, "dev".toList, "false".toList)

/-- The claim over a decision and a reader: the reader answers exactly the map's meaning, and the
decision skips exactly a pull request into dev whose change list is readable and non-empty, on a
parsed map, when the reader reads no changed path. -/
def ExactGate
    (f : Event → List Char → Option (List (List Char)) → Option ReadMap → Decision)
    (g : Option ReadMap → List Char → List Char → Bool) : Prop :=
  (∀ m job q, g m job q = true ↔ InMap m job q) ∧
    ∀ ev job cs m, f ev job cs m = Decision.skipJob ↔
      (ev = prDev ∧ ∃ l mp, cs = some l ∧ l ≠ [] ∧ m = some mp ∧ ∀ q ∈ l, g m job q = false)

/-- A gated job's result passes: it succeeded, or it was skipped while the path-gate job succeeded
and the gate's output for the job is `skip`. -/
def Passes (gateOk : Bool) (gateOut : List (List Char × List Char)) (job result : List Char) :
    Prop :=
  result = "success".toList ∨
    (result = "skipped".toList ∧ gateOk = true ∧ gateOut.lookup job = some "skip".toList)

instance (gateOk : Bool) (gateOut : List (List Char × List Char)) (job result : List Char) :
    Decidable (Passes gateOk gateOut job result) :=
  inferInstanceAs (Decidable (result = "success".toList ∨
    (result = "skipped".toList ∧ gateOk = true ∧ gateOut.lookup job = some "skip".toList)))

/-- The claim over an admission: it refuses exactly the gated jobs whose results do not pass, in
the order of `needs`, and passes on every job the gate does not decide for, with its result, in
order. -/
def AdmitExact
    (f : List (List Char × List Char) → Bool → List (List Char × List Char) →
      List (List Char) × List (List Char × List Char)) : Prop :=
  ∀ needs gateOk gateOut,
    (f needs gateOk gateOut).1 =
        (needs.filter (fun p => decide (p.1 ∈ gated ∧ ¬ Passes gateOk gateOut p.1 p.2))).map
          Prod.fst ∧
      (f needs gateOk gateOut).2 = needs.filter (fun p => decide (p.1 ∉ gated))

instance (e q : List Char) : Decidable (EntryReads e q) :=
  inferInstanceAs (Decidable (e = ['*'] ∨ e = q ∨ (e.getLast? = some '/' ∧ e <+: q)))

instance instDecidableInMap : (m : Option ReadMap) → (job q : List Char) →
    Decidable (InMap m job q)
  | none, _, _ => isFalse (by simp [InMap])
  | some mp, job, q =>
    match h : mp.jobs.lookup job with
    | none => isTrue (by simp only [InMap, h])
    | some r =>
      if hr : ∃ e ∈ mp.always ++ r, EntryReads e q then isTrue (by simp only [InMap, h]; exact hr)
      else isFalse (by simp only [InMap, h]; exact hr)

/-! ## The port, read by the map's meaning -/

theorem prefix_of_iff (d q : List Char) :
    prefix_of d q = true ↔ d.getLast? = some '/' ∧ d <+: q := by
  simp only [prefix_of, Bool.and_eq_true, beq_iff_eq, List.isPrefixOf_iff_prefix]

theorem reads_entry_iff (e q : List Char) : reads_entry e q = true ↔ EntryReads e q := by
  simp only [reads_entry, EntryReads, Bool.or_eq_true, beq_iff_eq, prefix_of_iff, or_assoc]

theorem reads_iff (m : Option ReadMap) (job q : List Char) :
    reads m job q = true ↔ InMap m job q := by
  cases m with
  | none => simp [reads, InMap]
  | some mp =>
    simp only [reads, InMap]
    cases mp.jobs.lookup job with
    | none => simp
    | some r => simp only [List.any_eq_true, reads_entry_iff]

theorem decideGate_skip (ev : Event) (job : List Char) (cs : Option (List (List Char)))
    (m : Option ReadMap) :
    decideGate ev job cs m = Decision.skipJob ↔
      (ev = prDev ∧ ∃ l mp, cs = some l ∧ l ≠ [] ∧ m = some mp ∧ ∀ q ∈ l, reads m job q = false) := by
  obtain ⟨kind, baseRef, release⟩ := ev
  by_cases hev : kind = "pull_request".toList ∧ baseRef = "dev".toList ∧ release = "false".toList
  · obtain ⟨rfl, rfl, rfl⟩ := hev
    cases cs with
    | none => simp [decideGate, prDev]
    | some l =>
      cases l with
      | nil => simp [decideGate, prDev]
      | cons q qs =>
        cases m with
        | none => simp [decideGate, prDev]
        | some mp => simp [decideGate, prDev]
  · have hne : (kind, baseRef, release) ≠ prDev := by
      intro h
      simp only [prDev, Prod.mk.injEq] at h
      exact hev h
    have hc : (kind != "pull_request".toList || baseRef != "dev".toList ||
        release != "false".toList) = true := by
      simp only [Bool.or_eq_true, bne_iff_ne, ne_eq]
      by_cases h1 : kind = "pull_request".toList
      · by_cases h2 : baseRef = "dev".toList
        · exact Or.inr (fun h3 => hev ⟨h1, h2, h3⟩)
        · exact Or.inl (Or.inr h2)
      · exact Or.inl (Or.inl h1)
    have hr : decideGate (kind, baseRef, release) job cs m = Decision.runJob := by
      simp only [decideGate]
      rw [ite_eq_left hc]
    constructor
    · intro h
      rw [hr] at h
      exact Decision.noConfusion h
    · intro h
      exact absurd h.1 hne

/-! ## The theorems -/

/-- The gate skips exactly the changes no read reaches: `reads` answers the map's meaning, and the
decision skips exactly a pull request into dev, with a readable, non-empty change list and a parsed
map, that changes no path the job reads. -/
theorem the_gate_skips_exactly_the_changes_no_read_reaches : ExactGate decideGate reads :=
  ⟨reads_iff, decideGate_skip⟩

/-- A skipped job passes only on the gate's own skip: the admission refuses exactly the gated jobs
whose results do not pass, in order, and passes every other job on. -/
theorem a_skipped_job_passes_only_on_the_gates_own_skip : AdmitExact admitGate := by
  intro needs gateOk gateOut
  induction needs with
  | nil => simp [admitGate]
  | cons p needs ih =>
    obtain ⟨job, result⟩ := p
    obtain ⟨ih1, ih2⟩ := ih
    simp only [admitGate]
    cases hg : gated.contains job with
    | false =>
      have hm : job ∉ gated := fun h => by
        have := List.contains_iff_mem.mpr h
        rw [hg] at this
        exact Bool.noConfusion this
      simp [hm, ih1, ih2]
    | true =>
      have hm : job ∈ gated := List.contains_iff_mem.mp hg
      cases hs : (result == "success".toList) with
      | true =>
        have hp : Passes gateOk gateOut job result := Or.inl (beq_iff_eq.mp hs)
        simp [hm, hp, ih1, ih2]
      | false =>
        cases hk : (result == "skipped".toList && gateOk &&
            gateOut.lookup job == some "skip".toList) with
        | true =>
          have hp : Passes gateOk gateOut job result := by
            simp only [Bool.and_eq_true, beq_iff_eq] at hk
            exact Or.inr ⟨hk.1.1, hk.1.2, hk.2⟩
          simp [hm, hp, ih1, ih2]
        | false =>
          have hp : ¬ Passes gateOk gateOut job result := by
            rintro (h | ⟨h1, h2, h3⟩)
            · simp [h] at hs
            · simp [h1, h2, h3] at hk
          simp [hm, hp, ih1, ih2]

/-! ## The witnesses -/

/-- At a pull request into dev that changes `docs/a.md`, on a map whose one job reads `crates/`,
the base runs the job, though no read reaches the change: the claim is false of it. -/
theorem the_base_runs_a_job_on_a_change_it_never_reads : ¬ ExactGate alwaysRun reads := by
  intro h
  have hs := (h.2 prDev "rust".toList (some ["docs/a.md".toList])
    (some ⟨[], [("rust".toList, ["crates/".toList])]⟩)).2
    ⟨rfl, ["docs/a.md".toList], ⟨[], [("rust".toList, ["crates/".toList])]⟩, rfl, by simp, rfl,
      by decide +kernel⟩
  simp [alwaysRun] at hs

/-- A skipped `rust` that the gate skipped, with the gate's success and no mutation leg skipped,
is refused by the base's count: the claim is false of it. -/
theorem the_base_refuses_a_job_the_gate_skipped : ¬ AdmitExact baseAdmit := by
  intro h
  have h1 := (h [("rust".toList, "skipped".toList)] true [("rust".toList, "skip".toList)]).1
  revert h1
  decide +kernel

/-! ## Every hypothesis set is inhabited -/

/-- A `*` entry reads a path no other entry names. -/
example : InMap (some ⟨[], [("census".toList, ["*".toList])]⟩) "census".toList
    "docs/a.md".toList := by
  decide +kernel

/-- An exact entry reads its own path. -/
example : InMap (some ⟨[], [("rust".toList, ["Cargo.toml".toList])]⟩) "rust".toList
    "Cargo.toml".toList := by
  decide +kernel

/-- A directory prefix reads a path under it. -/
example : InMap (some ⟨[], [("rust".toList, ["crates/".toList])]⟩) "rust".toList
    "crates/a/src/lib.rs".toList := by
  decide +kernel

/-- An `always` entry reads a path the job's own reads do not name. -/
example : InMap (some ⟨["scripts/check.sh".toList], [("rust".toList, ["crates/".toList])]⟩)
    "rust".toList "scripts/check.sh".toList := by
  decide +kernel

/-- A job with no entry reads every path. -/
example : InMap (some ⟨[], [("rust".toList, ["crates/".toList])]⟩) "web".toList
    "docs/a.md".toList := by
  decide +kernel

/-- The skip side of `ExactGate` is inhabited: a pull request into dev, a readable non-empty
change list, a parsed map, and no changed path read. -/
example : ∃ (ev : Event) (job : List Char) (cs : Option (List (List Char))) (m : Option ReadMap),
    ev = prDev ∧ ∃ l mp, cs = some l ∧ l ≠ [] ∧ m = some mp ∧ ∀ q ∈ l, reads m job q = false :=
  ⟨prDev, "rust".toList, some ["docs/a.md".toList],
    some ⟨[], [("rust".toList, ["crates/".toList])]⟩, rfl, ["docs/a.md".toList],
    ⟨[], [("rust".toList, ["crates/".toList])]⟩, rfl, by simp, rfl, by decide +kernel⟩

/-- A gated result passes by success, whatever the gate gave. -/
example : Passes false [] "rust".toList "success".toList := by
  decide +kernel

/-- A gated result passes by the gate's own skip. -/
example : Passes true [("rust".toList, "skip".toList)] "rust".toList "skipped".toList := by
  decide +kernel

/-- A gated job is refused: skipped while the path-gate job failed. -/
example : "rust".toList ∈ gated ∧
    ¬ Passes false [("rust".toList, "skip".toList)] "rust".toList "skipped".toList := by
  decide +kernel

/-- A job the gate does not decide for is passed on. -/
example : "mutation-rust".toList ∉ gated := by
  decide +kernel

end Formal.PathGate
