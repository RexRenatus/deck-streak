import Formal.XpReconciliation

/-!
# Formal.XpReconciliationVectors

The XP floor's half of the vector writer (SPEC-389 R10, R11). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean XpReconciliation` to `run`, which prints one header line naming the
first covered item, the digest the entry recorded for it and the axes, then the settle vectors and
the trace vectors. The formal checker byte-compares the output with
`formal/vectors/xp-reconciliation.jsonl`, and `--write-vectors` writes it; the file is never edited
by hand.

A settle vector is one input of the settle rule: a held row of none, or of an amount of the axis,
open or closed; a request of an amount of the axis, open or closed; and either cause. It records the
row held after. A trace vector is one word of length 1 to 4 over six letters. G is a language
answer shown at 7 and priced at 7. H is a law answer shown at 7 and priced at 14. U is an undo, S a
sync. R is one fold's read and write, with a `score90` amount of 200 when the server holds an odd
number of answers, else 0. N is the close, then one fold's read and write with a `score90` amount
of 0. It records every settle, with its step, source, request and the row held after, and every
settle point, with its step, shown XP and confirmed XP. Steps count from 1. This module carries no
header line, so the registry lists it as support.
-/

namespace Formal.XpReconciliationVectors

open Formal.XpReconciliation

/-- A JSON string. The writer quotes only paths, item names, hex digests, sources, causes and
words, which carry no character JSON escapes. -/
def jsonString (s : String) : String :=
  "\"" ++ s ++ "\""

/-- A JSON boolean. -/
def jsonBool (b : Bool) : String :=
  if b then "true" else "false"

/-- A JSON array of `items`. -/
def jsonArray (items : List String) : String :=
  "[" ++ ",".intercalate items ++ "]"

/-- The first `covers` line of a module: its path, item and recorded digest. -/
def coverOf (module : String) : Option (String × String × String) :=
  (module.splitOn "\n").findSome? fun line =>
    match (line.splitOn " ").filter (· ≠ "") with
    | [_, _, "covers", path, anchor, digest] =>
      if anchor.startsWith "anchor=" && digest.startsWith "digest=" then
        some (path, (anchor.drop 7).toString, (digest.drop 7).toString)
      else none
    | _ => none

/-- The amounts a row or a request holds. -/
def amounts : List Nat := [0, 30, 50, 4294967295]

/-- The trace letters, in their order. -/
def alphabet : List Char := "GHUSRN".toList

/-- The longest word. -/
def maxLength : Nat := 4

/-- Every row of the axis: each amount, open then closed. -/
def rows : List Row :=
  amounts.flatMap fun amount => [false, true].map fun closed => ⟨amount, closed⟩

/-- Every held row: none, then each row of the axis. -/
def helds : List (Option Row) :=
  none :: rows.map some

/-- Both causes. -/
def causes : List Cause :=
  [.recompute, .ownersCorrection]

/-- Every settle input: each held row, each request, each cause. -/
def settleCases : List (Option Row × Row × Cause) :=
  helds.flatMap fun held => rows.flatMap fun request => causes.map fun cause => (held, request, cause)

/-- Every word of length `n` over the alphabet, in the alphabet's order. -/
def wordsOf : Nat → List (List Char)
  | 0 => [[]]
  | n + 1 => alphabet.flatMap fun c => (wordsOf n).map (c :: ·)

/-- Every word of length 1 to `maxLength`, shortest first. -/
def words : List (List Char) :=
  (List.range maxLength).flatMap fun n => wordsOf (n + 1)

/-- G: a language answer the device shows at 7 and the server prices at 7. -/
def languageAnswer : Answer := ⟨some .language, 7, 7⟩

/-- H: a law answer the device shows at 7 and the server prices at 14. -/
def lawAnswer : Answer := ⟨some .law, 7, 14⟩

/-- The `score90` amount of an R letter's write: 200 when the server holds an odd number of the
day's answers, else 0. -/
def oddBonus (s : State) : Nat :=
  if s.server.length % 2 = 1 then 200 else 0

/-- The events one letter stands for, in the state it starts from. -/
def letterEvents (s : State) : Char → List Event
  | 'G' => [.grade languageAnswer]
  | 'H' => [.grade lawAnswer]
  | 'U' => [.undo]
  | 'S' => [.sync]
  | 'R' => [.read 0, .write 0 (oddBonus s)]
  | 'N' => [.close, .read 0, .write 0 0]
  | _ => []

/-- A row as JSON: its amount and closed flag. -/
def rowJson (row : Row) : String :=
  "[" ++ toString row.amount ++ "," ++ jsonBool row.closed ++ "]"

/-- A cause as JSON. -/
def causeJson : Cause → String
  | .recompute => jsonString "recompute"
  | .ownersCorrection => jsonString "owners_correction"

/-- One settle vector line. -/
def settleLine (input : Option Row × Row × Cause) : String :=
  let (held, request, cause) := input
  let heldJson := match held with
    | none => "null"
    | some row => rowJson row
  "{\"kind\":\"settle\",\"held\":" ++ heldJson ++ ",\"request\":" ++ rowJson request ++
    ",\"cause\":" ++ causeJson cause ++ ",\"after\":" ++ rowJson (settleRule held request cause) ++
    "}"

/-- The settles one event makes from `s`: each source, its request and the row held after. -/
def settlesOf (s : State) (event : Event) : List (String × Row × Row) :=
  match event with
  | .write fold bonus =>
    match s.reads.lookup fold with
    | none => []
    | some facts =>
      let after := step s event
      let requests := writeRequests s facts bonus
      [("reviews", requests.1, after.reviews.getD ⟨0, false⟩),
        ("reviews_law", requests.2.1, after.reviewsLaw.getD ⟨0, false⟩),
        ("score90", requests.2.2, after.score90.getD ⟨0, false⟩)]
  | _ => []

/-- One settle of a trace as JSON: its step, source, request and the row held after. -/
def settleItem (stepNumber : Nat) (settle : String × Row × Row) : String :=
  let (source, request, after) := settle
  "[" ++ toString stepNumber ++ "," ++ jsonString source ++ "," ++ toString request.amount ++ "," ++
    jsonBool request.closed ++ "," ++ toString after.amount ++ "," ++ jsonBool after.closed ++ "]"

/-- One settle point as JSON: its step, shown XP and confirmed XP. -/
def pointItem (stepNumber : Nat) (s : State) : String :=
  "[" ++ toString stepNumber ++ "," ++ toString (shownXp s) ++ "," ++ toString (confirmed s) ++ "]"

/-- The events of one letter applied in order from `s`, with the settles they make. -/
def applyEvents (stepNumber : Nat) (s : State) (events : List Event) : State × List String :=
  events.foldl (fun (acc : State × List String) event =>
    (step acc.1 event, acc.2 ++ (settlesOf acc.1 event).map (settleItem stepNumber))) (s, [])

/-- A word's settles and settle points, from `s` at step `stepNumber`. -/
def replay (s : State) (stepNumber : Nat) : List Char → List String × List String
  | [] => ([], [])
  | letter :: rest =>
    let (next, settles) := applyEvents stepNumber s (letterEvents s letter)
    let point := if settlePoint next then [pointItem stepNumber next] else []
    let (laterSettles, laterPoints) := replay next (stepNumber + 1) rest
    (settles ++ laterSettles, point ++ laterPoints)

/-- One trace vector line. -/
def traceLine (word : List Char) : String :=
  let (settles, points) := replay init 1 word
  "{\"kind\":\"trace\",\"trace\":" ++ jsonString (String.ofList word) ++ ",\"settles\":" ++
    jsonArray settles ++ ",\"points\":" ++ jsonArray points ++ "}"

/-- The header line: what the vectors were written from, the axes, and how many follow. -/
def headerLine (path anchor digest : String) : String :=
  "{\"schema\":\"phx.formal.vectors.v1\",\"entry\":" ++ jsonString "XpReconciliation" ++
    ",\"covers\":" ++ jsonString path ++ ",\"anchor\":" ++ jsonString anchor ++
    ",\"digest\":" ++ jsonString digest ++ ",\"alphabet\":" ++
    jsonString (String.ofList alphabet) ++ ",\"max_length\":" ++ toString maxLength ++
    ",\"amounts\":" ++ jsonArray (amounts.map toString) ++ ",\"vectors\":" ++
    toString (settleCases.length + words.length) ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/XpReconciliation.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/XpReconciliation.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine path anchor digest)
    for input in settleCases do
      IO.println (settleLine input)
    for word in words do
      IO.println (traceLine word)
    return 0

end Formal.XpReconciliationVectors
