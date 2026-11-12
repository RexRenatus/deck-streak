import Formal.SensitiveDeck

/-!
# Formal.SensitiveDeckVectors

The kept-away rule's half of the vector writer (#751). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean SensitiveDeck` to `run`, which prints one header line naming the
first covered item and the digest the entry recorded for it, then one line per deck of a fixed
tree, with its name's parts, and one line per marked set and card, with the port's answer. The
formal checker byte-compares the output with `formal/vectors/sensitive-deck.jsonl`, and
`--write-vectors` writes it; the file is never edited by hand.

The tree holds a deck, a parent with a child and a grandchild, a deck whose name begins with the
parent's name but is not under it, a filtered deck, and a deck whose second part repeats the
parent's name. The marked sets are the unreadable set, the empty set, each deck on its own, a deck
the tree lacks, and two decks. The cards sit in their home deck, are borrowed by the filtered deck
either way, and name a deck the tree lacks on either side. This module carries no header line, so
the registry lists it as support.
-/

namespace Formal.SensitiveDeckVectors
open Formal.SensitiveDeck

/-- A JSON string. The writer quotes only paths, item names, hex digests and the tree's name
parts, which carry no character JSON escapes. -/
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

/-- The tree: each deck's id and its name's parts. -/
def treeParts : List (Int × List String) :=
  [(1, ["Default"]), (2, ["Law"]), (3, ["Law", "Contracts"]), (4, ["Law", "Contracts", "Cases"]),
    (5, ["Lawyer"]), (6, ["Filtered"]), (7, ["Lawyer", "Law"])]

/-- A name's parts joined by the separator, as characters. -/
def joinParts (parts : List String) : List Char :=
  (String.intercalate (String.singleton deckSeparator) parts).toList

/-- The tree as the rule reads it. -/
def tree : Tree := fun d => (treeParts.lookup d).map joinParts

/-- The marked sets, the unreadable one first. -/
def markedSets : List (Option (List Int)) :=
  [none, some [], some [1], some [2], some [3], some [4], some [5], some [6], some [7], some [99],
    some [3, 5]]

/-- The cards: each one's home deck, then its current deck. -/
def cards : List (Int × Int) :=
  [(1, 1), (2, 2), (3, 3), (4, 4), (5, 5), (7, 7), (4, 6), (6, 4), (1, 6), (5, 6), (8, 1), (1, 8)]

/-- Every rule input, in a fixed order: the marked set, then the card. -/
def ruleInputs : List (Option (List Int) × (Int × Int)) :=
  markedSets.flatMap fun marked => cards.map fun card => (marked, card)

/-- The answer's name. -/
def answerName : Admission → String
  | .admitted => "admitted"
  | .keptAway => "kept_away"
  | .unresolved => "unresolved"
  | .unreadable => "unreadable"

/-- A marked set as JSON: `null` when it could not be read. -/
def markedJson : Option (List Int) → String
  | none => "null"
  | some decks => "[" ++ ",".intercalate (decks.map toString) ++ "]"

/-- One tree vector line: the deck and its name's parts. -/
def treeLine (deck : Int × List String) : String :=
  "{\"rule\":\"tree\",\"deck\":" ++ toString deck.1 ++ ",\"parts\":[" ++
    ",".intercalate (deck.2.map jsonString) ++ "]}"

/-- One rule vector line: the input and the port's answer. -/
def ruleLine (input : Option (List Int) × (Int × Int)) : String :=
  let (marked, home, current) := input
  "{\"rule\":\"admits\",\"marked\":" ++ markedJson marked ++ ",\"home\":" ++ toString home ++
    ",\"current\":" ++ toString current ++ ",\"admission\":" ++
    jsonString (answerName (admits marked tree home current)) ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/SensitiveDeck.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/SensitiveDeck.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    let count := treeParts.length + ruleInputs.length
    IO.println (headerLine "SensitiveDeck" path anchor digest count)
    for deck in treeParts do
      IO.println (treeLine deck)
    for input in ruleInputs do
      IO.println (ruleLine input)
    return 0

end Formal.SensitiveDeckVectors
