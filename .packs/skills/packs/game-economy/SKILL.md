---
name: game-economy
description: >-
  Teaches, and checks, a study game's economy against gamify v9's exact math and against the
  ethics of reward design: the per-review XP table and level curve, the Bloom-tier multiplier on
  the tagged track, chest odds with their pity and a truthful disclosure, chests never sold,
  coin faucets and sinks, the daily loss cap, only coins confiscable, and streak forgiveness
  without pressure. Runs over any repository's economy.json through
  scripts/game-economy-probe.py --root. Use when porting, tuning or reviewing XP, chests,
  coins, streaks, wagers or penalties.
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/game-economy

A study game's economy, checked against gamify v9's exact math and against the ethics of reward
design (SPEC-V2-2224 / ADR-V2-2224). DeckStreak ports v9 with full parity, so its XP, chests,
coins and streaks must come out of the port with the same numbers they went in with. The owner's
rules come with them: "coins are the only confiscable stake", and no dark pattern may lean on a
streak.

Three things live here:

- **the reference economy**, [templates/economy.json](templates/economy.json): v9's exact values
  in one file, schema `phx.game.economy.v1`. It is also the template a project copies to its
  root as `economy.json`, the ONE place its engine reads every constant from;
- **the check**, `scripts/game-economy-probe.py`, standard-library Python that judges any tree's
  `economy.json` through `--root` against the reference, or against another one through
  `--reference FILE` once the owner records a different decision;
- **the teaching** below: the math, the invariants, and what the engine must do that a static
  read cannot see.

Which seats consume this pack is its catalog row's `consumes`, the one record of that edge, so
this body names none.

```
phxd pack probe --pack game-economy --root PATH --format json
```

## How it composes

- **ux-laws** reads the product's COPY and UI: `gamification.streak-loss-copy`,
  `gamification.streak-forgiveness`, `gamification.streak-repair-sold`,
  `gamification.reward-odds`, `gamification.paid-random-odds`, `deceptive.fake-urgency` and
  `deceptive.confirmshaming`. This pack reads the MECHANICS in `economy.json`, so the two never
  judge the same thing. Run both; a streak that is forgiving in the config and threatening in
  the copy is still a dark pattern.
- **notifications-policy** owns the celebration ladder, including the streak-death day's calm
  tier and the honest-outcome cap (no loss disguised as a win, no fabricated near-miss).
- **data-migration** proves the port of v9's stored data; this pack proves the rules that act on
  it.

## The rows

Thirteen rows, all `tree`-scoped, one per class of `game-economy-probe.py`. Each runs
`python3 {skills}/../scripts/game-economy-probe.py --root {root} --reference {skills}/packs/game-economy/templates/economy.json check <class>`
under a 60-second wall, so the reference always comes from this pack, whatever tree `--root`
names.

Stage `declare`: 1 row (1 blocking, 0 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `economy-declared` | block | `economy-undeclared` | `economy.json` is missing or not JSON, its schema is not `phx.game.economy.v1`, a key is missing or unknown (a typo would otherwise hide from parity), or a value has the wrong JSON type. A boolean is not a number; an integer where the reference writes a float is the same number |

Stage `xp`: 3 rows (3 blocking, 0 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `xp-table` | block | `xp-drift` | the XP base, the rounding, an ease, maturity or type multiplier, a daily bonus or a level-curve coefficient differs from the reference; the curve does not start level 1 at 0 XP or its per-level cost does not rise; a multiplier is not positive |
| `tier-multiplier` | block | `tier-multiplier-drift` | the Bloom tiers differ from T1 1.0, T2 2.0, T3 3.0, T4 5.0, a higher tier does not earn more, the tagged track differs or is empty, or the untagged rate is not 1.0 |
| `xp-inflation` | block | `xp-uncapped` | a bonus or a chest payout differs from the reference, a bonus cap is not positive, a multiplier-derived bonus is not in `day_base_excludes` (it would compound), or a payout range runs backwards |

Stage `chests`: 3 rows (3 blocking, 0 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `chest-odds` | block | `chest-odds-invalid` | the odds differ from 70/22/7/1 or do not sum to 100; the pity differs (ramp after 8 dry chests, +5 points each, ceiling 40, Epic guaranteed by the 14th, Legendary by the 40th); the ramp starts at or after the guarantee; the ceiling is below the base Epic odds; the Legendary guarantee comes first; or the per-day cap, the 15-card effort floor or the 10-minute session gap differs |
| `odds-disclosed` | block | `odds-disclosure-false` | `chests.disclosure` names no file, a named file is missing or lies outside the tree, a rarity's odds are stated nowhere, a stated percent differs from the percent rolled, or a guarantee is stated nowhere |
| `chest-not-sold` | block | `paid-random-reward` | a chest is purchasable, coins are sold for money, a shop item is priced in anything but earned coins, or a shop item grants a chest, loot, a spin or any randomized reward |

Stage `coins`: 4 rows (3 blocking, 1 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `coin-flows` | block | `coin-flow-drift` | the mint (base XP divided by 25, at most 40 a day), an earn amount or a shop price differs from the reference, the mint's divisor or cap is not positive, or a price is not positive |
| `coin-balance` | advisory | `coin-inflation-risk` | the shop has no repeatable sink, a repeatable sink costs more than 30 days of the full daily mint (`--max-days-to-afford` moves the horizon), or every sink costs less than one day of it. Advisory |
| `loss-cap` | block | `loss-uncapped` | the daily loss cap (the lesser of 100 coins and 30% of the wallet at rollover), the fines (2% of the wallet, the flat fines) or the zero floor differs; the cap is not positive; its share is not above 0 and at most 1; the wallet floor is not 0; or the fine share exceeds the cap's |
| `only-coins-confiscable` | block | `non-coin-confiscation` | a penalty debits anything but coins or nothing, or `unconfiscable` omits XP, the streak, the level, badges or CEFR progress |

Stage `streaks`: 2 rows (2 blocking, 0 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `streak-rules` | block | `streak-drift` | the streak, governor or wager values differ from the reference; one missed day breaks a streak; the freeze cap is below the starting freezes; or the skip tariff is not a rising list of prices |
| `streak-pressure` | block | `streak-pressure` | a new streak starts with no freeze, freezes are not earned by study, a freeze is sold for anything but coins, a penalty is not opt-in, a lapse does not suppress penalties, or the wager is not opt-in, is armed without the governor, stakes more than half the wallet, is not voided with a refund in standby and in a lapse, or stakes the streak itself; or coming back after a lapse is not rewarded |

Every class prints one line per finding, `<class>: <finding>`, and ends with `examined N`, the
values it read. The script exits 0 when green, 1 on a finding, 2 on a usage error, and 3 when
VOID: the economy or the reference could not be read. VOID is never a pass, and a missing
`economy.json` is `economy-declared`'s finding and every other class's VOID.

Severity: parity with v9 is `block`, because the owner chose full parity and the digest calls the
math "fixed and must port verbatim". The invariants are `block` where they carry a platform rule
(Apple 3.1.1, Google Play and Korea's probability-disclosure law all key on a sold random item)
or one of v9's own anti-goals (no confiscation beyond coins, no unbounded loss or faucet, no
imposed penalty). Only the faucet-and-sink ratio is a judgement, so only `coin-balance` advises.

## The math, verbatim

- **Per-review XP** is `round(10 × ease × maturity × type × tier)`, multiplied left to right.
  Ease: again 0.5, hard 1.0, good 1.2, easy 1.0. Maturity by the review's NEW interval: 21 days
  or more 2.0, above 0 1.3, else 1.0. Type: learn 0.8, review 1.0, relearn 1.1, filtered 0.7.
- **Round half to even.** v9 is Python, and Python's `round` sends 6.5 to 6. An again answer on
  a young card in review earns 6 XP in v9; a port using Rust's `f64::round` pays 7. Use
  `f64::round_ties_even`.
- **Daily bonuses:** studied 50, backlog cleared 100, streak 5 a day up to 250, a score of 90 or
  more 200, each graduation 30.
- **Levels:** reaching level L takes `50L² − 50L` XP, so level 1 is 0, level 2 is 100, level 3
  is 300 and each level L costs `100L`. The inverse is `max(1, (50 + isqrt(2500 + 200·xp)) // 100)`.
- **Bloom tiers** multiply the tagged track only: T1 1, T2 2, T3 3, T4 5. An untagged card and
  every other track keep 1.0, so the untagged path is byte-identical to the plain one.
- **Bonuses never compound:** consistency (1 + 0.15 per on-pace day, at most 2.5), the next-day
  buff (25% of review XP, at most 150) and the double-XP token (2 hours, at most 300) are left
  out of the day's base.
- **Chests:** one per real session (a 10-minute gap splits sessions; at least 15 distinct cards),
  at most 3 a day, rolled once on the server and stored. Odds 70/22/7/1; after 8 chests without
  an Epic, +5 Epic points per chest up to 40; an Epic guaranteed by the 14th and a Legendary by
  the 40th. Payouts: common 10-25, rare 30-60, each capped at the greater of 25 and 30% of the
  session's review XP; legendary 150; an untapped Epic resolves to 50.
- **Coins:** minted at `min(40, day_base_xp // 25)`; a fine is `max(configured, ceil(2% of the
  wallet))`; losses are capped at `min(100, 30% of the wallet at rollover)` and nothing is taken
  from an empty wallet. A freeze costs 150, a scroll pass 40, and skips cost 0, 50 then 100 in a
  month.
- **Streaks:** one freeze at the start, at most 3, one more every 7 streak days; one missed day
  spends a freeze and two break the streak; freeze drops are capped at one a month across every
  source.

`python3 scripts/game-economy-probe.py golden` prints the per-review XP of all 240 combinations
and the XP each of the first 100 levels needs, computed the way v9 computes them. The port's own
tests compare against that table.

## The ethics the rows encode

- **Coins are the only confiscable stake.** XP, the streak, levels, badges and CEFR progress can
  never be taken; the bound is the anti-abandonment guarantee (v9's first rule).
- **A streak rewards coming back.** Missing one day barely dents a forming habit (Lally et al.
  2010), and a broken streak's pull fades when it can be repaired (Silverman and Barasch 2023),
  so freezes are earned by study and a return after a lapse is rewarded.
- **Nothing that rolls is sold.** Every rule on randomized rewards keys on a purchase: Apple's
  3.1.1 and Google Play's odds disclosure, Korea's probability-item law, and the EU consumer
  network's principles, which exclude a currency earned only by play. Keeping chests and coins
  unsold keeps DeckStreak outside all of them, and `chest-not-sold` holds that line.
- **The odds shown are the odds rolled.** A misstated odd is deceptive whether or not the item is
  sold, so `odds-disclosed` compares every stated percent with the configuration.
- **No pressure beyond the owner's choice.** Penalties are opt-in, bounded by the loss cap and
  silenced in a lapse; a wager stakes coins, never the streak. Temporal dark patterns such as
  playing by appointment (Zagal, Björk and Lewis 2013) are what freezes and skip days defuse.

What no static read can see, and the engine does:

- **Roll once, on the server.** Draw from a cryptographic source, store the result keyed by the
  session, and never re-roll on a re-sync; the Mini App only animates the stored rarity.
- **Read the constants from economy.json.** Load the file at start-up or embed it at build time;
  a constant typed twice is a constant that drifts. No model improvises a grant.
- **Measure the faucets.** Graduation XP that never survives, quests nobody can finish and a
  daily board that is rarely winnable are runtime facts; log them and review them.

## How a project applies this

1. **Declare the economy.** Copy `templates/economy.json` to the repository root as
   `economy.json`, and name the file or files that show the chest odds in
   `chests.disclosure`, for example the Mini App's odds page or its message catalog.
2. **Build from it.** The engine reads every constant from `economy.json`; the port's tests
   compare their XP with the `golden` table.
3. **Show the odds.** The disclosure file states each rarity with its percent (`Epic 7%` or
   `7% epic`) and both guarantees.
4. **Run the check.** `phxd pack probe` with `--pack game-economy` against the tree in CI and
   before each release. A different decision by the owner is a new reference with an ADR,
   passed through `--reference`.

## References

The access date of every source and the Context7 answers are recorded in SPEC-V2-2224 §8.

- Apple App Store Review Guidelines, 3.1.1: https://developer.apple.com/app-store/review/guidelines/
- Google Play Payments policy: https://support.google.com/googleplay/android-developer/answer/9858738
- Korea's probability-disclosure enforcement: https://www.kimchang.com/en/insights/detail.kc?sch_section=4&idx=29487
- EU consumer network, key principles on in-game virtual currencies: https://commission.europa.eu/document/download/8af13e88-6540-436c-b137-9853e7fe866a_en?filename=Key+principles+on+in-game+virtual+currencies.pdf
- FTC, Bringing Dark Patterns to Light: https://www.ftc.gov/system/files/ftc_gov/pdf/P214800+Dark+Patterns+Report+9.14.2022+-+FINAL.pdf
- Zagal, Björk and Lewis 2013, Dark patterns in the design of games: https://files01.core.ac.uk/download/pdf/301007767.pdf
- Lally et al. 2010, How are habits formed: https://onlinelibrary.wiley.com/doi/abs/10.1002/ejsp.674
- Silverman and Barasch 2023, On or off track: https://academic.oup.com/jcr/article-abstract/49/6/1095/6623414
- Dixon et al. 2010, losses disguised as wins: https://onlinelibrary.wiley.com/doi/10.1111/j.1360-0443.2010.03050.x
- Clark et al. 2009, near-misses: https://pubmed.ncbi.nlm.nih.gov/19217383/
- Sailer and Homner 2020, gamification of learning: https://eric.ed.gov/?id=EJ1245270
- Lehdonvirta and Castronova 2014, Virtual Economies: https://direct.mit.edu/books/monograph/3441/Virtual-EconomiesDesign-and-Analysis
- Rust `f64::round_ties_even`: https://doc.rust-lang.org/stable/std/primitive.f64.html
- Context7 ids: `/websites/doc_rust-lang_stable_std`, `/python/cpython`
