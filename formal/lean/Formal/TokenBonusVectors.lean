import Formal.TokenBonus

/-!
# Formal.TokenBonusVectors

The token bonus's half of the vector writer (#102, #103). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean TokenBonus` to `run`, which prints one header line naming the
covered item and the digest the entry recorded for it, then one line per input with the port's
answer: the bonus in XP, or null when there is none. The formal checker byte-compares the output
with `formal/vectors/token-bonus.jsonl`, and `--write-vectors` writes it; the file is never edited
by hand.

The study day is the kernel's rule at a rollover hour of 4 with no offset, so an instant `i` falls
on epoch day `d` when `(i - 4 h) / 1 day = d`, the floor division the kernel takes. The vectors
print the hour and the day, and the Rust test builds that rule from them.

Three windows of two hours sit on the day: one across its start, one inside it and one across the
next day's start. Each window takes one review at a time, at an instant one before its start, at
it, one after it, at its middle, one before its end, at its end and one after it, of every kind
from -1 to 4 and of the eases 0, 1 and 4. Each window also takes twelve lists that reach the cap
from below, at and above it, add past the `i64` bounds, mix reviews that count with reviews that
do not, and hold none. This module carries no header line, so the registry lists it as support.
-/

namespace Formal.TokenBonusVectors
open Formal.TokenBonus

/-- A JSON string. The writer quotes only paths, item names, hex digests and the rule's name,
which carry no character JSON escapes. -/
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

/-- An hour, in milliseconds. -/
def hourMs : Int := 3600000

/-- A day, in milliseconds. -/
def dayMs : Int := 86400000

/-- The rollover hour of the vectors' rule. -/
def rolloverHour : Int := 4

/-- The vectors' study day, as an epoch day. -/
def studyDay : Int := 19680

/-- Whether instant `i` falls on the study day under the vectors' rule. -/
def onDay (i : Int) : Bool :=
  decide ((i - rolloverHour * hourMs) / dayMs = studyDay)

/-- The study day's first instant. -/
def dayStart : Int := studyDay * dayMs + rolloverHour * hourMs

/-- The three windows: across the day's start, inside the day, across the next day's start. -/
def windows : List (Int × Int) :=
  [(dayStart - hourMs, dayStart + hourMs),
   (dayStart + 6 * hourMs, dayStart + 8 * hourMs),
   (dayStart + 23 * hourMs, dayStart + 25 * hourMs)]

/-- The instants a single review takes in window `w`, around its start, middle and end. -/
def positions (w : Int × Int) : List Int :=
  [w.1 - 1, w.1, w.1 + 1, (w.1 + w.2) / 2, w.2 - 1, w.2, w.2 + 1]

/-- The kinds a single review takes: one below and one above the study kinds, and each of them. -/
def kinds : List Int := [-1, 0, 1, 2, 3, 4]

/-- The eases a single review takes: no answer, the least answer and the largest. -/
def eases : List Int := [0, 1, 4]

/-- Every single-review input: the window, then a review of 7 XP. -/
def singleInputs : List ((Int × Int) × List Review) :=
  windows.flatMap fun w => (positions w).flatMap fun i => kinds.flatMap fun k =>
    eases.map fun e => (w, [⟨i, k, e, 7⟩])

/-- The lists window `w` takes beside its single reviews. -/
def lists (w : Int × Int) : List (List Review) :=
  let mid := (w.1 + w.2) / 2
  [[⟨mid, 1, 3, 150⟩, ⟨mid + 1, 1, 3, 150⟩],
   [⟨mid, 1, 3, 150⟩, ⟨mid + 1, 1, 3, 150⟩, ⟨mid + 2, 2, 1, 1⟩],
   [⟨mid, 0, 2, 301⟩],
   [⟨mid, 1, 3, 299⟩, ⟨mid + 1, 3, 4, 1⟩],
   [⟨mid, 1, 3, 299⟩],
   [⟨mid, 1, 3, 0⟩],
   [⟨mid, 1, 3, i64Max⟩, ⟨mid + 1, 1, 3, i64Max⟩],
   [⟨w.1 - 1, 1, 3, 100⟩, ⟨mid, 1, 3, 50⟩],
   [⟨mid, 4, 3, 100⟩, ⟨mid + 1, 1, 3, 20⟩],
   [],
   [⟨w.2, 1, 3, 100⟩, ⟨w.1 - 1, 1, 3, 100⟩],
   [⟨mid, 1, 3, -5⟩]]

/-- Every list input: the window, then the list. -/
def listInputs : List ((Int × Int) × List Review) :=
  windows.flatMap fun w => (lists w).map fun rs => (w, rs)

/-- A review as a JSON array: its instant, kind, ease and base XP. -/
def reviewJson (r : Review) : String :=
  "[" ++ toString r.id ++ "," ++ toString r.kind ++ "," ++ toString r.ease ++ "," ++
    toString r.baseXp ++ "]"

/-- The bonus as JSON: its XP, or null. -/
def bonusJson : Option Int → String
  | some xp => toString xp
  | none => "null"

/-- One vector line: the input and the port's answer. -/
def line (input : (Int × Int) × List Review) : String :=
  let (w, rs) := input
  "{\"rule\":\"token_bonus_xp\",\"start\":" ++ toString w.1 ++ ",\"end\":" ++ toString w.2 ++
    ",\"rollover_hour\":" ++ toString rolloverHour ++ ",\"study_day\":" ++ toString studyDay ++
    ",\"reviews\":[" ++ ",".intercalate (rs.map reviewJson) ++ "],\"bonus\":" ++
    bonusJson (tokenBonusXp w.1 w.2 onDay rs) ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/TokenBonus.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/TokenBonus.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    let inputs := singleInputs ++ listInputs
    IO.println (headerLine "TokenBonus" path anchor digest inputs.length)
    for input in inputs do
      IO.println (line input)
    return 0

end Formal.TokenBonusVectors
