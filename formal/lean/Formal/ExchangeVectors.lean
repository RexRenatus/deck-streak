import Formal.Exchange

/-!
# Formal.ExchangeVectors

The XP exchange readout's half of the vector writer (SPEC-075 R4, R5, A13). `Formal/Vectors.lean`
dispatches `lean --run Formal/Vectors.lean Exchange` to `run`, which prints one header line naming
the first covered item and the digest the entry recorded for it, then one line per input with the
port's answer: each bucket's source, XP, graduated cards and whether its rate is defined. The
formal checker byte-compares the output with `formal/vectors/exchange.jsonl`, and `--write-vectors`
writes it; the file is never edited by hand.

The inputs are every sequence of up to three rows drawn from a pool of six, against each of three
graduation maps. The pool holds two sources of one bucket paid on one day, an upper-case bucket that
sorts first, a source with no `:` and no XP, a source with two `:` and one that starts with `:`; the
maps hold no rollup, a rollup with no graduation beside a missing day, and a graduation on every
day. This module carries no header line, so the registry lists it as support.
-/

namespace Formal.ExchangeVectors

open Formal.Exchange

/-- A JSON string. The writer quotes only paths, item names, hex digests and the pool's sources,
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

/-- The rows a vector draws from. -/
def pool : List Row :=
  [⟨1, "quest:1".toList, 5⟩, ⟨1, "quest:2".toList, 7⟩, ⟨2, "Quest:9".toList, 3⟩,
    ⟨3, "reviews".toList, 0⟩, ⟨2, "a:b:c".toList, 4⟩, ⟨1, ":x".toList, 2⟩]

/-- The graduation maps a vector reads. -/
def graduationMaps : List (List (Int × Int)) :=
  [[], [(1, 2), (2, 0)], [(1, 1), (2, 3), (3, 4)]]

/-- Every sequence of up to three rows of the pool, shortest first. -/
def sequences : List (List Row) :=
  [[]] ++ pool.map (fun a => [a]) ++ pool.flatMap (fun a => pool.map fun b => [a, b]) ++
    pool.flatMap fun a => pool.flatMap fun b => pool.map fun c => [a, b, c]

/-- Every input, in a fixed order: each graduation map, then each sequence. -/
def inputs : List (List Row × List (Int × Int)) :=
  graduationMaps.flatMap fun grads => sequences.map fun rows => (rows, grads)

/-- A JSON array of `items`. -/
def jsonArray (items : List String) : String :=
  "[" ++ ",".intercalate items ++ "]"

/-- One row as JSON. -/
def rowJson (r : Row) : String :=
  "{\"day\":" ++ toString r.day ++ ",\"source\":" ++ jsonString (String.ofList r.source) ++
    ",\"amount\":" ++ toString r.amount ++ "}"

/-- One day's graduations as JSON. -/
def graduationJson (g : Int × Int) : String :=
  "{\"day\":" ++ toString g.1 ++ ",\"graduations\":" ++ toString g.2 ++ "}"

/-- One bucket's answer as JSON. -/
def rateJson (x : Rate) : String :=
  "{\"source\":" ++ jsonString (String.ofList x.source) ++ ",\"total_xp\":" ++ toString x.total ++
    ",\"graduated_cards\":" ++ toString x.graduated ++ ",\"rate_defined\":" ++
    (if x.defined then "true" else "false") ++ "}"

/-- One vector line: the input and the port's answer. -/
def vectorLine (input : List Row × List (Int × Int)) : String :=
  let (rows, grads) := input
  "{\"rows\":" ++ jsonArray (rows.map rowJson) ++ ",\"graduations\":" ++
    jsonArray (grads.map graduationJson) ++ ",\"rates\":" ++
    jsonArray ((exchangeRates rows grads).map rateJson) ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/Exchange.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/Exchange.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine "Exchange" path anchor digest inputs.length)
    for input in inputs do
      IO.println (vectorLine input)
    return 0

end Formal.ExchangeVectors
