import Formal.AgentPinGroups

/-!
# Formal.AgentPinGroupsVectors

The prune pin's half of the vector writer (#474, SPEC-469). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean AgentPinGroups` to `run`, which prints one header line naming the
first covered item and the digest the entry recorded for it, then one line per input with the
port's answer. The formal checker byte-compares the output with
`formal/vectors/agent-pin-groups.jsonl`, and `--write-vectors` writes it; the file is never edited
by hand.

**The shebang vectors**, 96: for no, one and two U+FEFF marks, for each of sixteen heads (the
empty text, `#`, `#!`, `#![`, an inner attribute, `#!` before an attribute after a space, a tab, a
no-break space and an ideographic space, `#!` before a command, `#!!`, `#!#`, `#!` after a space or
a no-break space, `#[` and `!#`), for no tail and a tail of one item: the source, and whether
`opensWithAShebang` refuses it.

**The count vectors**, 576: a probe, the call `sqlx : : query !` and a parenthesis group holding the
statement, written inside one group of each delimiter, after each of eight prefixes (none, `#`,
`# !`, `!`, `! #`, `# #`, `# ! !` and `x #`), with each of six heads (none, `doc`, `r#doc`, `Doc`,
`docs` and the string `"doc"`, each but none followed by `=`), at the top or inside one brace
group, with no call beside it or a second probe after it. The source is the tree written out with
one space between tokens, a group as its opener, a space, its tokens, a space and its closer, and a
string literal as its cooked text between quotes; `run` and `delete_words` are what
`prunePinProblems` reads under the lexer that maps that source to the tree.

This module carries no header line, so the registry lists it as support.
-/

namespace Formal.AgentPinGroupsVectors
open Formal.AgentPinGroups

/-- Four lowercase hexadecimal digits of `n`, for `n` below 65536. -/
def hex4 (n : Nat) : List Char :=
  let digits := Nat.toDigits 16 n
  List.replicate (4 - digits.length) '0' ++ digits

/-- One character as a JSON string writes it: a quote and a backslash escaped, a newline and a tab
by their letters, and every other character below U+0020 or above U+007E by its four hex digits. -/
def escaped (ch : Char) : List Char :=
  if ch = '"' then ['\\', '"']
  else if ch = '\\' then ['\\', '\\']
  else if ch = '\n' then ['\\', 'n']
  else if ch = '\t' then ['\\', 't']
  else if ch.toNat < 0x20 ∨ ch.toNat > 0x7e then '\\' :: 'u' :: hex4 ch.toNat
  else [ch]

/-- A JSON string. -/
def jsonString (text : List Char) : String :=
  String.ofList ('"' :: text.flatMap escaped ++ ['"'])

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

/-- No, one and two byte order marks. -/
def marks : List (List Char) :=
  [[], ['﻿'], ['﻿', '﻿']]

/-- The sixteen heads a shebang vector opens with, after its marks. -/
def shebangHeads : List String :=
  ["", "#", "#!", "#![", "#![allow(unused)]", "#! [allow(unused)]", "#!\t[allow(unused)]",
    "#!env run-cargo-script", "#!!", "#!#", " #!", "#[", "!#", " #!",
    "#! [allow(unused)]", "#!　[allow(unused)]"]

/-- The two tails a shebang vector ends with. -/
def shebangTails : List String :=
  ["", "\nfn prune_before() {}\n"]

/-- Every shebang source, in order: the marks, then the head, then the tail. -/
def shebangSources : List (List Char) :=
  marks.flatMap fun mark => shebangHeads.flatMap fun head =>
    shebangTails.map fun tail => mark ++ head.toList ++ tail.toList

/-- One shebang vector line: the source and whether the port refuses it. -/
def shebangLine (source : List Char) : String :=
  "{\"rule\":\"opens_with_a_shebang\",\"source\":" ++ jsonString source ++ ",\"refused\":" ++
    toString (opensWithAShebang source) ++ "}"

/-- The probe: `sqlx : : query !` and a parenthesis group holding the statement. -/
def probe : List Tok :=
  [.ident "sqlx".toList, .punct ':', .punct ':', .ident "query".toList, .punct '!',
    .group .parenthesis [.lit .str prune]]

/-- The eight prefixes before the group. -/
def countPrefixes : List (List Tok) :=
  [[], [.punct '#'], [.punct '#', .punct '!'], [.punct '!'], [.punct '!', .punct '#'],
    [.punct '#', .punct '#'], [.punct '#', .punct '!', .punct '!'], [.ident ['x'], .punct '#']]

/-- The three delimiters of the group. -/
def countDelims : List Delim :=
  [.parenthesis, .bracket, .brace]

/-- The six heads the group opens with. -/
def countHeads : List (Option Tok) :=
  [none, some (.ident "doc".toList), some (.ident "r#doc".toList), some (.ident "Doc".toList),
    some (.ident "docs".toList), some (.lit .str "doc".toList)]

/-- The group's tokens: the head and `=` before the probe, or the probe alone. -/
def holding : Option Tok → List Tok
  | none => probe
  | some head => head :: .punct '=' :: probe

/-- The context: the prefix, then one group of the delimiter holding the head and the probe. -/
def context (pre : List Tok) (delim : Delim) (head : Option Tok) : List Tok :=
  pre ++ [.group delim (holding head)]

/-- The context at depth 1, or inside one brace group at depth 2. -/
def atDepth (depth : Nat) (tokens : List Tok) : List Tok :=
  if depth = 2 then [.group .brace tokens] else tokens

/-- The tokens with no call beside them, or a second probe after them at the top. -/
def beside (withProbe : Bool) (tokens : List Tok) : List Tok :=
  if withProbe then tokens ++ probe else tokens

/-- Every count tree, in order: the depth, the call beside, the prefix, the delimiter, the head. -/
def countTrees : List (List Tok) :=
  [1, 2].flatMap fun depth => [false, true].flatMap fun withProbe =>
    countPrefixes.flatMap fun pre => countDelims.flatMap fun delim =>
      countHeads.map fun head => beside withProbe (atDepth depth (context pre delim head))

/-- A delimiter's opener. -/
def openerOf : Delim → List Char
  | .parenthesis => ['(']
  | .bracket => ['[']
  | .brace => ['{']
  | .none => []

/-- A delimiter's closer. -/
def closerOf : Delim → List Char
  | .parenthesis => [')']
  | .bracket => [']']
  | .brace => ['}']
  | .none => []

mutual
/-- One token written out as Rust. -/
def renderToken : Tok → List Char
  | .ident name => name
  | .punct ch => [ch]
  | .lit _ cooked => '"' :: cooked ++ ['"']
  | .group delim inner => openerOf delim ++ ' ' :: renderTokens inner ++ ' ' :: closerOf delim

/-- Tokens written out as Rust, one space between two tokens. -/
def renderTokens : List Tok → List Char
  | [] => []
  | token :: rest =>
    renderToken token ++
      match rest with
      | [] => []
      | _ :: _ => ' ' :: renderTokens rest
end

/-- One count vector line: the tree's source and the two counts the port reads of it, under the
lexer that maps the source to the tree; nothing if the port does not read it. -/
def countLine (tree : List Tok) : Option String :=
  let source := renderTokens tree
  match prunePinProblems (fun text => if text = source then some tree else none) source with
  | .read run words =>
    some ("{\"rule\":\"counts\",\"source\":" ++ jsonString source ++ ",\"run\":" ++
      toString run ++ ",\"delete_words\":" ++ toString words ++ "}")
  | .shebang | .doesNotLex => none

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/AgentPinGroups.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/AgentPinGroups.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    let counts := countTrees.filterMap countLine
    if counts.length ≠ countTrees.length then
      IO.eprintln "a count vector's tree was not read as tokens"
      return 1
    IO.println
      (headerLine "AgentPinGroups" path anchor digest (shebangSources.length + counts.length))
    for source in shebangSources do
      IO.println (shebangLine source)
    for line in counts do
      IO.println line
    return 0

end Formal.AgentPinGroupsVectors
