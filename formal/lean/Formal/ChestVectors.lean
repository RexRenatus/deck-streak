import Formal.Chest

/-!
# Formal.ChestVectors

The chest's half of the vector writer (#102, #103). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean Chest` to `run`, which prints one header line naming the first
covered item and the digest the entry recorded for it, then one line per input with the port's
answer: for `epic_odds_pts`, the Epic points; for `roll_rarity`, the rarity; for `payout_xp`, the
XP paid. The formal checker byte-compares the output with `formal/vectors/chest.jsonl`, and
`--write-vectors` writes it; the file is never edited by hand.

The odds' inputs sit on either side of the ramp's start and of the counter that reaches the
ceiling, with buffs below, at and above 0. The roll's inputs sit on either side of both guarantees
(a counter one below each, at it and past it), with and without a buff, for five draws whose
percent the code computes exactly: the quarters 0, 1/4, 1/2 and 3/4, which give 0, 25, 50 and 75,
and the largest draw below 1, whose percent lies in (99, 100). The payout's inputs are every rarity
at those five draws, over a session XP on either side of each cap the floor and the share decide.

The roll's `below t` is `25 * q < t` at the quarter `q`, and `100 <= t` at the largest draw, since
every threshold is a whole number of points. The payout's `band width` is `q * width / 4` at the
quarter `q`, and `width - 1` at the largest draw. Its `share xp` is `3 * xp / 10`: the vectors
print only XP at which the code's `(xp as f64 * 0.30) as i64` was measured equal to it. This module
carries no header line, so the registry lists it as support.
-/

namespace Formal.ChestVectors
open Formal.Chest

/-- A JSON string. The writer quotes only paths, item names, hex digests, rule, draw and rarity
names, which carry no character JSON escapes. -/
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

/-- The lowercase name the stored row and the Rust `Rarity::name` carry. -/
def rarityName : Rarity → String
  | .common => "common"
  | .rare => "rare"
  | .epic => "epic"
  | .legendary => "legendary"

/-- A draw the vectors print: a quarter `q` of 0 to 3, or the largest draw below 1. -/
inductive Draw where
  | quarter (q : Int)
  | top

/-- The draw's name in a vector line: `q0` to `q3`, or `top`. -/
def drawName : Draw → String
  | .quarter q => "q" ++ toString q
  | .top => "top"

/-- Whether the draw's percent is below the whole-number threshold `t`. -/
def drawBelow : Draw → Int → Bool
  | .quarter q, t => decide (25 * q < t)
  | .top, t => decide (100 ≤ t)

/-- The draw's truncated place in a band of `width` XP. -/
def drawBand : Draw → Int → Int
  | .quarter q, width => q * width / 4
  | .top, width => width - 1

/-- The session's truncated review-XP share, at the XP the vectors print. -/
def share (xp : Int) : Int := 3 * xp / 10

/-- The five draws. -/
def draws : List Draw := [.quarter 0, .quarter 1, .quarter 2, .quarter 3, .top]

/-- The odds' counters. -/
def oddsCounters : List Int := [-1, 0, 7, 8, 9, 10, 12, 13, 14, 15, 20, 30, 39, 40]

/-- The odds' buffs. -/
def oddsBuffs : List Int := [-10, 0, 5, 10, 20, 40]

/-- Every odds input: the counter, then the buff. -/
def oddsInputs : List (Int × Int) :=
  oddsCounters.flatMap fun se => oddsBuffs.map fun b => (se, b)

/-- The roll's counters since the last Epic. -/
def rollSinceEpic : List Int := [0, 1, 7, 8, 9, 10, 11, 12, 13, 14, 15, 39]

/-- The roll's counters since the last Legendary. -/
def rollSinceLegendary : List Int := [0, 1, 13, 38, 39, 40, 41]

/-- The roll's buffs: none, and the Ascendant's or the challenge's 10. -/
def rollBuffs : List Int := [0, 10]

/-- Every roll input: the counters, the buff, then the draw. -/
def rollInputs : List (Int × Int × Int × Draw) :=
  rollSinceEpic.flatMap fun se => rollSinceLegendary.flatMap fun sl =>
    rollBuffs.flatMap fun b => draws.map fun d => (se, sl, b, d)

/-- The payout's session XP: either side of the floor's reach and of each cap a Rare meets. -/
def payoutXps : List Int := [0, 1, 9, 10, 50, 83, 84, 86, 87, 90, 100, 120, 199, 200, 1000]

/-- The four rarities. -/
def rarities : List Rarity := [.common, .rare, .epic, .legendary]

/-- Every payout input: the rarity, the draw, then the session XP. -/
def payoutInputs : List (Rarity × Draw × Int) :=
  rarities.flatMap fun r => draws.flatMap fun d => payoutXps.map fun xp => (r, d, xp)

/-- One odds vector line: the input and the port's answer. -/
def oddsLine (input : Int × Int) : String :=
  let (se, b) := input
  "{\"rule\":\"epic_odds_pts\",\"since_epic\":" ++ toString se ++ ",\"buff_pts\":" ++
    toString b ++ ",\"odds\":" ++ toString (epicOddsPts se b) ++ "}"

/-- One roll vector line: the input and the port's answer. -/
def rollLine (input : Int × Int × Int × Draw) : String :=
  let (se, sl, b, d) := input
  "{\"rule\":\"roll_rarity\",\"draw\":" ++ jsonString (drawName d) ++ ",\"since_epic\":" ++
    toString se ++ ",\"since_legendary\":" ++ toString sl ++ ",\"buff_pts\":" ++ toString b ++
    ",\"rarity\":" ++ jsonString (rarityName (rollRarity (drawBelow d) se sl b)) ++ "}"

/-- One payout vector line: the input and the port's answer. -/
def payoutLine (input : Rarity × Draw × Int) : String :=
  let (r, d, xp) := input
  "{\"rule\":\"payout_xp\",\"rarity\":" ++ jsonString (rarityName r) ++ ",\"draw\":" ++
    jsonString (drawName d) ++ ",\"session_base_xp\":" ++ toString xp ++ ",\"payout\":" ++
    toString (payoutXp share (drawBand d) r xp) ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/Chest.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/Chest.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    let count := oddsInputs.length + rollInputs.length + payoutInputs.length
    IO.println (headerLine "Chest" path anchor digest count)
    for input in oddsInputs do
      IO.println (oddsLine input)
    for input in rollInputs do
      IO.println (rollLine input)
    for input in payoutInputs do
      IO.println (payoutLine input)
    return 0

end Formal.ChestVectors
