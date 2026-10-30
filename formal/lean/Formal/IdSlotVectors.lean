import Formal.IdSlot

/-!
# Formal.IdSlotVectors

The id allocator's half of the vector writer (SPEC-509). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean IdSlot` to `run`, which prints one header line naming the covered
item and the digest the entry recorded for it, then one line per input with the port's id, or
`null` where there is none. The inputs are floors and instants at both ends of the 64-bit range
and around an instant of today, each with the server's slot, the first two device slots and the
last. The formal checker byte-compares the output with `formal/vectors/id-slot.jsonl`, and
`--write-vectors` writes it; the file is never edited by hand. This module carries no header line,
so the registry lists it as support.
-/

namespace Formal.IdSlotVectors
open Formal.IdSlot

/-- A JSON string. The writer quotes only paths, item names and hex digests, which carry no
character JSON escapes. -/
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
def headerLine (path anchor digest : String) (count : Nat) : String :=
  "{\"schema\":\"phx.formal.vectors.v1\",\"entry\":\"IdSlot\"" ++
    ",\"covers\":" ++ jsonString path ++ ",\"anchor\":" ++ jsonString anchor ++
    ",\"digest\":" ++ jsonString digest ++ ",\"vectors\":" ++ toString count ++ "}"

def floors : List Int :=
  [-9223372036854775808, -1, 0, 1, 63, 64, 1761955200000, 9223372036854775742,
    9223372036854775806, 9223372036854775807]

def nows : List Int :=
  [-9223372036854775808, -5, 0, 1, 64, 1761955200000, 9223372036854775807]

def slots : List Int := [0, 1, 2, 63]

def idJson : Option Int → String
  | none => "null"
  | some id => toString id

def lines : List String :=
  floors.flatMap fun floor => nows.flatMap fun now => slots.map fun slot =>
    "{\"floor\":" ++ toString floor ++ ",\"now\":" ++ toString now ++ ",\"slot\":" ++
      toString slot ++ ",\"id\":" ++ idJson (nextId floor now slot) ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/IdSlot.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/IdSlot.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine path anchor digest lines.length)
    for line in lines do
      IO.println line
    return 0

end Formal.IdSlotVectors
