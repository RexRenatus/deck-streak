import Formal.SkipTariff

/-!
# Formal.SkipTariffVectors

The skip tariff's half of the vector writer (#108). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean SkipTariff` to `run`, which prints one header line naming the first
covered item and the digest the entry recorded for it, then one line per input with the port's
answer: for `price`, the price at a count of the month's earlier skips; for `settle_applied`, the
price, the amount paid and whether the skip went unfunded at a count and a balance; for
`settle_undone`, the refund of an amount paid. The formal checker byte-compares the output with
`formal/vectors/skip-tariff.jsonl`, and `--write-vectors` writes it; the file is never edited by
hand.

The counts run from 0 to 6, past the ladder's last step. The charge's counts run from 0 to 4 and
its balances sit on either side of each price and of 0; together they hold every count and balance
the `skip_tariff` golden's cases charge at. The refund's amounts are every amount a charge pays
here. A balance is never negative, as the wallet's floor keeps it, so the Rust test can set each
one. This module carries no header line, so the registry lists it as support.
-/

namespace Formal.SkipTariffVectors
open Formal.SkipTariff

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

/-- The price's counts. -/
def priceInputs : List Nat := List.range 7

/-- The charge's balances. -/
def balances : List Int := [0, 1, 19, 20, 30, 49, 50, 51, 99, 100, 101, 500]

/-- Every charge input, in a fixed order: the count, then the balance. -/
def chargeInputs : List (Nat × Int) :=
  (List.range 5).flatMap fun e => balances.map fun b => (e, b)

/-- The refund's amounts paid. -/
def refundInputs : List Int := [0, 1, 19, 20, 30, 49, 50, 51, 99, 100]

/-- One price vector line: the input and the port's answer. -/
def priceLine (earlier : Nat) : String :=
  "{\"rule\":\"price\",\"earlier\":" ++ toString earlier ++ ",\"price\":" ++
    toString (price skipTariffCoins earlier) ++ "}"

/-- One charge vector line: the input and the port's answer. -/
def chargeLine (input : Nat × Int) : String :=
  let (e, b) := input
  let (charged, paid, unfunded) := settleApplied e b
  "{\"rule\":\"settle_applied\",\"earlier\":" ++ toString e ++ ",\"balance\":" ++ toString b ++
    ",\"price\":" ++ toString charged ++ ",\"paid\":" ++ toString paid ++
    ",\"unfunded\":" ++ toString unfunded ++ "}"

/-- One refund vector line: the input and the port's answer. -/
def refundLine (paid : Int) : String :=
  "{\"rule\":\"settle_undone\",\"paid\":" ++ toString paid ++ ",\"refunded\":" ++
    toString (settleUndone paid) ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/SkipTariff.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/SkipTariff.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    let count := priceInputs.length + chargeInputs.length + refundInputs.length
    IO.println (headerLine "SkipTariff" path anchor digest count)
    for earlier in priceInputs do
      IO.println (priceLine earlier)
    for input in chargeInputs do
      IO.println (chargeLine input)
    for paid in refundInputs do
      IO.println (refundLine paid)
    return 0

end Formal.SkipTariffVectors
