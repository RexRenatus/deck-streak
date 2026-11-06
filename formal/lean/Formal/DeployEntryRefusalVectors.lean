import Formal.DeployEntryRefusal

/-!
# Formal.DeployEntryRefusalVectors

The deploy settings check's half of the vector writer (#484, SPEC-473). `Formal/Vectors.lean`
dispatches `lean --run Formal/Vectors.lean DeployEntryRefusal` to `run`, which prints one header
line naming the covered function and the digest the entry recorded for it, then one line per input
with the block's entries as hex, the names a start-up file makes as variables, and the port's
answer: `proceed`, or `refuse` with the refusal's whole stderr line as hex. The formal checker
byte-compares the output with `formal/vectors/deploy-entry-refusal.jsonl`, and `--write-vectors`
writes it; the file is never edited by hand.

Each answer is the port's, with the shell's variables modelled as the names bash makes of the
block's entries (each entry that holds `=` and whose key is a shell name, cut at its first `=`, in
order), followed by the made names. No input gives the shell more than one prefixed name that is
not listed, so the answer does not depend on the order bash lists its variables in. SPEC-473's A2
runs `deploy.sh caddy-install` with no tag on each block exactly and compares the exit status and
the whole stderr with the line's. This module carries no header line, so the registry lists it as
support.
-/

namespace Formal.DeployEntryRefusalVectors

open Formal.DeployEntryRefusal

/-- A JSON string. The writer quotes only paths, item names, verdicts, hex and the made names,
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

/-- One hexadecimal digit, lower case. -/
def hexDigit (n : Nat) : Char :=
  if n < 10 then Char.ofNat (48 + n) else Char.ofNat (87 + n)

/-- Bytes as lower-case hex, two digits each. -/
def hex (b : Bytes) : String :=
  String.join (b.map fun c => String.ofList [hexDigit (c.toNat / 16), hexDigit (c.toNat % 16)])

/-- A made name's bytes as text: every made name is ASCII. -/
def text (b : Bytes) : String :=
  String.ofList (b.map fun c => Char.ofNat c.toNat)

/-- A byte that may begin a shell name: a letter A to Z or a to z, or `_`. -/
def isNameStart (c : UInt8) : Bool :=
  decide (65 ≤ c ∧ c ≤ 90) || decide (97 ≤ c ∧ c ≤ 122) || decide (c = 95)

/-- A byte that may follow in a shell name: those, and a digit. -/
def isNameByte (c : UInt8) : Bool :=
  isNameStart c || decide (48 ≤ c ∧ c ≤ 57)

/-- A shell name. -/
def shellName : Bytes → Bool
  | [] => false
  | c :: rest => isNameStart c && rest.all isNameByte

/-- The names bash makes variables of: each entry that holds `=` and whose key is a shell name,
cut at its first `=`, in order. -/
def imported (block : List Bytes) : List Bytes :=
  (block.filter fun e => e.contains 61 && shellName (cut e)).map cut

/-- A setting's entry: the name, `=`, and the value; the host setting's value is always empty. -/
def setting (name value : Bytes) : Bytes :=
  if name = ascii "DECKSTREAK_DEPLOY_HOST" then name ++ ascii "=" else name ++ ascii "=" ++ value

/-- The inputs whose step proceeds past the check, each a block's entries and the made names. -/
def proceeding : List (List Bytes × List Bytes) :=
  [([[]], []), ([[], []], []), ([ascii "A=1"], []), ([ascii "NOEQ"], []), ([ascii "=lead"], []),
    ([ascii "a b=c"], []), ([[255] ++ ascii "=1"], []),
    ([ascii "DECKSTREAK_DEPLOYX=1", ascii "deckstreak_deploy_repo=1",
      ascii "XDECKSTREAK_DEPLOY_REPO=1", ascii "DECKSTREAK_DEPLOY"], [])] ++
  listedSettings.map (fun name => ([setting name (ascii "x")], [])) ++
  [(listedSettings.map fun name => setting name (ascii "x"), []),
    (listedSettings.reverse.map fun name => setting name (ascii "x"), []),
    ([ascii "DECKSTREAK_DEPLOY_REPO="], []),
    ([ascii "DECKSTREAK_DEPLOY_REPO=a=b"], []),
    ([ascii "DECKSTREAK_DEPLOY_REPO=" ++ [255, 254], ascii "DECKSTREAK_DEPLOY_KEEP=" ++ [226, 128, 139]],
      []),
    ([ascii "DECKSTREAK_DEPLOY_CADDY_DIR=/x", [], ascii "A=1", ascii "NOEQ"], []),
    ([ascii "A=1"], [ascii "DECKSTREAK_DEPLOY_REPO"])]

/-- The inputs whose step the check refuses. -/
def refusing : List (List Bytes × List Bytes) :=
  [([], []), ([], [ascii "DECKSTREAK_DEPLOY_REPO"]), ([], [ascii "DECKSTREAK_DEPLOY_ZED"]),
    ([ascii "DECKSTREAK_DEPLOY_REPO"], []),
    ([ascii "DECKSTREAK_DEPLOY_REPO=a", ascii "DECKSTREAK_DEPLOY_REPO"], []),
    ([ascii "DECKSTREAK_DEPLOY_REPO", ascii "DECKSTREAK_DEPLOY_REPO=a"], [])] ++
  listedSettings.map (fun name => ([setting name (ascii "a"), setting name (ascii "b")], [])) ++
  [([ascii "DECKSTREAK_DEPLOY_CADDY_DIR=/x", ascii "DECKSTREAK_DEPLOY_CADDY_DIR="], []),
    ([ascii "DECKSTREAK_DEPLOY_=x"], []),
    ([ascii "DECKSTREAK_DEPLOY_CADDY-DIR=/x"], []),
    ([pfx ++ [1] ++ ascii "=/x"], []),
    ([ascii "DECKSTREAK_DEPLOY_*=1"], []),
    ([ascii "DECKSTREAK_DEPLOY_REPO *=1"], []),
    ([ascii "DECKSTREAK_DEPLOY_CADDY_DIR\n=1"], []),
    ([ascii "DECKSTREAK_DEPLOY_CADDY"], []),
    ([ascii "DECKSTREAK_DEPLOY_REPO2=x"], []),
    ([pfx ++ [195, 169] ++ ascii "=1"], []),
    ([ascii "DECKSTREAK_DEPLOY_REPO" ++ [226, 128, 139] ++ ascii "=1"], []),
    ([pfx ++ [255] ++ ascii "=1"], []),
    ([ascii "A=1"], [ascii "DECKSTREAK_DEPLOY_ZED"]),
    ([ascii "DECKSTREAK_DEPLOY_REPO", ascii "DECKSTREAK_DEPLOY_ZED=1"], []),
    ([ascii "DECKSTREAK_DEPLOY_REPO=a", ascii "DECKSTREAK_DEPLOY_CADDY-DIR=/x",
      ascii "DECKSTREAK_DEPLOY_REPO=b"], []),
    ([ascii "DECKSTREAK_DEPLOY_CADDY_DIR=/x", ascii "DECKSTREAK_DEPLOY_CADDYFILE=/y",
      ascii "DECKSTREAK_DEPLOY_CADDY_CONFIG=/z", ascii "DECKSTREAK_DEPLOY_CADDYFILE=/w"], [])]

/-- Every input, in a fixed order: those that proceed, then those refused. -/
def inputs : List (List Bytes × List Bytes) := proceeding ++ refusing

/-- The prefixed names of a list of variable names that are not listed. -/
def unlistedNames (vars : List Bytes) : List Bytes :=
  vars.filter fun v => startsWith pfx v && !named v listedSettings

/-- Each input gives the shell at most one prefixed name that is not listed, so the order bash
lists its variables in moves no answer. -/
theorem each_input_gives_the_shell_at_most_one_unlisted_name :
    ∀ input ∈ inputs, (unlistedNames (imported input.1 ++ input.2)).length ≤ 1 := by
  decide +kernel

/-- The port's answer on an input. -/
def answer (input : List Bytes × List Bytes) : Verdict :=
  refuseUnlistedSettings (imported input.1 ++ input.2) (some (layout input.1))

/-- An answer, as the vectors spell it. -/
def verdictText : Verdict → String
  | .proceed => "\"verdict\":\"proceed\""
  | .refuse fault => "\"verdict\":\"refuse\",\"said\":" ++ jsonString (hex fault.line)

/-- One vector line: the block's entries as hex, the made names, and the port's answer. -/
def vectorLine (input : List Bytes × List Bytes) : String :=
  "{\"entries\":[" ++ ",".intercalate (input.1.map fun e => jsonString (hex e)) ++
    "],\"made\":[" ++ ",".intercalate (input.2.map fun m => jsonString (text m)) ++ "]," ++
    verdictText (answer input) ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/DeployEntryRefusal.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/DeployEntryRefusal.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine "DeployEntryRefusal" path anchor digest inputs.length)
    for input in inputs do
      IO.println (vectorLine input)
    return 0

end Formal.DeployEntryRefusalVectors
