import Formal.CoinShares

/-!
# Formal.CoinSharesVectors

The coin shares' half of the vector writer (#890). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean CoinShares` to `run`, which prints one header line naming the first
covered item and the digest the entry recorded for it, then one line per input with the port's
answer: for `daily_loss_cap`, the cap; for `scaled_fine`, the fine. The formal checker
byte-compares the output with `formal/vectors/coin-shares.jsonl`, and `--write-vectors` writes it;
the file is never edited by hand.

The wallets sit at both ends of the coin type, on either side of 0, of the first whole hundreds,
of the wallet whose share reaches the cap, and past 2^53. The configured fines are below, at and
above 0. This module carries no header line, so the registry lists it as support.
-/

namespace Formal.CoinSharesVectors
open Formal.CoinShares

/-- A JSON string. The writer quotes only paths, item names and hex digests, which carry no
character JSON escapes. -/
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

/-- The cap's wallets. -/
def capInputs : List Int :=
  [-9223372036854775808, -1, 0, 1, 3, 4, 9, 10, 11, 99, 100, 101, 199, 200, 299, 300, 333, 334,
    335, 1000, 9007199254741001, 9223372036854775807]

/-- The fine's configured fines. -/
def fineConfigured : List Int := [-1, 0, 1, 2, 7]

/-- The fine's wallets. -/
def fineWallets : List Int :=
  [-9223372036854775808, -1, 0, 1, 49, 50, 51, 99, 100, 101, 149, 150, 151, 349, 350, 351,
    9007199254741001, 9223372036854775807]

/-- Every fine input, in a fixed order: the configured fine, then the wallet. -/
def fineInputs : List (Int × Int) :=
  fineConfigured.flatMap fun c => fineWallets.map fun w => (c, w)

/-- One cap vector line: the input and the port's answer. -/
def capLine (wallet : Int) : String :=
  "{\"rule\":\"daily_loss_cap\",\"wallet_at_rollover\":" ++ toString wallet ++ ",\"cap\":" ++
    toString (dailyLossCap wallet) ++ "}"

/-- One fine vector line: the input and the port's answer. -/
def fineLine (input : Int × Int) : String :=
  let (c, w) := input
  "{\"rule\":\"scaled_fine\",\"configured\":" ++ toString c ++ ",\"wallet\":" ++ toString w ++
    ",\"fine\":" ++ toString (scaledFine c w) ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/CoinShares.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/CoinShares.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine "CoinShares" path anchor digest (capInputs.length + fineInputs.length))
    for wallet in capInputs do
      IO.println (capLine wallet)
    for input in fineInputs do
      IO.println (fineLine input)
    return 0

end Formal.CoinSharesVectors
