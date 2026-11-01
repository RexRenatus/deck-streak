import Formal.Fsrs7MemoryState

/-!
# Formal.Fsrs7MemoryStateVectors

The replay's half of the vector writer (#823). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean Fsrs7MemoryState` to `run`, which prints one header line naming
the covered item and the digest the entry recorded for it, then one line per input: the histories
as they arrive, and the same histories as the port reads them (`Review.normal`). The formal checker
byte-compares the output with `formal/vectors/fsrs7-memory-state.jsonl`, and `--write-vectors`
writes it; the file is never edited by hand.

The port leaves the crate's arithmetic abstract, so a line carries no state: the Rust test replays
both lists through the pinned crate and holds the three claims on what it returns. Every state is
in the bounds, the batch returns what the card-by-card replay returns, and the histories as read
replay to the same states as the histories as they arrive.

A review is `[rating,gap]`, and a gap is an `f32` written `[0]` for NaN, `[1]` for negative
infinity, `[2]` for positive infinity, and `[3,n,d]` for the finite value `n / d`, each a
power-of-two denominator, so the value is exact in an `f32`. The histories are the empty one, every
one-review history over seven ratings (0 to 5 and 9) and ten gaps (NaN, both infinities, -4,
-1/8, 0, 1/2, 10, 100000 and 2^47 days), every two-review history that opens at a gap of zero, and
three-review histories over a subset; each is replayed alone, beside itself, and beside the next
history. After them come the stability update's lines: every state over six stabilities, three
difficulties and three fast stabilities, all in the bounds, beside each of the ten gaps, with the
bounds the two update claims give for the slow and the fast stability. The Rust test feeds each
state and gap to the pinned crate's `FSRS::next_states_with_elapsed_days` and holds the Again
state at or below the lapse bound and the Hard, Good and Easy states at or above the success bound.
This module carries no header line, so the registry lists it as support.
-/

namespace Formal.Fsrs7MemoryStateVectors

open Formal.Fsrs7MemoryState

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

/-- A review as its rating and gap. -/
def reviewJson (r : Review) : String :=
  "[" ++ toString r.rating ++ "," ++ f32Json r.gap ++ "]"

/-- A history as its reviews. -/
def itemJson (item : List Review) : String :=
  "[" ++ ",".intercalate (item.map reviewJson) ++ "]"

/-- A list of histories. -/
def itemsJson (items : List (List Review)) : String :=
  "[" ++ ",".intercalate (items.map itemJson) ++ "]"

/-- The ratings. -/
def ratings : List Nat := [0, 1, 2, 3, 4, 5, 9]

/-- The gaps, in days. -/
def gaps : List F32 :=
  [.nan, .negInf, .posInf, .fin (-4), .fin (-1 / 8), .fin 0, .fin (1 / 2), .fin 10, .fin 100000,
    .fin 140737488355328]

/-- Every history, in a fixed order. -/
def histories : List (List Review) :=
  [[]] ++
  (ratings.flatMap fun r => gaps.map fun g => [⟨r, g⟩]) ++
  (ratings.flatMap fun r1 => ratings.flatMap fun r2 => gaps.map fun g =>
    [⟨r1, .fin 0⟩, ⟨r2, g⟩]) ++
  ([0, 1, 3].flatMap fun r2 => gaps.flatMap fun g2 => [1, 4].flatMap fun r3 =>
    [.nan, .fin (1 / 2), .fin 10].map fun g3 => [⟨3, .fin 0⟩, ⟨r2, g2⟩, ⟨r3, g3⟩])

/-- Every input: each history alone, beside itself, and beside the next. -/
def inputs : List (List (List Review)) :=
  (histories.map fun h => [h]) ++ (histories.map fun h => [h, h]) ++
    (histories.zip histories.tail).map fun (a, b) => [a, b]

/-- One vector line: the histories as they arrive, and as the port reads them. -/
def vectorLine (items : List (List Review)) : String :=
  "{\"items\":" ++ itemsJson items ++ ",\"normal\":" ++
    itemsJson (items.map (·.map Review.normal)) ++ "}"

/-- The stabilities of the stability update's states, in days: the least power of two at or above
the floor, then 1/2, 1, 10, 4096 and the ceiling. -/
def stepStabilities : List Rat := [1 / 8192, 1 / 2, 1, 10, 4096, 36500]

/-- The difficulties: the floor, 11/2 and the ceiling. -/
def stepDifficulties : List Rat := [1, 11 / 2, 10]

/-- The fast stabilities. -/
def stepFastStabilities : List Rat := [1 / 8192, 3, 36500]

/-- Every state in the bounds the update starts from, beside every gap. -/
def steps : List (State × F32) :=
  stepStabilities.flatMap fun s => stepDifficulties.flatMap fun d =>
    stepFastStabilities.flatMap fun sf => gaps.map fun g => (⟨.fin s, .fin d, .fin sf⟩, g)

/-- A finite value as its tagged integers. -/
def ratJson (q : Rat) : String := f32Json (.fin q)

/-- One stability-update line: the state, the gap, and for the slow and the fast stability the
bounds the claims give: at least `min s sMax` after a success, at most `max s sMin` after a
lapse. -/
def stepLine (st : State) (g : F32) : String :=
  let bounds (x : F32) : String × String :=
    match x with
    | .fin q => (ratJson (min q sMax), ratJson (max q sMin))
    | _ => ("[0]", "[0]")
  let (sLeast, sMost) := bounds st.s
  let (fLeast, fMost) := bounds st.sf
  "{\"state\":[" ++ f32Json st.s ++ "," ++ f32Json st.d ++ "," ++ f32Json st.sf ++
    "],\"elapsed\":" ++ f32Json g ++ ",\"atLeast\":" ++ sLeast ++ ",\"atMost\":" ++ sMost ++
    ",\"fastAtLeast\":" ++ fLeast ++ ",\"fastAtMost\":" ++ fMost ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/Fsrs7MemoryState.lean"
  match coversOf module with
  | [] =>
    IO.eprintln "Formal/Fsrs7MemoryState.lean names no covered item"
    return 1
  | (path, anchor, digest) :: rest =>
    let digests := digest :: rest.map fun (_, _, recorded) => recorded
    IO.println (headerLine "Fsrs7MemoryState" path anchor digest digests
      (inputs.length + steps.length))
    for items in inputs do
      IO.println (vectorLine items)
    for (st, g) in steps do
      IO.println (stepLine st g)
    return 0

end Formal.Fsrs7MemoryStateVectors
