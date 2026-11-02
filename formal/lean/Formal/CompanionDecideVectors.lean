import Formal.CompanionDecide

/-!
# Formal.CompanionDecideVectors

The scheduling companion's half of the vector writer (SPEC-501). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean CompanionDecide` to `run`, which prints one header line naming the
first covered item and the digest the entry recorded for it, then one line per input with the
port's answer: for `decide`, the outcome and the admitted due date; for `negotiate`, the outcome.
The formal checker byte-compares the output with `formal/vectors/companion-decide.jsonl`, and
`--write-vectors` writes it; the file is never edited by hand.

The decision's inputs are every session state, reply kind and status around one valid press;
then, for a due date with status ok, every combination of a stale or current epoch, a wrong or
right card, a wrong or right press, a changed or negotiated model, five intervals on and past
both bounds, and a due day on or one past the sum; then the `i32` and `i64` extremes. The
handshake's inputs are every status, three majors, both models and two minors. This module
carries no header line, so the registry lists it as support.
-/

namespace Formal.CompanionDecideVectors
open Formal.CompanionDecide

/-- A JSON string. The writer quotes only paths, item names, hex digests and outcome names,
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

/-- A status's name. -/
def statusName : Status → String
  | .ok => "ok"
  | .absent => "absent"
  | .fixtureRefuses => "fixture_refuses"
  | .busy => "busy"
  | .error => "error"
  | .conflict => "conflict"
  | .unsupported => "unsupported"

/-- A refusal's name. -/
def refusalName : Refusal → String
  | .notNegotiated => "not_negotiated"
  | .absent => "absent"
  | .mismatch => "mismatch"
  | .fixture => "fixture"
  | .status s => "status:" ++ statusName s
  | .staleEpoch => "stale_epoch"
  | .wrongCard => "wrong_card"
  | .wrongStage => "wrong_stage"
  | .modelChanged => "model_changed"
  | .intervalOutOfRange => "interval_out_of_range"
  | .dueDayInconsistent => "due_day_inconsistent"

/-- Every status. -/
def statuses : List Status :=
  [.ok, .absent, .fixtureRefuses, .busy, .error, .conflict, .unsupported]

/-- One decision's input. -/
structure DecideInput where
  session : Session
  asked : Asked
  epoch : Nat
  reply : Reply

/-- The press every input asks about, and its session's epoch and model. -/
def press : Asked := ⟨42, 1000, 100⟩

/-- A due date with status `st` for the press, from model 5, in 3 days. -/
def dueFor (st : Status) : Reply := .due st 42 1000 3 103 5

/-- Every session state, reply kind and status, around the one valid press. -/
def kindInputs : List DecideInput :=
  [none, some 5].flatMap fun model =>
    statuses.flatMap fun st =>
      [dueFor st, .other st].map fun reply => ⟨⟨7, model⟩, press, 7, reply⟩

/-- Every combination of the due date's own checks, for a due date with status ok. -/
def checkInputs : List DecideInput :=
  [6, 7].flatMap fun epoch =>
    [42, 43].flatMap fun card =>
      [999, 1000].flatMap fun tMs =>
        [4, 5].flatMap fun model =>
          [0, 1, 3, 36500, 36501].flatMap fun iv =>
            [0, 1].map fun off =>
              ⟨⟨7, some 5⟩, press, epoch, .due .ok card tMs iv (100 + iv + off) model⟩

/-- The extremes: a sum past `i32`, the least study day, and the `i64` ends of a card and a
press. -/
def extremeInputs : List DecideInput :=
  [⟨⟨7, some 5⟩, ⟨42, 1000, 2147483597⟩, 7, .due .ok 42 1000 100 2147483647 5⟩,
   ⟨⟨7, some 5⟩, ⟨42, 1000, -2147483648⟩, 7, .due .ok 42 1000 1 (-2147483647) 5⟩,
   ⟨⟨7, some 5⟩, ⟨-9223372036854775808, 9223372036854775807, 100⟩, 7,
     .due .ok (-9223372036854775808) 9223372036854775807 3 103 5⟩]

/-- Every decision input, in a fixed order. -/
def decideInputs : List DecideInput :=
  kindInputs ++ checkInputs ++ extremeInputs

/-- Every hello: each status, three majors, both models and two minors. -/
def helloInputs : List Hello :=
  statuses.flatMap fun st =>
    [0, 1, 2].flatMap fun major =>
      [Model.trained, Model.fixture].flatMap fun model =>
        [0, 3].map fun minor => ⟨st, major, minor, model, 5⟩

/-- One decision vector line: the input and the port's answer. -/
def decideLine (input : DecideInput) : String :=
  let (kind, st, card, tMs, iv, dd, md) := match input.reply with
    | .due st card tMs iv dd md => ("due", st, card, tMs, iv, dd, md)
    | .other st => ("other", st, (0 : Int), (0 : Int), 0, (0 : Int), 0)
  let model := match input.session.model with
    | none => "null"
    | some m => toString m
  let outcome := match decideReply input.session input.asked input.epoch input.reply with
    | .ok d =>
      "\"ok\",\"due\":[" ++ toString d.card ++ "," ++ toString d.interval ++ "," ++
        toString d.dueDay ++ "]"
    | .error r => jsonString (refusalName r) ++ ",\"due\":null"
  "{\"rule\":\"decide\",\"session_epoch\":" ++ toString input.session.epoch ++
    ",\"session_model\":" ++ model ++ ",\"card\":" ++ toString input.asked.card ++
    ",\"t_ms\":" ++ toString input.asked.tMs ++ ",\"study_day\":" ++
    toString input.asked.studyDay ++ ",\"replied_epoch\":" ++ toString input.epoch ++
    ",\"reply\":" ++ jsonString kind ++ ",\"status\":" ++ jsonString (statusName st) ++
    ",\"reply_card\":" ++ toString card ++ ",\"staged_t_ms\":" ++ toString tMs ++
    ",\"interval_days\":" ++ toString iv ++ ",\"due_day\":" ++ toString dd ++
    ",\"model\":" ++ toString md ++ ",\"outcome\":" ++ outcome ++ "}"

/-- A model's name. -/
def modelName : Model → String
  | .trained => "trained"
  | .fixture => "fixture"

/-- One handshake vector line: the input and the port's answer. -/
def helloLine (h : Hello) : String :=
  let outcome := match negotiate h with
    | .ok m => "\"ok\",\"model_id\":" ++ toString m
    | .error r => jsonString (refusalName r) ++ ",\"model_id\":null"
  "{\"rule\":\"negotiate\",\"status\":" ++ jsonString (statusName h.status) ++
    ",\"major\":" ++ toString h.major ++ ",\"minor\":" ++ toString h.minor ++
    ",\"model\":" ++ jsonString (modelName h.model) ++ ",\"hello_model_id\":" ++
    toString h.modelId ++ ",\"outcome\":" ++ outcome ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/CompanionDecide.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/CompanionDecide.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine "CompanionDecide" path anchor digest
      (decideInputs.length + helloInputs.length))
    for input in decideInputs do
      IO.println (decideLine input)
    for h in helloInputs do
      IO.println (helloLine h)
    return 0

end Formal.CompanionDecideVectors
