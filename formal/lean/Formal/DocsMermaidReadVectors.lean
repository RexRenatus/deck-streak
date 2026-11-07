import Formal.DocsMermaidRead

/-!
# Formal.DocsMermaidReadVectors

The docs Mermaid reader's half of the vector writer (#493). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean DocsMermaidRead` to `run`, which prints one header line naming the
reader and its function, then one line per text with the port's reading of it: the blocks read and
the refusals, with no refusal of the reader's other rules. The web test
`web/app/src/lib/docs-mermaid-read.formal.test.ts` asserts each line against `readMermaid` itself,
and the formal checker byte-compares the output with `formal/vectors/docs-mermaid-read.jsonl`; the
file is never edited by hand.

**The fences of each text** come from `fencesOf`, a model, not a port, of the fences a CommonMark
parse opens in a text with no container: a line indented at most three columns opens a fence with
a run of three or more backticks, whose info string holds no backtick, or of three or more tildes;
a line indented at most three columns whose run of the fence's character is at least as long, with
only blanks after it, closes it; every other line of its body joins the fence's text, less up to
the fence's own indent, with a line feed; and a fence still open at the end closes there. A text
that ends with a line feed has no line after it.

**The texts** are every text of one to three lines drawn from nine lines, each line ended by a line
feed (819 texts), then 22 named texts. This module carries no header line, so the registry lists it
as support.
-/

namespace Formal.DocsMermaidReadVectors

open Formal.DocsMermaidRead

/-- The columns and the count of a line's leading blanks, a tab to the next multiple of four, and
the rest of the line. -/
def indentFrom : Nat → Nat → List Char → Nat × Nat × List Char
  | col, n, ' ' :: rest => indentFrom (col + 1) (n + 1) rest
  | col, n, '\t' :: rest => indentFrom (col + 4 - col % 4) (n + 1) rest
  | col, n, rest => (col, n, rest)

/-- A fence the model has opened and not closed. -/
structure Open where
  line : Nat
  ch : Char
  len : Nat
  offset : Nat
  info : List Char
  body : List Char

/-- A line that opens a fence: its character, its run's length, its indent and its info string. -/
def openerOf (line : List Char) : Option (Char × Nat × Nat × List Char) :=
  let (col, n, rest) := indentFrom 0 0 line
  if col ≤ 3 then
    match rest with
    | c :: _ =>
      if c == '`' then
        let run := runOf '`' rest
        if 3 ≤ run.1 && !(run.2.contains '`') then some ('`', run.1, n, run.2) else none
      else if c == '~' then
        let run := runOf '~' rest
        if 3 ≤ run.1 then some ('~', run.1, n, run.2) else none
      else none
    | [] => none
  else none

/-- A line that closes a fence of `ch` whose run is `len` long. -/
def closes (ch : Char) (len : Nat) (line : List Char) : Bool :=
  let (col, _, rest) := indentFrom 0 0 line
  let run := runOf ch rest
  col ≤ 3 && len ≤ run.1 && run.2.all isBlank

/-- A body line less up to `n` leading spaces or tabs. -/
def dropUpTo : Nat → List Char → List Char
  | 0, rest => rest
  | n + 1, c :: rest => if isBlank c then dropUpTo n rest else c :: rest
  | _ + 1, [] => []

/-- The fences of the lines from index `n`, with `o` the fence still open. -/
def fencesFrom : Option Open → Nat → List (List Char) → List Fence
  | none, _, [] => []
  | some o, _, [] => [⟨o.line, o.info, o.body⟩]
  | none, n, l :: ls =>
    match openerOf l with
    | some (ch, len, offset, info) => fencesFrom (some ⟨n + 1, ch, len, offset, info, []⟩) (n + 1) ls
    | none => fencesFrom none (n + 1) ls
  | some o, n, l :: ls =>
    if closes o.ch o.len l then ⟨o.line, o.info, o.body⟩ :: fencesFrom none (n + 1) ls
    else fencesFrom (some { o with body := o.body ++ dropUpTo o.offset l ++ ['\n'] }) (n + 1) ls

/-- The fences a CommonMark parse opens in a text with no container. -/
def fencesOf (text : List Char) : List Fence :=
  let lines := splitLines text
  let lines := if text.getLast? == some '\n' then lines.dropLast else lines
  fencesFrom none 0 lines

/-- The nine lines the grid draws from (SPEC-474 R5). -/
def gridLine : Nat → String
  | 1 => "```mermaid"
  | 2 => "~~~mermaid"
  | 3 => "```Mermaid"
  | 4 => "```text"
  | 5 => "```"
  | 6 => "~~~"
  | 7 => "    ```mermaid"
  | 8 => "flowchart TD"
  | _ => ""

/-- Every text of one, then two, then three grid lines, each length's tuples in lexicographic
order, each line ended by a line feed. -/
def gridTexts : List (String × String) :=
  let idx := (List.range 9).map (· + 1)
  let one := idx.map fun i => [i]
  let two := idx.flatMap fun i => idx.map fun j => [i, j]
  let three := idx.flatMap fun i => idx.flatMap fun j => idx.map fun k => [i, j, k]
  (one ++ two ++ three).map fun t =>
    ("grid." ++ ".".intercalate (t.map toString), String.join (t.map fun i => gridLine i ++ "\n"))

/-- The 22 named texts (SPEC-474 R5). -/
def namedTexts : List (String × String) :=
  [("none.empty-text", ""),
   ("none.prose", "flowchart TD\n"),
   ("read.backtick-fence", "```mermaid\nflowchart TD\n```\n"),
   ("read.tilde-fence", "~~~mermaid\nflowchart TD\n~~~\n"),
   ("read.blanks-around-the-word", "```  mermaid  \nflowchart TD\n```\n"),
   ("read.four-backtick-fence-holds-three", "````mermaid\nflowchart TD\n```\n````\n"),
   ("read.three-space-indent-strips-three", "   ```mermaid\n    flowchart TD\n   ```\n"),
   ("read.crlf-line-ends", "```mermaid\r\nflowchart TD\r\n```\r\n"),
   ("read.unclosed-at-the-end", "```mermaid\nflowchart TD"),
   ("refused.capital-letter", "```Mermaid\nflowchart TD\n```\n"),
   ("refused.word-and-more", "```mermaid extra\nflowchart TD\n```\n"),
   ("refused.character-reference", "```&#109;ermaid\nflowchart TD\n```\n"),
   ("refused.empty-block", "```mermaid\n```\n"),
   ("refused.blank-block", "```mermaid\n\n\n```\n"),
   ("refused.four-space-indent", "    ```mermaid\nflowchart TD\n    ```\n"),
   ("refused.tab-indent", "\t```mermaid\nflowchart TD\n"),
   ("refused.backtick-in-a-backtick-info", "```mermaid`\nflowchart TD\n```\n"),
   ("refused.backtick-in-a-tilde-info", "~~~mermaid`\nflowchart TD\n~~~\n"),
   ("witness.opener-in-a-tilde-fence", "~~~\n```mermaid\nflowchart TD\n```\n~~~\n"),
   ("refused.opener-in-a-four-backtick-fence", "````\n```mermaid\nflowchart TD\n```\n````\n"),
   ("refused.opener-in-an-unclosed-fence", "```\nflowchart TD\n```mermaid\nflowchart TD\n"),
   ("read.and-refused-in-one-text",
     "```mermaid\nflowchart TD\n```\n\n```Mermaid\nflowchart TD\n```\n")]

/-- Every text, in order: the grid, then the named texts. -/
def texts : List (String × String) :=
  gridTexts ++ namedTexts

/-- A hexadecimal digit. -/
def hexDigit (n : Nat) : String :=
  String.ofList (Nat.toDigits 16 n)

/-- One character as JSON writes it inside a string. -/
def escapeChar (c : Char) : String :=
  if c == '"' then "\\\""
  else if c == '\\' then "\\\\"
  else if c == '\n' then "\\n"
  else if c == '\r' then "\\r"
  else if c == '\t' then "\\t"
  else if c.toNat < 0x20 then "\\u00" ++ hexDigit (c.toNat / 16) ++ hexDigit (c.toNat % 16)
  else c.toString

/-- A JSON string. -/
def jsonString (cs : List Char) : String :=
  "\"" ++ String.join (cs.map escapeChar) ++ "\""

/-- The header line: what the vectors were written from, and how many follow. -/
def headerLine (count : Nat) : String :=
  "{\"schema\":\"phx.formal.vectors.v1\",\"entry\":\"DocsMermaidRead\"," ++
    "\"reader\":\"web/app/src/lib/docs-mermaid-read.js\",\"function\":\"readMermaid\"," ++
    "\"vectors\":" ++ toString count ++ "}"

/-- One block read. -/
def blockLine (b : Block) : String :=
  "{\"line\":" ++ toString b.line ++ ",\"source\":" ++ jsonString b.source ++ "}"

/-- One refusal. -/
def refusalLine (x : Refusal) : String :=
  "{\"line\":" ++ toString x.line ++ ",\"form\":" ++ jsonString x.form ++ "}"

/-- One vector line: the text's name, the text, and the port's reading of it. -/
def vectorLine (named : String × String) : String :=
  let (name, text) := named
  let r := readMermaid [] text.toList (fencesOf text.toList)
  "{\"name\":" ++ jsonString name.toList ++ ",\"text\":" ++ jsonString text.toList ++
    ",\"blocks\":[" ++ ",".intercalate (r.blocks.map blockLine) ++ "],\"refused\":[" ++
    ",".intercalate (r.refused.map refusalLine) ++ "]}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  IO.println (headerLine texts.length)
  for named in texts do
    IO.println (vectorLine named)
  return 0

end Formal.DocsMermaidReadVectors
