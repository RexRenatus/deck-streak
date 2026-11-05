import Formal.RetiredListShape

/-!
# Formal.RetiredListShapeVectors

The retired list reader's half of the vector writer (#421, SPEC-465). `Formal/Vectors.lean`
dispatches `lean --run Formal/Vectors.lean RetiredListShape` to `run`, which prints one header line
naming the covered item and the digest the entry recorded for it, then one line per document with
the answer of the reading that ships: the sentence it refuses the document with, or whether the
record it reads approves the row `S00030-DOUBLE`. The formal checker byte-compares the output with
`formal/vectors/retired-list-shape.jsonl`, and `--write-vectors` writes it; the file is never
edited by hand.

The documents are each wrong shape R1 to R4 name (a top-level array, string, number, boolean and
null; a `retired` value that is a number, null, an object, a string or a boolean; an entry that is
not an object, and one with no `id` or an `id` that is not a string, after a well-formed entry),
and well-formed lists that approve the row or do not (no `retired` key, a list holding `_`, a
blank or missing reason or approval, and a reason and an approval that are not strings).
SPEC-465's A14 writes each document as a fixture's retired list and reads `retired`'s answer.
This module carries no header line, so the registry lists it as support.
-/

namespace Formal.RetiredListShapeVectors

open Formal.RetiredListShape

/-- Four lower-case hex digits of a code point below 65536. -/
def hex4 (n : Nat) : String :=
  let digits := Nat.toDigits 16 n
  String.ofList (List.replicate (4 - digits.length) '0' ++ digits)

/-- One character of a JSON string: a quote and a backslash escaped, and every control or
non-ASCII character written as its `\u` escape. -/
def escapeChar (c : Char) : String :=
  if c = '"' then "\\\""
  else if c = '\\' then "\\\\"
  else if c.toNat < 32 || c.toNat > 126 then "\\u" ++ hex4 c.toNat
  else c.toString

/-- A JSON string. -/
def jsonString (text : List Char) : String :=
  "\"" ++ String.join (text.map escapeChar) ++ "\""

mutual
  /-- A document as JSON text, its object keys in their order. -/
  def render : Json → String
    | .obj fields => "{" ++ renderFields fields ++ "}"
    | .arr items => "[" ++ renderItems items ++ "]"
    | .str text => jsonString text
    | .num value => toString value
    | .bool value => if value then "true" else "false"
    | .null => "null"

  /-- An object's keys and values, comma separated. -/
  def renderFields : List (List Char × Json) → String
    | [] => ""
    | [(key, value)] => jsonString key ++ ":" ++ render value
    | (key, value) :: rest => jsonString key ++ ":" ++ render value ++ "," ++ renderFields rest

  /-- An array's items, comma separated. -/
  def renderItems : List Json → String
    | [] => ""
    | [item] => render item
    | item :: rest => render item ++ "," ++ renderItems rest
end

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
  "{\"schema\":\"phx.formal.vectors.v1\",\"entry\":" ++ jsonString entry.toList ++
    ",\"covers\":" ++ jsonString path.toList ++ ",\"anchor\":" ++ jsonString anchor.toList ++
    ",\"digest\":" ++ jsonString digest.toList ++ ",\"vectors\":" ++ toString count ++ "}"

/-- A text value. -/
def s (text : String) : Json :=
  .str text.toList

/-- An object of text keys. -/
def o (fields : List (String × Json)) : Json :=
  .obj (fields.map fun field => (field.1.toList, field.2))

/-- The row the fixture's list retires or leaves refused. -/
def leaving : String :=
  "S00030-DOUBLE"

/-- An entry naming `id`, with its reason and its approval. -/
def entry (id : String) (reason approval : Json) : Json :=
  o [("id", s id), ("reason", reason), ("approval", approval)]

/-- The leaving row's approved entry. -/
def approved : Json :=
  entry leaving (s "moved") (s "the maintainer, in #1")

/-- Another row's approved entry, well-formed, placed first where a later entry is wrong. -/
def other : Json :=
  entry "S00031-OTHER" (s "moved") (s "the maintainer, in #1")

/-- A document whose `retired` holds `entries`. -/
def listing (entries : List Json) : Json :=
  o [("retired", .arr entries)]

/-- Every document, in a fixed order. -/
def documents : List Json :=
  [ .arr [approved], .arr [], s "x", .num 1, .null, .bool true,
    o [("retired", .num 1)], o [("retired", .null)], o [("retired", o [])],
    o [("retired", s "")], o [("retired", s "x")], o [("retired", .bool true)],
    listing [other, .num 1], listing [s leaving],
    listing [other, o [("reason", s "moved"), ("approval", s "the maintainer, in #1")]],
    listing [other, o [("id", .arr [s leaving]), ("reason", s "moved"),
      ("approval", s "the maintainer, in #1")]],
    listing [o [("id", o [("row", s leaving)]), ("reason", s "moved"),
      ("approval", s "the maintainer, in #1")]],
    listing [o [("id", .num 30), ("reason", s "moved"), ("approval", s "the maintainer, in #1")]],
    listing [approved, .num 1],
    o [], o [("_", .arr [s "a note"])], o [("_", .arr [s "a note"]), ("retired", .arr [approved])],
    listing [], listing [approved],
    listing [entry "S00031-OTHER" (s "moved") (s ""), approved],
    listing [entry leaving (s "moved") (s "   ")],
    listing [entry leaving (s "moved") (s "\t \n 　")],
    listing [entry leaving (s "") (s "the maintainer, in #1")],
    listing [o [("id", s leaving), ("reason", s "moved")]],
    listing [o [("id", s leaving), ("approval", s "the maintainer, in #1")]],
    listing [entry leaving (.num 7) (.bool true)],
    listing [entry leaving .null (s "the maintainer, in #1")],
    listing [entry leaving (.arr []) (o [])] ]

/-- Whether the record holds the leaving row's id. -/
def approves (kept : List (List Char)) : Bool :=
  kept.any fun id => same id leaving.toList

/-- One vector line: the document and the answer of the reading that ships. -/
def vectorLine (document : Json) : String :=
  let answer :=
    match retirementRecords document with
    | .refused sentence => "\"refused\":" ++ jsonString sentence
    | .ids kept => "\"approves\":" ++ (if approves kept then "true" else "false")
    | .crashed => "\"crashed\":true"
  "{\"document\":" ++ render document ++ "," ++ answer ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/RetiredListShape.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/RetiredListShape.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine "RetiredListShape" path anchor digest documents.length)
    for document in documents do
      IO.println (vectorLine document)
    return 0

end Formal.RetiredListShapeVectors
