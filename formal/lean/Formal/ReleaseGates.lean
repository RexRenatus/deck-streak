-- @phx covers scripts/release_gates.py anchor=verdict digest=sha256:beb50cf29dffe279faec7a20f7472d734b2fe88098465989acd8d7e61d27e280
-- @phx covers scripts/release_gates.py anchor=refused digest=sha256:3254cd2507aa1cf1b5fc2882ff919f69a38f81033ae442649ede7baa65e8633e
-- @phx covers scripts/release_gates.py anchor=present_refusal digest=sha256:9f405cce3587513a31626c9eaf038b42730d60b305974d0bb84fe6627473017f
-- @phx covers scripts/release_gates.py anchor=pair_refusal digest=sha256:e9868ee51ebe120cd945caa1adb77444ab33cee34c390405da5d72a3c2bbb98b
-- @phx covers scripts/release_gates.py anchor=flag_refusal digest=sha256:541b5c7772415adf0380a9cb126a005ef01f2dfb22cf3acdc05b73a55253fd6c
-- @phx covers scripts/release_gates.py anchor=count_refusal digest=sha256:37658e5099acd8cd944ae118d1a7e8c1a4d65e0f82586e59ec5b8c6ab9da60e7
-- @phx covers scripts/release_gates.py anchor=read_gates digest=sha256:fdaddc34c59e817cad56981d79e96a83dd8d7bcfc10bdd4dcf054f29658f2ed7
-- @phx vectors formal/vectors/release-gates.jsonl
-- @phx cites #822
-- @phx theorem a_release_is_published_only_when_every_gate_holds ramp=report
-- @phx witness the_base_publishes_a_tag_with_no_record_on_file kills=a_release_is_published_only_when_every_gate_holds
-- @phx theorem each_refusal_names_its_gate_and_why ramp=report
-- @phx witness the_base_names_no_gate_for_a_tag_with_no_record kills=each_refusal_names_its_gate_and_why

/-!
# Formal.ReleaseGates

The release gate's decision (SPEC-502, ADR-520), ported from `scripts/release_gates.py`.

**What is ported.** `verdict`, `refused` and the four per-gate helpers, branch for branch:
`present_refusal` (`model`, which holds whenever it is on file), `pair_refusal` (a [the scheduler,
FSRS-7] pair, which holds when the scheduler's figure is below FSRS-7's), `flag_refusal` (a boolean
gate, which holds when true) and `count_refusal` (the worse cells, which hold at zero). A gate the
script names by its string is a constructor of `Gate` here, and the vectors print each by the
script's name. The pairs' figures are of any type `α` with the `<` the script applies, passed as
`lt`, so each claim holds for every instance of that operation; the vectors run it at `Int`.

**What is not ported.** `read_gates` and `read_record`, which turn the record's JSON into each
gate's value or `none` (absent, or of another type than its schema's), and `none` for the whole
record when it is not on file. Each is an input here; `read_gates` is covered, so a move of its
span re-reads this port, and the vectors carry their records through it in the release lane's test.

**The claims.** `Publishes` is the release's design stated over the record: the verdict is empty
exactly when the record is on file and every gate holds (`Holds`). `Names` states each refusal:
each gate is named at most once, and a gate is named exactly when it refuses, with only `record`
named when no record is on file, as `missing` exactly when it is not on file in its type
(`OnFile`).

**The witnesses.** At the base `d3b09d5e` the release job runs its publish step
(`release.yml:161-164`) for every tag, with no record read: its verdict is `publishesAlways`, the
empty list for every record. With no record on file it publishes, which `Publishes` refuses, and it
names no gate, which `Names` refuses.
-/

namespace Formal.ReleaseGates

/-- The record and each gate, as `release_gates.py` names them in its refusals. -/
inductive Gate where
  | record
  | model
  | logloss
  | rmseBins
  | sealedHolds
  | worseCells
  | simulatorWin
  | permissionOnFile
  | schedulerGateDone
  deriving DecidableEq, Repr

/-- Why a gate refuses: it is not on file in its type, or it is on file and does not hold. -/
inductive Why where
  | missing
  | failed
  deriving DecidableEq, Repr

/-- Each gate's value as `read_gates` gives it, `none` when the record lacks it or holds it in
another type than its schema's. -/
structure Gates (α : Type) where
  model : Option (List Char)
  logloss : Option (α × α)
  rmseBins : Option (α × α)
  sealedHolds : Option Bool
  worseCells : Option Int
  simulatorWin : Option Bool
  permissionOnFile : Option Bool
  schedulerGateDone : Option Bool

/-! ## The port -/

/-- `present_refusal`: a gate that holds whenever it is on file. -/
def presentRefusal (g : Gate) : Option (List Char) → List (Gate × Why)
  | none => [(g, Why.missing)]
  | some _ => []

/-- `pair_refusal`: a [the scheduler, FSRS-7] pair, which holds when `lt` puts the scheduler's
figure below FSRS-7's. -/
def pairRefusal {α : Type} (lt : α → α → Bool) (g : Gate) : Option (α × α) → List (Gate × Why)
  | none => [(g, Why.missing)]
  | some (a, b) => if lt a b then [] else [(g, Why.failed)]

/-- `flag_refusal`: a boolean gate, which holds when it is true. -/
def flagRefusal (g : Gate) : Option Bool → List (Gate × Why)
  | none => [(g, Why.missing)]
  | some b => if b then [] else [(g, Why.failed)]

/-- `count_refusal`: the worse cells, which hold at zero. -/
def countRefusal (g : Gate) : Option Int → List (Gate × Why)
  | none => [(g, Why.missing)]
  | some n => if n = 0 then [] else [(g, Why.failed)]

/-- `refused`: every refusing gate of a record, in the record's order, each with why. -/
def refused {α : Type} (lt : α → α → Bool) (r : Gates α) : List (Gate × Why) :=
  presentRefusal Gate.model r.model ++ pairRefusal lt Gate.logloss r.logloss ++
    pairRefusal lt Gate.rmseBins r.rmseBins ++ flagRefusal Gate.sealedHolds r.sealedHolds ++
    countRefusal Gate.worseCells r.worseCells ++ flagRefusal Gate.simulatorWin r.simulatorWin ++
    flagRefusal Gate.permissionOnFile r.permissionOnFile ++
    flagRefusal Gate.schedulerGateDone r.schedulerGateDone

/-- `verdict`: the record itself when none is on file, else each refusing gate. An empty list
publishes. -/
def verdict {α : Type} (lt : α → α → Bool) : Option (Gates α) → List (Gate × Why)
  | none => [(Gate.record, Why.missing)]
  | some r => refused lt r

/-- The base's verdict: the publish step runs for every tag, so nothing is ever refused. -/
def publishesAlways {α : Type} (_lt : α → α → Bool) (_rec : Option (Gates α)) :
    List (Gate × Why) :=
  []

/-! ## The claims -/

/-- Gate `g` holds of the record on file, as the release's design states each gate. No gate holds
when no record is on file. -/
def Holds {α : Type} (lt : α → α → Bool) : Option (Gates α) → Gate → Prop
  | none, _ => False
  | some _, Gate.record => True
  | some r, Gate.model => r.model ≠ none
  | some r, Gate.logloss => ∃ a b, r.logloss = some (a, b) ∧ lt a b = true
  | some r, Gate.rmseBins => ∃ a b, r.rmseBins = some (a, b) ∧ lt a b = true
  | some r, Gate.sealedHolds => r.sealedHolds = some true
  | some r, Gate.worseCells => r.worseCells = some 0
  | some r, Gate.simulatorWin => r.simulatorWin = some true
  | some r, Gate.permissionOnFile => r.permissionOnFile = some true
  | some r, Gate.schedulerGateDone => r.schedulerGateDone = some true

/-- Gate `g` is on file in its type, whether or not it holds. -/
def OnFile {α : Type} : Option (Gates α) → Gate → Prop
  | none, _ => False
  | some _, Gate.record => True
  | some r, Gate.model => r.model ≠ none
  | some r, Gate.logloss => r.logloss ≠ none
  | some r, Gate.rmseBins => r.rmseBins ≠ none
  | some r, Gate.sealedHolds => r.sealedHolds ≠ none
  | some r, Gate.worseCells => r.worseCells ≠ none
  | some r, Gate.simulatorWin => r.simulatorWin ≠ none
  | some r, Gate.permissionOnFile => r.permissionOnFile ≠ none
  | some r, Gate.schedulerGateDone => r.schedulerGateDone ≠ none

/-- The claim over a verdict: it is empty, so the release is published, exactly when the record is
on file and every gate holds. -/
def Publishes {α : Type} (lt : α → α → Bool)
    (f : (α → α → Bool) → Option (Gates α) → List (Gate × Why)) : Prop :=
  ∀ rec, f lt rec = [] ↔ ∀ g, Holds lt rec g

/-- The claim over a verdict: each gate is named at most once, and a gate is named exactly when it
refuses (only the record when none is on file), as `missing` exactly when it is not on file. -/
def Names {α : Type} (lt : α → α → Bool)
    (f : (α → α → Bool) → Option (Gates α) → List (Gate × Why)) : Prop :=
  ∀ rec, ((f lt rec).map Prod.fst).Nodup ∧
    ∀ g w, (g, w) ∈ f lt rec ↔
      ¬ Holds lt rec g ∧ (rec = none → g = Gate.record) ∧ (w = Why.missing ↔ ¬ OnFile rec g)

/-! ## Each helper, read by its result -/

theorem presentRefusal_eq_nil (g : Gate) (v : Option (List Char)) :
    presentRefusal g v = [] ↔ v ≠ none := by
  cases v <;> simp [presentRefusal]

theorem pairRefusal_eq_nil {α : Type} (lt : α → α → Bool) (g : Gate) (v : Option (α × α)) :
    pairRefusal lt g v = [] ↔ ∃ a b, v = some (a, b) ∧ lt a b = true := by
  cases v with
  | none => simp [pairRefusal]
  | some p =>
    obtain ⟨a, b⟩ := p
    cases h : lt a b
    · simp [pairRefusal, h]
    · simp [pairRefusal, h]
      exact ⟨a, b, ⟨rfl, rfl⟩, h⟩

theorem flagRefusal_eq_nil (g : Gate) (v : Option Bool) :
    flagRefusal g v = [] ↔ v = some true := by
  cases v with
  | none => simp [flagRefusal]
  | some b => cases b <;> simp [flagRefusal]

theorem countRefusal_eq_nil (g : Gate) (v : Option Int) :
    countRefusal g v = [] ↔ v = some 0 := by
  cases v with
  | none => simp [countRefusal]
  | some n => by_cases h : n = 0 <;> simp [countRefusal, h]

theorem mem_presentRefusal (g g' : Gate) (w : Why) (v : Option (List Char)) :
    (g', w) ∈ presentRefusal g v ↔ g' = g ∧ v = none ∧ w = Why.missing := by
  cases v <;> simp [presentRefusal, and_comm]

theorem mem_pairRefusal {α : Type} (lt : α → α → Bool) (g g' : Gate) (w : Why)
    (v : Option (α × α)) :
    (g', w) ∈ pairRefusal lt g v ↔
      g' = g ∧ ¬ (∃ a b, v = some (a, b) ∧ lt a b = true) ∧ (w = Why.missing ↔ v = none) := by
  cases v with
  | none => simp [pairRefusal]
  | some p =>
    obtain ⟨a, b⟩ := p
    cases h : lt a b <;> cases w <;> simp [pairRefusal, h]

theorem mem_flagRefusal (g g' : Gate) (w : Why) (v : Option Bool) :
    (g', w) ∈ flagRefusal g v ↔
      g' = g ∧ v ≠ some true ∧ (w = Why.missing ↔ v = none) := by
  cases v with
  | none => cases w <;> simp [flagRefusal]
  | some b => cases b <;> cases w <;> simp [flagRefusal]

theorem mem_countRefusal (g g' : Gate) (w : Why) (v : Option Int) :
    (g', w) ∈ countRefusal g v ↔
      g' = g ∧ v ≠ some 0 ∧ (w = Why.missing ↔ v = none) := by
  cases v with
  | none => cases w <;> simp [countRefusal]
  | some n => by_cases h : n = 0 <;> cases w <;> simp [countRefusal, h]

/-- The gates `refused` names, in the record's order. -/
def order : List Gate :=
  [Gate.model, Gate.logloss, Gate.rmseBins, Gate.sealedHolds, Gate.worseCells,
    Gate.simulatorWin, Gate.permissionOnFile, Gate.schedulerGateDone]

theorem presentRefusal_names (g : Gate) (v : Option (List Char)) :
    List.Sublist ((presentRefusal g v).map Prod.fst) [g] := by
  cases v <;> simp [presentRefusal]

theorem pairRefusal_names {α : Type} (lt : α → α → Bool) (g : Gate) (v : Option (α × α)) :
    List.Sublist ((pairRefusal lt g v).map Prod.fst) [g] := by
  cases v with
  | none => simp [pairRefusal]
  | some p =>
    obtain ⟨a, b⟩ := p
    cases h : lt a b <;> simp [pairRefusal, h]

theorem flagRefusal_names (g : Gate) (v : Option Bool) :
    List.Sublist ((flagRefusal g v).map Prod.fst) [g] := by
  cases v with
  | none => simp [flagRefusal]
  | some b => cases b <;> simp [flagRefusal]

theorem countRefusal_names (g : Gate) (v : Option Int) :
    List.Sublist ((countRefusal g v).map Prod.fst) [g] := by
  cases v with
  | none => simp [countRefusal]
  | some n => by_cases h : n = 0 <;> simp [countRefusal, h]

theorem refused_names {α : Type} (lt : α → α → Bool) (r : Gates α) :
    List.Sublist ((refused lt r).map Prod.fst) order := by
  simp only [refused, List.map_append, order]
  exact (((((((presentRefusal_names _ _).append (pairRefusal_names _ _ _)).append
    (pairRefusal_names _ _ _)).append (flagRefusal_names _ _)).append
    (countRefusal_names _ _)).append (flagRefusal_names _ _)).append
    (flagRefusal_names _ _)).append (flagRefusal_names _ _)

theorem order_nodup : order.Nodup := by decide

/-! ## The theorems -/

/-- The verdict that ships is empty exactly when the record is on file and every gate holds. -/
theorem a_release_is_published_only_when_every_gate_holds {α : Type} (lt : α → α → Bool) :
    Publishes lt verdict := by
  intro rec
  cases rec with
  | none =>
    constructor
    · intro h
      simp [verdict] at h
    · intro h
      exact absurd (h Gate.record) (by simp [Holds])
  | some r =>
    simp only [verdict, refused, List.append_eq_nil_iff, presentRefusal_eq_nil,
      pairRefusal_eq_nil, flagRefusal_eq_nil, countRefusal_eq_nil]
    constructor
    · rintro ⟨⟨⟨⟨⟨⟨⟨hm, hl⟩, hr⟩, hs⟩, hw⟩, hsim⟩, hp⟩, hd⟩ g
      cases g <;> simp_all [Holds]
    · intro h
      exact ⟨⟨⟨⟨⟨⟨⟨h Gate.model, h Gate.logloss⟩, h Gate.rmseBins⟩, h Gate.sealedHolds⟩,
        h Gate.worseCells⟩, h Gate.simulatorWin⟩, h Gate.permissionOnFile⟩,
        h Gate.schedulerGateDone⟩

/-- The verdict that ships names each refusing gate once, with why, and no other. -/
theorem each_refusal_names_its_gate_and_why {α : Type} (lt : α → α → Bool) :
    Names lt verdict := by
  intro rec
  cases rec with
  | none =>
    refine ⟨by simp [verdict], ?_⟩
    intro g w
    cases g <;> cases w <;> simp [verdict, Holds, OnFile]
  | some r =>
    refine ⟨order_nodup.sublist (refused_names lt r), ?_⟩
    intro g w
    simp only [verdict, refused, List.mem_append, mem_presentRefusal, mem_pairRefusal,
      mem_flagRefusal, mem_countRefusal]
    cases g <;> cases w <;> simp [Holds, OnFile]

/-! ## The witnesses: the base publishes every tag -/

/-- With no record on file, the base's verdict is empty, so it publishes a release no gate
admits: the claim is false of it. -/
theorem the_base_publishes_a_tag_with_no_record_on_file :
    ¬ ∀ {α : Type} (lt : α → α → Bool), Publishes lt publishesAlways := by
  intro h
  have hp := (h (α := Unit) (fun _ _ => false) none).1 rfl
  exact absurd (hp Gate.record) (by simp [Holds])

/-- With no record on file, the base names no gate, though the record refuses as missing: the
claim is false of it. -/
theorem the_base_names_no_gate_for_a_tag_with_no_record :
    ¬ ∀ {α : Type} (lt : α → α → Bool), Names lt publishesAlways := by
  intro h
  have hn := ((h (α := Unit) (fun _ _ => false) none).2 Gate.record Why.missing).2
  exact absurd (hn (by simp [Holds, OnFile])) (by simp [publishesAlways])

end Formal.ReleaseGates
