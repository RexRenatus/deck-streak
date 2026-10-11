import Formal.DueReorder

/-!
# Formal.DueReorderVectors

The reorder-only input's half of the vector writer (#824). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean DueReorder` to `run`, which prints one header line naming the first
covered item, the digest the entry recorded for it and every cover's digest (`null` and an empty
list while the entry covers nothing), then one line per input with the order the port presents. The
formal checker byte-compares the output with `formal/vectors/due-reorder.jsonl`, and
`--write-vectors` writes it; the file is never edited by hand.

Every line is read at today's study day 3 and snapshot 7. A queue entry is written as its card, due
date, interval, ease, difficulty and stability, each card's fields distinct from every other's, and
a score as the text `str::parse::<f64>` reads (`"0.25"`, `"-0"`, `"NaN"`, `"inf"`, `"-inf"`). The
inputs are:

- every arrangement of three cards, each card unscored or scored 0, 0.5, 1 or -0, so ties between
  equal scores and between 0 and -0 meet every stock order;
- four cards in stock order, each unscored or scored 0.25 or 0.75;
- three cards in stock order, each unscored or scored 0.5, NaN, inf, -inf, 1.25 or -0.25, so every
  invalid score meets a valid one and an absent one;
- the input switched off, absent, keyed to another day, another snapshot or both, an empty map, a
  map that scores only a card the queue lacks, a map whose every queued score is invalid, an empty
  queue, a one-card queue, and an applied map that also scores a card the queue lacks.

This module carries no header line, so the registry lists it as support.
-/

namespace Formal.DueReorderVectors

open Formal.DueReorder

/-- A JSON string. The writer quotes only paths, item names, hex digests and score text, which
carry no character JSON escapes. -/
def jsonString (s : String) : String :=
  "\"" ++ s ++ "\""

/-- A JSON string, or `null`. -/
def jsonOrNull : Option String → String
  | some s => jsonString s
  | none => "null"

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
def headerLine (entry : String) (covers : List (String × String × String)) (count : Nat) :
    String :=
  let first := covers.head?
  "{\"schema\":\"phx.formal.vectors.v1\",\"entry\":" ++ jsonString entry ++
    ",\"covers\":" ++ jsonOrNull (first.map (·.1)) ++
    ",\"anchor\":" ++ jsonOrNull (first.map (·.2.1)) ++
    ",\"digest\":" ++ jsonOrNull (first.map (·.2.2)) ++ ",\"digests\":[" ++
    ",".intercalate (covers.map fun cover => jsonString cover.2.2) ++ "],\"vectors\":" ++
    toString count ++ "}"

/-- Today's study day, for every line. -/
def today : Int := 3

/-- The collection's snapshot id, for every line. -/
def snapshot : Int := 7

/-- A multiple of a quarter as decimal text. -/
def quarterText (q : Int) : String :=
  let sign := if q < 0 then "-" else ""
  let n := q.natAbs
  let frac := match n % 4 with
    | 1 => ".25"
    | 2 => ".5"
    | 3 => ".75"
    | _ => ""
  sign ++ toString (n / 4) ++ frac

/-- A score as the text `str::parse::<f64>` reads. -/
def codeText : Code → String
  | .quarter q => quarterText q
  | .negZero => "-0"
  | .nan => "NaN"
  | .inf => "inf"
  | .negInf => "-inf"

/-- A card's queue entry: fields distinct from every other card's. -/
def entryOf (card : Int) : Entry :=
  ⟨card, 100 + card, card - 8, 2000 + 10 * card, card % 10, 30 + card⟩

/-- An entry as a JSON array: card, due, interval, ease, difficulty, stability. -/
def entryJson (e : Entry) : String :=
  "[" ++ ",".intercalate ([e.card, e.due, e.interval, e.ease, e.difficulty, e.stability].map
    toString) ++ "]"

/-- A list of entries as a JSON array. -/
def entriesJson (es : List Entry) : String :=
  "[" ++ ",".intercalate (es.map entryJson) ++ "]"

/-- One input: the switch, the input's day, snapshot and scores, and the queue's cards. -/
structure Case where
  /-- The switch. -/
  on : Bool
  /-- The input's day, snapshot and scores, or none. -/
  input : Option (Int × Int × List (Int × Code))
  /-- The queue's cards, in stock order. -/
  queue : List Int

/-- The input a case names. -/
def inputOf (input : Int × Int × List (Int × Code)) : Input Code :=
  ⟨input.1, input.2.1, fun card => input.2.2.lookup card⟩

/-- An input as JSON, `null` when absent. -/
def inputJson : Option (Int × Int × List (Int × Code)) → String
  | none => "null"
  | some (day, snap, scores) =>
    "{\"day\":" ++ toString day ++ ",\"snapshot\":" ++ toString snap ++ ",\"scores\":[" ++
      ",".intercalate (scores.map fun (card, code) =>
        "[" ++ toString card ++ "," ++ jsonString (codeText code) ++ "]") ++ "]}"

/-- Every map that gives each card one of the codes, `none` meaning no score. -/
def assignments (codes : List (Option Code)) : List Int → List (List (Int × Code))
  | [] => [[]]
  | card :: cards => codes.flatMap fun code => (assignments codes cards).map fun rest =>
    match code with
    | some k => (card, k) :: rest
    | none => rest

/-- No score, 0, 0.5, 1 and -0. -/
def orderCodes : List (Option Code) :=
  [none, some (.quarter 0), some (.quarter 2), some (.quarter 4), some .negZero]

/-- No score, 0.25 and 0.75. -/
def fourCodes : List (Option Code) :=
  [none, some (.quarter 1), some (.quarter 3)]

/-- No score, 0.5, and every invalid kind: NaN, inf, -inf, above 1 and below 0. -/
def invalidCodes : List (Option Code) :=
  [none, some (.quarter 2), some .nan, some .inf, some .negInf, some (.quarter 5),
    some (.quarter (-1))]

/-- Every arrangement of three cards. -/
def arrangements : List (List Int) :=
  [[11, 12, 13], [11, 13, 12], [12, 11, 13], [12, 13, 11], [13, 11, 12], [13, 12, 11]]

/-- Three valid, distinct scores. -/
def mixed : List (Int × Code) :=
  [(11, .quarter 1), (12, .quarter 4), (13, .quarter 2)]

/-- Every input, in a fixed order. -/
def cases : List Case :=
  (arrangements.flatMap fun queue =>
      (assignments orderCodes [11, 12, 13]).map fun m => ⟨true, some (today, snapshot, m), queue⟩) ++
    ((assignments fourCodes [11, 12, 13, 14]).map fun m =>
      ⟨true, some (today, snapshot, m), [11, 12, 13, 14]⟩) ++
    ((assignments invalidCodes [11, 12, 13]).map fun m =>
      ⟨true, some (today, snapshot, m), [11, 12, 13]⟩) ++
    [⟨false, some (today, snapshot, mixed), [11, 12, 13]⟩,
      ⟨true, none, [11, 12, 13]⟩,
      ⟨false, none, [11, 12, 13]⟩,
      ⟨true, some (today - 1, snapshot, mixed), [11, 12, 13]⟩,
      ⟨true, some (today + 1, snapshot, mixed), [11, 12, 13]⟩,
      ⟨true, some (today, snapshot - 1, mixed), [11, 12, 13]⟩,
      ⟨true, some (today - 1, snapshot - 1, mixed), [11, 12, 13]⟩,
      ⟨true, some (today, snapshot, []), [11, 12, 13]⟩,
      ⟨true, some (today, snapshot, [(99, .quarter 4)]), [11, 12, 13]⟩,
      ⟨true, some (today, snapshot, [(11, .nan), (12, .inf), (13, .quarter 5), (99, .quarter 4)]),
        [11, 12, 13]⟩,
      ⟨true, some (today, snapshot, mixed), []⟩,
      ⟨true, some (today, snapshot, mixed), [13]⟩,
      ⟨true, some (today, snapshot, mixed ++ [(99, .quarter 4)]), [11, 12, 13]⟩]

/-- One vector line: the input and the order the port presents. -/
def caseLine (c : Case) : String :=
  let queue := c.queue.map entryOf
  let out := reorder Code codeOps c.on today snapshot (c.input.map inputOf) queue
  "{\"rule\":\"reorder\",\"on\":" ++ toString c.on ++ ",\"today\":" ++ toString today ++
    ",\"snapshot\":" ++ toString snapshot ++ ",\"input\":" ++ inputJson c.input ++
    ",\"queue\":" ++ entriesJson queue ++ ",\"output\":" ++ entriesJson out ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/DueReorder.lean"
  IO.println (headerLine "DueReorder" (coversOf module) cases.length)
  for c in cases do
    IO.println (caseLine c)
  return 0

end Formal.DueReorderVectors
