import Formal.Fsrs7Stock

/-!
# Formal.Fsrs7StockVectors

The stock projection's half of the vector writer (#823). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean Fsrs7Stock` to `run`, which prints one header line naming the
first covered item, the digest the entry recorded for it and every cover's digest, then one line
per input with the port's own answer. The formal checker byte-compares the output with
`formal/vectors/fsrs7-stock.jsonl`, and `--write-vectors` writes it; the file is never edited by
hand.

An `f32` is written `[0]` for NaN, `[1]` for negative infinity, `[2]` for positive infinity, and
`[3,n,d]` for the finite value `n / d`, each a power-of-two denominator, so the value is exact in
an `f32`. A difficulty line carries a replayed difficulty and the stock difficulty the port gives
it. A gap line carries a gap, the gap the curve reads (`F32.maxZero`), and whether that reading is
the review itself, a gap of 0, where the curve reads its ceiling. The port leaves each component
of the curve abstract, so a gap line carries no retrievability: the Rust test reads the pinned
crate's curve at each gap and holds the band, the ceiling at the review and the fall on what it
returns. A stability line carries a 90-percent interval and the stock stability the port gives it,
which `stock::stock_stability` must give bit for bit. This module carries no header line, so the
registry lists it as support.
-/

namespace Formal.Fsrs7StockVectors

open Formal.Fsrs7Stock

/-- A JSON string. The writer quotes only paths, item names and hex digests, which carry no
character JSON escapes. -/
def jsonString (s : String) : String :=
  "\"" ++ s ++ "\""

/-- Every `covers` line of a module, in order: its path, item and recorded digest. -/
def coversOf (module : String) : List (String × String × String) :=
  (module.splitOn "\n").filterMap fun line =>
    match (line.splitOn " ").filter (· ≠ "") with
    | [_, _, "covers", path, anchor, digest] =>
      if anchor.startsWith "anchor=" && digest.startsWith "digest=" then
        some (path, (anchor.drop 7).toString, (digest.drop 7).toString)
      else none
    | _ => none

/-- The header line: what the vectors were written from, and how many follow. -/
def headerLine (entry path anchor digest : String) (digests : List String) (count : Nat) :
    String :=
  "{\"schema\":\"phx.formal.vectors.v1\",\"entry\":" ++ jsonString entry ++
    ",\"covers\":" ++ jsonString path ++ ",\"anchor\":" ++ jsonString anchor ++
    ",\"digest\":" ++ jsonString digest ++ ",\"digests\":[" ++
    ",".intercalate (digests.map jsonString) ++ "],\"vectors\":" ++ toString count ++ "}"

/-- An `f32` as its tagged integers. -/
def f32Json : F32 → String
  | .nan => "[0]"
  | .negInf => "[1]"
  | .posInf => "[2]"
  | .fin q => s!"[3,{q.num},{q.den}]"

/-- The replayed difficulties: NaN, both infinities, below, at, inside, at and past the range. -/
def difficulties : List F32 :=
  [.nan, .negInf, .posInf, .fin (-5), .fin 0, .fin (1 / 2), .fin 1, .fin (11 / 2), .fin 10,
    .fin (41 / 4), .fin 11, .fin 1099511627776]

/-- The gaps, in days, NaN first, then in increasing order. -/
def gaps : List F32 :=
  [.nan, .negInf, .fin (-4), .fin (-1 / 8), .fin 0, .fin (1 / 1024), .fin (1 / 2), .fin 1,
    .fin 10, .fin 100, .fin 36500, .fin 1073741824, .fin 140737488355328, .posInf]

/-- One difficulty line. -/
def difficultyLine (d : F32) : String :=
  "{\"difficulty\":" ++ f32Json d ++ ",\"stock\":" ++ f32Json (stockDifficulty d) ++ "}"

/-- One gap line. -/
def gapLine (g : F32) : String :=
  let read := g.maxZero
  "{\"gap\":" ++ f32Json g ++ ",\"read\":" ++ f32Json read ++ ",\"review\":" ++
    (if read = .fin 0 then "1" else "0") ++ "}"

/-- The 90-percent intervals: NaN, both infinities, below zero, zero, two below the floor (the
Again interval of a first review under the default parameters among them), the floor, the next
`f32` above it, and three inside the scheduler's range. -/
def stabilities : List F32 :=
  [.nan, .negInf, .fin (-1), .fin 0, .fin (1 / 1048576), .fin againInterval, .fin stabilityMin,
    .fin (1717987 / 17179869184), .fin (1 / 2), .fin 1, .fin 36500, .posInf]

/-- One stability line: an interval and the stock stability the port gives it. -/
def stabilityLine (v : F32) : String :=
  "{\"interval\":" ++ f32Json v ++ ",\"stability\":" ++ f32Json (stockStability v) ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/Fsrs7Stock.lean"
  match coversOf module with
  | [] =>
    IO.eprintln "Formal/Fsrs7Stock.lean names no covered item"
    return 1
  | (path, anchor, digest) :: rest =>
    let digests := digest :: rest.map fun (_, _, recorded) => recorded
    IO.println (headerLine "Fsrs7Stock" path anchor digest digests
      (difficulties.length + gaps.length + stabilities.length))
    for d in difficulties do
      IO.println (difficultyLine d)
    for g in gaps do
      IO.println (gapLine g)
    for v in stabilities do
      IO.println (stabilityLine v)
    return 0

end Formal.Fsrs7StockVectors
