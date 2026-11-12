-- @phx covers crates/quests/src/chests.rs anchor=epic_odds_pts digest=sha256:f07f770d9593d389c2a70d250579c0afafe73f06e2a9f8617f6333affe5fd102
-- @phx covers crates/quests/src/chests.rs anchor=roll_rarity digest=sha256:899fdf1fed9443650ac769110f0f9e4f7b47a1e589cdcb3e655777152cbacb82
-- @phx covers crates/quests/src/chests.rs anchor=payout_xp digest=sha256:2e964c18512e9d04b9a2a2537da0d7d118d133dcddacad8d145bad1ee83c943b
-- @phx vectors formal/vectors/chest.jsonl
-- @phx cites #102, #103
-- @phx theorem the_epic_odds_lie_between_the_base_and_the_ceiling ramp=report
-- @phx witness epic_odds_with_no_ceiling_leave_the_range kills=the_epic_odds_lie_between_the_base_and_the_ceiling
-- @phx theorem a_guaranteed_chest_is_its_rarity_whatever_the_draw ramp=report
-- @phx witness guarantees_read_on_the_bare_counters_come_one_chest_late kills=a_guaranteed_chest_is_its_rarity_whatever_the_draw
-- @phx theorem each_payout_keeps_its_cap_or_its_fixed_amount ramp=report
-- @phx witness a_payout_with_no_cap_passes_it kills=each_payout_keeps_its_cap_or_its_fixed_amount

/-!
# Formal.Chest

A chest's three pure rules (#102, #103), `epic_odds_pts`, `roll_rarity` and `payout_xp` in
`crates/quests/src/chests.rs`, as total functions over `Int`.

**The claim.** For every input:
- `the_epic_odds_lie_between_the_base_and_the_ceiling`: the Epic percent points are at least the
  base 7 and never pass the ceiling 40, whatever the pity counter and the buffs;
- `a_guaranteed_chest_is_its_rarity_whatever_the_draw`: when the chest since the last Legendary
  reaches 40 (`since_legendary + 1 >= 40`) the chest is a Legendary, and otherwise when the chest
  since the last Epic reaches 14 (`since_epic + 1 >= 14`) it is an Epic, for every draw;
- `each_payout_keeps_its_cap_or_its_fixed_amount`: a Common or a Rare pays at most its cap,
  `max(25, share)` where `share` is the session's truncated review-XP share; a Legendary pays 150
  and an Epic 0.

**What is ported, and how the floats are.** The code computes in `f64`; the port performs the same
operations over `Int`, so no claim rests on a fact about floating point:
- `epic_odds_pts` adds the base, the ramp and the clamped buff and takes the least of that and the
  ceiling. Every operand is a whole number of points (the constants, and the callers' buffs 0 and
  10), so `Int` is the code's arithmetic on every input it is given;
- `roll_rarity` compares the clamped draw's percent with three thresholds. The port takes that
  comparison as an argument, `below t`, read "the draw's percent is below `t`", and the claims hold
  for every such predicate, so for every draw;
- `payout_xp` truncates two products, the session XP's share (`xp * 0.30`) and the draw's place in
  the band (`draw * width`). Both are arguments of the port, `share` and `band`, and the claim holds
  for every pair. `share` is NOT proved equal to `floor(3 * xp / 10)`: 0.30 has no exact `f64`, so
  the vectors record it only at the XP where the truncation was measured equal to it.

The constants are those at the head of `chests.rs`; the vectors carry them to the Rust test, which
fails if one moves.

**The witnesses** are ports that each break one claim at one input: odds with no ceiling, at a
counter of 30; guarantees read on the bare counters (`since_legendary >= 40`), at a counter of 39
with a draw that folds to a Common; and a payout with no cap, a Rare at the band's bottom for a
session of 0 XP.
-/

namespace Formal.Chest

/-- A chest's rarity, `Rarity` in `chests.rs`. -/
inductive Rarity where
  | common
  | rare
  | epic
  | legendary
  deriving DecidableEq, Repr

/-- `BASE_ODDS_RARE`, in percent points. -/
def baseOddsRare : Int := 22

/-- `BASE_ODDS_EPIC`, in percent points. -/
def baseOddsEpic : Int := 7

/-- `BASE_ODDS_LEGENDARY`, in percent points. -/
def baseOddsLegendary : Int := 1

/-- `EPIC_ODDS_CEILING_PCT`: the most Epic points a ramp and its buffs can reach. -/
def epicOddsCeilingPct : Int := 40

/-- `PITY_EPIC_RAMP_AFTER`: chests since the last Epic before the odds ramp. -/
def pityEpicRampAfter : Int := 8

/-- `PITY_EPIC_RAMP_PTS`: Epic points for each chest beyond the ramp's start. -/
def pityEpicRampPts : Int := 5

/-- `PITY_EPIC_GUARANTEE`: the chest since the last Epic that is an Epic whatever the draw. -/
def pityEpicGuarantee : Int := 14

/-- `PITY_LEGENDARY_GUARANTEE`: the chest since the last Legendary that is a Legendary. -/
def pityLegendaryGuarantee : Int := 40

/-- `COMMON_XP`: a Common's payout band. -/
def commonXp : Int × Int := (10, 25)

/-- `RARE_XP`: a Rare's payout band. -/
def rareXp : Int × Int := (30, 60)

/-- `LEGENDARY_XP`: a Legendary's payout. -/
def legendaryXp : Int := 150

/-- `PAYOUT_CAP_FLOOR_XP`: the least a Common or Rare payout's cap allows. -/
def payoutCapFloorXp : Int := 25

/-- `epic_odds_pts`: base plus the ramp plus the clamped buff, never past the ceiling. -/
def epicOddsPts (sinceEpic buffPts : Int) : Int :=
  let ramp := max (sinceEpic - pityEpicRampAfter) 0 * pityEpicRampPts
  min epicOddsCeilingPct (baseOddsEpic + ramp + max buffPts 0)

/-- `roll_rarity`: the guarantees first, then the draw's fold. `below t` is the code's
`u.clamp(0.0, 1.0) * 100.0 < t`. -/
def rollRarity (below : Int → Bool) (sinceEpic sinceLegendary buffPts : Int) : Rarity :=
  if pityLegendaryGuarantee ≤ sinceLegendary + 1 then .legendary
  else if pityEpicGuarantee ≤ sinceEpic + 1 then .epic
  else
    let epic := epicOddsPts sinceEpic buffPts
    if below baseOddsLegendary then .legendary
    else if below (baseOddsLegendary + epic) then .epic
    else if below (baseOddsLegendary + epic + baseOddsRare) then .rare
    else .common

/-- `payout_xp`: `share xp` is the code's `(xp as f64 * PAYOUT_SESSION_FRAC) as i64`, and
`band width` its `(draw * width as f64) as i64`. -/
def payoutXp (share band : Int → Int) (rarity : Rarity) (sessionBaseXp : Int) : Int :=
  let cap := max payoutCapFloorXp (share sessionBaseXp)
  match rarity with
  | .legendary => legendaryXp
  | .epic => 0
  | .common => min cap (commonXp.1 + band (commonXp.2 - commonXp.1 + 1))
  | .rare => min cap (rareXp.1 + band (rareXp.2 - rareXp.1 + 1))

/-- The wrong variant: the odds with no ceiling. -/
def epicOddsPtsWithNoCeiling (sinceEpic buffPts : Int) : Int :=
  let ramp := max (sinceEpic - pityEpicRampAfter) 0 * pityEpicRampPts
  baseOddsEpic + ramp + max buffPts 0

/-- The wrong variant: the guarantees read on the bare counters, one chest late. -/
def rollRarityOnTheBareCounters (below : Int → Bool) (sinceEpic sinceLegendary buffPts : Int) :
    Rarity :=
  if pityLegendaryGuarantee ≤ sinceLegendary then .legendary
  else if pityEpicGuarantee ≤ sinceEpic then .epic
  else
    let epic := epicOddsPts sinceEpic buffPts
    if below baseOddsLegendary then .legendary
    else if below (baseOddsLegendary + epic) then .epic
    else if below (baseOddsLegendary + epic + baseOddsRare) then .rare
    else .common

/-- The wrong variant: a payout with no cap. -/
def payoutXpWithNoCap (_share band : Int → Int) (rarity : Rarity) (_sessionBaseXp : Int) : Int :=
  match rarity with
  | .legendary => legendaryXp
  | .epic => 0
  | .common => commonXp.1 + band (commonXp.2 - commonXp.1 + 1)
  | .rare => rareXp.1 + band (rareXp.2 - rareXp.1 + 1)

/-- The claim, stated once: the Epic odds lie between the base and the ceiling. -/
def OddsInRange (odds : Int → Int → Int) : Prop :=
  ∀ se buff : Int, 7 ≤ odds se buff ∧ odds se buff ≤ 40

/-- The claim, stated once: a guaranteed chest is its rarity whatever the draw. -/
def GuaranteesHold (roll : (Int → Bool) → Int → Int → Int → Rarity) : Prop :=
  ∀ (below : Int → Bool) (se sl buff : Int),
    (40 ≤ sl + 1 → roll below se sl buff = .legendary) ∧
      (sl + 1 < 40 → 14 ≤ se + 1 → roll below se sl buff = .epic)

/-- The claim, stated once: a Common or Rare pays at most its cap, a Legendary 150, an Epic 0. -/
def PaysWithinItsCap (payout : (Int → Int) → (Int → Int) → Rarity → Int → Int) : Prop :=
  ∀ (share band : Int → Int) (rarity : Rarity) (xp : Int),
    ((rarity = .common ∨ rarity = .rare) → payout share band rarity xp ≤ max 25 (share xp)) ∧
      (rarity = .legendary → payout share band rarity xp = 150) ∧
      (rarity = .epic → payout share band rarity xp = 0)

theorem the_epic_odds_lie_between_the_base_and_the_ceiling : OddsInRange epicOddsPts := by
  intro se buff
  simp only [epicOddsPts, epicOddsCeilingPct, baseOddsEpic, pityEpicRampAfter, pityEpicRampPts]
  omega

theorem a_guaranteed_chest_is_its_rarity_whatever_the_draw : GuaranteesHold rollRarity := by
  intro below se sl buff
  refine ⟨fun hl => ?_, fun hl he => ?_⟩
  · simp [rollRarity, pityLegendaryGuarantee, hl]
  · have hn : ¬ 40 ≤ sl + 1 := by omega
    simp [rollRarity, pityLegendaryGuarantee, pityEpicGuarantee, hn, he]

theorem each_payout_keeps_its_cap_or_its_fixed_amount : PaysWithinItsCap payoutXp := by
  intro share band rarity xp
  cases rarity <;> simp [payoutXp, payoutCapFloorXp, legendaryXp, commonXp, rareXp] <;> omega

theorem epic_odds_with_no_ceiling_leave_the_range : ¬ OddsInRange epicOddsPtsWithNoCeiling := by
  intro h
  have := (h 30 0).2
  simp only [epicOddsPtsWithNoCeiling, baseOddsEpic, pityEpicRampAfter, pityEpicRampPts] at this
  omega

theorem guarantees_read_on_the_bare_counters_come_one_chest_late :
    ¬ GuaranteesHold rollRarityOnTheBareCounters := by
  intro h
  have := (h (fun _ => false) 0 39 0).1 (by decide)
  revert this
  decide

theorem a_payout_with_no_cap_passes_it : ¬ PaysWithinItsCap payoutXpWithNoCap := by
  intro h
  have := (h (fun xp => 3 * xp / 10) (fun _ => 0) .rare 0).1 (Or.inr rfl)
  revert this
  decide

end Formal.Chest
