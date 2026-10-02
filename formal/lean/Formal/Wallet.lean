-- @phx covers crates/economy/src/rules.rs anchor=clip_debit digest=sha256:445fb9f9b87efed0a5bb85317173ac2da096244522e2f68106692e09ab3eb89f
-- @phx covers crates/economy/src/rules.rs anchor=mint_for_base_xp digest=sha256:499486a1493177486c91c1b87c17dba656d3cf67952b612c1f57f3ec36ce677b
-- @phx vectors formal/vectors/wallet.jsonl
-- @phx cites #106
-- @phx theorem clip_debit_pays_within_its_bounds ramp=report
-- @phx witness a_clip_that_skips_the_wallet_overpays kills=clip_debit_pays_within_its_bounds
-- @phx theorem the_mint_lies_between_zero_and_the_cap ramp=report
-- @phx witness a_mint_with_no_daily_cap_leaves_the_range kills=the_mint_lies_between_zero_and_the_cap
-- @phx theorem the_mint_is_monotone_in_the_base ramp=report
-- @phx witness a_mint_over_the_remainder_falls_as_the_base_grows kills=the_mint_is_monotone_in_the_base

/-!
# Formal.Wallet

The coin wallet's two pure rules (#106), `clip_debit` and `mint_for_base_xp` in
`crates/economy/src/rules.rs`, as total functions over `Int`.

**The claim.** For every input:
- `clip_debit_pays_within_its_bounds`: a request of 0 or less pays nothing and forgives nothing;
  every debit pays at least 0; and a positive request, against a wallet and a remaining cap that
  are not negative, pays at most the least of the request, the wallet and the remaining cap;
- `the_mint_lies_between_zero_and_the_cap`: a day's mint is at least 0 and at most the daily cap,
  40 coins;
- `the_mint_is_monotone_in_the_base`: more base XP never mints fewer coins.

**What is ported.** Both functions read `i64` and call only `min`, `max`, a comparison and
`div_euclid` by a positive constant, none of which overflows, so the port over `Int` is exact.
For a positive divisor `div_euclid` is floor division, which `Int`'s `/` is. The constants are
`COIN_MINT_XP_DIVISOR` (25) and `COIN_MINT_DAILY_CAP` (40) from `crates/economy/src/constants.rs`;
the vectors carry them to the Rust test, which fails if either moves.

**What is not ported.** The loss cap and the scaled fine go through floats; their parity is proved
by the goldens.

**The witnesses** are ports that each break one claim at one input: a clip that skips the wallet
(the mutant that drops `.min(wallet)`), a mint with no daily cap, and a mint over the base's
remainder in place of its quotient.
-/

namespace Formal.Wallet

/-- `COIN_MINT_XP_DIVISOR`: the base XP that buys one coin. -/
def coinMintXpDivisor : Int := 25

/-- `COIN_MINT_DAILY_CAP`: the most coins a day mints. -/
def coinMintDailyCap : Int := 40

/-- `clip_debit`: the amount allowed, clipped to the wallet and the remaining cap and never below 0,
and whether any part of the request was forgiven. -/
def clipDebit (requested wallet capRemaining : Int) : Int × Bool :=
  if requested ≤ 0 then (0, false)
  else
    let allowed := max (min (min requested wallet) capRemaining) 0
    (allowed, decide (allowed < requested))

/-- `mint_for_base_xp`: one coin per divisor of base XP, at most the cap, none for a base of 0 or
less. -/
def mintForBaseXp (baseXp : Int) : Int :=
  if baseXp ≤ 0 then 0 else min coinMintDailyCap (baseXp / coinMintXpDivisor)

/-- The wrong variant: the clip skips the wallet, so a debit can pay past the balance. -/
def clipDebitWithoutTheWallet (requested _wallet capRemaining : Int) : Int × Bool :=
  if requested ≤ 0 then (0, false)
  else
    let allowed := max (min requested capRemaining) 0
    (allowed, decide (allowed < requested))

/-- The wrong variant: a mint with no daily cap. -/
def mintForBaseXpWithNoCap (baseXp : Int) : Int :=
  if baseXp ≤ 0 then 0 else baseXp / coinMintXpDivisor

/-- The wrong variant: a mint over the base's remainder in place of its quotient. -/
def mintForBaseXpOverTheRemainder (baseXp : Int) : Int :=
  if baseXp ≤ 0 then 0 else min coinMintDailyCap (baseXp % coinMintXpDivisor)

/-- The claim, stated once: a debit pays within its bounds. -/
def PaysWithin (clip : Int → Int → Int → Int × Bool) : Prop :=
  ∀ r w c : Int,
    (r ≤ 0 → clip r w c = (0, false)) ∧ 0 ≤ (clip r w c).1 ∧
      (0 < r → 0 ≤ w → 0 ≤ c → (clip r w c).1 ≤ min r (min w c))

/-- The claim, stated once: the mint lies between 0 and the daily cap. -/
def InRange (mint : Int → Int) : Prop :=
  ∀ b : Int, 0 ≤ mint b ∧ mint b ≤ 40

/-- The claim, stated once: the mint never falls as the base grows. -/
def MintMonotone (mint : Int → Int) : Prop :=
  ∀ b b' : Int, b ≤ b' → mint b ≤ mint b'

theorem clip_debit_pays_within_its_bounds : PaysWithin clipDebit := by
  intro r w c
  unfold clipDebit
  split
  · exact ⟨fun _ => rfl, Int.le_refl 0, fun hr => absurd hr (by omega)⟩
  · rename_i h
    dsimp only
    exact ⟨fun hr => absurd hr h, by omega, fun _ _ _ => by omega⟩

theorem the_mint_lies_between_zero_and_the_cap : InRange mintForBaseXp := by
  intro b
  simp only [mintForBaseXp, coinMintDailyCap, coinMintXpDivisor]
  split <;> omega

theorem the_mint_is_monotone_in_the_base : MintMonotone mintForBaseXp := by
  intro b b' hle
  simp only [mintForBaseXp, coinMintDailyCap, coinMintXpDivisor]
  split <;> split <;> omega

theorem a_clip_that_skips_the_wallet_overpays : ¬ PaysWithin clipDebitWithoutTheWallet := by
  intro h
  have := (h 5 2 10).2.2 (by decide) (by decide) (by decide)
  revert this
  decide

theorem a_mint_with_no_daily_cap_leaves_the_range : ¬ InRange mintForBaseXpWithNoCap := by
  intro h
  have := (h 2500).2
  simp [mintForBaseXpWithNoCap, coinMintXpDivisor] at this

theorem a_mint_over_the_remainder_falls_as_the_base_grows :
    ¬ MintMonotone mintForBaseXpOverTheRemainder := by
  intro h
  have := h 24 25 (by decide)
  simp only [mintForBaseXpOverTheRemainder, coinMintDailyCap, coinMintXpDivisor] at this
  omega

end Formal.Wallet
