-- @phx covers crates/economy/src/rules.rs anchor=daily_loss_cap digest=sha256:77a8bf2ee883b555654d674cb4bdbae51c1193b21a0040b0efcaa230919cccbc
-- @phx covers crates/economy/src/rules.rs anchor=scaled_fine digest=sha256:79544cd2bf4d5a3926e41eaca7ddae6366d6b37871bcf605eadc3a9a32e68bb7
-- @phx vectors formal/vectors/coin-shares.jsonl
-- @phx cites #890
-- @phx theorem the_cap_is_the_wallet_share_rounded_down ramp=report
-- @phx witness a_cap_rounded_up_takes_more_than_its_share kills=the_cap_is_the_wallet_share_rounded_down
-- @phx theorem the_fine_is_the_wallet_share_rounded_up ramp=report
-- @phx witness a_fine_rounded_down_takes_less_than_its_share kills=the_fine_is_the_wallet_share_rounded_up
-- @phx theorem the_share_steps_fit_the_coin_type ramp=report
-- @phx witness a_share_taken_as_one_product_leaves_the_coin_type kills=the_share_steps_fit_the_coin_type

/-!
# Formal.CoinShares

The two coin rules that take a share of the wallet (#890), `daily_loss_cap` and `scaled_fine` in
`crates/economy/src/rules.rs`, as total functions over `Int`.

**The claim.** For every input:
- `the_cap_is_the_wallet_share_rounded_down`: a wallet of 0 or less caps a day's loss at 0, and a
  positive wallet at the greatest whole number that is at most 100 and at most 30 percent of it;
- `the_fine_is_the_wallet_share_rounded_up`: a configured fine of 0 or less is 0, against a wallet
  of 0 or less it is the configured fine, and otherwise it is the least whole number that is at
  least the configured fine and at least 2 percent of the wallet;
- `the_share_steps_fit_the_coin_type`: for every wallet from 1 to `i64::MAX`, each value an
  operator of either share computes lies from 0 to `i64::MAX`, so the Rust arithmetic never
  overflows and the port over `Int` is exact.

**What is ported.** Both functions read `i64` and call only `min`, `max`, a comparison, `+`, `*`,
and `/` and `%` by the positive constant 100. Past the guards the wallet is positive, where Rust's
truncating `/` and `%` are `Int`'s `/` and `%`. The constants are `DAILY_LOSS_CAP_COINS` (100),
`DAILY_LOSS_CAP_WALLET_PERCENT` (30) and `FINE_WALLET_PERCENT` (2) from
`crates/economy/src/constants.rs`; the vectors carry them to the Rust test, which fails if one
moves.

**The witnesses** are ports that each break one claim at one input: a cap that rounds its share
up, a fine that rounds its share down, and a share taken as one product of the wallet and its
percent, which leaves the coin type at the largest wallet.
-/

namespace Formal.CoinShares

/-- `DAILY_LOSS_CAP_COINS`: the most coins a day may lose. -/
def dailyLossCapCoins : Int := 100

/-- `DAILY_LOSS_CAP_WALLET_PERCENT`: the percent of the day's starting wallet the cap allows. -/
def dailyLossCapWalletPercent : Int := 30

/-- `FINE_WALLET_PERCENT`: the percent of the wallet a fine takes at least. -/
def fineWalletPercent : Int := 2

/-- `i64::MAX`, the largest wallet the coin type holds. -/
def coinMax : Int := 9223372036854775807

/-- `daily_loss_cap`: none for a wallet of 0 or less, otherwise the lesser of the cap and the
wallet's share, taken over its whole hundreds and its remainder and rounded down. -/
def dailyLossCap (wallet : Int) : Int :=
  if wallet ≤ 0 then 0
  else
    min dailyLossCapCoins
      (wallet / 100 * dailyLossCapWalletPercent + wallet % 100 * dailyLossCapWalletPercent / 100)

/-- `scaled_fine`: none for a configured fine of 0 or less, the configured fine for a wallet of 0
or less, otherwise the greater of the configured fine and the wallet's share, rounded up. -/
def scaledFine (configured wallet : Int) : Int :=
  if configured ≤ 0 then 0
  else if wallet ≤ 0 then configured
  else
    max configured
      (wallet / 100 * fineWalletPercent + (wallet % 100 * fineWalletPercent + 99) / 100)

/-- Each value an operator of the two shares computes: the cap's three, then the fine's four. -/
def shareSteps (wallet : Int) : List Int :=
  [wallet / 100 * dailyLossCapWalletPercent,
    wallet % 100 * dailyLossCapWalletPercent,
    wallet / 100 * dailyLossCapWalletPercent + wallet % 100 * dailyLossCapWalletPercent / 100,
    wallet / 100 * fineWalletPercent,
    wallet % 100 * fineWalletPercent,
    wallet % 100 * fineWalletPercent + 99,
    wallet / 100 * fineWalletPercent + (wallet % 100 * fineWalletPercent + 99) / 100]

/-- The wrong variant: a cap that rounds its share up. -/
def dailyLossCapRoundedUp (wallet : Int) : Int :=
  if wallet ≤ 0 then 0
  else
    min dailyLossCapCoins
      (wallet / 100 * dailyLossCapWalletPercent +
        (wallet % 100 * dailyLossCapWalletPercent + 99) / 100)

/-- The wrong variant: a fine that rounds its share down. -/
def scaledFineRoundedDown (configured wallet : Int) : Int :=
  if configured ≤ 0 then 0
  else if wallet ≤ 0 then configured
  else max configured (wallet / 100 * fineWalletPercent + wallet % 100 * fineWalletPercent / 100)

/-- The wrong variant: each share taken as one product of the wallet and its percent. -/
def shareStepsAsOneProduct (wallet : Int) : List Int :=
  [wallet * dailyLossCapWalletPercent, wallet * fineWalletPercent]

/-- The claim, stated once: the cap is the wallet's 30 percent rounded down, and at most 100. -/
def RoundsDown (cap : Int → Int) : Prop :=
  ∀ w : Int,
    (w ≤ 0 → cap w = 0) ∧
      (0 < w → cap w ≤ 100 ∧ 100 * cap w ≤ 30 * w ∧ (cap w = 100 ∨ 30 * w < 100 * (cap w + 1)))

/-- The claim, stated once: the fine is the wallet's 2 percent rounded up, and at least the
configured fine. -/
def RoundsUp (fine : Int → Int → Int) : Prop :=
  ∀ c w : Int,
    (c ≤ 0 → fine c w = 0) ∧ (0 < c → w ≤ 0 → fine c w = c) ∧
      (0 < c → 0 < w →
        c ≤ fine c w ∧ 2 * w ≤ 100 * fine c w ∧ (fine c w = c ∨ 100 * (fine c w - 1) < 2 * w))

/-- The claim, stated once: for every wallet the coin type holds above 0, each step lies in it. -/
def StepsFit (steps : Int → List Int) : Prop :=
  ∀ w : Int, 0 < w → w ≤ coinMax → ∀ x ∈ steps w, 0 ≤ x ∧ x ≤ coinMax

theorem the_cap_is_the_wallet_share_rounded_down : RoundsDown dailyLossCap := by
  intro w
  simp only [dailyLossCap, dailyLossCapCoins, dailyLossCapWalletPercent]
  split <;> omega

theorem the_fine_is_the_wallet_share_rounded_up : RoundsUp scaledFine := by
  intro c w
  simp only [scaledFine, fineWalletPercent]
  split <;> (try split) <;> omega

theorem the_share_steps_fit_the_coin_type : StepsFit shareSteps := by
  intro w h0 h1 x hx
  simp only [shareSteps, dailyLossCapWalletPercent, fineWalletPercent, List.mem_cons,
    List.not_mem_nil, or_false] at hx
  simp only [coinMax] at h1 ⊢
  omega

theorem a_cap_rounded_up_takes_more_than_its_share : ¬ RoundsDown dailyLossCapRoundedUp := by
  intro h
  have := (h 1).2 (by decide)
  revert this
  decide

theorem a_fine_rounded_down_takes_less_than_its_share : ¬ RoundsUp scaledFineRoundedDown := by
  intro h
  have := (h 1 51).2.2 (by decide) (by decide)
  revert this
  decide

theorem a_share_taken_as_one_product_leaves_the_coin_type :
    ¬ StepsFit shareStepsAsOneProduct := by
  intro h
  have := h coinMax (by decide) (by decide) (coinMax * dailyLossCapWalletPercent)
    (by simp [shareStepsAsOneProduct])
  revert this
  decide

end Formal.CoinShares
