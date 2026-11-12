import Formal.RecordClassBinding

/-!
# Formal.RecordClassBindingVectors

The record class binding's half of the vector writer (#822, SPEC-437). `Formal/Vectors.lean`
dispatches `lean --run Formal/Vectors.lean RecordClassBinding` to `run`, which prints one header
line naming the first covered item and the digest the entry recorded for it, then one line per
input with the records the binding that ships judges and each one's verdict. The formal checker
byte-compares the output with `formal/vectors/record-class-binding.jsonl`, and `--write-vectors`
writes it; the file is never edited by hand.

The inputs are both classes, each judged beside a scripts record and an oracle record, each record
binding 0, 1 or 2 mutants, so every input holds a record of the other class. SPEC-437's A17 builds
each record as a fixture whose anchor binds that many mutants of its own file, judges the class
through `judge_python`, and compares the records judged and each verdict with the line's. This
module carries no header line, so the registry lists it as support.
-/

namespace Formal.RecordClassBindingVectors

open Formal.RecordClassBinding

/-- A JSON string. The writer quotes only paths, class names, item names, verdicts and hex
digests, which carry no character JSON escapes. -/
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

/-- The two Python classes. -/
def classes : List String := ["scripts", "oracle"]

/-- The binding counts each record takes. -/
def counts : List Nat := [0, 1, 2]

/-- Every input, in a fixed order: each judged class, then the scripts record's count, then the
oracle record's. -/
def inputs : List (String × List Record) :=
  classes.flatMap fun klass =>
    counts.flatMap fun s =>
      counts.map fun o => (klass, [⟨1, "scripts".toList, s⟩, ⟨2, "oracle".toList, o⟩])

/-- A verdict's name, as the vectors spell it. -/
def verdictName : Verdict → String
  | .valid => "valid"
  | .stale => "stale"
  | .ambiguous => "ambiguous"

/-- One record of an input. -/
def recordLine (r : Record) : String :=
  "{\"index\":" ++ toString r.index ++ ",\"class\":" ++ jsonString (String.ofList r.klass) ++
    ",\"binds\":" ++ toString r.binds ++ "}"

/-- One record the binding judged, with its verdict. -/
def judgedLine (p : Record × Verdict) : String :=
  "{\"index\":" ++ toString p.1.index ++ ",\"verdict\":" ++ jsonString (verdictName p.2) ++ "}"

/-- One vector line: the judged class, its records, and what the binding that ships judged. -/
def vectorLine (input : String × List Record) : String :=
  let (klass, records) := input
  "{\"class\":" ++ jsonString klass ++ ",\"records\":[" ++
    ",".intercalate (records.map recordLine) ++ "],\"judged\":[" ++
    ",".intercalate ((binding klass.toList records).map judgedLine) ++ "]}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/RecordClassBinding.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/RecordClassBinding.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine "RecordClassBinding" path anchor digest inputs.length)
    for input in inputs do
      IO.println (vectorLine input)
    return 0

end Formal.RecordClassBindingVectors
