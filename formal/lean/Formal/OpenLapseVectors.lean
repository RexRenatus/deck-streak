import Formal.OpenLapse

/-!
# Formal.OpenLapseVectors

The open lapse walk's half of the vector writer (#472). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean OpenLapse` to `run`, which prints one header line naming the
covered item and the digest the entry recorded for it, then one line per input with the port's
answer. `phxd formal check` byte-compares the output with `formal/vectors/open-lapse.jsonl`, and
`--write-vectors` writes it; the file is never edited by hand.

The inputs are derived from five axes, in this order: the window's first day (the smallest day,
the day after it, day zero and the fourth day before the largest), the threshold (one and three),
the window's width (one to three days), each day's state, and today (from the day before the
window's first day to the day after its last). The window's first day holds an entry, so it is a
study review, a zero count, or a zero count on a skip day. A day between the first and the last
has no entry, a study review, a zero count, or no entry on a skip day. The last day of a window
wider than one is a study review, a zero count or a skip day, never a day with nothing, so no two
windows read alike. A today below the smallest day is dropped. The crate's test
`crates/streaks/tests/formal_vectors_open_lapse.rs` derives the same inputs from the same axes and
answers each with the Rust function. This module carries no header line, so the registry lists it
as support.
-/

namespace Formal.OpenLapseVectors

open Formal.OpenLapse

/-- One day of a window. -/
inductive DayState where
  | absent
  | review
  | zero
  | skip
  | zeroSkip

/-- The states of the window's first day, which holds an entry. -/
def firstStates : List DayState := [DayState.review, DayState.zero, DayState.zeroSkip]

/-- The states of a day between the first and the last. -/
def middleStates : List DayState :=
  [DayState.absent, DayState.review, DayState.zero, DayState.skip]

/-- The states of the last day of a window wider than one. -/
def lastStates : List DayState := [DayState.review, DayState.zero, DayState.skip]

/-- Every list of `n` middle days followed by a last day. -/
def tails : Nat → List (List DayState)
  | 0 => lastStates.map fun s => [s]
  | n + 1 => middleStates.flatMap fun s => (tails n).map fun rest => s :: rest

/-- Every window of a width. -/
def windows : Nat → List (List DayState)
  | 0 => []
  | 1 => firstStates.map fun s => [s]
  | width + 2 => firstStates.flatMap fun s => (tails width).map fun rest => s :: rest

/-- The window's first days. -/
def origins : List Int64 := [Int64.minValue, Int64.minValue + 1, 0, Int64.maxValue - 3]

/-- The thresholds. -/
def thresholds : List UInt32 := [1, 3]

/-- The window's widths. -/
def widths : List Nat := [1, 2, 3]

/-- Each day with its offset from the window's first day. -/
def indexed : Nat → List DayState → List (Nat × DayState)
  | _, [] => []
  | i, s :: rest => (i, s) :: indexed (i + 1) rest

/-- The window's entries, in key order. -/
def countsOf (origin : Int64) (states : List DayState) : Counts :=
  (indexed 0 states).filterMap fun (i, s) =>
    match s with
    | DayState.review => some (origin + Int64.ofNat i, 1)
    | DayState.zero => some (origin + Int64.ofNat i, 0)
    | DayState.zeroSkip => some (origin + Int64.ofNat i, 0)
    | _ => none

/-- The window's skip days. -/
def skipsOf (origin : Int64) (states : List DayState) : List Int64 :=
  (indexed 0 states).filterMap fun (i, s) =>
    match s with
    | DayState.skip => some (origin + Int64.ofNat i)
    | DayState.zeroSkip => some (origin + Int64.ofNat i)
    | _ => none

/-- Today: the day before the window's first day to the day after its last, within the type. -/
def todays (origin : Int64) (width : Nat) : List Int64 :=
  (List.range (width + 2)).filterMap fun (k : Nat) =>
    let day : Int := origin.toInt + (k : Int) - 1
    if Int64.minValue.toInt ≤ day then some (Int64.ofInt day) else none

/-- One input. -/
structure Input where
  today : Int64
  counts : Counts
  skips : List Int64
  threshold : UInt32

/-- Every input, in a fixed order. -/
def inputs : List Input :=
  origins.flatMap fun origin =>
    thresholds.flatMap fun threshold =>
      widths.flatMap fun width =>
        (windows width).flatMap fun states =>
          (todays origin width).map fun today =>
            ⟨today, countsOf origin states, skipsOf origin states, threshold⟩

/-- A JSON array of already-rendered values. -/
def jsonArray (items : List String) : String := "[" ++ ",".intercalate items ++ "]"

/-- A day as JSON. -/
def dayJson (day : Int64) : String := toString day.toInt

/-- An outcome as JSON: the day, `null` for none, and a string for a walk that did not answer. -/
def outcomeJson : Outcome → String
  | Outcome.answer (some day) => dayJson day
  | Outcome.answer none => "null"
  | Outcome.overflow => "\"overflow\""
  | Outcome.outOfFuel => "\"out-of-fuel\""

/-- One vector line. -/
def vectorLine (input : Input) : String :=
  "{\"today\":" ++ dayJson input.today ++ ",\"counts\":" ++
    jsonArray (input.counts.map fun (day, count) =>
      jsonArray [dayJson day, toString count]) ++
    ",\"skips\":" ++ jsonArray (input.skips.map dayJson) ++
    ",\"threshold\":" ++ toString input.threshold ++ ",\"answer\":" ++
    outcomeJson (openLapse input.today input.counts input.skips input.threshold).outcome ++ "}"

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
  "{\"schema\":\"phx.formal.vectors.v1\",\"entry\":\"OpenLapse\",\"covers\":\"" ++ path ++
    "\",\"anchor\":\"" ++ anchor ++ "\",\"digest\":\"" ++ digest ++ "\",\"vectors\":" ++
    toString count ++ "}"

/-- `lean --run Formal/Vectors.lean OpenLapse`: print the entry's vectors on stdout. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/OpenLapse.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/OpenLapse.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine path anchor digest inputs.length)
    for input in inputs do
      IO.println (vectorLine input)
    return 0

end Formal.OpenLapseVectors
