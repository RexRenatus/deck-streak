import Formal.TokenProgress

/-!
# Formal.TokenProgressVectors

The token progress entry's half of the vector writer (#822, SPEC-480). `Formal/Vectors.lean`
dispatches `lean --run Formal/Vectors.lean TokenProgress` to `run`, which prints one header line
naming the first covered item and the digest the entry recorded for it, then one line per text
with the tokens the loop that ships reads at the witness helpers. The formal checker
byte-compares the output with `formal/vectors/token-progress.jsonl`, and `--write-vectors` writes
it; the file is never edited by hand.

The texts hold no quote, apostrophe, `#` or `*`, so the real `rust_literal` answers none on every
reading, as the witness helpers' `literal` does, and no block comment opens. SPEC-480's A8 reads
each text through `rust_tokens` and compares its tokens with the line's. This module carries no
header line, so the registry lists it as support.
-/

namespace Formal.TokenProgressVectors

open Formal.TokenProgress

/-- A JSON string: a quote, a backslash and a newline are escaped, the only characters among the
texts, paths, item names and hex digests the writer quotes that JSON escapes. -/
def jsonString (s : String) : String :=
  let body := s.toList.foldl (fun out c =>
    if c == '"' then out ++ "\\\""
    else if c == '\\' then out ++ "\\\\"
    else if c == '\n' then out ++ "\\n"
    else out.push c) ""
  "\"" ++ body ++ "\""

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

/-- Every text, in a fixed order: the empty text, a space, each token class alone and in
company, a `//` comment inside a line and on a line of its own, and a word that opens like a raw
string and is none. -/
def texts : List String :=
  ["", " ", "{", "a", "_", "{};", "ab c1", "_a b_", "x // c\ny", "a\n//\n", "1_2 {x}", "br x"]

/-- One token, as `[start, end]`. -/
def tokenLine (token : Nat × Nat) : String :=
  "[" ++ toString token.1 ++ "," ++ toString token.2 ++ "]"

/-- One vector line: the text and the tokens the loop that ships reads, or `none` when it reads
no tokens. -/
def vectorLine (text : String) : Option String :=
  match rustTokens witnessHelpers (text.length + 1) text.toList with
  | .tokens ts =>
    some ("{\"text\":" ++ jsonString text ++ ",\"tokens\":[" ++
      ",".intercalate (ts.map tokenLine) ++ "]}")
  | _ => none

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/TokenProgress.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/TokenProgress.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    match texts.mapM vectorLine with
    | none =>
      IO.eprintln "a text of Formal/TokenProgressVectors.lean reads no tokens"
      return 1
    | some lines =>
      IO.println (headerLine "TokenProgress" path anchor digest lines.length)
      for line in lines do
        IO.println line
      return 0

end Formal.TokenProgressVectors
