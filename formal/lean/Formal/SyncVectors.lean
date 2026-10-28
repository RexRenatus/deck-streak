import Formal.SyncClock
import Formal.ReviewKey
import Formal.FieldMerge
import Formal.PushJudge

/-!
# Formal.SyncVectors

The vector writers of the four sync entries (SPEC-504, ADR-522): `SyncClock`, `ReviewKey`,
`FieldMerge` and `PushJudge`. `Formal/Vectors.lean` dispatches each entry's name to its `run`
here, which prints one header line and then one line per input with the port's own answer. No
entry covers code yet, so each header names no covered item; the sync core's Rust tests read these
files byte-equal when it lands. The formal checker byte-compares the output with
`formal/vectors/<entry>.jsonl`, and `--write-vectors` writes it; the files are never edited by hand.
This module carries no header line, so the registry lists it as support.
-/

namespace Formal.SyncVectors

open Formal.SyncOrder

/-- The header line: the entry, no covered item, and how many vectors follow. -/
def headerLine (entry : String) (count : Nat) : String :=
  "{\"schema\":\"phx.formal.vectors.v1\",\"entry\":\"" ++ entry ++
    "\",\"covers\":null,\"anchor\":null,\"digest\":null,\"vectors\":" ++ toString count ++ "}"

def emit (entry : String) (lines : List String) : IO UInt32 := do
  IO.println (headerLine entry lines.length)
  for line in lines do
    IO.println line
  return 0

def small : List Nat := [0, 1, 2, 5]

/-! ## SyncClock -/

def clockLines : List String :=
  small.flatMap fun last => small.flatMap fun seen => small.map fun pt =>
    "{\"last\":" ++ toString last ++ ",\"seen\":" ++ toString seen ++ ",\"pt\":" ++ toString pt ++
      ",\"tick\":" ++ toString (Formal.SyncClock.tick last seen pt) ++ "}"

/-! ## ReviewKey -/

def keyJson (k : Key) : String :=
  "[" ++ toString k.hlc ++ "," ++ toString k.origin ++ "," ++ toString k.seq ++ "]"

def keysJson (l : List Key) : String :=
  "[" ++ ",".intercalate (l.map keyJson) ++ "]"

def keyInputs : List (List Key) :=
  [ [],
    [⟨1, 1, 1⟩],
    [⟨2, 1, 1⟩, ⟨1, 2, 1⟩],
    [⟨1, 2, 1⟩, ⟨1, 1, 1⟩],
    [⟨1, 1, 2⟩, ⟨1, 1, 1⟩],
    [⟨3, 1, 2⟩, ⟨1, 2, 1⟩, ⟨2, 1, 1⟩],
    [⟨2, 2, 1⟩, ⟨2, 1, 3⟩, ⟨2, 1, 2⟩, ⟨1, 3, 1⟩] ]

def keyLines : List String :=
  keyInputs.map fun l =>
    "{\"arrived\":" ++ keysJson l ++ ",\"sorted\":" ++ keysJson (Formal.ReviewKey.sortKeys l) ++ "}"

/-! ## FieldMerge -/

def editJson (e : Edit) : String :=
  "[" ++ toString e.hlc ++ "," ++ toString e.origin ++ "," ++ toString e.seq ++ "," ++
    toString e.val ++ "]"

def regJson : Option Edit → String
  | none => "null"
  | some e => editJson e

def editInputs : List (List Edit) :=
  [ [],
    [⟨1, 1, 1, 10⟩],
    [⟨1, 1, 1, 10⟩, ⟨1, 2, 1, 20⟩],
    [⟨1, 2, 1, 20⟩, ⟨1, 1, 1, 10⟩],
    [⟨2, 1, 2, 30⟩, ⟨1, 2, 1, 20⟩, ⟨1, 1, 1, 10⟩],
    [⟨1, 1, 1, 10⟩, ⟨1, 1, 1, 10⟩] ]

def mergeLines : List String :=
  editInputs.map fun l =>
    "{\"edits\":[" ++ ",".intercalate (l.map editJson) ++ "],\"register\":" ++
      regJson (Formal.FieldMerge.apply Formal.FieldMerge.merge l) ++ "}"

/-! ## PushJudge -/

open Formal.PushJudge in
def opJson (o : Op) : String :=
  "[" ++ toString o.origin ++ "," ++ toString o.seq ++ "," ++ toString o.body ++ "]"

open Formal.PushJudge in
def verdictJson : Verdict → String
  | .accepted => "\"accepted\""
  | .duplicate => "\"duplicate\""
  | .rejected => "\"rejected\""

open Formal.PushJudge in
def judgeInputs : List (List Nat × List Op × Op) :=
  [ ([], [], ⟨1, 1, 7⟩),
    ([], [⟨1, 1, 7⟩], ⟨1, 1, 7⟩),
    ([], [⟨1, 1, 7⟩], ⟨1, 1, 8⟩),
    ([], [⟨1, 1, 7⟩], ⟨1, 2, 7⟩),
    ([], [⟨1, 1, 7⟩], ⟨2, 1, 7⟩),
    ([2], [⟨1, 1, 7⟩], ⟨2, 1, 7⟩),
    ([2], [], ⟨1, 1, 7⟩) ]

def judgeLines : List String :=
  judgeInputs.map fun (revoked, log, o) =>
    let r := Formal.PushJudge.judge revoked log o
    "{\"revoked\":[" ++ ",".intercalate (revoked.map toString) ++ "],\"log\":[" ++
      ",".intercalate (log.map opJson) ++ "],\"op\":" ++ opJson o ++ ",\"verdict\":" ++
      verdictJson r.1 ++ ",\"after\":[" ++ ",".intercalate (r.2.map opJson) ++ "]}"

def runClock : IO UInt32 := emit "SyncClock" clockLines
def runKey : IO UInt32 := emit "ReviewKey" keyLines
def runMerge : IO UInt32 := emit "FieldMerge" mergeLines
def runJudge : IO UInt32 := emit "PushJudge" judgeLines

end Formal.SyncVectors
