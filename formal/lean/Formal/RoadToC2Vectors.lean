import Formal.RoadToC2

/-!
# Formal.RoadToC2Vectors

Road to C2's half of the vector writer (#85). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean RoadToC2` to `run`, which prints one header line naming the first
covered item and the digest the entry recorded for it, then one line per input with the port's
answer: for `course_progress`, the current band of each list of achieved flags; for `band_step`,
the step and its band; for `mastery_pillar`, the pillar; and for `parse_unit`, the unit or `null`.
The formal checker byte-compares the output with `formal/vectors/road-to-c2.jsonl`, and
`--write-vectors` writes it; the file is never edited by hand.

The current band's inputs are every list of six flags, one per band of `CEFR_BANDS`. The band
step's inputs are every stored band (none, each band, and a band with no place) against every
current band (each band, and the one with no place). The pillar's inputs sit on either side of 0
and of the cap's count 10, and at the `i64` extremes. The deck names hold a unit with and without
leading zeros, after Unicode white space, the largest `u32` and the first unit past it, several
units, and names that hold none. This module carries no header line, so the registry lists it as
support.
-/

namespace Formal.RoadToC2Vectors
open Formal.RoadToC2

/-- A JSON string. The writer quotes paths, item names, hex digests, bands and deck names, so it
escapes the quote, the backslash and every control character. -/
def jsonString (s : String) : String :=
  let hex (n : Nat) : String :=
    let digits := Nat.toDigits 16 n
    String.ofList (List.replicate (4 - digits.length) '0' ++ digits)
  let escape (c : Char) : String :=
    if c == '"' then "\\\""
    else if c == '\\' then "\\\\"
    else if c.toNat < 0x20 then "\\u" ++ hex c.toNat
    else String.singleton c
  "\"" ++ String.join (s.toList.map escape) ++ "\""

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

/-- Every list of `n` flags, in a fixed order. -/
def flagLists : Nat → List (List Bool)
  | 0 => [[]]
  | n + 1 => (flagLists n).flatMap fun rest => [false :: rest, true :: rest]

/-- The current band's inputs: every list of flags, one per band. -/
def currentInputs : List (List Bool) :=
  flagLists cefrBands.length

/-- A band with no place in `CEFR_BANDS`. -/
def placeless : String := "Z9"

/-- The band step's stored bands. -/
def storedInputs : List (Option String) :=
  none :: (cefrBands ++ [placeless]).map some

/-- The band step's current bands. -/
def reachedInputs : List String :=
  cefrBands ++ [placeless]

/-- Every band step input, in a fixed order: the stored band, then the current one. -/
def stepInputs : List (Option String × String) :=
  storedInputs.flatMap fun stored => reachedInputs.map fun current => (stored, current)

/-- The pillar's inputs. -/
def pillarInputs : List Int :=
  [-9223372036854775808, -1000, -11, -10, -4, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 20, 1000,
    9223372036854775807]

/-- The deck names. -/
def unitInputs : List String :=
  ["Unit 3", "Unit 03", "Unit 000", "Unit3", "Unit  12", "Unit\t7", "Unit 9",
    "Unit　5", "Unit\u000B6", "Beta Course\u001FUnit 07", "Unit x Unit 4", "Unit 5 Unit 6",
    "UnitUnit 8", "Units 4", "unit 4", "Unit 12a", "Unit -3", "Unit ", "", "Lesson 4",
    "Unit 4294967295", "Unit 4294967296", "Unit 00004294967295", "Unit 99999999999999999999",
    "Unit 4294967296 Unit 2"]

/-- A flag list as JSON. -/
def flagsJson (flags : List Bool) : String :=
  "[" ++ ",".intercalate (flags.map toString) ++ "]"

/-- A band's name from its place. -/
def bandName (place : Nat) : String :=
  cefrBands.getD place "?"

/-- One current band vector line: the flags and the port's current band. -/
def currentLine (flags : List Bool) : String :=
  "{\"rule\":\"course_progress\",\"achieved\":" ++ flagsJson flags ++ ",\"current_band\":" ++
    jsonString (bandName (courseProgress flags)) ++ "}"

/-- A band step as its JSON fields. -/
def stepJson : BandStep → String
  | BandStep.firstSighting band => "\"step\":\"first_sighting\",\"band\":" ++ jsonString band
  | BandStep.bandUp band => "\"step\":\"band_up\",\"band\":" ++ jsonString band
  | BandStep.unchanged => "\"step\":\"unchanged\",\"band\":null"

/-- One band step vector line: the stored and current bands and the port's step. -/
def stepLine (input : Option String × String) : String :=
  let (stored, current) := input
  let storedJson := match stored with
    | none => "null"
    | some band => jsonString band
  "{\"rule\":\"band_step\",\"stored\":" ++ storedJson ++ ",\"current\":" ++ jsonString current ++
    "," ++ stepJson (bandStep cefrPlace stored current) ++ "}"

/-- One pillar vector line: the leech count and the port's pillar. -/
def pillarLine (leeches : Int) : String :=
  "{\"rule\":\"mastery_pillar\",\"law_leech_active\":" ++ toString leeches ++ ",\"pillar\":" ++
    toString (masteryPillar leeches) ++ "}"

/-- One unit vector line: the deck name and the port's unit, `null` for none. -/
def unitLine (deckName : String) : String :=
  let unit := match parseUnit deckName with
    | none => "null"
    | some unit => toString unit
  "{\"rule\":\"parse_unit\",\"deck_name\":" ++ jsonString deckName ++ ",\"unit\":" ++ unit ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/RoadToC2.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/RoadToC2.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    let count := currentInputs.length + stepInputs.length + pillarInputs.length +
      unitInputs.length
    IO.println (headerLine "RoadToC2" path anchor digest count)
    for flags in currentInputs do
      IO.println (currentLine flags)
    for input in stepInputs do
      IO.println (stepLine input)
    for leeches in pillarInputs do
      IO.println (pillarLine leeches)
    for deckName in unitInputs do
      IO.println (unitLine deckName)
    return 0

end Formal.RoadToC2Vectors
