import Formal.TableCensusReach

/-!
# Formal.TableCensusReachVectors

The table census reach's half of the vector writer (#604, SPEC-331). `Formal/Vectors.lean`
dispatches `lean --run Formal/Vectors.lean TableCensusReach` to `run`, which prints one header line
naming the first covered item and the digest the entry recorded for it, then one line per tree with
the files dev's reader refuses and the files the reader that ships refuses. The formal checker
byte-compares the output with `formal/vectors/table-census-reach.jsonl`, and `--write-vectors`
writes it; the file is never edited by hand.

The inputs are SPEC-331 A1's population for each census's name, as the three census tests plant
it: at each position of the name, the piece before it and the piece after it are lone literals in
two files, and its character is a lone literal in a third, in five lone shapes, a letter in both
cases. A tree records the literals, their items, and the names that can name an item: the named
const's `MARK`, and a format template's placeholder names. This module carries no header line, so
the registry lists it as support.
-/

namespace Formal.TableCensusReachVectors

open Formal.TableCensusReach

/-- A JSON string. The writer quotes only paths, labels, item names and hex digests, which carry
no character JSON escapes. -/
def jsonString (s : String) : String :=
  "\"" ++ s ++ "\""

/-- A JSON array of strings. -/
def jsonStrings (xs : List String) : String :=
  "[" ++ ",".intercalate (xs.map jsonString) ++ "]"

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

/-- The three censuses' names. -/
def tables : List String := ["xp_ledger", "xp_settlement", "coin_ledger"]

/-- The third file of every A1 tree. -/
def charFile : String := "crates/streaks/src/a1_char.rs"

/-- The five lone shapes: each one's label, the literals it writes around a character, and the
names in it that can name an item. -/
def shapes (c : Char) : List (String × List Literal × List (String × String)) :=
  [("method argument", [⟨charFile, [c], none⟩], []),
   ("matches! arm", [⟨charFile, [c], none⟩], []),
   ("format template", [⟨charFile, [c, '{', 'n', '}'], none⟩],
     (if c.isAlpha || c = '_' then [(charFile, String.ofList [c])] else []) ++ [(charFile, "n")]),
   ("named const used alone", [⟨charFile, [c], some "MARK"⟩], [(charFile, "MARK")]),
   ("split set", [⟨charFile, [c], none⟩, ⟨charFile, ['.'], none⟩], [])]

/-- A1's trees for `table`, each with its label, in the tests' order. -/
def trees (table : String) : List (String × Tree) :=
  let name := table.toList
  (List.range name.length).flatMap fun pos =>
    let c := name.getD pos ' '
    let before := name.take pos
    let after := name.drop (pos + 1)
    let cases := if c.isAlpha then [c, c.toUpper] else [c]
    cases.flatMap fun case =>
      (shapes case).map fun (shape, literals, mentions) =>
        let lone :=
          (if before = [] then [] else [⟨"crates/quests/src/a1_before.rs", before, none⟩]) ++
            (if after = [] then [] else [⟨"crates/quests/src/a1_after.rs", after, none⟩])
        let label := table ++ "@" ++ toString pos ++ " '" ++ String.ofList [case] ++ "' " ++ shape
        (label,
          { name := name
            literals := lone ++ literals
            files := (lone ++ literals).map (·.file) |>.eraseDups
            included := []
            mentions := mentions
            named := []
            failClosed := [] })

/-- Every input, in a fixed order: the tables, then each table's trees. -/
def inputs : List (String × String × Tree) :=
  tables.flatMap fun table => (trees table).map fun (label, tree) => (table, label, tree)

/-- One vector line: the table, the tree, and the files each reader refuses. -/
def vectorLine (input : String × String × Tree) : String :=
  let (table, label, tree) := input
  "{\"table\":" ++ jsonString table ++ ",\"tree\":" ++ jsonString label ++ ",\"dev\":" ++
    jsonStrings (refusalsOld tree).eraseDups ++ ",\"reach\":" ++
    jsonStrings (refusals tree).eraseDups ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/TableCensusReach.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/TableCensusReach.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine "TableCensusReach" path anchor digest inputs.length)
    for input in inputs do
      IO.println (vectorLine input)
    return 0

end Formal.TableCensusReachVectors
