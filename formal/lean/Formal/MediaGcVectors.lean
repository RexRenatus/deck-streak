import Formal.MediaGc

/-!
# Formal.MediaGcVectors

The media sweep's half of the vector writer (SPEC-513). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean MediaGc` to `run`, which prints one header line naming the first
covered item and the digest the entry recorded for it, then one line per input with the port's
answer. The formal checker byte-compares the output with `formal/vectors/media-gc.jsonl`, and
`--write-vectors` writes it; the file is never edited by hand.

The inputs are every combination of whether the file is named, pinned and imported, five added
instants (-1, 0, 1, 5 and 10), two sweep instants (0 and 10) and three windows (0, 5 and 10), in
that order; then the extremes: the `i64` bounds as the added and sweep instants each way, under
no window and the `u64` maximum. The crate's test `crates/media/tests/formal_vectors_gc.rs`
answers each line with `fate`. This module carries no header line, so the registry lists it as
support.
-/

namespace Formal.MediaGcVectors

open Formal.MediaGc

/-- The facts' flags, in order. -/
def flags : List Bool := [false, true]

/-- The added instants, in order. -/
def addeds : List Int := [-1, 0, 1, 5, 10]

/-- The sweep instants, in order. -/
def nows : List Int := [0, 10]

/-- The windows, in order. -/
def graces : List Nat := [0, 5, 10]

/-- The `i64` bounds. -/
def minI64 : Int := -9223372036854775808
def maxI64 : Int := 9223372036854775807

/-- The `u64` maximum. -/
def maxU64 : Nat := 18446744073709551615

/-- A reason's name, as the Rust `Keep`'s. -/
def keepName : Keep → String
  | .live => "live"
  | .pinned => "pinned"
  | .recent => "in_grace"
  | .imported => "imported"

/-- A fate's name. -/
def fateName : Fate → String
  | .keep reason => keepName reason
  | .delete => "delete"

/-- A flag as JSON. -/
def flagJson (b : Bool) : String := if b then "true" else "false"

/-- One line. -/
def line (facts : Facts) (policy : SweepPolicy) : String :=
  "{\"referenced\":" ++ flagJson facts.referenced ++ ",\"pinned\":" ++ flagJson facts.pinned ++
    ",\"imported\":" ++ flagJson facts.imported ++ ",\"added_ms\":" ++ toString facts.addedMs ++
    ",\"now_ms\":" ++ toString policy.nowMs ++ ",\"grace_ms\":" ++ toString policy.graceMs ++
    ",\"fate\":\"" ++ fateName (fate facts policy) ++ "\"}"

/-- Every input of the small grid, in order. -/
def grid : List (Facts × SweepPolicy) :=
  flags.flatMap fun referenced =>
    flags.flatMap fun pinned =>
      flags.flatMap fun imported =>
        addeds.flatMap fun added =>
          nows.flatMap fun now =>
            graces.map fun grace =>
              (⟨referenced, pinned, imported, added⟩, ⟨now, grace⟩)

/-- The extremes: an unnamed, unpinned, not imported file at each bound. -/
def extremes : List (Facts × SweepPolicy) :=
  [(minI64, maxI64), (maxI64, minI64)].flatMap fun (added, now) =>
    [0, maxU64].map fun grace => (⟨false, false, false, added⟩, ⟨now, grace⟩)

/-- Every line, in order. -/
def lines : List String :=
  (grid ++ extremes).map fun (facts, policy) => line facts policy

/-- The covered path, anchor and digest of the module's first `covers` line. -/
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
  "{\"schema\":\"phx.formal.vectors.v1\",\"entry\":\"MediaGc\",\"covers\":\"" ++ path ++
    "\",\"anchor\":\"" ++ anchor ++ "\",\"digest\":\"" ++ digest ++ "\",\"vectors\":" ++
    toString count ++ "}"

/-- `lean --run Formal/Vectors.lean MediaGc`: print the entry's vectors on stdout. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/MediaGc.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/MediaGc.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine path anchor digest lines.length)
    for line in lines do
      IO.println line
    return 0

end Formal.MediaGcVectors
