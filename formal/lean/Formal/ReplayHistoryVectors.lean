import Formal.ReplayHistory

/-!
# Formal.ReplayHistoryVectors

The history selection's half of the vector writer (#641, SPEC-386 R15). `Formal/Vectors.lean`
dispatches `lean --run Formal/Vectors.lean ReplayHistory` to `run`, which prints one header line
naming the first covered item, the digest the entry recorded for it and every cover's digest, then
one line per row set with the histories the port selects. The formal checker byte-compares the
output with `formal/vectors/replay-history.jsonl`, and `--write-vectors` writes it; the file is
never edited by hand.

The inputs are every row set of one card with one to three rows, and of two cards, the first with
one or two rows and the second with one, each row drawn from eight shapes the engine's review log
holds: a learning step before the card's first factor, a relearning step, a review, the engine's
Forget, its set-due-date, a reschedule, an unrated review, and a rated manual entry with the factor
0. The rows' ids are distinct and less than a day apart, and each set arrives in id order and in
reverse. This module carries no header line, so the registry lists it as support.
-/

namespace Formal.ReplayHistoryVectors

open Formal.ReplayHistory

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

/-- The eight row shapes, as ease, kind and factor: a learning step before the first factor, a
relearning step, a review, the engine's Forget, its set-due-date, a reschedule, an unrated review,
and a rated manual entry with the factor 0. -/
def shapes : List (Nat × Nat × Nat) :=
  [(3, 0, 0), (1, 2, 2300), (4, 1, 2650), (0, 4, 0), (0, 4, 2500), (0, 5, 2500), (0, 1, 2500),
    (3, 4, 0)]

/-- Every sequence of `n` shapes. -/
def words : Nat → List (List (Nat × Nat × Nat))
  | 0 => [[]]
  | n + 1 => shapes.flatMap fun shape => (words n).map (shape :: ·)

/-- The id of the row at a slot: a day after the epoch, then ten and a half hours apart. -/
def idAt (slot : Nat) : Int := 86400000 + 37800000 * slot

/-- A row of a card at a slot, in a shape. -/
def rowAt (cid : Int) (slot : Nat) (shape : Nat × Nat × Nat) : RevlogRow :=
  ⟨cid, idAt slot, shape.1, shape.2.1, shape.2.2⟩

/-- The one card's row sets, in id order: one to three rows of card 200. -/
def oneCard : List (List RevlogRow) :=
  [1, 2, 3].flatMap fun n =>
    (words n).map fun word => (([0, 1, 2].take n).zip word).map fun (slot, shape) =>
      rowAt 200 slot shape

/-- The two cards' row sets, in id order: card 200's rows at slots 0 and 2, card 100's at slot 1,
so the second card's row falls between the first card's. -/
def twoCards : List (List RevlogRow) :=
  ((words 1) ++ (words 2)).flatMap fun first =>
    (words 1).map fun second =>
      let firstRows := ([0, 2].zip first).map fun (slot, shape) => rowAt 200 slot shape
      let secondRows := ([1].zip second).map fun (slot, shape) => rowAt 100 slot shape
      firstRows.take 1 ++ secondRows ++ firstRows.drop 1

/-- Every input, in a fixed order: each row set in id order, then reversed. -/
def inputs : List (List RevlogRow) :=
  (oneCard ++ twoCards).flatMap fun rows => [rows, rows.reverse]

/-- A row as its five integers. -/
def rowJson (row : RevlogRow) : String :=
  s!"[{row.cid},{row.id},{row.ease},{row.kind},{row.factor}]"

/-- A review as its rating and its distance in milliseconds. -/
def reviewJson (review : Review) : String :=
  s!"[{review.rating},{review.delta}]"

/-- A history as its card, its last kept id and its reviews. -/
def historyJson (history : CardHistory) : String :=
  s!"[{history.cid},{history.lastId},[" ++ ",".intercalate (history.reviews.map reviewJson) ++ "]]"

/-- One vector line: the rows as they arrive, and the port's histories. -/
def vectorLine (rows : List RevlogRow) : String :=
  "{\"rows\":[" ++ ",".intercalate (rows.map rowJson) ++ "],\"histories\":[" ++
    ",".intercalate ((histories rows).map historyJson) ++ "]}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/ReplayHistory.lean"
  match coversOf module with
  | [] =>
    IO.eprintln "Formal/ReplayHistory.lean names no covered item"
    return 1
  | (path, anchor, digest) :: rest =>
    let digests := digest :: rest.map fun (_, _, recorded) => recorded
    IO.println (headerLine "ReplayHistory" path anchor digest digests inputs.length)
    for rows in inputs do
      IO.println (vectorLine rows)
    return 0

end Formal.ReplayHistoryVectors
