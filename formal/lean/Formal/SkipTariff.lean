-- @phx covers crates/economy/src/tariff.rs anchor=price digest=sha256:63276c8915de3c34258f19c98f14295994ad8be124c0faade868064bfb378039
-- @phx covers crates/economy/src/wallet.rs anchor=debit_floored_on digest=sha256:b453615922d0bf5aa4f940affbbdd8273c361ac82165aff2c2094299c87f9bbd
-- @phx covers crates/coordination/src/skip/mod.rs anchor=settle_applied digest=sha256:7df3e27a07ba3c716aef05c9eb92ce04d5e1b727eaf8543da0f6eb0c41445faa
-- @phx covers crates/coordination/src/skip/mod.rs anchor=settle_undone digest=sha256:61aa16597ae2e193cf7a29d4606a0def0d356b2d28672b102eadb08705dbb350
-- @phx vectors formal/vectors/skip-tariff.jsonl
-- @phx cites #108
-- @phx theorem the_price_is_a_ladder_entry ramp=report
-- @phx witness a_price_summed_over_the_ladder_leaves_it kills=the_price_is_a_ladder_entry
-- @phx theorem the_price_never_falls_as_the_count_grows ramp=report
-- @phx witness a_price_with_no_clamp_falls_past_the_ladders_end kills=the_price_never_falls_as_the_count_grows
-- @phx theorem the_last_price_repeats ramp=report
-- @phx witness a_price_with_no_clamp_stops_repeating kills=the_last_price_repeats
-- @phx theorem the_amount_paid_stays_within_the_price_and_the_wallet ramp=report
-- @phx witness a_charge_that_skips_the_wallet_overpays kills=the_amount_paid_stays_within_the_price_and_the_wallet
-- @phx theorem the_refund_is_what_the_skip_paid ramp=report
-- @phx witness a_refund_of_the_price_returns_more_than_was_paid kills=the_refund_is_what_the_skip_paid

/-!
# Formal.SkipTariff

The skip day's tariff (#108; SPEC-083 R8 to R10, section 10's T3 and T4; ADR-321 D6, D11 and
D13): the price `price` in `crates/economy/src/tariff.rs` reads off the ladder, the amount the
floor-clipped debit `debit_floored_on` in `crates/economy/src/wallet.rs` pays, and the charge
`settle_applied` and the refund `settle_undone` in `crates/coordination/src/skip/mod.rs` take, as
total functions over `Int` and `Nat`.

**The claim.** For every count of the month's earlier skips and every balance:
- `the_price_is_a_ladder_entry`: the price is one of the ladder's prices;
- `the_price_never_falls_as_the_count_grows`: a later skip of the month never costs less than an
  earlier one;
- `the_last_price_repeats`: every skip from the ladder's last step on costs the last price;
- `the_amount_paid_stays_within_the_price_and_the_wallet`: the amount paid is at least 0, at most
  the price and at most what the wallet holds, and the skip is unfunded exactly when it paid less
  than its price;
- `the_refund_is_what_the_skip_paid`: an undo refunds exactly what the skip paid, so a free skip,
  and a skip the empty wallet paid nothing for, refund nothing.

**What is ported.** `price` reads the `i64` entries of a slice at a `usize` count: here the count
is `Nat` and the entries are `Int`, read with `min` and a default of 0, which neither overflows nor
wraps, so the port is exact. `debit_floored_on` pays `min(amount, max(0, balance - WALLET_FLOOR))`
on a key it has not written, and nothing for a request of 0 or less; a retry on a written key
answers the amount held, which is this port's first answer. `settle_applied` prices the skip, asks
`debit_floored_on` for that price even when it is 0, which pays nothing and writes no movement, and
records the shortfall; `settle_undone` asks `refund_on` for what the skip paid, which credits it
only when it is more than 0. The ports' guards on 0 answer what those two calls answer. The ladder
is `economy.json`'s `streak.skip_tariff_coins` and the floor is
`WALLET_FLOOR` (`crates/economy/src/constants.rs`); the vectors carry both to the Rust test, which
fails if either moves.

**What is not ported.** The count itself, the month's other applied skips not undone on an earlier
study day, is a filter over the record that A11's golden comparison proves. The ledger's key, which
takes the charge and the refund once a skip, is `tla/SkipDayOnce`'s, and the wallet's floor under
every port is `tla/WalletFloor`'s.

**The witnesses** are ports that each break one claim at one input: a price that sums the ladder's
steps up to the count, a price with no clamp at the ladder's last step, a charge that skips the
wallet, and a refund of the price in place of the amount paid.
-/

namespace Formal.SkipTariff

/-- `streak.skip_tariff_coins` in `economy.json`, the ladder `tariff::ladder` reads: the price of a
month's first skip, of its second, and of every later one. -/
def skipTariffCoins : List Int := [0, 50, 100]

/-- `WALLET_FLOOR`: the balance no debit takes the wallet below. -/
def walletFloor : Int := 0

/-- `price`: the ladder's entry at the count of the month's earlier skips, the last price
repeating, and 0 for an empty ladder. -/
def price (ladder : List Int) (earlier : Nat) : Int :=
  match ladder.length with
  | 0 => 0
  | last + 1 => ladder.getD (min earlier last) 0

/-- `debit_floored_on`'s amount paid on a key it has not written: nothing for a request of 0 or
less, and otherwise the request clipped to what the wallet holds above the floor. -/
def debitFlooredOn (amount balance : Int) : Int :=
  if amount ≤ 0 then 0 else min amount (max (balance - walletFloor) 0)

/-- `settle_applied`'s charge: the skip's price, the amount paid, and whether the skip went
unfunded. At a price of 0 the code still asks the wallet, which pays nothing for a request of 0,
so the guard here answers the same 0. -/
def settleApplied (earlier : Nat) (balance : Int) : Int × Int × Bool :=
  let charged := price skipTariffCoins earlier
  let paid := if 0 < charged then debitFlooredOn charged balance else 0
  (charged, paid, decide (paid < charged))

/-- `settle_undone`'s refund: what the skip paid, which `refund_on` credits only when it is more
than 0. -/
def settleUndone (paid : Int) : Int :=
  if 0 < paid then paid else 0

/-- The wrong variant: a price that sums the ladder's steps up to the count, so a month's third
skip pays the first two prices beside its own. -/
def priceSummedOverTheLadder (ladder : List Int) (earlier : Nat) : Int :=
  (ladder.take (earlier + 1)).foldl (· + ·) 0

/-- The wrong variant: a price with no clamp, so a count past the ladder's last step reads the
default of 0. -/
def priceWithNoClamp (ladder : List Int) (earlier : Nat) : Int :=
  ladder.getD earlier 0

/-- The wrong variant: a charge that skips the wallet, so the skip pays its whole price whatever
the balance. -/
def settleAppliedWithoutTheWallet (earlier : Nat) (_balance : Int) : Int × Int × Bool :=
  let charged := price skipTariffCoins earlier
  (charged, charged, false)

/-- The refund an undo credits after a skip was charged at a count and a balance. -/
def refundAfter (earlier : Nat) (balance : Int) : Int :=
  settleUndone (settleApplied earlier balance).2.1

/-- The wrong variant: a refund of the skip's price in place of the amount it paid. -/
def refundOfThePrice (earlier : Nat) (balance : Int) : Int :=
  settleUndone (settleApplied earlier balance).1

/-- The claim, stated once: every price is a ladder entry. -/
def OnTheLadder (p : Nat → Int) : Prop :=
  ∀ n : Nat, p n ∈ skipTariffCoins

/-- The claim, stated once: the price never falls as the count grows. -/
def NeverFalls (p : Nat → Int) : Prop :=
  ∀ a b : Nat, a ≤ b → p a ≤ p b

/-- The claim, stated once: from the ladder's last step on, every skip costs the last price. -/
def LastRepeats (p : Nat → Int) : Prop :=
  ∀ n : Nat, skipTariffCoins.length - 1 ≤ n →
    p n = skipTariffCoins.getD (skipTariffCoins.length - 1) 0

/-- The claim, stated once: the amount paid lies between 0 and both the price and the wallet, and
the skip is unfunded exactly when it paid less than its price. -/
def PaidWithin (charge : Nat → Int → Int × Int × Bool) : Prop :=
  ∀ (e : Nat) (b : Int),
    0 ≤ (charge e b).2.1 ∧ (charge e b).2.1 ≤ (charge e b).1 ∧ (charge e b).2.1 ≤ max b 0 ∧
      (charge e b).2.2 = decide ((charge e b).2.1 < (charge e b).1)

/-- The claim, stated once: the refund is exactly what the skip paid. -/
def RefundsWhatWasPaid (refund : Nat → Int → Int) : Prop :=
  ∀ (e : Nat) (b : Int), refund e b = (settleApplied e b).2.1

/-- The configured ladder's price at a count of two or more is its last price. -/
theorem price_from_the_last_step (n : Nat) : price skipTariffCoins (n + 2) = 100 := by
  simp only [price, skipTariffCoins, List.length_cons, List.length_nil]
  rw [Nat.min_eq_right (by omega)]
  rfl

/-- The configured ladder's price is never negative. -/
theorem price_is_never_negative (n : Nat) : 0 ≤ price skipTariffCoins n := by
  match n with
  | 0 => decide
  | 1 => decide
  | n + 2 => rw [price_from_the_last_step]; decide

theorem the_price_is_a_ladder_entry : OnTheLadder (price skipTariffCoins) := by
  intro n
  match n with
  | 0 => decide
  | 1 => decide
  | n + 2 => rw [price_from_the_last_step]; decide

theorem the_price_never_falls_as_the_count_grows : NeverFalls (price skipTariffCoins) := by
  intro a b hle
  match a, b with
  | 0, 0 => decide
  | 0, 1 => decide
  | 0, b + 2 => rw [price_from_the_last_step]; decide
  | 1, 0 => omega
  | 1, 1 => decide
  | 1, b + 2 => rw [price_from_the_last_step]; decide
  | a + 2, 0 => omega
  | a + 2, 1 => omega
  | a + 2, b + 2 => rw [price_from_the_last_step, price_from_the_last_step]; decide

theorem the_last_price_repeats : LastRepeats (price skipTariffCoins) := by
  intro n hn
  match n with
  | 0 => simp [skipTariffCoins] at hn
  | 1 => simp [skipTariffCoins] at hn
  | n + 2 => rw [price_from_the_last_step]; decide

theorem the_amount_paid_stays_within_the_price_and_the_wallet : PaidWithin settleApplied := by
  intro e b
  have hp := price_is_never_negative e
  simp only [settleApplied, debitFlooredOn, walletFloor]
  by_cases hpos : 0 < price skipTariffCoins e
  · have hnp : ¬ price skipTariffCoins e ≤ 0 := by omega
    simp only [hpos, hnp, ↓reduceIte]
    exact ⟨by omega, by omega, by omega, trivial⟩
  · simp only [hpos, ↓reduceIte]
    exact ⟨Int.le_refl 0, hp, by omega, trivial⟩

theorem the_refund_is_what_the_skip_paid : RefundsWhatWasPaid refundAfter := by
  intro e b
  have := (the_amount_paid_stays_within_the_price_and_the_wallet e b).1
  simp only [refundAfter, settleUndone]
  split <;> omega

theorem a_price_summed_over_the_ladder_leaves_it :
    ¬ OnTheLadder (priceSummedOverTheLadder skipTariffCoins) := by
  intro h
  have := h 2
  revert this
  decide

theorem a_price_with_no_clamp_falls_past_the_ladders_end :
    ¬ NeverFalls (priceWithNoClamp skipTariffCoins) := by
  intro h
  have := h 2 3 (by decide)
  revert this
  decide

theorem a_price_with_no_clamp_stops_repeating :
    ¬ LastRepeats (priceWithNoClamp skipTariffCoins) := by
  intro h
  have := h 3 (by decide)
  revert this
  decide

theorem a_charge_that_skips_the_wallet_overpays :
    ¬ PaidWithin settleAppliedWithoutTheWallet := by
  intro h
  have := (h 1 20).2.2.1
  revert this
  decide

theorem a_refund_of_the_price_returns_more_than_was_paid :
    ¬ RefundsWhatWasPaid refundOfThePrice := by
  intro h
  have := h 1 20
  revert this
  decide

end Formal.SkipTariff
