import Formal.CaptionBound

/-!
# Formal.CaptionBoundVectors

The photo caption bound's half of the vector writer (#480). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean CaptionBound` to `run`, which prints one header line naming the
first covered item and the digest the entry recorded for it, then one line per caption with its
runs of code points, the port's unit count and the port's answer. The formal checker
byte-compares the output with `formal/vectors/caption-bound.jsonl`, and `--write-vectors` writes
it; the file is never edited by hand.

The captions are the empty caption, then for each of U+0061, U+00E9, U+65E5, U+D7FF, U+E000,
U+FFFF, U+10000, U+1F600 and U+10FFFF, at 1,023, 1,024 and 1,025 units, the caption filled with
that character (with one U+0061 after it when a remainder is left) and the caption of U+0061
letters with that character last. A caption is its runs, each a code point and how many times it
stands. This module carries no header line, so the registry lists it as support.
-/

namespace Formal.CaptionBoundVectors
open Formal.CaptionBound

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

/-- A caption as its runs: each a code point and how many times it stands, in order. -/
abbrev Runs := List (Nat × Nat)

/-- The characters the captions are filled with and end with, by code point. -/
def codePoints : List Nat := [97, 233, 26085, 55295, 57344, 65535, 65536, 128512, 1114111]

/-- The unit counts each character's captions are written at: one under the limit, the limit, and
one over it. -/
def targets : List Nat := [1023, 1024, 1025]

/-- The units one character yields, by the port's encoder. -/
def unitsOf (code : Nat) : Nat :=
  (encodeUtf16 (Char.ofNat code)).length

/-- The caption of `target` units filled with the character: the character as many times as its
units fit, then one letter when a remainder is left. -/
def filled (code target : Nat) : Runs :=
  [(code, target / unitsOf code)] ++ (if target % unitsOf code = 0 then [] else [(97, 1)])

/-- The caption of `target` units whose last character is this one, after letters. -/
def last (code target : Nat) : Runs :=
  [(97, target - unitsOf code), (code, 1)]

/-- Every caption the writer prints, in order. -/
def captions : List Runs :=
  [] :: codePoints.flatMap fun code =>
    targets.flatMap fun target => [filled code target, last code target]

/-- The caption a vector names: its runs, each its character repeated. -/
def expand (runs : Runs) : List Char :=
  runs.flatMap fun (code, count) => List.replicate count (Char.ofNat code)

/-- The runs as JSON, with no space. -/
def runsJson (runs : Runs) : String :=
  "[" ++ ",".intercalate (runs.map fun (code, count) =>
    "[" ++ toString code ++ "," ++ toString count ++ "]") ++ "]"

/-- Whether the port's check takes the caption. -/
def takes (caption : List Char) : Bool :=
  match checkCaption caption with
  | .ok () => true
  | .error _ => false

/-- One vector line: the caption's runs, the port's unit count and the port's answer. -/
def captionLine (runs : Runs) : String :=
  let caption := expand runs
  "{\"rule\":\"caption\",\"runs\":" ++ runsJson runs ++ ",\"units\":" ++
    toString (countUnits caption) ++ ",\"takes\":" ++ toString (takes caption) ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/CaptionBound.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/CaptionBound.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine "CaptionBound" path anchor digest captions.length)
    for runs in captions do
      IO.println (captionLine runs)
    return 0

end Formal.CaptionBoundVectors
