import Formal.ReleaseGates

/-!
# Formal.ReleaseGatesVectors

The release gate's half of the vector writer (SPEC-502). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean ReleaseGates` to `run`, which prints one header line naming the
first covered item (`verdict`) and the digest the entry recorded for it, then one line per input:
the record as `release_gates.py` reads it (`null` when none is on file; a gate that is `none` is
left out) and the refusals the verdict that ships gives, at `Int` with its `<`. The formal checker
byte-compares the output with `formal/vectors/release-gates.jsonl`, and `--write-vectors` writes
it; the file is never edited by hand. The release lane's test reads every vector's record through
the script and compares its refusals.

The inputs: no record; a passing record; each gate missing alone and failing alone; an equal pair
and a negative count; every gate missing; every gate failing; and one mixed record. This module
carries no header line, so the registry lists it as support.
-/

namespace Formal.ReleaseGatesVectors

open Formal.ReleaseGates

/-- A JSON string. The writer quotes only paths, item names, hex digests and gate names, which
carry no character JSON escapes. -/
def jsonString (s : String) : String :=
  "\"" ++ s ++ "\""

/-- The first `covers` line of a module: its path, item and recorded digest. -/
def coverOf (module : String) : Option (String × String × String) :=
  (module.splitOn "\n").findSome? fun line =>
    match (line.splitOn " ").filter (· ≠ "") with
    | [_, _, "covers", path, anchor, digest] =>
      if anchor.startsWith "anchor=" && digest.startsWith "digest=" then
        some (path, (anchor.drop 7).toString, (digest.drop 7).toString)
      else none
    | _ => none

/-- The header line: what the vectors were written from, and how many follow. -/
def headerLine (entry path anchor digest : String) (count : Nat) : String :=
  "{\"schema\":\"phx.formal.vectors.v1\",\"entry\":" ++ jsonString entry ++
    ",\"covers\":" ++ jsonString path ++ ",\"anchor\":" ++ jsonString anchor ++
    ",\"digest\":" ++ jsonString digest ++ ",\"vectors\":" ++ toString count ++ "}"

/-- Each gate by the name `release_gates.py` gives it. -/
def gateName : Gate → String
  | Gate.record => "record"
  | Gate.model => "model"
  | Gate.logloss => "dev.logloss"
  | Gate.rmseBins => "dev.rmse_bins"
  | Gate.sealedHolds => "sealed_holds"
  | Gate.worseCells => "worse_cells"
  | Gate.simulatorWin => "simulator_win"
  | Gate.permissionOnFile => "permission_on_file"
  | Gate.schedulerGateDone => "scheduler_gate_done"

/-- Each reason by the script's word. -/
def whyName : Why → String
  | Why.missing => "missing"
  | Why.failed => "failed"

/-- The `<` the script applies to two integers. -/
def lt (a b : Int) : Bool :=
  decide (a < b)

/-- A sha256 the script reads as on file. -/
def weights : List Char :=
  "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".toList

/-- A record on which every gate holds. -/
def passing : Gates Int :=
  { model := some weights, logloss := some (30, 32), rmseBins := some (50, 61),
    sealedHolds := some true, worseCells := some 0, simulatorWin := some true,
    permissionOnFile := some true, schedulerGateDone := some true }

/-- A record that holds no gate: each is on file, and every one but the model fails. -/
def failing : Gates Int :=
  { model := some weights, logloss := some (32, 30), rmseBins := some (61, 50),
    sealedHolds := some false, worseCells := some 1, simulatorWin := some false,
    permissionOnFile := some false, schedulerGateDone := some false }

/-- A record with every gate missing. -/
def bare : Gates Int :=
  { model := none, logloss := none, rmseBins := none, sealedHolds := none, worseCells := none,
    simulatorWin := none, permissionOnFile := none, schedulerGateDone := none }

/-- A record with gates missing, failing and holding at once. -/
def mixed : Gates Int :=
  { model := none, logloss := some (32, 30), rmseBins := some (50, 61),
    sealedHolds := none, worseCells := some 0, simulatorWin := some false,
    permissionOnFile := some true, schedulerGateDone := none }

/-- Each input record, `none` when no record is on file. -/
def inputs : List (Option (Gates Int)) :=
  [ none,
    some passing,
    some { passing with model := none },
    some { passing with logloss := none },
    some { passing with rmseBins := none },
    some { passing with sealedHolds := none },
    some { passing with worseCells := none },
    some { passing with simulatorWin := none },
    some { passing with permissionOnFile := none },
    some { passing with schedulerGateDone := none },
    some { passing with logloss := some (32, 30) },
    some { passing with rmseBins := some (61, 50) },
    some { passing with sealedHolds := some false },
    some { passing with worseCells := some 1 },
    some { passing with simulatorWin := some false },
    some { passing with permissionOnFile := some false },
    some { passing with schedulerGateDone := some false },
    some { passing with logloss := some (30, 30) },
    some { passing with worseCells := some (-1) },
    some bare,
    some failing,
    some mixed ]

/-- A JSON boolean. -/
def boolJson (b : Bool) : String :=
  if b then "true" else "false"

/-- A pair as a JSON array of two integers. -/
def pairJson (p : Int × Int) : String :=
  "[" ++ toString p.1 ++ "," ++ toString p.2 ++ "]"

/-- One `"key":value` member when the value is on file, else nothing. -/
def member {β : Type} (key : String) (render : β → String) : Option β → List String
  | none => []
  | some v => [jsonString key ++ ":" ++ render v]

/-- A JSON object of its members. -/
def objectJson (members : List String) : String :=
  "{" ++ ",".intercalate members ++ "}"

/-- The record as the script reads it: a gate that is `none` is left out, and `dev` is left out
when both of its pairs are. -/
def recordJson : Option (Gates Int) → String
  | none => "null"
  | some r =>
    let dev := member "logloss" pairJson r.logloss ++ member "rmse_bins" pairJson r.rmseBins
    objectJson
      (member "model" (fun m => jsonString (String.ofList m)) r.model ++
        (if dev.isEmpty then [] else [jsonString "dev" ++ ":" ++ objectJson dev]) ++
        member "sealed_holds" boolJson r.sealedHolds ++
        member "worse_cells" (fun n => toString n) r.worseCells ++
        member "simulator_win" boolJson r.simulatorWin ++
        member "permission_on_file"
          (fun b => "{\"value\":" ++ boolJson b ++ ",\"date\":null}") r.permissionOnFile ++
        member "scheduler_gate_done" boolJson r.schedulerGateDone)

/-- The refusals as a JSON array of `[gate, why]` pairs. -/
def refusalsJson (rs : List (Gate × Why)) : String :=
  "[" ++ ",".intercalate (rs.map fun (g, w) =>
    "[" ++ jsonString (gateName g) ++ "," ++ jsonString (whyName w) ++ "]") ++ "]"

/-- One vector line: the record and the refusals the verdict that ships gives. -/
def vectorLine (rec : Option (Gates Int)) : String :=
  "{\"record\":" ++ recordJson rec ++ ",\"refused\":" ++ refusalsJson (verdict lt rec) ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/ReleaseGates.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/ReleaseGates.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine "ReleaseGates" path anchor digest inputs.length)
    for rec in inputs do
      IO.println (vectorLine rec)
    return 0

end Formal.ReleaseGatesVectors
