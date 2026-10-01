import Formal.NextMilestone

/-!
# Formal.NextMilestoneVectors

The next milestone's half of the vector writer (SPEC-073 R14). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean NextMilestone` to `run`, which prints one header line naming the
first covered item and the digest the entry recorded for it, then one line per input with the
port's answer: the ladder's place in the tie order, the rung, the current value and what is left.
`phxd formal check` byte-compares the output with `formal/vectors/next-milestone.jsonl`, and
`--write-vectors` writes it; the file is never edited by hand.

The inputs are every combination of each ladder's values at zero and on either side of each rung
(the rung less one, and the rung itself), so every rung is approached and reached, each ladder is
complete in some input, and all three are complete in one. This module carries no header line, so
the registry lists it as support.
-/

namespace Formal.NextMilestoneVectors

open Formal.NextMilestone

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
def headerLine (entry path anchor digest : String) (count : Nat) : String :=
  "{\"schema\":\"phx.formal.vectors.v1\",\"entry\":" ++ jsonString entry ++
    ",\"covers\":" ++ jsonString path ++ ",\"anchor\":" ++ jsonString anchor ++
    ",\"digest\":" ++ jsonString digest ++ ",\"vectors\":" ++ toString count ++ "}"

/-- A ladder's values at zero and on either side of each rung. -/
def around (ladder : List Nat) : List Nat :=
  0 :: ladder.flatMap fun rung => [rung - 1, rung]

/-- Every input, in a fixed order: reviews, then the streak, then mature cards. -/
def inputs : List (Nat × Nat × Nat) :=
  (around reviewLadder).flatMap fun reviews =>
    (around streakLadder).flatMap fun streak =>
      (around matureLadder).map fun mature => (reviews, streak, mature)

/-- One vector line: the input and the port's answer, `null` if it gave none. -/
def vectorLine (input : Nat × Nat × Nat) : String :=
  let (reviews, streak, mature) := input
  let answer :=
    match nextMilestoneOf reviews streak mature with
    | none => "null"
    | some c =>
      "{\"ladder\":" ++ toString c.idx ++ ",\"target\":" ++ toString c.rung ++
        ",\"current\":" ++ toString c.value ++ ",\"remaining\":" ++ toString (c.rung - c.value) ++
        "}"
  "{\"reviews\":" ++ toString reviews ++ ",\"streak\":" ++ toString streak ++ ",\"mature\":" ++
    toString mature ++ ",\"milestone\":" ++ answer ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/NextMilestone.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/NextMilestone.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine "NextMilestone" path anchor digest inputs.length)
    for input in inputs do
      IO.println (vectorLine input)
    return 0

end Formal.NextMilestoneVectors
