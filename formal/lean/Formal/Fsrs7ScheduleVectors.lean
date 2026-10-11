import Formal.Fsrs7Schedule

/-!
# Formal.Fsrs7ScheduleVectors

The schedule's half of the vector writer (#823). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean Fsrs7Schedule` to `run`, which prints one header line naming the
first covered item, the digest the entry recorded for it and every cover's digest, then one line
per input with the port's own answer. The formal checker byte-compares the output with
`formal/vectors/fsrs7-schedule.jsonl`, and `--write-vectors` writes it; the file is never edited by
hand.

An `f32` is written `[0]` for NaN, `[1]` for negative infinity, `[2]` for positive infinity, and
`[3,n,d]` for the finite value `n / d`, each a power-of-two denominator, so the value is exact in
an `f32`. A schedule is `[]` when `scheduled` refuses and `[ivl,due]` otherwise.

The schedule inputs are every engine day of three (today's count 0, 1000, and one a few days short
of `i32::MAX`), every instant at nine offsets from the instant today began, from two days before it
to five days after, each at a whole second and 999 ms past it, every interval of thirteen (NaN,
both infinities, a negative value, zero, a quarter day, the three halves 1/2, 3/2 and 5/2 that
round away from zero, a value near the first Easy's interval, the crate's largest stability, and
two past `u32::MAX`), and every maximum of five. The
write's inputs are two replays, one with a schedule and one without, over three cards. This module
carries no header line, so the registry lists it as support.
-/

namespace Formal.Fsrs7ScheduleVectors

open Formal.Fsrs7Schedule

/-- A JSON string. The writer quotes only paths, item names and hex digests, which carry no
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

/-- An `f32` as its tagged integers. -/
def f32Json : F32 → String
  | .nan => "[0]"
  | .negInf => "[1]"
  | .posInf => "[2]"
  | .fin q => s!"[3,{q.num},{q.den}]"

/-- A schedule, or its refusal. -/
def scheduleJson : Option (Nat × Int) → String
  | none => "[]"
  | some (i, due) => s!"[{i},{due}]"

/-- The engine days: today's count and the next rollover, in seconds. -/
def days : List EngineDay :=
  [⟨0, 86400⟩, ⟨1000, 950400⟩, ⟨2147483600, 950400⟩]

/-- Seconds from the instant today began. -/
def offsets : List Int :=
  [-172800, -1, 0, 1, 86399, 86400, 86401, 260434, 432000]

/-- The scheduler's intervals. -/
def intervals : List F32 :=
  [.nan, .negInf, .posInf, .fin (-5 / 2), .fin 0, .fin (1 / 4), .fin (1 / 2), .fin (3 / 2),
    .fin (5 / 2), .fin (1724 / 32), .fin 36500, .fin 4294967296, .fin 1099511627776]

/-- The maximum intervals. -/
def maxima : List Nat :=
  [0, 1, 3, 36500, u32Max]

/-- One schedule input: the day, the last review in milliseconds, the interval and the maximum. -/
structure Input where
  day : EngineDay
  last : Int
  interval : F32
  maxIvl : Nat

/-- Every schedule input, in a fixed order. -/
def inputs : List Input :=
  days.flatMap fun day =>
    offsets.flatMap fun offset =>
      [0, 999].flatMap fun ms =>
        intervals.flatMap fun interval =>
          maxima.map fun maxIvl =>
            ⟨day, (day.nextDayAt - 86400 + offset) * 1000 + ms, interval, maxIvl⟩

/-- One schedule line: the input, the engine day of the last review, and the port's schedule. -/
def scheduleLine (x : Input) : String :=
  "{\"day\":[" ++ toString x.day.daysElapsed ++ "," ++ toString x.day.nextDayAt ++
    "],\"last\":" ++ toString x.last ++ ",\"interval\":" ++ f32Json x.interval ++
    ",\"max\":" ++ toString x.maxIvl ++ ",\"last_day\":" ++ toString (dayOf x.day x.last) ++
    ",\"schedule\":" ++ scheduleJson (scheduled x.interval x.last x.day x.maxIvl) ++ "}"

/-- A card as its memory state, interval and due. -/
def cardJson (c : Card) : String :=
  let memory := match c.memory with
    | none => "[]"
    | some (s, d) => "[" ++ f32Json s ++ "," ++ f32Json d ++ "]"
  "[" ++ memory ++ "," ++ toString c.interval ++ "," ++ toString c.due ++ "]"

/-- A replay as its stability, difficulty and schedule. -/
def replayJson (r : CardReplay) : String :=
  "[" ++ f32Json r.stability ++ "," ++ f32Json r.difficulty ++ "," ++ scheduleJson r.schedule ++
    "]"

/-- The write's replays. -/
def replays : List CardReplay :=
  [⟨.fin (1724 / 32), .fin 1, some (54, 1054)⟩, ⟨.fin (1 / 4), .fin (13 / 2), none⟩]

/-- The write's cards: a new card, a review card and a card already written. -/
def cards : List Card :=
  [⟨none, 0, 0, 0⟩, ⟨some (.fin 3, .fin 5), 7, 993, 0⟩, ⟨some (.fin 2, .fin 4), 54, 1054, 0⟩]

/-- One write line: the replay, the card, and the port's card after one write and after two. -/
def writeLine (r : CardReplay) (c : Card) : String :=
  "{\"replay\":" ++ replayJson r ++ ",\"card\":" ++ cardJson c ++ ",\"written\":" ++
    cardJson (stockFields r c) ++ ",\"twice\":" ++ cardJson (stockFields r (stockFields r c)) ++
    "}"

/-- Every write input. -/
def writes : List (CardReplay × Card) :=
  replays.flatMap fun r => cards.map fun c => (r, c)

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/Fsrs7Schedule.lean"
  match coversOf module with
  | [] =>
    IO.eprintln "Formal/Fsrs7Schedule.lean names no covered item"
    return 1
  | (path, anchor, digest) :: rest =>
    let digests := digest :: rest.map fun (_, _, recorded) => recorded
    IO.println (headerLine "Fsrs7Schedule" path anchor digest digests
      (inputs.length + writes.length))
    for x in inputs do
      IO.println (scheduleLine x)
    for (r, c) in writes do
      IO.println (writeLine r c)
    return 0

end Formal.Fsrs7ScheduleVectors
