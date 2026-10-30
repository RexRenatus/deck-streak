import Formal.MediaBind

/-!
# Formal.MediaBindVectors

The write-once bindings' half of the vector writer (SPEC-513). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean MediaBind` to `run`, which prints one header line naming the
first covered item and the digest the entry recorded for it, then one line per input with the
port's answer. The formal checker byte-compares the output with `formal/vectors/media-bind.jsonl`,
and `--write-vectors` writes it; the file is never edited by hand.

The inputs are every bind over five slots (unbound, and bound live or released to hash 0 or 1),
three claims (none, 0 and 1) and two hashes (0 and 1), in that order, then every release over the
same five slots. The crate's test `crates/media/tests/formal_vectors_bind.rs` reads a hash `n` as
the hash whose 32 bytes are all `n`, and answers each line with `step` or `release_step`. This
module carries no header line, so the registry lists it as support.
-/

namespace Formal.MediaBindVectors

open Formal.MediaBind

/-- The slots, in order. -/
def slots : List (Option Binding) :=
  [none, some ⟨0, .live⟩, some ⟨0, .released⟩, some ⟨1, .live⟩, some ⟨1, .released⟩]

/-- The claims, in order. -/
def claims : List (Option Nat) := [none, some 0, some 1]

/-- The hashes, in order. -/
def hashes : List Nat := [0, 1]

/-- A state's name. -/
def stateName : BindState → String
  | .live => "live"
  | .released => "released"

/-- A slot as JSON. -/
def slotJson : Option Binding → String
  | none => "null"
  | some b => "{\"hash\":" ++ toString b.hash ++ ",\"state\":\"" ++ stateName b.state ++ "\"}"

/-- A claim as JSON. -/
def claimJson : Option Nat → String
  | none => "null"
  | some n => toString n

/-- A bind's outcome name. -/
def outcomeName : Outcome → String
  | .new => "new"
  | .unchanged => "unchanged"
  | .revived => "revived"
  | .mismatch => "mismatch"
  | .conflict => "conflict"

/-- A release's outcome name. -/
def releaseName : ReleaseOutcome → String
  | .released => "released"
  | .alreadyReleased => "already_released"
  | .unbound => "unbound"

/-- One bind's line. -/
def bindLine (slot : Option Binding) (claimed : Option Nat) (hash : Nat) : String :=
  let (after, outcome) := step slot claimed hash
  "{\"op\":\"bind\",\"slot\":" ++ slotJson slot ++ ",\"claimed\":" ++ claimJson claimed ++
    ",\"hash\":" ++ toString hash ++ ",\"after\":" ++ slotJson after ++ ",\"outcome\":\"" ++
    outcomeName outcome ++ "\"}"

/-- One release's line. -/
def releaseLine (slot : Option Binding) : String :=
  let (after, outcome) := releaseStep slot
  "{\"op\":\"release\",\"slot\":" ++ slotJson slot ++ ",\"after\":" ++ slotJson after ++
    ",\"outcome\":\"" ++ releaseName outcome ++ "\"}"

/-- Every line, in order. -/
def lines : List String :=
  (slots.flatMap fun slot =>
    claims.flatMap fun claimed =>
      hashes.map fun hash => bindLine slot claimed hash) ++
  slots.map releaseLine

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
  "{\"schema\":\"phx.formal.vectors.v1\",\"entry\":\"MediaBind\",\"covers\":\"" ++ path ++
    "\",\"anchor\":\"" ++ anchor ++ "\",\"digest\":\"" ++ digest ++ "\",\"vectors\":" ++
    toString count ++ "}"

/-- `lean --run Formal/Vectors.lean MediaBind`: print the entry's vectors on stdout. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/MediaBind.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/MediaBind.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine path anchor digest lines.length)
    for line in lines do
      IO.println line
    return 0

end Formal.MediaBindVectors
