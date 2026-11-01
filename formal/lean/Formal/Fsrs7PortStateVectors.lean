import Formal.Fsrs7PortState

/-!
# Formal.Fsrs7PortStateVectors

The port state's half of the vector writer (#823). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean Fsrs7PortState` to `run`, which prints one header line naming the
first covered item, the digest the entry recorded for it and every cover's digest, then one line
per input with the port's own answer. The formal checker byte-compares the output with
`formal/vectors/fsrs7-port-state.jsonl`, and `--write-vectors` writes it; the file is never edited
by hand.

An event is written as its eight signed fields: card, note, deck, preset, grade, answer time, key
and study day. A fold's answer is `ok` or the refusal's kind; a restore's answer is `ok:<events>`
or the load error's kind, `refused:<kind>` for an event the fold refuses. Bytes are lowercase hex.

The fold inputs are fifteen event sequences, each folded in turn into the default model's new
state: the empty one, keys in order, an equal key, a key before the last, keys across zero and at
both ends of the signed range, grades 0, 5 and 255, answer times at, past and far past the cap, a
refusal between two kept events, and fields at the ends of their ranges. Each line holds every
answer, the checkpoint of the state after them, and the restore of that checkpoint. The restore
inputs are twenty-three byte strings: a good checkpoint cut short at each check, with its magic,
format or identifier changed, one byte long, the bare header at counts 0, 1, 2^32, 2^59 and
2^64 - 1, and checkpoints holding an event the fold refuses. The `usize` is 64 bits, the target the
Rust test runs on. This module carries no header line, so the registry lists it as support.
-/

namespace Formal.Fsrs7PortStateVectors

open Formal.Fsrs7PortState

/-- A JSON string. The writer quotes only paths, item names, hex and answer kinds, which carry no
character JSON escapes. -/
def jsonString (s : String) : String :=
  "\"" ++ s ++ "\""

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
def headerLine (entry path anchor digest : String) (digests : List String) (count : Nat) :
    String :=
  "{\"schema\":\"phx.formal.vectors.v1\",\"entry\":" ++ jsonString entry ++
    ",\"covers\":" ++ jsonString path ++ ",\"anchor\":" ++ jsonString anchor ++
    ",\"digest\":" ++ jsonString digest ++ ",\"digests\":[" ++
    ",".intercalate (digests.map jsonString) ++ "],\"vectors\":" ++ toString count ++ "}"

/-- A signed 64-bit integer's bit pattern. -/
def i64 (i : Int) : Fin (2 ^ 64) := bits 64 (i % 2 ^ 64).toNat

/-- A signed 32-bit integer's bit pattern. -/
def i32 (i : Int) : Fin (2 ^ 32) := bits 32 (i % 2 ^ 32).toNat

/-- A 32-bit pattern read signed. -/
def signed32 (k : Fin (2 ^ 32)) : Int :=
  if k.val < 2 ^ 31 then k.val else (k.val : Int) - 2 ^ 32

/-- An event from its signed fields. -/
def ev (card note deck preset : Int) (grade answerMs : Nat) (key studyDay : Int) : Event :=
  ⟨i64 card, i64 note, i64 deck, i64 preset, bits 8 grade, bits 32 answerMs, i64 key, i32 studyDay⟩

/-- An event of card 1, note 2, deck 3 and preset 4, answered in a second on day 0. -/
def base (key : Int) (grade : Nat) : Event :=
  ev 1 2 3 4 grade 1000 key 0

/-- An event as its eight signed fields. -/
def eventJson (e : Event) : String :=
  s!"[{signed e.card},{signed e.note},{signed e.deck},{signed e.preset},{e.grade.val}," ++
    s!"{e.answerMs.val},{signed e.key},{signed32 e.studyDay}]"

/-- A fold refusal's kind. -/
def foldErrorTag : FoldError → String
  | .otherModel => "other_model"
  | .keyNotAfter _ _ => "key_not_after"
  | .gradeOutOfRange _ => "grade_out_of_range"
  | .answerTooLong _ => "answer_too_long"

/-- A fold's answer. -/
def foldedTag : Folded → String
  | .ok => "ok"
  | .err e => foldErrorTag e

/-- A restore's answer. -/
def loadedTag : Loaded → String
  | .ok st => s!"ok:{st.events.length}"
  | .err .truncated => "truncated"
  | .err .notACheckpoint => "not_a_checkpoint"
  | .err .otherModel => "other_model"
  | .err (.refused e) => "refused:" ++ foldErrorTag e

/-- A byte as two lowercase hex digits. -/
def hexByte (b : Byte) : String :=
  (Nat.digitChar (b.val / 16)).toString ++ (Nat.digitChar (b.val % 16)).toString

/-- Bytes as lowercase hex. -/
def hex (bs : List Byte) : String :=
  String.join (bs.map hexByte)

/-- The model the vectors fold under: the pinned revision's defaults, no parameter bits. -/
def model : Model := ⟨[]⟩

/-- The largest `usize` of the 64-bit target the Rust test runs on. -/
def usizeMax : Nat := 2 ^ 64 - 1

/-- Events folded in turn: each answer, and the state after them all. -/
def foldSeq : State → List Event → List Folded × State
  | st, [] => ([], st)
  | st, e :: es =>
    let r := fold model st e
    let rest := foldSeq r.1 es
    (r.2 :: rest.1, rest.2)

/-- The fold inputs. -/
def sequences : List (List Event) :=
  [[],
    [base 10 3],
    [base 10 3, ev 1 2 3 4 1 0 11 1, ev 7 8 9 10 4 3600000 12 2],
    [base 10 3, base 10 3],
    [base 10 3, base 9 2],
    [base (-1) 3, base 0 3],
    [base (-9223372036854775808) 3, base 9223372036854775807 3],
    [base 5 0],
    [base 5 5],
    [base 5 255],
    [ev 1 2 3 4 3 3600000 5 0],
    [ev 1 2 3 4 3 3600001 5 0],
    [ev 1 2 3 4 3 4294967295 5 0],
    [base 5 3, base 6 0, base 7 3],
    [ev (-9223372036854775808) (-1) 9223372036854775807 0 2 0 1 (-2147483648),
      ev 0 0 0 0 1 0 2 2147483647]]

/-- One fold line: the events, each fold's answer, the checkpoint of the state after them, and the
restore of that checkpoint. -/
def foldLine (es : List Event) : String :=
  let r := foldSeq (newState model) es
  let bytes := checkpoint r.2
  "{\"events\":[" ++ ",".intercalate (es.map eventJson) ++ "],\"answers\":[" ++
    ",".intercalate (r.1.map fun a => jsonString (foldedTag a)) ++ "],\"checkpoint\":" ++
    jsonString (hex bytes) ++ ",\"restore\":" ++
    jsonString (loadedTag (restore usizeMax model bytes)) ++ "}"

/-- A checkpoint's header at an event count. -/
def header (count : Nat) : List Byte :=
  checkpointMagic ++ [checkpointFormat] ++ fsrs7Version ++ le 8 count

/-- A checkpoint's bytes for events no fold checked. -/
def raw (es : List Event) : List Byte :=
  header es.length ++ es.flatMap encEvent

/-- A good checkpoint of three events. -/
def good : List Byte :=
  raw [base 10 3, base 11 1, base 12 4]

/-- The restore inputs. -/
def restores : List (List Byte) :=
  [[], good.take 3, good.take 4, good.take 44, good.take 45,
    good.set 0 69, good.set 4 2, good.set 4 0, good.set 5 101, good.set 36 1,
    good.take (45 + 49), good.take (45 + 49 * 3 - 1), good ++ [0],
    header 0, header 0 ++ [0], header 1, header (2 ^ 32), header (2 ^ 59), header (2 ^ 64 - 1),
    raw [base 5 0], raw [base 5 3, base 5 3], raw [base 5 3, base 4 3],
    raw [ev 1 2 3 4 3 3600001 5 0]]

/-- One restore line: the bytes and the restore's answer. -/
def restoreLine (bytes : List Byte) : String :=
  "{\"bytes\":" ++ jsonString (hex bytes) ++ ",\"restore\":" ++
    jsonString (loadedTag (restore usizeMax model bytes)) ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/Fsrs7PortState.lean"
  match coversOf module with
  | [] =>
    IO.eprintln "Formal/Fsrs7PortState.lean names no covered item"
    return 1
  | (path, anchor, digest) :: rest =>
    let digests := digest :: rest.map fun (_, _, recorded) => recorded
    IO.println (headerLine "Fsrs7PortState" path anchor digest digests
      (sequences.length + restores.length))
    for es in sequences do
      IO.println (foldLine es)
    for bytes in restores do
      IO.println (restoreLine bytes)
    return 0

end Formal.Fsrs7PortStateVectors
