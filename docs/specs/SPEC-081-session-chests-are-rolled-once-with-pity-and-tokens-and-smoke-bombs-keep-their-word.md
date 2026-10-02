# SPEC-081: session chests are rolled once with their pity, and tokens and smoke bombs keep their word

- **Wave:** W3. **Issue:** #102, #103, #104 (epic #4). **Context(s):** `deck-streak-quests` (sessions,
  the effort floor, the roll and its pity, payouts, vaulting, the untapped Epic's fallback, the
  double-XP token and its window, the Perfect Week smoke bomb, and their tables);
  `deck-streak-coordination` (the recompute's chest step: chest XP through the grant port, the token
  bonus through the settle port, the Epic freeze through the streaks' freeze port, every message
  through the router); `deck-streak-api` and `deck-streak-bot` (open, pick, activate); the Mini App
  (the chest inventory and the odds page).
- **Decided by:** ADR-012 (the parity oracle), ADR-024 (`getrandom`, admitted for the session ids,
  is the draw's source here too), ADR-041 (the one router), ADR-071 (each study day is settled once,
  in order), ADR-072 (the token bonus is settled XP), and ADR-081 (one draw per chest from the
  operating system's generator, stored with its pity in one write).
- **Prerequisites:** SPEC-071 (the recompute's fold and each study day's reviews), SPEC-072 (the
  grant and settle ports, the day base and the Ascendant buff), SPEC-076 (the streaks' freeze port
  and its caps), SPEC-082 (the shop, whose screen and `/shop` answer gain this SPEC's tokens and
  smoke bombs), SPEC-041 (the router), SPEC-084 (the reveal tiers; until it lands a reveal renders
  as SPEC-041's line), SPEC-026 (the bot's callbacks), SPEC-028 (the Mini App shell).
  **Mutation band:** `S08100-S08199`.
- **Status:** delivered by E2 in part (moved from `docs/specs/planned/` with its tests, ADR-016):
  the quests core of #102 and #103 in one pull request built in four parts. Section 3c names the
  criteria the next pull requests deliver: E2b wires the core (the fold's steps, the use cases, the
  API, the bot, the Mini App, the odds page), E2c is #104 and E3 (#107) is the shop's arms.
  `docs/red-first/SPEC-081.md` and ADR-081's acceptance come with the last part of E2.

## 1. The problem, measured

- **Nothing exists.** `crates/quests/src/` holds only `lib.rs` (read at `dev` c3d769b).
  `economy.json` declares the chest odds with an empty `chests.disclosure`, so the game-economy
  pack's odds-disclosure check is deferred to this issue (#102).
- **What is ported.** The predecessor's session chests (`gamification/chests.py:sessions_from_reviews`,
  `eligible_sessions`, `roll_rarity`, `epic_odds_pts`, `payout_xp`; `effort.py:meets_chest_floor`;
  `pipeline_layers/loot.py:LootLayer._grant_session_chests`, `open_chest`, `pick_epic_prize`,
  `_sweep_stale_chests`, `vaulted_chests`), its double-XP tokens
  (`pipeline_layers/loot.py:LootLayer.activate_double_xp`, `_recompute_token_xp`) and its Perfect
  Week smoke bombs (`pipeline_layers/loot.py:LootLayer._maybe_earn_smoke_bomb`), at `27ee2bc`.
- **Rules the issues paraphrase, settled against the code.**
  - The day's cap counts every chest of the day, the daily quests' challenge chest and the weekly
    chest included (`database.py:GamifyStore.chests_for_day` returns them all), so a completed
    challenge quest leaves room for fewer session chests.
  - A Legendary roll adds one to the Epic counter, and an Epic adds one to the Legendary counter;
    only the rolled rarity's own counter returns to zero (the pity update in
    `_grant_session_chests` and `_pay_quest`).
  - The token's bonus is the review XP at the base rate, without the Bloom-tier multiplier
    (`_recompute_token_xp` calls `gamification/xp.py:reviews_xp` with no tier map), and it counts
    only the reviews of the study day being settled, so a window that crosses the rollover pays on
    two study days, each capped.
  - The token's two hours, its bonus cap, the smoke bomb's hold and its crown minimum, the day's
    chest cap and the vault hour are literals in the layer, not named constants: their goldens are
    the layer's own functions through stub-store adapters (§7), never a constants read.
  - The predecessor earns and shows smoke bombs, and its celebration says one "cancels a night's
    pending penalties", but no code of it spends one. The spend is inert, and DeckStreak promises
    nothing it cannot do (#104's criterion; the owner decides the spend, #268).
- **Traps.**
  - The predecessor's uniform draw (`random.SystemRandom().random()`) is a 53-bit fraction, folded
    into a rarity at percent thresholds; the port's draw must be a 53-bit fraction in [0, 1) too, or
    the thresholds move.
  - A session's key is its study day and its first review's instant. A late review that shifts a
    session's start by less than a session gap must not buy a second chest; the predecessor skips a
    session whose start lies that close to an existing one.
  - The predecessor vaults by the clock of the recompute that earns the chest (`now_ms`), not the
    session's. Under one scheduled sync a study day, just after the rollover and inside quiet hours
    (ADR-037), a chest the scheduled sync settles is vaulted for the morning, and one an
    owner-triggered recompute earns in the afternoon is sealed and announced.
- **What the parity oracle proves.** Sessions, their effort and eligibility; the rarity roll at
  every threshold and both guarantees; the Epic odds with the ramp, its ceiling and the buffs; the
  payout bands and cap; the grant step's cap, drift guard, skip day, pity update and vault rule; the
  challenge and weekly chests' shapes; opening, the Epic choice and its caps, the sweep; token
  activation and the token bonus; the Perfect Week; and the constants.
- **Prerequisites.** SPEC-071 for the fold and the reviews, SPEC-072 for the ports and the
  Ascendant buff, SPEC-076 for the freeze port, SPEC-041 and SPEC-084 for the router and its tiers.
  SPEC-080 lands after this delivery and calls its challenge and weekly chests and its Perfect Week
  settlement. The declared skip days come from the recompute, which passes none until SPEC-083
  lands, as SPEC-049's lapse slice takes them.

## 2. Requirements

Sessions and the roll (#102)

R1. A session is a run of study reviews of one study day with no gap of 10 minutes or more between
    consecutive reviews (`goldens/chests.constants.json`, `chests.SESSION_GAP_MS`), bounded by its
    first and last review's instants. Its effort counts its study reviews, their distinct cards, and
    their minutes with each answer capped as analytics caps it. A session is eligible when it has at
    least 15 distinct cards (`constants.REAL_EFFORT_CHEST_MIN_DISTINCT`). The sessions, their effort
    and their eligibility equal `goldens/sessions_from_reviews.json`,
    `goldens/eligible_sessions.json` and `goldens/meets_chest_floor.json`.
R2. The chest step registers in phase 4 of SPEC-071's fold and runs for each study day the fold
    settles or evaluates (ADR-071); like every rule the predecessor applies only to the current
    study day, it grants nothing for the past days the first recompute backfills. For each eligible
    session, in order of its start, it grants at most one chest per key (the study day, the origin
    `session`, the session's start), until the day holds as many chests as
    `chest_settings.per_day_max` (the default 3, `goldens/session_chests_granted.json`), counting
    every chest already on that day. It skips a session whose start lies within one session gap of an
    existing session chest's start, and it grants none on a declared skip day. What it grants equals
    `goldens/session_chests_granted.json`.
R3. The rarity is `roll_rarity(u, since_epic, since_legendary, buff_pts)`: base odds of 70, 22, 7
    and 1 percent for Common, Rare, Epic and Legendary (`chests.BASE_ODDS`); Epic odds that gain 5
    points (`chests.PITY_EPIC_RAMP_PTS`) for each chest beyond 8 (`chests.PITY_EPIC_RAMP_AFTER`)
    since the last Epic, capped at 40 (`chests.EPIC_ODDS_CEILING_PCT`); an Epic guaranteed when the
    chest is the 14th since the last Epic (`chests.PITY_EPIC_GUARANTEE`) and a Legendary when it is
    the 40th since the last Legendary (`chests.PITY_LEGENDARY_GUARANTEE`); and 10 Epic points on a
    study day with the Ascendant buff SPEC-072 records (`goldens/session_chests_granted.json`). The
    rarity and the Epic odds equal `goldens/roll_rarity.json` and `goldens/epic_odds_pts.json`.
R4. The payout is `payout_xp(rarity, u, session_xp)`: a Common draws from 10 to 25 and a Rare from
    30 to 60 (`chests.COMMON_XP`, `chests.RARE_XP`), each capped at the greater of 25 and 30
    percent (`chests.PAYOUT_SESSION_FRAC`) of the session's own review XP at the base rate; a
    Legendary pays 150 (`chests.LEGENDARY_XP`); an Epic pays 0, because its choice is the prize. It
    equals `goldens/payout_xp.json`.
R5. Each chest takes two draws from the operating system's generator, one for its rarity and one
    for its payout, each a 53-bit fraction in [0, 1) (ADR-081). The chest row with its rarity and
    payout and the pity counters after it are written in one write through the kernel's
    repository base. A failed draw writes nothing, and the next recompute rolls that session. A
    chest row, once written, is read by every later recompute and never rolled again, and opening
    it reveals what was stored.
R6. The pity counters are one row, `pity`. After an Epic the Epic counter is 0 and the Legendary
    counter gains one; after a Legendary the Legendary counter is 0 and the Epic counter gains one;
    after any other rarity both gain one (`goldens/session_chests_granted.json`).
R7. A chest is vaulted when the recompute that grants it runs at or after the local day's vault
    hour (`chest_settings.vault_hour`, the default 21, `goldens/session_chests_granted.json`) or
    inside quiet hours (SPEC-041's window); otherwise it is sealed and announced through the router
    as the celebration event `chest` with no rarity, which the ladder renders as the predecessor's
    plain line (T2, `goldens/requested_tier.json`, SPEC-084), under the dedupe key
    `chest_earned:<chest id>`, with an Open button and an Open in app button.

Opening, the Epic choice and the sweep (#102)

R8. The owner opens a sealed or vaulted chest from the bot or the Mini App; the first open moves
    it to `opened`, and every later one changes nothing. A Common, Rare or Legendary chest's payout
    is granted once through the grant port as `chest:<chest id>` (scope `once`, track `language`,
    the predecessor's default) on the chest's own study day, and is revealed through the router as
    the celebration event `chest` with its rarity (the ladder's tier for it) under the dedupe key
    `chest_open:<chest id>`, after which the chest is `resolved`. An Epic chest opens to a pending
    choice. Opening equals `goldens/open_chest.json`.
R9. An Epic's choice is a double-XP token or a streak freeze. The freeze goes through the streaks'
    `grant_freeze` with the reason `chest`; when the hold cap (3, `constants.STREAK_FREEZE_CAP`) or
    this calendar month's cap on dropped freezes (1, `constants.FREEZE_DROP_MONTHLY_CAP`, counting
    the chest, weekly-quest and season reasons) refuses it, a token is granted instead and the
    reveal says so. The choice is settled once, and its reveal is the celebration event `chest` with
    the rarity Epic under the chest's `chest_open` key. It equals `goldens/pick_epic_prize.json`.
R10. At each recompute of the current study day, in phase 4 of the fold and after the closed days
    are settled, the sweep resolves every chest of an earlier study day that is still sealed,
    vaulted or opened: its payout is granted once as `chest:<chest id>`, and an Epic with no choice
    pays 50 (`chests.EPIC_FALLBACK_XP`). A vaulted chest of the day before is spared until the next
    study day's sweep. The sweep equals `goldens/sweep_stale_chests.json`.
R11. The chest port also grants the daily quests' two chests (SPEC-080): the challenge chest, one
    per study day with the origin `challenge`, rolled with 10 Epic points and a session base of 200
    XP for its payout cap, with the pity counters updated as for a session chest and never vaulted
    (`goldens/challenge_chest.json`); and the weekly chest, one per study day with the origin
    `weekly`, an Epic without a roll, paying 0 and leaving the pity counters untouched
    (`goldens/weekly_chest.json`).

Double-XP tokens (#103)

R12. A token is granted by an Epic's choice and held until the owner activates it. Activation
    takes the oldest held token and opens a window of two hours from the activation instant; it is
    refused while another token's window is open, and refused when no token is held
    (`goldens/activate_double_xp.json`). The window's bounds are stored in epoch milliseconds.
R13. The token bonus is a derived bonus in phase 5 of the fold, after the day's base XP, and this
    delivery adds `2x:<token id>` to ADR-072's closed registry of derived sources: for each
    activated token and each study day the fold settles or evaluates, the settle port sets
    `2x:<token id>` (track `language`) to the lesser of 300 (`goldens/recompute_token_xp.json`) and
    the review XP at the base rate of that study day's study reviews inside the window; a day with
    no such review settles nothing; a token whose window has ended is marked consumed. The bonus is
    excluded from the day's base (SPEC-072), so it never compounds. It equals
    `goldens/recompute_token_xp.json`.

Perfect Week smoke bombs (#104)

R14. A Perfect Week is settled once per week, keyed by the week's first study day in
    `perfect_weeks`: the week is crowned when its crown days are at least its days that are not
    declared skip days and at least 5; a crowned week adds one smoke bomb to `inventory` unless 2
    are held; the week is recorded settled whether or not it earned one. It equals
    `goldens/smoke_bomb.json`. The daily quests call it on each Monday with last week's crown days
    (SPEC-080); until they do, no week is settled.
R15. The Perfect Week's celebration (the event `quest_all`, the dedupe key
    `smoke_bomb:<epoch day of the week's start>`) and every screen and message that shows smoke
    bombs say what the owner holds and that it has no use yet. No text promises that a smoke bomb
    cancels a penalty, or anything else the code cannot do.

The odds disclosure and the constants

R16. The Mini App's odds page, `/chests/odds`, states each rarity's odds (Common 70%, Rare 22%,
    Epic 7%, Legendary 1%), the Epic ramp and its ceiling, both guarantees (an Epic by the 14th
    chest since the last Epic, a Legendary by the 40th since the last Legendary), the Ascendant and
    challenge buffs of 10 Epic points, that a chest's rarity and payout are rolled once when it is
    earned, and that chests are never sold; every number it states is R3's, from
    `goldens/chests.constants.json`, `goldens/session_chests_granted.json` and
    `goldens/challenge_chest.json`. Its strings live in the message catalogs, and `economy.json`'s
    `chests.disclosure` names the page and the catalogs that hold them.
R17. The quests context's chest and token constants equal `goldens/chests.constants.json` and
    `economy.json`'s `chests` and `xp.bonuses.double_xp_token`, read by one test from both files.

Surfaces, tables and names

R18. The API serves the chest inventory (the chests by state with their study day, the tokens held
    and the active window's end, the smoke bombs held), opens a chest, settles an Epic's choice
    (`token` or `freeze`) and activates a token: GET /api/chests, POST /api/chests/{id}/open,
    POST /api/chests/{id}/pick and POST /api/tokens/activate. Each answers the owner's session only.
R19. The bot answers the callbacks on a chest message (open, pick the token, pick the freeze,
    activate) behind the owner gate (SPEC-026). It gains no command: the shop's answer (SPEC-082)
    gains the tokens held, the active window's end and the smoke bombs held, with an Activate
    button that activates the oldest held token, as the predecessor's shop board does
    (`pipeline_layers/economy.py:EconomyLayer.shop_board`).
R20. The Mini App's `/chests` shows sealed and vaulted chests with Open, an opened Epic's choice,
    the tokens held with Activate, the active window's live countdown from the server's end, and
    the smoke bombs held. Its open animation plays the rarity the server returns and never draws.
    Both routes join `ROUTES`. The shop screen (SPEC-082) shows the same token card and the smoke
    bombs held, so a held token is activated from either screen.
R21. The quests context owns six tables, each `STRICT` with `created_at`, created by
    `migrations/008101_quests_chests_tokens_and_inventory.sql`: `chests` (unique on study day,
    origin and session start), `pity` (one row), `xp_tokens`, `inventory`, `perfect_weeks` and
    `chest_settings` (one row). Each is registered in the context map, declared in `privacy.json`
    and listed in the quests data-rights port, exported and erased; `pity` and `chest_settings` are
    reset in place (the counters to 0, the settings to their defaults).
R22. No identifier this delivery declares in the quests context says `lootbox` or `crate` (the
    lexicon's lock for `chest`), or `gold`, `gem` or `credit` (its lock for `coin`).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | sessions, their effort and their eligibility equal the goldens | `sessions_and_their_effort_match_the_predecessors_goldens` |
| A2 | the rarity roll and the Epic odds equal the goldens at every threshold and both guarantees | `the_rarity_roll_matches_the_predecessors_golden` |
| A3 | the payout equals the golden, bands and cap included | `the_payout_matches_the_predecessors_golden` |
| A4 | the grant step equals the golden: the day's cap counting every chest, the drift guard, the skip day, the pity update and the vault rule | `the_session_chest_grant_matches_the_predecessors_golden` |
| A5 | a second recompute of the same study day never rolls a session again | `a_second_recompute_never_rolls_a_session_again` |
| A6 | a failed draw writes no chest and changes no pity counter | `a_failed_draw_writes_no_chest_and_no_pity` |
| A7 | the challenge and weekly chests equal their goldens | `the_challenge_and_weekly_chests_match_the_predecessors_goldens` |
| A8 | opening pays once on the chest's own study day and reveals the stored rarity | `opening_pays_once_and_reveals_the_stored_rarity` |
| A9 | an Epic's freeze refused by a cap becomes a token, and the choice settles once | `a_capped_freeze_choice_becomes_a_token` |
| A10 | the sweep resolves stale chests, pays an untapped Epic 50 and spares yesterday's vaulted chests | `the_sweep_matches_the_predecessors_golden` |
| A11 | token activation equals the golden: the oldest held token, one window at a time, two hours | `token_activation_matches_the_predecessors_golden` |
| A12 | the token bonus equals the golden, split by study day and capped per token | `the_token_bonus_matches_the_predecessors_golden` |
| A15 | the chest and token constants equal the golden and `economy.json` | `the_chest_constants_equal_the_golden_and_economy_json` |
| A21 | the six tables are exported and erased, `pity` and `chest_settings` reset in place | `the_chest_tables_are_exported_and_erased` |

```acceptance
A1: cargo test -p deck-streak-quests --test chests_sessions -- --exact sessions_and_their_effort_match_the_predecessors_goldens
A2: cargo test -p deck-streak-quests --test chests_roll -- --exact the_rarity_roll_matches_the_predecessors_golden
A3: cargo test -p deck-streak-quests --test chests_roll -- --exact the_payout_matches_the_predecessors_golden
A4: cargo test -p deck-streak-quests --test chests_grant -- --exact the_session_chest_grant_matches_the_predecessors_golden
A5: cargo test -p deck-streak-quests --test chests_grant -- --exact a_second_recompute_never_rolls_a_session_again
A6: cargo test -p deck-streak-quests --test chests_grant -- --exact a_failed_draw_writes_no_chest_and_no_pity
A7: cargo test -p deck-streak-quests --test chests_grant -- --exact the_challenge_and_weekly_chests_match_the_predecessors_goldens
A8: cargo test -p deck-streak-quests --test chests_open -- --exact opening_pays_once_and_reveals_the_stored_rarity
A9: cargo test -p deck-streak-quests --test chests_open -- --exact a_capped_freeze_choice_becomes_a_token
A10: cargo test -p deck-streak-quests --test chests_open -- --exact the_sweep_matches_the_predecessors_golden
A11: cargo test -p deck-streak-quests --test tokens_window -- --exact token_activation_matches_the_predecessors_golden
A12: cargo test -p deck-streak-quests --test tokens_window -- --exact the_token_bonus_matches_the_predecessors_golden
A15: cargo test -p deck-streak-quests --test chests_roll -- --exact the_chest_constants_equal_the_golden_and_economy_json
A21: cargo test -p deck-streak-quests --test chests_rights -- --exact the_chest_tables_are_exported_and_erased
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The game-economy pack's odds-disclosure check is made
enforced when this delivery merges; every other pack named here stays enforced, with no row
deferred for this delivery.

| id | criterion | decided by |
|---|---|---|
| B1 | over `economy.json` and every file its `chests.disclosure` names (`web/app/src/routes/chests/odds/+page.svelte` and the message catalogs `web/app/messages/*.json`), every rarity's stated percent equals the percent rolled, both guarantees are stated, and no chest, coin or randomized reward is sold | the game-economy pack |
| B2 | over `notifications-policy.json` and every delivery call under `crates/`, the chest messages and reveals reach the owner only through the one router | the notifications-policy pack |
| B3 | over `privacy.json`, `PRIVACY.md` and `crates/quests/src/data_rights.rs`, each of the six new tables has a category with its purpose, basis, retention, export and erase | the privacy-gdpr pack |
| B4 | over `web/app/src/routes/chests/` (the inventory and the odds page, in both colour schemes), the rendered screens meet WCAG 2.2 AA | the accessibility pack |
| B5 | over the chest and smoke-bomb strings in `web/app/messages/*.json` and the message texts in `crates/coordination/src/chests/`, no copy states a false urgency, misstates an odd, or promises what the code cannot do | the ux-laws pack |
| B6 | over the identifiers declared in `crates/quests/src/`, none says a word the lexicon replaces for `chest` or `coin` | the ddd probe's lexicon locks |

## 3c. Delivered by the next pull requests

This SPEC lands in four pull requests. This one (E2) delivers the quests core of #102 and #103 in
four parts: part 1 the sessions, the effort floor, the roll, the payout, the constants and their
goldens (A1, A2, A3, A15); part 2 the draw, the pity, the store, the migration and the grant step's
quests side with the challenge and weekly chests (A4 to A7); part 3 opening, the Epic's choice, the
sweep, token activation and the bonus rule (A8 to A12); part 4 the data rights (A21), the privacy
declaration, the context map and the mutation rows. E2b wires the core into the fold, the use cases,
the API, the bot, the Mini App and the odds page; E2c is #104 (the Perfect Week and its smoke bombs);
E3 (#107) delivers the shop's arms. The table below holds the criteria a later pull request
delivers, each row naming that pull request, and the lines under it are their fence lines, each
prefixed with that pull request. A later pull request moves each of its criteria back verbatim: the
row into section 3's table, without the `delivered by` column, and the fence line into the
acceptance fence, without the prefix.

| id | criterion | decided by | delivered by |
|---|---|---|---|
| A13 | the Perfect Week equals the golden: skip days lower the bar, the cap of 2, one settlement a week | `the_perfect_week_matches_the_predecessors_golden` | E2c |
| A14 | no message or string promises a smoke-bomb spend | `test_no_copy_promises_a_smoke_bomb_spend` | E2c |
| A16 | the odds page states every rarity's odds and both guarantees as `economy.json` declares them | `states every rarity's odds and both guarantees as economy.json declares them` | E2b |
| A17 | the open animation plays the rarity the server stored and never draws | `plays the stored rarity and never draws` | E2b |
| A18 | the chest step settles a closed day's chests once and grants none for a past day the first recompute backfills; their XP is granted once and the token bonus settled in phase 5 | `the_chest_step_settles_a_closed_day_once` | E2b |
| A19 | the chest routes answer the owner's session only | `the_chest_routes_answer_only_the_owner` | E2b |
| A20 | a chest callback opens a chest once, for the owner only | `a_chest_callback_opens_once_for_the_owner_only` | E2b |
| A22 | the shop's answer lists the tokens held, the active window's end and the smoke bombs held, and its Activate activates the oldest held token | `the_shop_answer_lists_tokens_and_smoke_bombs_and_activates_the_oldest` | E3 |
| A23 | the shop screen shows the token card and the smoke bombs held | `shows the token card and the smoke bombs held on the shop` | E3 |

E2c: A13: cargo test -p deck-streak-quests --test chests_inventory -- --exact the_perfect_week_matches_the_predecessors_golden
E2c: A14: python3 -m unittest discover -s scripts/tests -p test_chests_copy.py -k test_no_copy_promises_a_smoke_bomb_spend
E2b: A16: pnpm exec vitest run web/app/src/lib/chests/odds.test.ts -t "states every rarity's odds and both guarantees as economy.json declares them"
E2b: A17: pnpm exec vitest run web/app/src/lib/chests/open.test.ts -t "plays the stored rarity and never draws"
E2b: A18: cargo test -p deck-streak-coordination --test chests_steps -- --exact the_chest_step_settles_a_closed_day_once
E2b: A19: cargo test -p deck-streak-api --test chests_routes -- --exact the_chest_routes_answer_only_the_owner
E2b: A20: cargo test -p deck-streak-bot --test chests_callbacks -- --exact a_chest_callback_opens_once_for_the_owner_only
E3: A22: cargo test -p deck-streak-bot --test chests_shop -- --exact the_shop_answer_lists_tokens_and_smoke_bombs_and_activates_the_oldest
E3: A23: pnpm exec vitest run web/app/src/lib/chests/shop-tokens.test.ts -t "shows the token card and the smoke bombs held on the shop"

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/quests/src/lib.rs` | `deck-streak-quests` | changed: the modules below |
| `crates/quests/src/sessions.rs` | `deck-streak-quests` | added: sessions, their effort, the effort floor |
| `crates/quests/src/chests.rs` | `deck-streak-quests` | added: the constants, the roll, the Epic odds, the payout, the chest port's three grants |
| `crates/quests/src/draw.rs` | `deck-streak-quests` | added: the draw from the operating system's generator as a 53-bit fraction (ADR-081) |
| `crates/quests/src/pity.rs` | `deck-streak-quests` | added: the pity counters and their update |
| `crates/quests/src/chest_store.rs` | `deck-streak-quests` | added: the repository over `chests`, `pity` and `chest_settings` |
| `crates/quests/src/tokens.rs` | `deck-streak-quests` | added: activation, the window, the bonus rule, `xp_tokens` |
| `crates/quests/src/inventory.rs` | `deck-streak-quests` | added: the Perfect Week settlement, `inventory`, `perfect_weeks` |
| `crates/quests/src/data_rights.rs` | `deck-streak-quests` | added: the quests data-rights port with the six tables |
| `crates/quests/Cargo.toml` | `deck-streak-quests` | changed: `getrandom` (ADR-024) and the workspace dependencies it uses; `serde` and `serde_json` as dev-dependencies for the golden reader (SPEC-029 R8) |
| `crates/quests/tests/chests_sessions.rs` | `deck-streak-quests` | added |
| `crates/quests/tests/chests_roll.rs` | `deck-streak-quests` | added |
| `crates/quests/tests/chests_grant.rs` | `deck-streak-quests` | added |
| `crates/quests/tests/chests_open.rs` | `deck-streak-quests` | added |
| `crates/quests/tests/chests_inventory.rs` | `deck-streak-quests` | added |
| `crates/quests/tests/chests_rights.rs` | `deck-streak-quests` | added |
| `crates/quests/tests/tokens_window.rs` | `deck-streak-quests` | added |
| `migrations/008101_quests_chests_tokens_and_inventory.sql` | `deck-streak-quests` | added: the six tables, `STRICT`, with `created_at` (SPEC-020 R15, R18) |
| `crates/coordination/src/chests/mod.rs` | `deck-streak-coordination` | added: open, the Epic choice, activation, the inventory view |
| `crates/coordination/src/chests/messages.rs` | `deck-streak-coordination` | added: the chest occasions and their texts |
| `crates/coordination/src/recompute/chests.rs` | `deck-streak-coordination` | added: the chest step and the sweep in phase 4 of SPEC-071's fold, and the token bonus in phase 5 |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: the steps registered in their phases |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the chests module |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the registry gains the quests port |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded rows for each of the six tables |
| `crates/coordination/tests/chests_steps.rs` | `deck-streak-coordination` | added |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the chest port joined to the grant, settle and freeze ports |
| `crates/api/src/chests_routes.rs` | `deck-streak-api` | added: the four routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the chest routes mounted behind the owner's session |
| `crates/api/tests/chests_routes.rs` | `deck-streak-api` | added |
| `crates/bot/src/chests_commands.rs` | `deck-streak-bot` | added: the chest callbacks |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the callback prefixes registered |
| `crates/bot/tests/chests_callbacks.rs` | `deck-streak-bot` | added |
| `crates/bot/src/shop_commands.rs` | `deck-streak-bot` | changed: the shop's answer gains the tokens held, the window's end, the smoke bombs held and the Activate button (R19) |
| `crates/bot/tests/chests_shop.rs` | `deck-streak-bot` | added: A22 |
| `web/app/src/routes/chests/+page.svelte` | miniapp | added: the inventory |
| `web/app/src/routes/chests/odds/+page.svelte` | miniapp | added: the odds disclosure |
| `web/app/src/lib/chests/api.ts` | miniapp | added: the chest routes' client |
| `web/app/src/lib/chests/ChestCard.svelte` | miniapp | added: a chest, its open animation of the stored rarity, an Epic's choice |
| `web/app/src/lib/chests/TokenCard.svelte` | miniapp | added: held tokens, Activate, the window's countdown from the server's end |
| `web/app/src/lib/chests/odds.test.ts` | miniapp | added |
| `web/app/src/lib/chests/open.test.ts` | miniapp | added |
| `web/app/src/routes/shop/+page.svelte` | miniapp | changed: the shop shows the token card and the smoke bombs held (R20) |
| `web/app/src/lib/chests/shop-tokens.test.ts` | miniapp | added: A23 |
| `web/app/src/lib/routes.ts` | miniapp | changed: the chests route and the odds route join `ROUTES` |
| `web/app/src/lib/startapp.ts` | miniapp | changed: the startapp token that opens the chests route from a chest message |
| `web/app/messages/*.json` | miniapp | changed: the chest, token, smoke-bomb and odds strings, in each locale's catalog |
| `economy.json` | repo | changed: `chests.disclosure` names the odds page and the message catalogs |
| `scripts/tests/test_chests_copy.py` | repo | added: no string or message text promises a smoke-bomb spend |
| `tools/parity-oracle/registry/spec_081.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/sessions_from_reviews.json` | repo | added: the golden of `gamification/chests.py:sessions_from_reviews` (§7) |
| `tools/parity-oracle/goldens/eligible_sessions.json` | repo | added: the golden of `gamification/chests.py:eligible_sessions` (§7) |
| `tools/parity-oracle/goldens/meets_chest_floor.json` | repo | added: the golden of `effort.py:meets_chest_floor` (§7) |
| `tools/parity-oracle/goldens/roll_rarity.json` | repo | added: the golden of `gamification/chests.py:roll_rarity` (§7) |
| `tools/parity-oracle/goldens/epic_odds_pts.json` | repo | added: the golden of `gamification/chests.py:epic_odds_pts` (§7) |
| `tools/parity-oracle/goldens/payout_xp.json` | repo | added: the golden of `gamification/chests.py:payout_xp` (§7) |
| `tools/parity-oracle/goldens/session_chests_granted.json` | repo | added: the golden of `pipeline_layers/loot.py:LootLayer._grant_session_chests` (§7) |
| `tools/parity-oracle/goldens/challenge_chest.json` | repo | added: the golden of `pipeline_layers/loot.py:LootLayer._pay_quest` for the challenge slot (§7) |
| `tools/parity-oracle/goldens/weekly_chest.json` | repo | added: the golden of `pipeline_layers/loot.py:LootLayer._update_weekly_quest`'s chest (§7) |
| `tools/parity-oracle/goldens/open_chest.json` | repo | added: the golden of `pipeline_layers/loot.py:LootLayer.open_chest` (§7) |
| `tools/parity-oracle/goldens/pick_epic_prize.json` | repo | added: the golden of `pipeline_layers/loot.py:LootLayer.pick_epic_prize` (§7) |
| `tools/parity-oracle/goldens/sweep_stale_chests.json` | repo | added: the golden of `pipeline_layers/loot.py:LootLayer._sweep_stale_chests` (§7) |
| `tools/parity-oracle/goldens/activate_double_xp.json` | repo | added: the golden of `pipeline_layers/loot.py:LootLayer.activate_double_xp` (§7) |
| `tools/parity-oracle/goldens/recompute_token_xp.json` | repo | added: the golden of `pipeline_layers/loot.py:LootLayer._recompute_token_xp` (§7) |
| `tools/parity-oracle/goldens/smoke_bomb.json` | repo | added: the golden of `pipeline_layers/loot.py:LootLayer._maybe_earn_smoke_bomb` (§7) |
| `tools/parity-oracle/goldens/chests.constants.json` | repo | added: the constants golden (§7) |
| `scripts/mutation-rows.d/S08100-S08199.json` | repo | added: the hand-proved rows (§9) |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains the six tables |
| `privacy.json` | repo | changed: the chests, tokens and inventory categories, and the chest settings category |
| `PRIVACY.md` | repo | changed: one line per new category |
| `docs/schematics/quests-and-chests-lifecycle.md` | docs | added by the W3 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/specs/SPEC-081-session-chests-are-rolled-once-with-pity-and-tokens-and-smoke-bombs-keep-their-word.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-081-a-chest-is-rolled-once-from-the-os-generator-and-stored-with-its-pity.md` | docs | changed: accepted |
| `docs/red-first/SPEC-081.md` | docs | added |
| `Cargo.lock` | workspace | changed |
| `.sqlx/` | workspace | changed: the offline query cache for the new queries |
| `changelog.d/` fragment | repo | added |
| the box-run packs' private wiring (ADR-069) | the maintainer's | changed: the game-economy pack's odds-disclosure check is no longer deferred |

## 5. What this does NOT do

- It builds no smoke-bomb spend: the predecessor promises one and has none, so the spend is inert
  and waits for the owner's decision (#268).
- It builds no chest lock and no ransom: the doomscroll rail's first rung sets them (#110).
- It sends no morning brief and no vaulted-chests line in one; `/chests` shows vaulted chests until
  the brief exists (#122).
- It changes no widget footer for the active token (#121).
- It builds no settings screen for the day's chest cap or the vault hour; both keep the
  predecessor's defaults until it exists (#57).
- It draws no share art for a Legendary chest (#125).
- It imports none of the predecessor's chests, tokens, pity counters or inventory (#61).
- It sells no chest, no token and no randomized reward, now or later (#172).

## 6. Risks

- **The draw is folded at a different threshold** (a 64-bit fraction, or a comparison written with
  `<=`). Detected by `goldens/roll_rarity.json`, whose `threshold` cases put the draw exactly on
  each boundary and one step below it (A2).
- **The day's cap forgets the challenge or weekly chest.** Detected by the `cap` class of
  `goldens/session_chests_granted.json` (A4).
- **A late review buys a second chest for one session.** Detected by the `drift` class of the same
  golden and by A5.
- **The odds page drifts from what is rolled.** Detected by A16, which reads `economy.json`, and by
  the game-economy pack over the named files (B1).
- **A vaulted chest waits unseen until W5's morning brief.** `/chests` lists it from the recompute
  that vaulted it, and the sweep resolves it to its XP on the next study day, so it is never lost.
- **The operating system's generator fails on the host.** The chest step writes nothing for that
  session, logs one warning naming the session's key and never a draw, and the next recompute rolls
  it (A6).
- **A token's window crosses the rollover.** The bonus pays on both study days, each capped, as the
  predecessor's does; the `rollover` class of `goldens/recompute_token_xp.json` holds it (A12).
- **Under one daily sync, most chests are vaulted.** The scheduled sync runs inside quiet hours, so
  the chests it settles wait for the morning; an owner-triggered recompute earns sealed chests the
  owner can open at once. Intended (ADR-037, ADR-071).
- **Copy drifts into a promise.** Detected by A14 over every string and message text, and by the
  ux-laws pack (B5).

## 7. Parity goldens

Each adapter builds what JSON cannot carry and calls the predecessor; none computes a rule
(ADR-029). A stub store is a stand-in holding only the case's rows, as `registry/spec_023.py`'s
`with_a_stub_store` is. Every instant is epoch milliseconds, every day an epoch day number.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `sessions_from_reviews` | `gamification/chests.py:sessions_from_reviews` | adapter | `Review` rows from the case's instants, cards, eases, types and times; returns each session's bounds, reviews, distinct cards and minutes |
| `eligible_sessions` | `gamification/chests.py:eligible_sessions` | adapter | the same reviews' sessions; returns the eligible bounds |
| `meets_chest_floor` | `effort.py:meets_chest_floor` | adapter | a `RealEffort` from the case's counts |
| `roll_rarity` | `gamification/chests.py:roll_rarity` | function | none; classes `threshold`, `guarantee`, `ceiling` |
| `epic_odds_pts` | `gamification/chests.py:epic_odds_pts` | function | none; classes `ramp`, `ceiling`, `buff` |
| `payout_xp` | `gamification/chests.py:payout_xp` | function | none; class `cap` |
| `session_chests_granted` | `pipeline_layers/loot.py:LootLayer._grant_session_chests` | adapter | a stand-in layer over a stub store holding the day's chests, settings, pity, buffs and skip day, the recompute's clock and quiet state, the chest lock patched off, and `random.SystemRandom` patched to return the case's listed draws; returns the chests inserted and the pity after; classes `cap`, `drift`, `skip`, `vault`, `buff` |
| `challenge_chest` | `pipeline_layers/loot.py:LootLayer._pay_quest` (slot `q3`) | adapter | a stand-in layer over a stub store and patched draws; returns the chest inserted and the pity after |
| `weekly_chest` | `pipeline_layers/loot.py:LootLayer._update_weekly_quest` | adapter | a stand-in layer whose stub store completes the weekly quest; returns the chest inserted and the pity after |
| `open_chest` | `pipeline_layers/loot.py:LootLayer.open_chest` | adapter | a stub store holding one chest and a recorder for the celebration; returns the grant, the states and the celebration's event, rarity and key |
| `pick_epic_prize` | `pipeline_layers/loot.py:LootLayer.pick_epic_prize` | adapter | a stub store holding one opened Epic, the freezes held and this month's dropped freezes; returns the choice settled, the freeze or token granted and the reveal's note |
| `sweep_stale_chests` | `pipeline_layers/loot.py:LootLayer._sweep_stale_chests` | adapter | a stub store holding chests of several study days and states; returns the chests resolved and the grants |
| `activate_double_xp` | `pipeline_layers/loot.py:LootLayer.activate_double_xp` | adapter | a stub store holding tokens and a patched clock; returns the answer and the window's bounds in epoch milliseconds |
| `recompute_token_xp` | `pipeline_layers/loot.py:LootLayer._recompute_token_xp` | adapter | a stub store holding activated tokens, the case's reviews and a patched clock; returns the `2x` grants per study day and the tokens consumed; class `rollover` |
| `smoke_bomb` | `pipeline_layers/loot.py:LootLayer._maybe_earn_smoke_bomb` | adapter | a stub store holding the week's crown days, skip days, the smoke bombs held and whether the week was settled; returns whether one was earned, the count held and the settlement |
| `chests.constants` | `gamification/chests.py` and `constants.py` | constants | `chests.BASE_ODDS`, `EPIC_ODDS_CEILING_PCT`, `PITY_EPIC_RAMP_AFTER`, `PITY_EPIC_RAMP_PTS`, `PITY_EPIC_GUARANTEE`, `PITY_LEGENDARY_GUARANTEE`, `SESSION_GAP_MS`, `COMMON_XP`, `RARE_XP`, `LEGENDARY_XP`, `EPIC_FALLBACK_XP`, `PAYOUT_SESSION_FRAC`; `constants.REAL_EFFORT_CHEST_MIN_DISTINCT`, `STREAK_FREEZE_CAP`, `FREEZE_DROP_MONTHLY_CAP` |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `chests` | `deck-streak-quests` | `migrations/008101_quests_chests_tokens_and_inventory.sql` | its chests, each keyed by study day; its challenge and weekly markers become the origins `challenge` and `weekly`, and its message id is not kept | exported and erased |
| `pity` | `deck-streak-quests` | the same migration | its one pity row | reset in place: both counters to 0 |
| `xp_tokens` | `deck-streak-quests` | the same migration | its tokens, their instants in epoch milliseconds | exported and erased |
| `inventory` | `deck-streak-quests` | the same migration | its inventory of smoke bombs | exported and erased |
| `perfect_weeks` | `deck-streak-quests` | the same migration | the weeks its notification ledger marks as settled for a smoke bomb | exported and erased |
| `chest_settings` | `deck-streak-quests` | the same migration | its settings for the day's chest cap and the vault hour | reset in place: the defaults |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S08101-EPIC-BASE-ODDS-SEVEN` | `crates/quests/src/chests.rs` | an Epic's base odds are 7 percent | `chests_roll::the_rarity_roll_matches_the_predecessors_golden` |
| `S08102-LEGENDARY-GUARANTEE-AT-FORTY` | `crates/quests/src/chests.rs` | the 40th chest since the last Legendary is Legendary | `chests_roll::the_rarity_roll_matches_the_predecessors_golden` |
| `S08103-EPIC-GUARANTEE-AT-FOURTEEN` | `crates/quests/src/chests.rs` | the 14th chest since the last Epic is Epic | `chests_roll::the_rarity_roll_matches_the_predecessors_golden` |
| `S08104-EPIC-RAMP-AFTER-EIGHT` | `crates/quests/src/chests.rs` | the Epic ramp starts after 8 chests | `chests_roll::the_rarity_roll_matches_the_predecessors_golden` |
| `S08105-EPIC-CEILING-FORTY` | `crates/quests/src/chests.rs` | the Epic odds never pass 40 points | `chests_roll::the_rarity_roll_matches_the_predecessors_golden` |
| `S08106-SESSION-GAP-TEN-MINUTES` | `crates/quests/src/sessions.rs` | a gap of 10 minutes splits a session | `chests_sessions::sessions_and_their_effort_match_the_predecessors_goldens` |
| `S08107-EFFORT-FLOOR-FIFTEEN-CARDS` | `crates/quests/src/sessions.rs` | a chest needs 15 distinct cards | `chests_sessions::sessions_and_their_effort_match_the_predecessors_goldens` |
| `S08108-EPIC-FALLBACK-FIFTY` | `crates/quests/src/chests.rs` | an untapped Epic resolves to 50 XP | `chests_open::the_sweep_matches_the_predecessors_golden` |
| `S08109-TOKEN-CAP-THREE-HUNDRED` | `crates/quests/src/tokens.rs` | a token's bonus is capped at 300 XP a study day | `tokens_window::the_token_bonus_matches_the_predecessors_golden` |
| `S08110-ONE-CHEST-PER-SESSION-KEY` | `migrations/008101_quests_chests_tokens_and_inventory.sql` | the unique key that makes a session's roll happen once (a script-mutation row whose cargo killer is in `deck-streak-quests`) | `chests_grant::a_second_recompute_never_rolls_a_session_again` |

## 10. Amendments, 2026-10-02: what E2 corrects beside the manifest, and the criteria it no longer lists

Section 3's table now holds only the criteria this pull request delivers (A1 to A12, A15 and A21):
the rows of A13, A14, A16 to A20, A22 and A23 moved out of it, and each stays verbatim in section
3c's table with its fence line there. This section is insert-only: section 4's table and the text
above are unchanged, and each finding below names the text it replaces.

- **T1. Section 4 gains a row.** Add `| \`crates/progression/src/settle.rs\` |
  \`deck-streak-progression\` | changed: the derived registry admits \`2x:<token id>\` (ADR-072's
  registry, R13) |`. The settle port refuses `2x:<token id>` as not derived until it does.
- **T2. Where the steps register.** Section 4, the row of `crates/coordination/src/recompute/mod.rs`:
  old "changed: the steps registered in their phases"; new "changed: the `chests` module, and the
  chest arm of `AwardOffers::offer` (ADR-303)". The row of `crates/daemon/src/wiring.rs`: old
  "changed: the chest port joined to the grant, settle and freeze ports"; new "changed: the chest
  step and the sweep registered in phase 4 and the token bonus in phase 5
  (`recompute_fold_with_relights`)".
- **T3. R7 and "Decided by".** R7, old "otherwise it is sealed and announced through the router as
  the celebration event `chest` with no rarity"; new "otherwise it is sealed, and its announcement
  is owed: the chest row carries an announced mark, and the fold's offers hand it to the router
  between the fold's writes (ADR-303) as the celebration event `chest` with no rarity, marking it
  once the router answers". ADR-303 joins the "Decided by" list, because the fold's steps run inside
  the day's write and its offers run between writes.
- **T4. R9's freeze.** Old "The freeze goes through the streaks' `grant_freeze` with the reason
  `chest`;"; new "The freeze goes through the streaks' freeze port with the reason `chest`, in the
  same write that settles the choice (`grant_freeze_on`, a connection-level twin of
  `grant_freeze`, as `grant_on` is of the grant port);". Section 4 gains the row
  `crates/coordination/src/freeze.rs`, changed, because two concurrent picks could each pay a freeze
  otherwise.
- **T6. The shop arms are E3's.** Section 4's rows of `crates/bot/src/shop_commands.rs` and
  `web/app/src/routes/shop/+page.svelte` name files that do not exist at this base, so neither is
  changed by this SPEC's E2. Those rows, the rows of `crates/bot/tests/chests_shop.rs` and
  `web/app/src/lib/chests/shop-tokens.test.ts`, A22, A23 and the shop sentences of R19 and R20 are
  delivered by E3 (#107), as section 3c lists.
- **T7. R21 after the #104 split.** Old "six tables"; new "four tables (`chests`, `pity`,
  `xp_tokens`, `chest_settings`) by `migrations/008101_quests_chests_and_tokens.sql`; `inventory` and
  `perfect_weeks` by `migrations/008102_quests_inventory_and_perfect_weeks.sql` (E2c)". Section 8's
  table follows R21, and the migration's name in section 4 and in row S08110 is the new one.
- **T8. What quests cannot compute.** R1, old "capped as analytics caps it"; new "capped at the cap
  the caller passes (coordination passes analytics' `ANSWER_TIME_CAP_SECONDS`; quests does not depend
  on analytics, docs/CONTEXT-MAP.md)". R4 and R13 each gain "as the caller computes it" after
  "review XP at the base rate".
- **T9. R17's freeze caps.** The constants golden lists `STREAK_FREEZE_CAP` and
  `FREEZE_DROP_MONTHLY_CAP`, which are the streaks context's constants; quests declares neither. R17
  ends "equal the golden's chest entries; its two freeze caps are held by the streaks context's own
  constants".

The manifest rows this pull request's first part does not touch, each left as section 4 names it
and delivered by a later part or pull request:

- `crates/quests/src/draw.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/quests/src/pity.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/quests/src/chest_store.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/quests/src/inventory.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/quests/src/data_rights.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/quests/tests/chests_grant.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/quests/tests/chests_open.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/quests/tests/chests_inventory.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/quests/tests/chests_rights.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/quests/tests/tokens_window.rs`: unchanged in this part; delivered by a later part or pull request
- `migrations/008101_quests_chests_tokens_and_inventory.sql`: unchanged in this part; delivered by a later part or pull request
- `crates/coordination/src/chests/mod.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/coordination/src/chests/messages.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/coordination/src/recompute/chests.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/coordination/src/recompute/mod.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/coordination/src/lib.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/coordination/src/data_rights_registry.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/coordination/tests/data_rights_symmetry.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/coordination/tests/chests_steps.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/daemon/src/wiring.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/api/src/chests_routes.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/api/src/router.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/api/tests/chests_routes.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/bot/src/chests_commands.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/bot/src/commands.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/bot/tests/chests_callbacks.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/bot/src/shop_commands.rs`: unchanged in this part; delivered by a later part or pull request
- `crates/bot/tests/chests_shop.rs`: unchanged in this part; delivered by a later part or pull request
- `web/app/src/routes/chests/+page.svelte`: unchanged in this part; delivered by a later part or pull request
- `web/app/src/routes/chests/odds/+page.svelte`: unchanged in this part; delivered by a later part or pull request
- `web/app/src/lib/chests/api.ts`: unchanged in this part; delivered by a later part or pull request
- `web/app/src/lib/chests/ChestCard.svelte`: unchanged in this part; delivered by a later part or pull request
- `web/app/src/lib/chests/TokenCard.svelte`: unchanged in this part; delivered by a later part or pull request
- `web/app/src/lib/chests/odds.test.ts`: unchanged in this part; delivered by a later part or pull request
- `web/app/src/lib/chests/open.test.ts`: unchanged in this part; delivered by a later part or pull request
- `web/app/src/routes/shop/+page.svelte`: unchanged in this part; delivered by a later part or pull request
- `web/app/src/lib/chests/shop-tokens.test.ts`: unchanged in this part; delivered by a later part or pull request
- `web/app/src/lib/routes.ts`: unchanged in this part; delivered by a later part or pull request
- `web/app/src/lib/startapp.ts`: unchanged in this part; delivered by a later part or pull request
- `web/app/messages/*.json`: unchanged in this part; delivered by a later part or pull request
- `economy.json`: unchanged in this part; delivered by a later part or pull request
- `scripts/tests/test_chests_copy.py`: unchanged in this part; delivered by a later part or pull request
- `tools/parity-oracle/goldens/session_chests_granted.json`: unchanged in this part; delivered by a later part or pull request
- `tools/parity-oracle/goldens/challenge_chest.json`: unchanged in this part; delivered by a later part or pull request
- `tools/parity-oracle/goldens/weekly_chest.json`: unchanged in this part; delivered by a later part or pull request
- `tools/parity-oracle/goldens/open_chest.json`: unchanged in this part; delivered by a later part or pull request
- `tools/parity-oracle/goldens/pick_epic_prize.json`: unchanged in this part; delivered by a later part or pull request
- `tools/parity-oracle/goldens/sweep_stale_chests.json`: unchanged in this part; delivered by a later part or pull request
- `tools/parity-oracle/goldens/activate_double_xp.json`: unchanged in this part; delivered by a later part or pull request
- `tools/parity-oracle/goldens/recompute_token_xp.json`: unchanged in this part; delivered by a later part or pull request
- `tools/parity-oracle/goldens/smoke_bomb.json`: unchanged in this part; delivered by a later part or pull request
- `scripts/mutation-rows.d/S08100-S08199.json`: unchanged in this part; delivered by a later part or pull request
- `docs/CONTEXT-MAP.md`: unchanged in this part; delivered by a later part or pull request
- `privacy.json`: unchanged in this part; delivered by a later part or pull request
- `PRIVACY.md`: unchanged in this part; delivered by a later part or pull request
- `docs/schematics/quests-and-chests-lifecycle.md`: unchanged in this part; delivered by a later part or pull request
- `docs/decisions/ADR-081-a-chest-is-rolled-once-from-the-os-generator-and-stored-with-its-pity.md`: unchanged in this part; delivered by a later part or pull request
- `docs/red-first/SPEC-081.md`: unchanged in this part; delivered by a later part or pull request
- `Cargo.lock`: unchanged in this part; delivered by a later part or pull request
- `.sqlx/`: unchanged in this part; delivered by a later part or pull request
