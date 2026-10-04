import Formal.HabitWriting

/-!
# Formal.HabitWritingVectors

The writing day's half of the vector writer (#94). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean HabitWriting` to `run`, which prints one header line naming the
first covered item and the digest the entry recorded for it, then one line carrying the two
amounts, then one line per input with the port's answer. The formal checker byte-compares the
output with `formal/vectors/habit-writing.jsonl`, and `--write-vectors` writes it; the file is
never edited by hand.

The courses are synthetic codes. Each writing set (none, one, two and three courses) takes each
list of confirmations on two study days: none, one course, every writing course, a course outside
the writing set, and every course on the other day only. A `writing_day_xp` line carries the
port's per-course amounts and `write:all`; an `all_confirmed_days` line carries the days the port
returns, read on every day the inputs name and printed in order once each, as the code's set
holds them. This module carries no header line, so the registry lists it as support.
-/

namespace Formal.HabitWritingVectors
open Formal.HabitWriting

/-- A JSON string. The writer quotes only paths, item names, hex digests, rule names and the
synthetic course codes, which carry no character JSON escapes. -/
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

/-- The study day the inputs are read on. -/
def studyDay : Int := 19680

/-- The other study day the inputs name. -/
def otherDay : Int := 19681

/-- The days a line reads `all_confirmed_days` on, in order. -/
def days : List Int := [studyDay, otherDay]

/-- The writing sets: none, one, two and three synthetic courses. -/
def writingSets : List (List String) :=
  [[], ["qaa"], ["qaa", "qab"], ["qaa", "qab", "qac"]]

/-- The confirmation lists of one writing set. -/
def rowLists (writing : List String) : List (List (String × Int)) :=
  [[],
   [("qaa", studyDay)],
   writing.map fun code => (code, studyDay),
   ("qzz", studyDay) :: writing.map fun code => (code, studyDay),
   writing.map fun code => (code, otherDay),
   (writing.map fun code => (code, studyDay)) ++ (writing.take 1).map fun code => (code, otherDay)]

/-- Every input once: the writing set, then the confirmations. -/
def inputs : List (List String × List (String × Int)) :=
  (writingSets.flatMap fun w => (rowLists w).map fun rs => (w, rs)).eraseDups

/-- A list of codes as JSON. -/
def codesJson (codes : List String) : String :=
  "[" ++ ",".intercalate (codes.map jsonString) ++ "]"

/-- A confirmation as JSON: its code and its epoch day. -/
def rowJson (row : String × Int) : String :=
  "[" ++ jsonString row.1 ++ "," ++ toString row.2 ++ "]"

/-- A course's amount as JSON: its code and its XP. -/
def grantJson (grant : String × Nat) : String :=
  "[" ++ jsonString grant.1 ++ "," ++ toString grant.2 ++ "]"

/-- The line carrying the two amounts the Rust test checks first. -/
def constantsLine : String :=
  "{\"rule\":\"constants\",\"WRITING_XP_PER_DAY\":" ++ toString writingXpPerDay ++
    ",\"WRITING_XP_ALL_THREE_BONUS\":" ++ toString writingXpAllThreeBonus ++ "}"

/-- One `writing_day_xp` line: the input and the port's answer. -/
def dayLine (input : List String × List (String × Int)) : String :=
  let (w, rs) := input
  let xp := writingDayXp w studyDay rs
  "{\"rule\":\"writing_day_xp\",\"writing\":" ++ codesJson w ++ ",\"day\":" ++ toString studyDay ++
    ",\"rows\":[" ++ ",".intercalate (rs.map rowJson) ++ "],\"courses\":[" ++
    ",".intercalate (xp.1.map grantJson) ++ "],\"all\":" ++ toString xp.2 ++ "}"

/-- One `all_confirmed_days` line: the input and the days the port returns, in order. -/
def daysLine (input : List String × List (String × Int)) : String :=
  let (w, rs) := input
  let got := days.filter fun d => decide (d ∈ allConfirmedDays w rs)
  "{\"rule\":\"all_confirmed_days\",\"writing\":" ++ codesJson w ++ ",\"rows\":[" ++
    ",".intercalate (rs.map rowJson) ++ "],\"days\":[" ++ ",".intercalate (got.map toString) ++
    "]}"

/-- Print the header, the amounts and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/HabitWriting.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/HabitWriting.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    let lines := constantsLine :: (inputs.map dayLine ++ inputs.map daysLine)
    IO.println (headerLine "HabitWriting" path anchor digest lines.length)
    for line in lines do
      IO.println line
    return 0

end Formal.HabitWritingVectors
