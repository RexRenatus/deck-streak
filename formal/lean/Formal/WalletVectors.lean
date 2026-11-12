import Formal.Wallet

/-!
# Formal.WalletVectors

The coin wallet's half of the vector writer (#106). `Formal/Vectors.lean` dispatches
`lean --run Formal/Vectors.lean Wallet` to `run`, which prints one header line naming the first
covered item and the digest the entry recorded for it, then one line per input with the port's
answer: for `clip_debit`, the amount allowed and whether any part was forgiven; for
`mint_for_base_xp`, the coins minted. The formal checker byte-compares the output with
`formal/vectors/wallet.jsonl`, and `--write-vectors` writes it; the file is never edited by hand.

The clip's inputs are every request, wallet and remaining cap from -2 to 6, so each argument is
negative, zero, below, equal to and above the others in some input. The mint's inputs sit on
either side of 0, of each multiple of the divisor near the bottom, and of the base that reaches
the cap. This module carries no header line, so the registry lists it as support.
-/

namespace Formal.WalletVectors
open Formal.Wallet

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

/-- The clip's argument range. -/
def clipRange : List Int := (List.range 9).map fun (n : Nat) => Int.ofNat n - 2

/-- Every clip input, in a fixed order: the request, then the wallet, then the remaining cap. -/
def clipInputs : List (Int × Int × Int) :=
  clipRange.flatMap fun r => clipRange.flatMap fun w => clipRange.map fun c => (r, w, c)

/-- The mint's inputs. -/
def mintInputs : List Int :=
  [-26, -25, -1, 0, 1, 24, 25, 26, 49, 50, 51, 999, 1000, 1001, 1024, 1025, 1026, 5000]

/-- One clip vector line: the input and the port's answer. -/
def clipLine (input : Int × Int × Int) : String :=
  let (r, w, c) := input
  let (allowed, forgiven) := clipDebit r w c
  "{\"rule\":\"clip_debit\",\"requested\":" ++ toString r ++ ",\"wallet\":" ++ toString w ++
    ",\"cap_remaining\":" ++ toString c ++ ",\"allowed\":" ++ toString allowed ++
    ",\"forgiven\":" ++ toString forgiven ++ "}"

/-- One mint vector line: the input and the port's answer. -/
def mintLine (base : Int) : String :=
  "{\"rule\":\"mint_for_base_xp\",\"base_xp\":" ++ toString base ++ ",\"minted\":" ++
    toString (mintForBaseXp base) ++ "}"

/-- Print the header and every vector. -/
def run : IO UInt32 := do
  let module ← IO.FS.readFile "Formal/Wallet.lean"
  match coverOf module with
  | none =>
    IO.eprintln "Formal/Wallet.lean names no covered item"
    return 1
  | some (path, anchor, digest) =>
    IO.println (headerLine "Wallet" path anchor digest (clipInputs.length + mintInputs.length))
    for input in clipInputs do
      IO.println (clipLine input)
    for base in mintInputs do
      IO.println (mintLine base)
    return 0

end Formal.WalletVectors
