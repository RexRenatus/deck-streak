import Formal.ReleaseCarry

/-!
# Formal.ReleaseCarryVectors

The release carry's half of the vector writer (#822, SPEC-493). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean ReleaseCarry` to `run`, which prints one header line naming the
first covered item (`union_rule`) and the digest the entry recorded for it, then one line per
input with the union rule that ships (`accepts`) and its answer. The formal checker byte-compares
the output with `formal/vectors/release-carry.jsonl`, and `--write-vectors` writes it; the file is
never edited by hand.

The inputs are listing / re-run / carried multisets of names, chosen to exercise the rule's three
checks: a listing that is the re-run names plus the carried ones with none in both (accepted), a
name both re-run and carried (refused), and a name missing or extra (refused). This module carries
no header line, so the registry lists it as support.
-/

namespace Formal.ReleaseCarryVectors

open Formal.ReleaseCarry

/-- A JSON string. The writer quotes only paths, item names, hex digests and single-character
names, which carry no character JSON escapes. -/
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

/-- Each input: the listing, the re-run legs and the carried names. -/
def inputs : List (List Name × List Name × List Name) :=
  [ ([], [], []),
    ([['a']], [['a']], []),
    ([['a']], [], [['a']]),
    ([['a'], ['a']], [['a']], [['a']]),
    ([['a'], ['b']], [['a']], [['b']]),
    ([['a']], [['b']], []),
    ([['a'], ['b']], [['a'], ['b']], []),
    ([['a']], [], []) ]

/-- A name as a JSON string. -/
def nameJson (n : Name) : String :=
  jsonString (String.ofList n)

/-- A multiset of names as a JSON array. -/
def listJson (l : List Name) : String :=
  "[" ++ ",".intercalate (l.map nameJson) ++ "]"

/-- One vector line: the three multisets and the answer the union rule that ships gives. -/
def vectorLine (inp : List Name × List Name × List Name) : String :=
  let (listed, rerun, carried) := inp
  "{\"listed\":" ++ listJson listed ++ ",\"rerun\":" ++ listJson rerun ++
    ",\"carried\":" ++ listJson carried ++ ",\"accepts\":" ++
    toString (accepts listed rerun carried) ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/ReleaseCarry.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/ReleaseCarry.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine "ReleaseCarry" path anchor digest inputs.length)
    for inp in inputs do
      IO.println (vectorLine inp)
    return 0

end Formal.ReleaseCarryVectors
