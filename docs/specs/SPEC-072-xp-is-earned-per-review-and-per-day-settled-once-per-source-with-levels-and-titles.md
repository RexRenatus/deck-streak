# SPEC-072: XP is earned per review and per day, settled once per source, with levels and their titles

- **Wave:** W3. **Issue:** #70, #71, #72, #73 (epic #4). **Context(s):** `deck-streak-progression`
  (per-review XP, the daily bonuses, `xp_settlement` and its `settle` port, level info and titles, the
  consistency run and bonus, the Ascendant buff and bonus); `deck-streak-ingest` (each card's Bloom
  tier, read from its note's tags); `deck-streak-coordination` (the recompute's XP steps, the level
  view and the level-up occasion); `deck-streak-api`, `deck-streak-bot` and the Mini App (`web/app`)
  (the level views).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-040 (the grant port and its
  append-only ledger), ADR-071 (a closed study day is settled once, in order, with its end-of-day
  state), and ADR-072 (derived XP is settled into its own table, and a closed day's XP never falls).
- **Prerequisites:** SPEC-040 (the grant port, `xp_ledger` and the level from the total), SPEC-041
  (the router, for the level-up line), SPEC-071 (the rollup, the score, the raw streak, the card
  snapshot and the settle fold), SPEC-026 (the bot's command table). **Mutation band:**
  `S07200-S07299`.
- **Status:** in delivery: moved from `docs/specs/planned/` to `docs/specs/` with its tests and
  `docs/red-first/SPEC-072.md` (ADR-016).

## 1. The problem, measured

- **What exists.** At `dev` c3d769b, `crates/progression/src/` holds only `lib.rs`. SPEC-040's
  delivery adds the grant port, the append-only `xp_ledger` and the level derived from its total.
  Nothing computes a review's XP, a daily bonus, a level's title, the consistency run or the Ascendant
  buff, and nothing in `crates/ingest/src/reader.rs` reads a note's tags.
- **What is ported** (predecessor `27ee2bc`, names only):
  - #70: per-review XP (`gamification/xp.py:review_xp`), the daily bonuses
    (`gamification/xp.py:daily_bonus_grants`), level info and titles (`gamification/xp.py:level_info`,
    `gamification/xp.py:level_title`), and the recompute of each day's study-owned XP
    (`pipeline.py:GamifyPipeline._recompute_day`, `pipeline.py:_STUDY_DAY_XP_SOURCES`);
  - #71: the Bloom tier from a note's tags (`anki_reader.py:_parse_tier`), applied to law cards only
    (`pipeline.py:GamifyPipeline._run_sync_cycle_impl` builds the tier map from law cards and splits
    the day's review XP into `reviews` and `reviews_law`);
  - #72: the on-pace run (`gamification/governor.py:tier_down_run` over
    `pipeline_layers/governor.py:GovernorLayer._on_pace_run`), the multiplier
    (`gamification/adaptive.py:consistency_multiplier`), the one-miss preview
    (`gamification/governor.py:projected_multiplier_drop`), and the grant
    (`pipeline_layers/governor.py:GovernorLayer._apply_day_bonuses`) over the day base
    (`database.py:GamifyStore.day_base_xp`);
  - #73: the arming (`pipeline_layers/governor.py:GovernorLayer._maybe_grant_ascendant`) and the grant
    (`GovernorLayer._apply_day_bonuses`).
- **The predecessor rewrites derived XP in place, and lowers a closed day.** Its recompute deletes a
  day's study-owned sources and writes back those that still qualify. `backlog_zero` qualifies only
  for the current study day, and a past day is scored without its card state, so the rollover removes
  a closed day's `backlog_zero` and can remove its `score90`. DeckStreak settles derived XP instead,
  and a closed day's settled XP never falls (ADR-072, CHARTER 5).
- **Traps a hand port falls into.**
  - Python's `round` sends a product that ends in .5 to its even neighbour, and the product is taken
    left to right in floating point. A port that rounds half away from zero, or multiplies in another
    order, earns one XP more on some reviews. The golden's `tie` cases show each one.
  - Maturity is judged by the review's NEW interval, not the interval before it.
  - SPEC-040 stores no level (its R8), so a level-up cannot compare a stored level with a new one.
- **Corrections to the issues, read in the predecessor's code.**
  - #73: the Ascendant amount is computed in `GovernorLayer._apply_day_bonuses`; its
    `_maybe_grant_ascendant` only arms the buff. Both are proved, each by its own golden.
  - #72: the day base counts quest XP. `database.py:GamifyStore.day_base_xp` leaves out the sources
    `readgoal:`, `leech:`, those starting `focus` and `chest`, `2x:`, `surprise`, `consistency` and
    `ascendant`, and counts `quest:q1` and `quest:q2`. So the consistency grant is unchanged when the
    Ascendant bonus, a token or a chest adds XP, and it moves with a quest's pay; the issue's "or quest
    XP" is not the predecessor's rule.
  - #72: "the last 90 rollup days" is the 90 most recent rollup rows, walked one calendar day at a
    time from the oldest to the newest, a day with no rollup scoring as a miss; the golden proves the
    limit, which `economy.json` names `window_days`.
  - #71: DeckStreak's ledger key carries the track (ADR-040), and the law review XP keeps the
    predecessor's source name `reviews_law` on the track `law`, so the goldens of the day bonuses and
    of the exchange readout (#80) hold without a renaming.
- **Where the constants live.** `economy.json`, the game-economy pack's reference, declares the
  per-review table, the daily bonuses, the level curve, the tier multipliers and the consistency and
  Ascendant bounds. The code reads them from it and types none; a test holds each one
  equal to the golden of the predecessor's constants. The day base's exclusions are a rule, not a
  constant: `economy.json`'s `day_base_excludes` names three of them, and the golden of `day_base_xp`
  proves the whole set.
- **What the parity oracle proves.** `review_xp` for every one of the 240 combinations of ease (four
  answers), maturity (mature, young, neither), review type (four) and tier (none and four), with the
  ties and the rows that are not study events; the daily bonuses; level info and titles across every
  title's threshold; the tier parse; the run, the multiplier, the preview and the calendar walk; the
  day base over synthetic ledger rows; the day bonuses; the arming; and the constants.
- **Prerequisites.** SPEC-040 (the grant port and `xp_ledger`, which this SPEC extends without
  changing), SPEC-071 (the rollup and its `score_at_close`, the raw streak, the card snapshot, the
  settle fold and its phases), SPEC-041 (the router), SPEC-026 (the bot's command table). The
  skip days read here stay empty until SPEC-083 records them (#108).

## 2. Requirements

Per-review XP and the law tier (#70, #71)

R1. A study review's XP is the product of the base and its ease, maturity, type and tier multipliers,
    taken left to right in that order in 64-bit floating point and rounded half to even; a row that is
    not a study event earns 0. It equals `goldens/review_xp.json` (`gamification/xp.py:review_xp`) for
    every case. The base is 10, ease 1 to 4 multiply by 0.5, 1.0, 1.2 and 1.0, maturity by the review's
    new interval is 2.0 at 21 days or more, 1.3 above 0 and below 21, else 1.0, and types 0 to 3
    multiply by 0.8, 1.0, 1.1 and 0.7 (`goldens/progression.constants.json`: `constants.XP_BASE`,
    `constants.EASE_XP_MULT`, `constants.XP_MATURE_MULT`, `constants.XP_YOUNG_MULT`,
    `constants.XP_NEUTRAL_MULT`, `constants.MATURE_IVL_DAYS`, `constants.TYPE_XP_MULT`).
R2. Progression reads every constant of R1, R4, R11, R15, R16 and R21 from `economy.json`'s `xp`
    section, embedded at build time; no such number is typed in its source. A test holds each value
    `economy.json` declares equal to its entry in `goldens/progression.constants.json`.
R3. Ingest reads each card's note tags in the same read as the card and keeps only its Bloom tier:
    the first whitespace-separated token that equals `T1`, `T2`, `T3` or `T4` ignoring case, or none,
    equal to `goldens/parse_tier.json` (`anki_reader.py:_parse_tier`). `Card` gains the tier; the tags
    text is dropped inside the read and never leaves ingest.
R4. The tier multiplies only a law-track card's review, by 1.0, 2.0, 3.0 and 5.0 for T1 to T4
    (`goldens/progression.constants.json`: `constants.TIER_XP_MULT`); a law card with no tier and
    every language card earn the multiplier 1.0, whatever their tags.
R5. A day's review XP is two sources: `reviews` on the track `language`, summed over the reviews of
    language-track cards, and `reviews_law` on the track `law`, summed over the reviews of law-track
    cards with their tiers. A law review never adds to `reviews`.

The settlement (ADR-072)

R6. Progression owns `xp_settlement`, created by `migrations/007201_progression_xp_settlement.sql`
    (`STRICT`, `created_at`, SPEC-020 R15 and R18): `study_day`, `source`, `track`, `amount` with
    `CHECK (amount >= 0)`, `closed`, and a unique index on (`study_day`, `source`, `track`).
R7. `settle` takes a study day, a source, a track, an unsigned amount and a cause (a recompute, or the
    owner's correction of a manual entry). A source outside the derived registry is refused before any
    write, by an error that names the rule and never the value. This SPEC registers `reviews`,
    `reviews_law`, `studied`, `backlog_zero`, `streak`, `score90`, `graduations`, `consistency` and
    `ascendant`; SPEC-078, SPEC-079 and SPEC-081 register theirs.
R8. In one `BEGIN IMMEDIATE` write through the kernel's repository base: while the study day is open,
    the amount replaces the stored one; once it has closed, a recompute stores the larger of the stored
    and the recomputed amount, and the owner's correction replaces it. One row is kept per study day,
    source and track.
R9. Only progression names `xp_settlement` in a query or a migration, and no crate but coordination
    calls `settle`. Inside coordination, a call outside `crates/coordination/src/recompute/` passes
    the owner's-correction cause, so only the recompute steps and the owner-correction use cases
    settle. A census over every crate's sources and every migration refuses a planted crate, a planted
    migration and a planted caller, and prints its examined count.
R10. The total XP, overall and per track, is the sum over `xp_ledger` and `xp_settlement`; the level
    and its title are derived from it at read and never stored (SPEC-040 R7, R8).

Daily bonuses and the level (#70)

R11. For each day the recompute evaluates (a closing day at its settle, and the current day), the
    daily bonuses equal `goldens/daily_bonus_grants.json` (`gamification/xp.py:daily_bonus_grants`)
    and are settled on the track `language`: `studied` 50 when the day has a review; `backlog_zero`
    100 when the day's card snapshot has no backlog and nothing due; `streak` 5 a day of the raw streak
    ending that day (SPEC-071), at most 250; `score90` 200 when the day's score is at least 90;
    `graduations` 30 each (`goldens/progression.constants.json`: `constants.XP_BONUS_STUDIED`,
    `constants.XP_BONUS_BACKLOG_ZERO`, `constants.XP_BONUS_STREAK_PER_DAY`,
    `constants.XP_BONUS_STREAK_CAP`, `constants.XP_BONUS_SCORE_90`, `constants.SCORE_90_THRESHOLD`,
    `constants.XP_BONUS_GRADUATION_EACH`).
R12. `backlog_zero` is evaluated only for a day that has its card snapshot: the current day, or a
    closing day at its settle with its end-of-day snapshot (ADR-071). `score90` reads the current
    day's live score, and for a closed day the rollup's `score_at_close`, the score that decided the
    day at its close (SPEC-071); never the historical form a closed day is re-scored in.
R13. Level info (the total, the level, its title and emoji, the XP into the level and the XP the next
    level needs) equals `goldens/level_info.json` (`gamification/xp.py:level_info`), and the title for
    each level equals `goldens/level_title.json` (`gamification/xp.py:level_title`): the predecessor's
    twelve titles, from level 1 to level 100 (`goldens/progression.constants.json`:
    `constants.LEVEL_TITLES`).
R14. When a recompute's writes take the total from one level to a higher one, coordination raises one
    celebration through the router (SPEC-041) with the event `level_up` and the dedupe key
    `level:<level>` for the level reached. The comparison is the level of the total before the
    recompute's first write against the level after its last; no level is stored.

The consistency run and bonus (#72)

R15. The on-pace run of a day equals `goldens/on_pace_run.json`
    (`pipeline_layers/governor.py:GovernorLayer._on_pace_run`): it folds, with
    `goldens/tier_down_run.json` (`gamification/governor.py:tier_down_run`), the 90 most recent
    rollups on or before the day, walked one calendar day at a time from the oldest to the day, the day
    itself left out. A day scoring at least 60 adds 1, a declared skip day leaves the run unchanged, and
    any other day, a day with no rollup included, subtracts 2 with a floor of 0
    (`goldens/progression.constants.json`: `constants.ON_PACE_SCORE`, `constants.TIER_DOWN_STEP`).
R16. The multiplier of a run equals `goldens/consistency_multiplier.json`
    (`gamification/adaptive.py:consistency_multiplier`): 1.0 for a run of 0, else 1 plus 0.15 a day, at
    most 2.5 (`goldens/progression.constants.json`: `gamification.adaptive.CONSISTENCY_STEP`,
    `gamification.adaptive.CONSISTENCY_CAP`). For a day with a review, `consistency` is settled as the
    whole part of the day base times the multiplier minus one, never below 0, equal to
    `goldens/day_bonuses.json` (`GovernorLayer._apply_day_bonuses`).
R17. The day base of a study day equals `goldens/day_base_xp.json`
    (`database.py:GamifyStore.day_base_xp`) over both XP tables and both tracks. It also leaves out the
    readings' own grants (every source starting `reading:`, SPEC-047), so the multiplier and the coin
    mint never scale a reading past the owner's 100 XP (SPEC-001 §9).
R18. The one-miss preview equals `goldens/projected_multiplier_drop.json`
    (`gamification/governor.py:projected_multiplier_drop`) for the current run.
R19. Coordination registers the review XP, the daily bonuses and the Ascendant arming in phase 2 of
    SPEC-071's fold (base XP), and the `consistency` and `ascendant` bonuses in phase 5 (derived
    bonuses), after every base source of the day. A later recompute that raises a base source of a
    closed day settles its derived bonuses again, under R8's raise-only rule.

The Ascendant buff and bonus (#73)

R20. Progression owns `buffs`, created by `migrations/007202_progression_buffs.sql` (`STRICT`,
    `created_at`): `study_day`, `kind` limited to `ascendant`, primary key (`study_day`, `kind`). In
    phase 2, before any chest of the day is rolled, a day is armed as an Ascendant day when, as `goldens/ascendant_arms.json`
    (`GovernorLayer._maybe_grant_ascendant`) decides, it holds no buff yet, is not a declared skip day,
    and the day before it had a review and a settled `backlog_zero` above 0.
R21. For an armed day with a review, `ascendant` is settled as the whole part of a quarter of the
    day's `reviews` and `reviews_law` XP, at most 150, equal to `goldens/day_bonuses.json`
    (`goldens/progression.constants.json`: `constants.ASCENDANT_XP_FRAC`,
    `constants.ASCENDANT_XP_CAP`).
R22. Progression answers whether a study day is an Ascendant day. The chest roll reads it for its Epic
    points (SPEC-081, #102); nothing else in this SPEC changes a chest.

The surfaces

R23. `GET /api/level` answers the owner's session only (SPEC-024) with: the level, its title and
    emoji, the total, the XP into the level and the XP the next level needs; today's XP by source and
    track, each marked `provisional` (the open day's settled XP) or `settled`; the run, its multiplier
    and the one-miss preview; and whether today is an Ascendant day.
R24. `GET /api/level/law-tiers` answers the owner only with the law-track cards in scope counted by
    tier (T1 to T4, and none) and today's `reviews_law` XP split by the tier of each review's card.
R25. The bot's `/level` answers with the predecessor's lines (`bot.py:CommandBot.handle_command`,
    `level`): the title's emoji, the level and the title; the total and the XP into the level over the
    XP the next level needs; and the consistency line when the multiplier is above 1.0.
R26. The Mini App's `/level` shows the level bar, today's XP by source with the provisional ones
    marked as settling at the day's close, the run as a flame meter with its multiplier and the one-miss
    preview beside it, and the Ascendant chip on an Ascendant day. `/level` joins the route table.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a review's XP equals the golden for every combination of ease, maturity, type and tier, the ties and the rows that are not study events (examined count reported, zero refused) | `review_xp_matches_the_parity_golden_for_every_combination` |
| A2 | every XP constant `economy.json` declares equals the predecessor's constant in the golden | `the_progression_constants_equal_the_predecessors_and_economy_json` |
| A3 | the tier parse equals the golden, whole tokens only, case ignored, the first tier kept | `the_bloom_tier_parse_matches_the_parity_golden` |
| A4 | a read keeps each card's tier, and no tags text reaches the read's output | `the_reader_keeps_each_cards_tier_and_never_its_tags` |
| A5 | a law review is settled under `reviews_law` on the law track and never adds to `reviews` | `a_law_review_never_adds_to_the_language_reviews_source` |
| A6 | an untagged law card and a tagged language card earn the multiplier 1.0 | `an_untagged_law_card_and_a_tagged_language_card_earn_the_base_rate` |
| A7 | a recompute raises a closed day's settled amount and never lowers it | `a_closed_days_settled_xp_is_raised_and_never_lowered_by_a_recompute` |
| A8 | the owner's correction replaces a closed day's settled amount | `the_owners_correction_replaces_a_closed_days_settled_xp` |
| A9 | the open day's settled amount follows the record, down as well as up | `the_open_days_settled_xp_follows_the_record` |
| A10 | settle keeps one row per study day, source and track | `settle_keeps_one_row_per_study_day_source_and_track` |
| A11 | a source outside the derived registry is refused before a write, and the error names the rule | `settle_refuses_a_source_outside_the_derived_registry` |
| A12 | only progression names `xp_settlement`, and only the recompute steps and the owner corrections call `settle`; planted fixtures are refused (examined count reported) | `only_progression_writes_xp_settlement_and_only_coordination_settles` |
| A13 | the total and the level are read from both XP tables | `the_level_is_read_from_both_xp_tables` |
| A14 | the daily bonuses equal the golden | `daily_bonus_grants_match_the_parity_golden` |
| A15 | `backlog_zero` is settled only for a day evaluated with its card snapshot | `backlog_zero_is_settled_only_for_a_day_with_its_snapshot` |
| A16 | level info and every title equal the goldens | `level_info_and_titles_match_the_parity_goldens` |
| A17 | a recompute that crosses a level raises one `level_up` occasion for the level reached, and a replay raises none | `a_level_up_is_raised_once_when_a_recompute_crosses_a_threshold` |
| A18 | two recomputes of one day leave identical XP by source and total | `two_recomputes_of_one_day_leave_identical_xp_totals` |
| A19 | the run, the calendar walk, the multiplier and the one-miss preview equal their goldens | `the_consistency_run_and_multiplier_match_the_parity_goldens` |
| A20 | the day base over both tables equals the golden | `the_day_base_matches_the_parity_golden_over_both_tables` |
| A21 | the consistency grant is unchanged when the Ascendant bonus, a token or a chest adds XP | `the_consistency_grant_is_unchanged_by_ascendant_token_and_chest_xp` |
| A22 | the day base leaves out the readings' grants | `the_day_base_leaves_out_the_readings_grants` |
| A23 | the consistency and Ascendant grants equal the golden of the day bonuses | `the_day_bonuses_match_the_parity_golden` |
| A24 | a day is armed as the golden arms it | `the_ascendant_arms_as_the_predecessor_arms_it` |
| A25 | a declared skip day is never armed | `the_ascendant_never_arms_on_a_skip_day` |
| A26 | the two level routes answer only the owner (401 or 403, no data) | `the_level_routes_answer_only_the_owner` |
| A27 | `/level` shows the title, the XP and the consistency line when the multiplier is above 1.0 | `level_shows_the_title_and_the_consistency_bonus` |
| A28 | the level screen marks today's provisional XP as settling at the day's close | `marks today's provisional XP as settling at the day's close` |
| A29 | the level screen shows the one-miss preview beside the flame meter | `shows the one-miss preview beside the flame meter` |
| A30 | a sync cycle whose fold crosses levels announces the level reached once, through the notification router, and a later cycle that crosses none announces nothing (R14) | `a_sync_cycle_announces_a_level_reached_once` |
| A31 | the daemon's composed router answers the owner `GET /api/level/law-tiers` with the seeded law-track card counts and today's `reviews_law` XP split by card tier, and answers no one else (R24) | `the_composed_router_serves_the_law_tiers_to_the_owner_alone` |

```acceptance
A1: cargo test -p deck-streak-progression --test xp_review -- --exact review_xp_matches_the_parity_golden_for_every_combination
A2: cargo test -p deck-streak-progression --test xp_constants -- --exact the_progression_constants_equal_the_predecessors_and_economy_json
A3: cargo test -p deck-streak-ingest --test xp_tier -- --exact the_bloom_tier_parse_matches_the_parity_golden
A4: cargo test -p deck-streak-ingest --test xp_tier -- --exact the_reader_keeps_each_cards_tier_and_never_its_tags
A5: cargo test -p deck-streak-coordination --test xp_steps -- --exact a_law_review_never_adds_to_the_language_reviews_source
A6: cargo test -p deck-streak-coordination --test xp_steps -- --exact an_untagged_law_card_and_a_tagged_language_card_earn_the_base_rate
A7: cargo test -p deck-streak-progression --test xp_settle -- --exact a_closed_days_settled_xp_is_raised_and_never_lowered_by_a_recompute
A8: cargo test -p deck-streak-progression --test xp_settle -- --exact the_owners_correction_replaces_a_closed_days_settled_xp
A9: cargo test -p deck-streak-progression --test xp_settle -- --exact the_open_days_settled_xp_follows_the_record
A10: cargo test -p deck-streak-progression --test xp_settle -- --exact settle_keeps_one_row_per_study_day_source_and_track
A11: cargo test -p deck-streak-progression --test xp_settle -- --exact settle_refuses_a_source_outside_the_derived_registry
A12: cargo test -p deck-streak-progression --test xp_census -- --exact only_progression_writes_xp_settlement_and_only_coordination_settles
A13: cargo test -p deck-streak-progression --test xp_level -- --exact the_level_is_read_from_both_xp_tables
A14: cargo test -p deck-streak-progression --test xp_bonus -- --exact daily_bonus_grants_match_the_parity_golden
A15: cargo test -p deck-streak-coordination --test xp_steps -- --exact backlog_zero_is_settled_only_for_a_day_with_its_snapshot
A16: cargo test -p deck-streak-progression --test xp_level -- --exact level_info_and_titles_match_the_parity_goldens
A17: cargo test -p deck-streak-coordination --test xp_steps -- --exact a_level_up_is_raised_once_when_a_recompute_crosses_a_threshold
A18: cargo test -p deck-streak-coordination --test xp_steps -- --exact two_recomputes_of_one_day_leave_identical_xp_totals
A19: cargo test -p deck-streak-progression --test xp_consistency -- --exact the_consistency_run_and_multiplier_match_the_parity_goldens
A20: cargo test -p deck-streak-progression --test xp_consistency -- --exact the_day_base_matches_the_parity_golden_over_both_tables
A21: cargo test -p deck-streak-progression --test xp_consistency -- --exact the_consistency_grant_is_unchanged_by_ascendant_token_and_chest_xp
A22: cargo test -p deck-streak-progression --test xp_consistency -- --exact the_day_base_leaves_out_the_readings_grants
A23: cargo test -p deck-streak-progression --test xp_ascendant -- --exact the_day_bonuses_match_the_parity_golden
A24: cargo test -p deck-streak-progression --test xp_ascendant -- --exact the_ascendant_arms_as_the_predecessor_arms_it
A25: cargo test -p deck-streak-progression --test xp_ascendant -- --exact the_ascendant_never_arms_on_a_skip_day
A26: cargo test -p deck-streak-api --test xp_routes -- --exact the_level_routes_answer_only_the_owner
A27: cargo test -p deck-streak-bot --test xp_commands -- --exact level_shows_the_title_and_the_consistency_bonus
A28: pnpm exec vitest run web/app/src/lib/level/LevelScreen.test.ts -t "marks today's provisional XP as settling at the day's close"
A29: pnpm exec vitest run web/app/src/lib/level/LevelScreen.test.ts -t "shows the one-miss preview beside the flame meter"
A30: cargo test -p deck-streak-coordination --test level_up_cycle -- --exact a_sync_cycle_announces_a_level_reached_once
A31: cargo test -p deck-streak-daemon --test law_tiers -- --exact the_composed_router_serves_the_law_tiers_to_the_owner_alone
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The game-economy, privacy-gdpr and accessibility packs
stay enforced; no row is deferred for this delivery.

| id | criterion | decided by |
|---|---|---|
| B1 | every economy check stays green over `economy.json`, examining the whole `xp` section (the base, each multiplier, the daily bonuses, the level curve, the tier multipliers and the bonus caps), now that the engine reads it | the game-economy pack |
| B2 | the privacy checks pass over `privacy.json`, `PRIVACY.md` and `crates/progression/src/data_rights.rs`, examining the categories of `xp_settlement` and `buffs` with their export and erase | the privacy-gdpr pack |
| B3 | the accessibility checks pass over `web/app/src/routes/level/+page.svelte` and `web/app/src/lib/level/*.svelte`, examining every element of the level screen | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/progression/src/review_xp.rs` | `deck-streak-progression` | added: a review's XP, the tier multiplier, rounding half to even |
| `crates/progression/src/bonus.rs` | `deck-streak-progression` | added: the daily bonuses |
| `crates/progression/src/settle.rs` | `deck-streak-progression` | added: the `settle` port, the derived registry, the repository over `xp_settlement` |
| `crates/progression/src/level.rs` | `deck-streak-progression` | added: level info and titles (SPEC-040's `xp.rs` keeps the curve) |
| `crates/progression/src/consistency.rs` | `deck-streak-progression` | added: the run, the calendar walk, the multiplier, the preview, the day base |
| `crates/progression/src/buffs.rs` | `deck-streak-progression` | added: the arming, the bonus, the repository over `buffs` |
| `crates/progression/src/economy_config.rs` | `deck-streak-progression` | added: `economy.json`'s `xp` section, embedded at build time |
| `crates/progression/src/ledger.rs` | `deck-streak-progression` | changed: the total sums both XP tables |
| `crates/progression/src/data_rights.rs` | `deck-streak-progression` | changed: `xp_settlement` and `buffs`, exported and erased |
| `crates/progression/src/lib.rs` | `deck-streak-progression` | changed: the modules above |
| `crates/progression/tests/xp_review.rs` | `deck-streak-progression` | added: A1 |
| `crates/progression/tests/xp_constants.rs` | `deck-streak-progression` | added: A2 |
| `crates/progression/tests/xp_settle.rs` | `deck-streak-progression` | added: A7 to A11 |
| `crates/progression/tests/xp_census.rs` | `deck-streak-progression` | added: A12 |
| `crates/progression/tests/xp_level.rs` | `deck-streak-progression` | added: A13, A16 |
| `crates/progression/tests/xp_bonus.rs` | `deck-streak-progression` | added: A14 |
| `crates/progression/tests/xp_consistency.rs` | `deck-streak-progression` | added: A19 to A22 |
| `crates/progression/tests/xp_ascendant.rs` | `deck-streak-progression` | added: A23 to A25 |
| `crates/ingest/src/tier.rs` | `deck-streak-ingest` | added: the Bloom tier and its parse |
| `crates/ingest/src/reader.rs` | `deck-streak-ingest` | changed: each card read with its note's tags, keeping only the tier |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the tier module |
| `crates/ingest/tests/xp_tier.rs` | `deck-streak-ingest` | added: A3, A4 |
| `crates/coordination/src/recompute/xp.rs` | `deck-streak-coordination` | added: the review XP and daily bonus step |
| `crates/coordination/src/recompute/day_bonuses.rs` | `deck-streak-coordination` | added: the arming and the consistency and Ascendant step |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: the XP step joins phase 2 and the derived-bonus step phase 5 of SPEC-071's fold |
| `crates/coordination/src/progression/level_view.rs` | `deck-streak-coordination` | added: the level view, the law tiers view and the level-up occasion |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the modules above |
| `crates/coordination/tests/xp_steps.rs` | `deck-streak-coordination` | added: A5, A6, A15, A17, A18 |
| `crates/api/src/xp_routes.rs` | `deck-streak-api` | added: `GET /api/level`, `GET /api/level/law-tiers` |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the two routes |
| `crates/api/tests/xp_routes.rs` | `deck-streak-api` | added: A26 |
| `crates/bot/src/xp_commands.rs` | `deck-streak-bot` | added: the level command |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the level command in the command table and the owner's menu |
| `crates/bot/tests/xp_commands.rs` | `deck-streak-bot` | added: A27 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: progression's settle and buff ports built for the recompute |
| `web/app/src/routes/level/+page.svelte` | miniapp | added |
| `web/app/src/lib/level/LevelScreen.svelte` | miniapp | added: the level bar, today's XP by source, the flame meter and the preview |
| `web/app/src/lib/level/AscendantChip.svelte` | miniapp | added |
| `web/app/src/lib/level/level.ts` | miniapp | added: the level view's types and its fetch |
| `web/app/src/lib/level/LevelScreen.test.ts` | miniapp | added: A28, A29 |
| `web/app/src/lib/routes.ts` | miniapp | changed: the level route joins `ROUTES` |
| `migrations/007201_progression_xp_settlement.sql` | `deck-streak-progression` | added |
| `migrations/007202_progression_buffs.sql` | `deck-streak-progression` | added |
| `.sqlx/query-00d4d3c52310751443a6ab29ef28518da77c16dbef37e14934442048cc6c21aa.json` | workspace | added: the offline cache of one checked query |
| `.sqlx/query-1611fe3acdceb17a40b192927fa53a083b6464a813ee34305fd6ccebc57cbc01.json` | workspace | added: the offline cache of one checked query |
| `.sqlx/query-3628b11cd4e2f5581759c0dbbd592c8b9fb7fa1b7a88cb05ee25be6ed0687026.json` | workspace | added: the offline cache of one checked query |
| `.sqlx/query-39292738c76822e580b13eb87b9ecc794b4e535d0d3f6c27b39c2badc4db148f.json` | workspace | added: the offline cache of one checked query |
| `.sqlx/query-50c38614b29bd47af1ee4b79021cedac6ff8012372a4a9765e93c79ae966ce74.json` | workspace | added: the offline cache of one checked query |
| `.sqlx/query-91dfa3f1302b2d4a95112ed0bf0310567da90218023b76768b75e263d158d67c.json` | workspace | added: the offline cache of one checked query |
| `.sqlx/query-9f7c546d1b4c1207ec959ad5ded0a67e6b10872ce7db8034c02e5342e7d280ff.json` | workspace | added: the offline cache of one checked query |
| `.sqlx/query-bea1916ddb9d8e6a3d27a1bf1ff6141c03ca010922b5633aa4812ffc9753aefe.json` | workspace | added: the offline cache of one checked query |
| `.sqlx/query-cc7e219feb4c5e11d69d8c49fe344a8106dc0ee0ae40c35ce1e57a8d4fae1abf.json` | workspace | added: the offline cache of one checked query |
| `.sqlx/query-d289416141deff3a3fa87a7abde501210dbbbd102fe2ca475f7dc74c1721a4e5.json` | workspace | added: the offline cache of one checked query |
| `.sqlx/query-d47109620f5ee4ef2fce871cd5a188ce4778dbe9abbfc07451c2883500c8586e.json` | workspace | added: the offline cache of one checked query |
| `.sqlx/query-d9d28ec4fd6ac6430f3136222f557b346797d9725a4b71e70d7db53608a5e9e3.json` | workspace | added: the offline cache of one checked query |
| `.sqlx/query-f911abf74980386bea70d5e4a4ec1ba8c02f50630bb3e6fc1d1b64a463ed5475.json` | workspace | added: the offline cache of one checked query |
| `Cargo.lock` | workspace | changed: the daemon's test dependencies (A31) |
| `crates/coordination/src/level_up.rs` | `deck-streak-coordination` | changed: the level-up occasion for the level a recompute reaches (R14) |
| `crates/coordination/src/sync_cycle.rs` | `deck-streak-coordination` | changed: the cycle reads the level before and after its recompute and announces the one reached (R14) |
| `crates/coordination/src/progression/mod.rs` | `deck-streak-coordination` | changed: the law-tiers module |
| `crates/coordination/src/progression/law_tiers.rs` | `deck-streak-coordination` | added: the law-tiers source over the collection copy (R24) |
| `crates/coordination/tests/level_up_cycle.rs` | `deck-streak-coordination` | added: A30 |
| `crates/coordination/tests/law_tiers.rs` | `deck-streak-coordination` | added: the law-tiers source over a seeded copy (R24, killers of three rows) |
| `crates/coordination/tests/settle_fold.rs` | `deck-streak-coordination` | changed: the fold's fixture carries the new steps |
| `crates/daemon/Cargo.toml` | `deck-streak-daemon` | changed: the test dependencies A31 uses |
| `crates/daemon/src/role_api.rs` | `deck-streak-daemon` | changed: the api role gives its router the law-tiers source (R24) |
| `crates/daemon/tests/law_tiers.rs` | `deck-streak-daemon` | added: A31 |
| `crates/analytics/tests/rollup_metrics.rs` | `deck-streak-analytics` | changed: the fixture card carries the tier field |
| `crates/readings/tests/support/mod.rs` | `deck-streak-readings` | changed: the fixture card carries the tier field |
| `crates/ingest/tests/support/synthetic.rs` | `deck-streak-ingest` | changed: a helper that tags a note in a copy |
| `crates/api/src/lib.rs` | `deck-streak-api` | changed: the `xp_routes` module is declared |
| `crates/bot/src/lib.rs` | `deck-streak-bot` | changed: the `xp_commands` module is declared |
| `crates/bot/tests/commands.rs` | `deck-streak-bot` | changed: the menu names `level` |
| `crates/bot/tests/messages/help.msg.json` | `deck-streak-bot` | changed: the help message lists the level command |
| `crates/bot/tests/messages/start.msg.json` | `deck-streak-bot` | changed: the start message lists the level command |
| `crates/notifications/tests/one_router.rs` | `deck-streak-notifications` | changed: the command replies the router census names |
| `crates/progression/tests/rights.rs` | `deck-streak-progression` | changed: the rights of the two new tables |
| `crates/progression/tests/xp_reads.rs` | `deck-streak-progression` | added: the reads answer what the tables hold (R6, R8, R17, R20 to R22) |
| `web/app/messages/en.json` | `web` | changed: the level screen's messages |
| `web/app/src/lib/api.ts` | `web` | changed: the level read (R23) |
| `web/app/src/lib/api.test.ts` | `web` | changed: the level read's test |
| `web/app/src/lib/level/level.test.ts` | `web` | changed: the level view's test |
| `web/app/src/lib/startapp.ts` | `web` | changed: the start menu names the level command |
| `web/app/src/lib/startapp.test.ts` | `web` | changed: the start menu's test |
| `web/app/src/routes/level.test.ts` | `web` | added: the level screen's test |
| `web/app/tests/a11y.spec.ts` | `web` | changed: the level screen's accessibility run (R26) |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `xp_settlement` and `buffs` |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | checked: progression's port is registered by SPEC-040; changed only if it is not |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded rows for `xp_settlement` and `buffs` |
| `privacy.json` | repo | changed: the categories `settled-xp` and `day-buffs` |
| `PRIVACY.md` | repo | changed: one line for each of the two categories |
| `tools/parity-oracle/registry/spec_072.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/review_xp.json` | repo | added: the golden of `gamification/xp.py:review_xp` (adapter: a review and a tier map, every combination) |
| `tools/parity-oracle/goldens/daily_bonus_grants.json` | repo | added: the golden of `gamification/xp.py:daily_bonus_grants` (adapter: grants as source and amount) |
| `tools/parity-oracle/goldens/level_info.json` | repo | added: the golden of `gamification/xp.py:level_info` (adapter: its fields) |
| `tools/parity-oracle/goldens/level_title.json` | repo | added: the golden of `gamification/xp.py:level_title` (function) |
| `tools/parity-oracle/goldens/parse_tier.json` | repo | added: the golden of `anki_reader.py:_parse_tier` (function) |
| `tools/parity-oracle/goldens/consistency_multiplier.json` | repo | added: the golden of `gamification/adaptive.py:consistency_multiplier` (function) |
| `tools/parity-oracle/goldens/tier_down_run.json` | repo | added: the golden of `gamification/governor.py:tier_down_run` (function) |
| `tools/parity-oracle/goldens/projected_multiplier_drop.json` | repo | added: the golden of `gamification/governor.py:projected_multiplier_drop` (function) |
| `tools/parity-oracle/goldens/on_pace_run.json` | repo | added: the golden of `pipeline_layers/governor.py:GovernorLayer._on_pace_run` (adapter: a stub store of rollups and skip days) |
| `tools/parity-oracle/goldens/day_bonuses.json` | repo | added: the golden of `pipeline_layers/governor.py:GovernorLayer._apply_day_bonuses` (adapter: a stub store and a patched run) |
| `tools/parity-oracle/goldens/ascendant_arms.json` | repo | added: the golden of `pipeline_layers/governor.py:GovernorLayer._maybe_grant_ascendant` (adapter: a stub store) |
| `tools/parity-oracle/goldens/day_base_xp.json` | repo | added: the golden of `database.py:GamifyStore.day_base_xp` (adapter: a temporary store database seeded from the case) |
| `tools/parity-oracle/goldens/progression.constants.json` | repo | added: the golden of the predecessor's XP, level and bonus constants (constants) |
| `scripts/mutation-rows.d/S07200-S07299.json` | repo | added: the rows of section 9 |
| `docs/specs/SPEC-072-xp-is-earned-per-review-and-per-day-settled-once-per-source-with-levels-and-titles.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-072-derived-xp-is-settled-into-its-own-table-and-a-closed-day-never-falls.md` | docs | changed: accepted |
| `docs/red-first/SPEC-072.md` | docs | added |
| `changelog.d/feat-xp-072.md` | repo | added: the changelog fragment |

## 5. What this does NOT do

- It sends no evening stakes card: it serves the one-miss preview, and the evening nudges show it
  (#117).
- It writes no morning-brief Ascendant line (#122) and no widget footer (#121).
- It gives a level-up the router's line only; the milestone pings' wording, and the queue-zero ping,
  are theirs (#128).
- It animates no level-up in the Mini App: a level-up the recompute raises is a background
  celebration, which the router sends to the bot (ADR-041), and its wording is the milestone pings'
  (#128).
- It rolls no chest: the Ascendant day's Epic points are the chest roll's (#102).
- It re-prices no XP source: the predecessor's re-pricing is inert (#267).
- It builds no law tab: the law block and its screen are Road to C2's (#134); this SPEC serves the tier
  numbers the tab shows.
- It serves no agent read tool for the level (#157).
- It imports none of the predecessor's XP rows (#61).

## 6. Risks

- **A product lands on the wrong side of a tie.** Rust's `f64::round` rounds half away from zero,
  and a reordered product changes the last bit. Detected by A1's `tie` cases and row S07201.
- **An event source is settled, or a derived source granted.** A settled quest pay could later be
  lowered, and a granted review XP could never follow its reviews. Detected by the registry's refusal
  (A11), the census (A12) and the recompute replay (A18).
- **DeckStreak's total exceeds the predecessor's during side by side.** A closed day keeps the
  `backlog_zero` and `score90` bonuses the predecessor removes at the rollover. Accepted by ADR-072;
  the side-by-side verification compares XP by source and expects it (#62).
- **`economy.json`'s `day_base_excludes` reads as the whole rule.** It names three of the sources the
  predecessor leaves out, so a reader who edits it expecting to change the day base changes nothing.
  Detected by A20 and row S07207, which hold the code to the golden.
- **A new XP source joins the day base by default.** The predecessor's rule leaves sources out by
  name, so any new source counts, as the readings' grants would. Detected by A22 and row S07208; each
  later SPEC that adds a source states whether the base counts it.
- **A derived bonus is settled before a base source of its day.** A quest paid after the bonus would
  miss it. Detected by the replay (A18) and A21, and by the phase order SPEC-071 tests.
- **A tier tag outside the four tokens.** The card earns the base rate, as in the predecessor; the
  law tiers view counts it as untagged, which shows the owner.

## 7. Parity goldens

Every golden is generated on the owner's machine from the predecessor at `27ee2bc` (SPEC-029), with
synthetic inputs only, and registered in `tools/parity-oracle/registry/spec_072.py`. SPEC-040 owns
`level_for_xp`, and this SPEC does not register it again.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `review_xp` | `gamification/xp.py:review_xp` | adapter | a `Review` from the case's ease, new interval, type and card id, and a card-to-tier map; the 240 combinations as class `combination`, the half-way products as `tie`, rows of type 4 or ease 0 as `non-study` |
| `daily_bonus_grants` | `gamification/xp.py:daily_bonus_grants` | adapter | the call with the case's five inputs, each returned grant as a source and an amount |
| `level_info` | `gamification/xp.py:level_info` | adapter | the returned fields as numbers and strings, for totals at and one XP below each title's threshold |
| `level_title` | `gamification/xp.py:level_title` | function | levels 1 to 120 |
| `parse_tier` | `anki_reader.py:_parse_tier` | function | tags with no tier, each tier in each case, a tier inside another token, two tiers, and a value that is not text |
| `consistency_multiplier` | `gamification/adaptive.py:consistency_multiplier` | function | runs from below 0 to past the cap |
| `tier_down_run` | `gamification/governor.py:tier_down_run` | function | seeded sequences of scores and skip flags |
| `projected_multiplier_drop` | `gamification/governor.py:projected_multiplier_drop` | function | runs from 0 to past the cap |
| `on_pace_run` | `pipeline_layers/governor.py:GovernorLayer._on_pace_run` | adapter | a stand-in layer over a stub store holding the case's rollups (epoch days and scores, more than 90 in some cases) and skip days, called with the excluded day; it returns the run |
| `day_bonuses` | `pipeline_layers/governor.py:GovernorLayer._apply_day_bonuses` | adapter | a stub store holding the day base, the buff and the `reviews` and `reviews_law` amounts, with `_on_pace_run` patched to the case's run; it returns the `consistency` and `ascendant` grants written |
| `ascendant_arms` | `pipeline_layers/governor.py:GovernorLayer._maybe_grant_ascendant` | adapter | a stub store holding an existing buff, a skip day, the previous day's rollup reviews and `backlog_zero` amount; it returns whether a buff row was written |
| `day_base_xp` | `database.py:GamifyStore.day_base_xp` | adapter | a temporary store database it creates from the case's synthetic ledger rows, each day an epoch day turned into the store's form inside the adapter; it returns the base |
| `progression.constants` | `constants.py` and `gamification/adaptive.py` | constants | `XP_BASE`, `EASE_XP_MULT`, `TYPE_XP_MULT`, `XP_MATURE_MULT`, `XP_YOUNG_MULT`, `XP_NEUTRAL_MULT`, `MATURE_IVL_DAYS`, `TIER_XP_MULT`, the six `XP_BONUS_*` and `SCORE_90_THRESHOLD`, `LEVEL_A`, `LEVEL_B`, `LEVEL_TITLES`, `ON_PACE_SCORE`, `TIER_DOWN_STEP`, `ASCENDANT_XP_FRAC`, `ASCENDANT_XP_CAP`, `gamification.adaptive.CONSISTENCY_STEP`, `gamification.adaptive.CONSISTENCY_CAP` |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `xp_settlement` | `progression` | `migrations/007201_progression_xp_settlement.sql` | the ledger rows of the derived sources (review XP, the daily bonuses, consistency and Ascendant, and later the habit, focus and token sources), each day as its epoch day and marked closed | exported and erased |
| `buffs` | `progression` | `migrations/007202_progression_buffs.sql` | the buff rows of the Ascendant kind; the chest-lock rows belong to the tripwire (#110) | exported and erased |

## 9. Mutation rows

A target outside a crate (a migration, `economy.json`) is a cargo-killed script mutation (SPEC-039).

| row | target | what it guards | killer |
|---|---|---|---|
| `S07201-REVIEW-XP-ROUNDING` | `src/review_xp.rs` | a review's XP rounds half to even as the predecessor's round() does (SPEC-072 R1, A1) | `xp_review::review_xp_matches_the_parity_golden_for_every_combination` |
| `S07202-MATURE-BOUNDARY` | `src/review_xp.rs` | a card at the mature interval earns the mature multiplier (SPEC-072 R1, A1) | `xp_review::review_xp_matches_the_parity_golden_for_every_combination` |
| `S07203-LAW-TIER-ONLY` | `src/recompute/xp.rs` | only a law card's tier scales its XP (SPEC-072 R4, A6) | `xp_steps::an_untagged_law_card_and_a_tagged_language_card_earn_the_base_rate` |
| `S07204-CLOSED-DAY-NEVER-FALLS` | `src/settle.rs` | a closed day's settled XP is raised and never lowered by a recompute (SPEC-072 R8, A7) | `xp_settle::a_closed_days_settled_xp_is_raised_and_never_lowered_by_a_recompute` |
| `S07205-SETTLEMENT-KEY` | `(study_day, source, track);` | xp_settlement holds one row per study day, source and track, held by the table's index (SPEC-072 R6, A10) | `xp_settle::settle_keeps_one_row_per_study_day_source_and_track` |
| `S07206-DERIVED-ONLY` | `src/settle.rs` | settle accepts only a source in the derived registry (SPEC-072 R7, A11) | `xp_settle::settle_refuses_a_source_outside_the_derived_registry` |
| `S07207-DAY-BASE-CHEST` | `src/consistency.rs` | the day base leaves out the chest source (SPEC-072 R17, A20) | `xp_consistency::the_day_base_matches_the_parity_golden_over_both_tables` |
| `S07208-DAY-BASE-READING` | `src/consistency.rs` | the day base leaves out the readings' grants (SPEC-072 R17, A22) | `xp_consistency::the_day_base_leaves_out_the_readings_grants` |
| `S07209-CONSISTENCY-WINDOW` | `"window_days": 90` | the consistency run looks back the predecessor's window (SPEC-072 R15, A19) | `xp_consistency::the_consistency_run_and_multiplier_match_the_parity_goldens` |
| `S07210-STREAK-CAP` | `"streak_cap": 250,` | the streak bonus is capped at the predecessor's cap (SPEC-072 R11, A14) | `xp_bonus::daily_bonus_grants_match_the_parity_golden` |
| `S07211-DERIVED-SOURCES` | `src/settle.rs` | the derived registry is the nine sources by their whole value (SPEC-072 R7; pinned outside the acceptance fence) | `xp_settle::the_derived_registry_and_the_tables_are_pinned_whole` |
| `S07212-SETTLEMENT-TABLE` | `src/settle.rs` | the settlement table is named xp_settlement (SPEC-072 R6; pinned outside the acceptance fence) | `xp_settle::the_derived_registry_and_the_tables_are_pinned_whole` |
| `S07213-BUFFS-TABLE` | `src/data_rights.rs` | the buffs table is named buffs (SPEC-072 R20; pinned outside the acceptance fence) | `xp_settle::the_derived_registry_and_the_tables_are_pinned_whole` |
| `S07214-ASCENDANT-KIND` | `src/buffs.rs` | the Ascendant buff is named ascendant (SPEC-072 R20; pinned outside the acceptance fence) | `xp_settle::the_derived_registry_and_the_tables_are_pinned_whole` |
| `S07215-LEVEL-TITLE-ADEPT` | `src/level.rs` | level 30 holds the predecessor's title (SPEC-072 R13, A2) | `xp_constants::the_progression_constants_equal_the_predecessors_and_economy_json` |
| `S07217-DAY-BASE-2X` | `src/consistency.rs` | the day base leaves out the 2x grants (SPEC-072 R17, A20) | `xp_consistency::the_day_base_matches_the_parity_golden_over_both_tables` |
| `S07218-XP-STEP-NAME` | `src/recompute/xp.rs` | the base-XP step is registered under its whole name (SPEC-072 R19; pinned outside the acceptance fence) | `xp_steps::the_step_names_and_the_level_up_kind_are_pinned_whole` |
| `S07219-BONUS-STEP-NAME` | `src/recompute/day_bonuses.rs` | the derived-bonuses step is registered under its whole name (SPEC-072 R19; pinned outside the acceptance fence) | `xp_steps::the_step_names_and_the_level_up_kind_are_pinned_whole` |
| `S07220-LEVEL-UP-KIND` | `src/level_up.rs` | the level-up line goes out as a celebration (SPEC-072 R14; pinned outside the acceptance fence) | `xp_steps::the_step_names_and_the_level_up_kind_are_pinned_whole` |
| `S07221-BONUS-SOURCES` | `src/recompute/xp.rs` | the bonus sources settled for a day include studied, backlog_zero, streak and graduations (SPEC-072 R11; pinned outside the acceptance fence) | `xp_steps::a_day_settles_the_bonus_sources` |
| `S07222-LEVEL-PATH` | `src/xp_routes.rs` | the level view is served at /api/level (SPEC-072 R23, A26) | `xp_routes::the_level_routes_answer_only_the_owner` |
| `S07223-LAW-TIERS-PATH` | `src/xp_routes.rs` | the law-tier counts are served at /api/level/law-tiers (SPEC-072 R24, A26) | `xp_routes::the_level_routes_answer_only_the_owner` |
| `S07224-BOT-LEVEL-COMMAND` | `src/commands.rs` | the bot menu names the level command (SPEC-072 R25; SPEC-026 A6) | `commands::the_menu_is_registered_for_the_owners_chat_only` |
| `S07225-LEVEL-UP-BEFORE-AFTER` | `src/sync_cycle.rs` | a sync cycle hands the router the level before the recompute and the level after it, in that order (SPEC-072 R14, A30) | `level_up_cycle::a_sync_cycle_announces_a_level_reached_once` |
| `S07226-LAW-TIERS-LAW-TRACK` | `src/progression/law_tiers.rs` | the law tiers count law-track cards alone, and price law-track reviews alone (SPEC-072 R24; pinned outside the acceptance fence) | `law_tiers::a_collection_with_no_law_root_has_no_law_cards_and_no_law_xp` |
| `S07227-LAW-TIERS-TODAY` | `src/progression/law_tiers.rs` | the law tiers' XP is the study day's reviews alone (SPEC-072 R24; pinned outside the acceptance fence) | `law_tiers::the_law_cards_are_counted_by_tier_and_today_s_reviews_are_priced_by_their_card` |
| `S07228-LAW-TIERS-NONE-SLOT` | `src/progression/law_tiers.rs` | a card with no tier counts in the fifth slot, after T4 (SPEC-072 R24; pinned outside the acceptance fence) | `law_tiers::the_law_cards_are_counted_by_tier_and_today_s_reviews_are_priced_by_their_card` |
| `S07229-LAW-TIERS-WIRED` | `src/role_api.rs` | the api role gives its router the law tiers' source (SPEC-072 R24, A31) | `law_tiers::the_composed_router_serves_the_law_tiers_to_the_owner_alone` |

## 10. Amendment, 2026-09-29: the settle census reads progression's own re-exports

Issue 397, decided by ADR-197. A12's census (R9) refuses a call of `settle` outside coordination by
the two names a caller cannot avoid, the operation's path and the request type `SettleRequest`, in
every crate but progression. Progression's own `src` was never read for what it exports, so a
renamed `pub use` there (`pub use settle::{settle as tally, SettleRequest as TallyRequest};`)
gave an outside caller two names that the census never looks for, and the caller was not refused.

- **Measured.** A planted renamed re-export in progression's `src`, with a planted caller in
  another crate that imports only the new names, produced no refusal at the base of this
  amendment: the refused list was empty where six refusals were owed.
- **The change.** The census collects the names progression's own `src` gives `settle`,
  `SettleRequest` and the `settle` module by a renaming `pub use`, whether the `use` is grouped or
  nested, sits in a nested module, or renames a name an earlier renaming made (a chain ends at its
  original). A source outside progression that names the progression crate and one of those names
  is refused as a call of `settle`, exactly as the originals are: a crate but coordination is
  refused by file, alias and original, and a coordination file outside the recompute steps keeps
  the owner's-correction rule.
- **No false refusal.** Progression's own use of its names, a coordination caller by the same
  rules as before, a mention in a comment, a source that never names the progression crate, and a
  `pub use` that renames nothing of the settlement stay accepted. Every earlier refusal and every
  examined count on the real tree are unchanged.
- **A32.** A tree with a renamed, chained, module-level and nested-module re-export in progression
  and callers that import only the new names is refused by name, and the accepted shapes above are
  not. The test is `crates/progression/tests/xp_census.rs`, beside A12's.
- **Rows.** Three, S07230 to S07232 in this SPEC's band (section 12): the mutation job reads a
  test-only diff as `not-applicable` (SPEC-039 section 11; SPEC-057 R22), so the census's own
  fixpoint, its `pub(` close and its grouped `self` rename are pinned by hand-proved rows.
- **Files.** `crates/progression/tests/xp_census.rs` (A32), `docs/decisions/ADR-197-*.md`,
  `.github/workflows/ci.yml` (the census's cache), `crates/progression/Cargo.toml` (`sha2`, a
  dev-dependency), `docs/red-first/SPEC-072.md` and `changelog.d/settle-census-397.md`.

## 11. Acceptance criteria of the 2026-09-29 amendment

| id | criterion | decided by |
|---|---|---|
| A32 | a renamed re-export of `settle`, its request or its module in progression's `src` is followed, including a module renamed inside a group (`settle::{self as ledger}`) and a chain whose link is read after the alias that uses it, and a caller outside coordination that names only the new names is refused by file, alias and original | progression `xp_census` tests: the fence's test, and `the_census_follows_a_grouped_module_renaming_and_a_chain_read_before_its_link` |

```acceptance
A32: cargo test -p deck-streak-progression --test xp_census -- --exact the_census_reads_progressions_own_reexports_as_it_reads_the_other_crates the_census_follows_a_grouped_module_renaming_and_a_chain_read_before_its_link
```

## 12. Amendment, 2026-09-30, round 6: the compiler is the census

Issue 397, decided by ADR-197 (its decision of round 6). Section 10's census, and the three rounds
of review that widened it, found a caller by reading Rust and TOML as text: it followed the names
progression's crate and its `settle` were bound to, form by form. Each review generated a new
population, and each found a form the reader did not follow. Round 5 generated one from Cargo's
documentation, TOML 1.0 and the Rust Reference (2390 cases), and round 3's rule missed 786 of its
members under each of three repairs. This amendment replaces the reader: the compiler finds every
caller, and the census reads no Rust to find one. Section 10's change and the reader's rows (S07230
to S07240, which never reached dev) are withdrawn with it; section 10's and 11's criterion, A32,
stands, and the compiler decides it.

- **The probe.** `crates/progression/build.rs` gives progression alone the cfg `settle_census` when
  the variable `SETTLE_CENSUS` is set, and under it `settle` carries `#[deprecated]` with the note
  "the settle census's probe". Without the variable, as in every ordinary build, the cfg is unset
  and the attribute is absent, so no build but the census's compiles differently.
- **The census.** It asks `cargo metadata` for the workspace, then has cargo check every target of
  every workspace package (`cargo check --workspace --all-targets`, with `--locked` when the tree
  holds a `Cargo.lock`) in a target directory of its own, with `SETTLE_CENSUS` set and `--force-warn
  deprecated` for every crate, which no `allow`, `expect`, `deny`, `forbid` or `--cap-lints` can
  silence. It runs four passes: debug assertions on and off, each with the unwind and the abort
  panic strategy, the only two settings of a profile that stable rustc offers as a cfg
  (`debug_assertions` and `panic`). Each pass names every package the workspace compiles,
  progression and every dependency among them, with its setting, so every build script and every
  macro compiles in it as the members do, and a macro expands as the crate that defines it wrote it
  for that state. When a member has a dev-dependency, the census runs the four passes again over the
  libraries and binaries alone (`cargo check --workspace`), which resolver 2 compiles without the
  features a dev-dependency asks for, as a build without tests does. Cargo's own defaults compile
  it: the census removes `RUSTC`, `RUSTC_WRAPPER`, `RUSTC_WORKSPACE_WRAPPER`, `RUSTFLAGS`,
  `CARGO_ENCODED_RUSTFLAGS` and every `CARGO_BUILD_*`, `CARGO_PROFILE_*`, `CARGO_TARGET_*` and
  `CARGO_UNSTABLE_*` variable from cargo's environment. Each `--config` setting is a `dev` one,
  which the `test` profile of tests and benches inherits. A package's named setting stands above the
  manifest's `package."*"`, `build-override` and base settings, and a `--config` setting above the
  manifest's own named one, `[profile.test]`'s among them; a manifest that names a package by
  another spec (`name@version`) gives it two settings, which cargo refuses (`the workspace does not
  compile`).
- **The callers.** Every deprecation rustc reports with the probe's note is a use of `settle`, in
  the package cargo compiled it in. Progression's own is accepted. Any other package's but
  coordination's is refused as `<file> calls settle, and only coordination's code may`. A use is
  named by the outermost file of the repository it is written in: rustc's span, then each macro call
  site it expanded from, each by its real path (every `..` and link resolved). Text an `include!`
  reads is named by the included file, as a `#[path]` module is, and not by the file that includes
  it (issue 481's "the outermost file the compiler points to").
  Coordination's use by a test, a bench or an example is accepted. Any other use by coordination is
  refused as `<file> calls settle outside the recompute steps, and only the owner's correction may`,
  unless every repository file of its expansion lies in `crates/coordination/src/recompute/` or
  names `SettleCause::OwnersCorrection` and never `SettleCause::Recompute` in its code (comments and
  literals aside). A use written in no file of the repository is named by its target's root and
  refused, a build script's output among them: rustc names an item a build script writes and
  `include!` reads by that output file alone, which lies in the census's own target directory, so
  the file that includes it cannot make it a recompute step's or a correction.
- **Refused by design.** What cargo is not asked to compile, or what rustc cannot report, the census
  refuses by name before it reads a use, rather than guess:
  - a workspace cargo cannot read, a `Cargo.lock` cargo would have to update among them (`cargo
    metadata cannot give the graph`), or one that does not compile in any pass (`the workspace does not
    compile`);
  - a package that has a build script and can name `settle`, since a build script's cfg reaches its
    own package alone and the census's passes never set a cfg from the build environment (`has a
    build script and can name settle`). The census reads the resolve graph, `cargo metadata
    --format-version 1 --locked --offline`, and refuses by name every package with a `custom-build`
    target that is, or depends on by a normal, a build or a dev edge at any depth, the package that
    defines `settle`. Progression's own script is admitted at one SHA-256, the constant
    `PROGRESSION_BUILD_SHA256`, and a script that hashes otherwise is refused (`is not the one pinned
    by PROGRESSION_BUILD_SHA256`); no other admission exists. A graph cargo cannot give (a command
    that fails, output that is not JSON, or one without the resolve graph) is refused, never
    skipped. The graph is read once, and the passes then compile the same tree, so an actor that
    changes a manifest or the lock file between the read and the passes can change what the census
    compiled but not what it read; the `--locked` of every pass makes a lock file that changed a
    refusal;
  - a member that declares a feature, since the census compiles none (`declares a feature, and the
    census compiles none`);
  - a proc-macro member, since rustc reports no deprecation inside a derive's expansion (`is a
    proc-macro crate`);
  - a `Cargo.toml` in the repository that is not a workspace member, and a path package that is not
    one (`is a package outside the workspace, which the census does not compile`);
  - a registry or git package that depends on progression (`depends on progression from outside the
    workspace`);
  - a `.cargo/config` or `.cargo/config.toml` at the repository's root, the one cargo reads there
    (`configures cargo, and the census compiles with cargo's own defaults`);
  - a use expanded through more than 1024 macro calls, and a cargo run past 1800 seconds, each a
    failure by name rather than a walk or a wait without end.
- **Out of the census's reach.** Each is disclosed by kind, as main's round-6 ruling decided, and issue 445 tracks them with pull request 425 named:
  - a wrapper function, a function pointer or a generic in progression's own code that calls
    `settle`: it is a new operation in the owner's code, which rustc reports as progression's own
    use, and a reviewer sees it there;
  - code no `cargo check --all-targets` compiles here: a doctest, a compile trybuild runs while a
    test runs, and code under a cfg no build script sets and no pass sets (another target, `doc`,
    `miri`);
  - code a proc-macro writes only when it sees the census's variable or flags;
  - a build script's cfg in a package that cannot name `settle`, read by a macro that package
    exports, where the macro expands a call to `settle` inside a package that can name it. The
    package with the script cannot be refused for it, since it has no path to `settle`, and it is
    the kind the graph refusal leaves: it holds for a git, a registry and a workspace package, and
    the admitted script of kernel is of the same kind;
  - an `include!` of a file of `crates/coordination/src/recompute/` written outside that folder, in
    one crate: rustc names the included file alone, so the census reads the call as a recompute
    step's; it is disclosed, and no textual reader of `include!` is added;
  - a registry or git package's own code, a derive from one among it; such a package is a
    dependency, and ADR-022's supply-chain rule admits it;
  - a build that compiles its packages in different debug-assertion states (a manifest's per-package
    or custom profile, `[profile.release.package.<dependency>]` among them, or the invoking
    machine's configuration), where a dependency's or another member's macro or re-export reaches
    `settle` only in a state its caller is not compiled in: the census compiles the workspace's
    packages in one state per pass, and a profile or a variable that a build script turns into a
    cfg is closed by the graph refusal above, not by a pass;
  - a build that selects some members only (`cargo build -p <member>`), whose dependencies resolve
    fewer features than the workspace's, where a dependency's macro reaches `settle` only without a
    feature another member asks for: the census compiles the workspace's own resolution;
  - the machine that runs the census: its toolchain, and the cargo configuration of its `CARGO_HOME`
    and of the folders above the repository.
- **The four-quote strings.** TOML 1.0 lets a multi-line string close with one or two quotes more
  than its delimiter. Cargo reads each manifest itself, so the census reads such a manifest exactly
  as cargo does and refuses none by design; round 5's two cases are in the killer's manifest axis,
  and each is judged right.
- **The killer.** `the_census_refuses_every_caller_the_compiler_finds` judges one generated
  population, each tree alone: round 5's 2095 valid cases (1049 members, 1046 controls: its 2390
  less the 291 cargo or rustc refused, two members the compiler reported elsewhere and their two
  controls) and 123 cases of the axes round 5 did not measure (54 members, 69 controls, 11 of them
  refused by construction): build scripts, proc-macro crates, tests, benches and examples, path and
  git dependencies, macros, cargo configuration, silencing lints, and coordination's attribution. It
  prints `examined N member(s) and M control(s) of <axis>` for each of its 17 axes, and `killer
  examined 2218 tree(s)` with no member escaping and no control judged wrongly; the 11 controls
  refused by construction (8 in a proc-macro crate, a feature, a cargo configuration and a path
  package outside the workspace) are refusals the census owes by design. At this section's base (the
  census of round 3) it fails: `members escaping: 815; controls judged wrongly: 14`.
- **The census's own trees.** `the_census_refuses_what_the_compiler_is_not_asked` plants ten trees:
  one for each refusal by design above, the two bounds aside (a `.cargo/config` and a
  `.cargo/config.toml` each, and a `Cargo.lock` cargo would have to update for a workspace it cannot
  read), and one for a use no file of the repository places. Each is refused by name, and alone.
  `the_census_fails_by_name_past_its_expansion_limit` and
  `the_census_fails_by_name_past_its_cargo_limit` hold the two bounds: a chain one call past the
  limit, and a cargo run given no time, each fail by name.
  `the_census_names_each_use_in_its_package_and_file` plants one workspace whose manifest sets debug
  assertions for tests and pins them on for one member, for progression and for a git dependency,
  with a use in each pass, selection, target kind, macro and file the rules above name: among them a
  use through progression's re-export and one through the git dependency's macro, each of which
  exists only without debug assertions, one through the git dependency's macro that exists only
  without the feature a member's dev-dependency asks for, and a correction coordination's build
  script writes and a recompute step includes, refused by coordination's root.
- **Round 4's population.** Round 4's 12307 members and 12338 controls, round 3's 144 and 144 among
  them, completed into cargo workspaces, were judged once, on 2026-09-30, outside CI, by the census
  this pull request commits at `078fc173` (the design judged the same census source): each of the
  12307 members was refused by its own caller's file and each of the 12338 controls was accepted.
  They are not in the killer: the census compiles every tree it judges, so judged each alone, as the
  killer judges its trees, they would run past the `rust` job's 60-minute timeout, for binding forms
  rustc resolves by the same name resolution the killer's identifier, export and manifest axes
  already exercise.
- **Cost.** The census compiles every member, so it costs build time where rounds 1 to 3 cost none:
  A12 runs eight passes (four over every target, four over libraries and binaries, since members
  have dev-dependencies), and the killer judges its 2218 trees, each by its own compile. Both run in
  the `rust` job beside `check.sh`'s own build and tests, and the census's target directory lies
  under the job's `target/tmp`, which the job keeps between runs under a cache key of its own (the
  toolchain pin and `Cargo.lock`), and the killer builds its stub once for each worker and lets only
  a case's own files recompile. A cold run, with no cache, judges the same trees and passes. The pull request's body carries the `rust`
  job's wall time before and after, read from CI. The `mutation-rows` job proves the rows a diff
  selects one after another within its 90 minutes; S07274 and S07275 each compile the real tree
  twice. The mutation battery (SPEC-039) runs progression's tests for each mutant of progression's
  code under a timeout of 300 s, which the killer alone exceeds, so a mutant no faster test kills
  would be reported as a timeout rather than as missed: the pull request's verdict passes a timeout,
  and the weekly table counts one as killed. This amendment does not take that decision; ADR-197's
  round 6 brings it to the owner.
- **Rows.** Forty-four, S07241 to S07284 in this SPEC's band: S07241 to S07252 pin each refusal by
  design and the census's return of them; S07253, S07254 and S07256 to S07271 its passes, the
  packages each names, its flags, its reading of rustc's report and its attribution; S07255 and
  S07273 its scrubbed environment; S07272 its expansion bound; S07274 and S07275 the probe
  itself (the build script's arming and the attribute on `settle`); and S07276 to S07284 the graph
  refusal (the reached package, its name, the pin, the build-script target, the transitive walk, its
  start, each edge, the pinned digest and the refusals it returns). Each is killed by a test of
  `xp_census.rs`, S07274 and S07275 by A12's own, and each was proved KILLED by its full id on a
  committed tree.
- **Files.** `crates/progression/build.rs` (new), `crates/progression/src/settle.rs` (the probe),
  `crates/progression/tests/xp_census.rs` (A12, A32, A33, A34, A35),
  `scripts/mutation-rows.d/S07200-S07299.json`, `docs/decisions/ADR-197-*.md`,
  `docs/red-first/SPEC-072.md` and `changelog.d/settle-census-397.md`.
- **Amendment: the lock's full feature graph.** The census reads the lock's full feature graph, so
  the environment that runs it must hold every crate the lock names, including those only a feature
  reaches; CI's `mutation-rows` job fetches them by the lock (`cargo fetch --locked`) before it
  proves a row.

## 13. Acceptance criteria of section 12's amendment

| id | criterion | decided by |
|---|---|---|
| A33 | a crate alias of progression's crate (`pub use deck_streak_progression as prog;` in another member) does not hide its caller: the caller that reaches `settle` through it is refused by file | progression `xp_census` tests: `the_census_follows_a_crate_alias` and the killer's identifier, export and manifest axes |
| A34 | every use of `settle` rustc reports in any target of any workspace package, in any of its passes, is a caller: refused in a package but progression and coordination, and judged by the cause rule in coordination; a tree the compiler is not asked about is refused by name; the census compiles with cargo's own defaults, and bounds its cargo runs and its expansion chains with a failure by name | progression `xp_census` tests: `the_census_refuses_every_caller_the_compiler_finds`, `the_census_refuses_what_the_compiler_is_not_asked`, `the_census_names_each_use_in_its_package_and_file`, `the_census_compiles_with_cargos_own_defaults`, `the_census_fails_by_name_past_its_expansion_limit`, `the_census_fails_by_name_past_its_cargo_limit` and `the_killer_plants_progressions_own_probe` |
| A35 | a package with a build script that is progression, or depends on it by a normal, build or dev edge of the resolve graph, is refused by name, but for progression's own build script at its pinned digest; a graph cargo cannot give is refused by name | progression `xp_census` tests: `a_build_script_in_a_package_that_depends_on_settle_is_refused_by_name`, `a_build_script_in_a_package_that_cannot_name_settle_is_accepted`, `a_one_byte_edit_of_progressions_build_script_is_refused_on_the_pin`, `a_corrupt_lock_file_is_refused_by_the_fail_closed_arm`, `a_build_script_reached_through_a_build_dependency_is_refused`, `a_build_script_reached_through_a_dev_dependency_is_refused` and `the_census_refuses_every_build_script_that_can_name_settle` |

```acceptance
A33: cargo test -p deck-streak-progression --test xp_census -- --exact the_census_follows_a_crate_alias
A34: cargo test -p deck-streak-progression --test xp_census -- --exact the_census_refuses_every_caller_the_compiler_finds the_census_refuses_what_the_compiler_is_not_asked the_census_names_each_use_in_its_package_and_file the_census_compiles_with_cargos_own_defaults the_census_fails_by_name_past_its_expansion_limit the_census_fails_by_name_past_its_cargo_limit the_killer_plants_progressions_own_probe
A35: cargo test -p deck-streak-progression --test xp_census -- --exact the_census_refuses_every_build_script_that_can_name_settle a_build_script_in_a_package_that_depends_on_settle_is_refused_by_name a_build_script_in_a_package_that_cannot_name_settle_is_accepted a_one_byte_edit_of_progressions_build_script_is_refused_on_the_pin a_corrupt_lock_file_is_refused_by_the_fail_closed_arm a_build_script_reached_through_a_build_dependency_is_refused a_build_script_reached_through_a_dev_dependency_is_refused
```

## 14. Amendment, 2026-09-30, round 8: the census judges the tree alone

Issue 397, decided by ADR-197 (its decision of round 8). This amendment is appended: section 12
stands as the record of rounds 6 and 7, and where a sentence of it no longer holds, this section
says so and says what holds instead.

- **The rule.** The census's verdict depends only on the tree it judges. Every input the verdict
  can read is part of the tree, pinned by the lock file or by the toolchain pin, made empty before
  each census compile, or refused by name. An input the census cannot place in one of those four
  is refused by name, never trusted.
- **The owner, by path.** The owner is the workspace member whose manifest is
  `crates/progression/Cargo.toml`, found by that path and never by a package's name. The census
  refuses by name a graph where no member has that manifest, where more than one has it, where the
  owner's package is not named `deck-streak-progression`, where any other package of the graph
  carries the owner's name (whatever its version or its source), and where the owner has no build
  script or more than one. A graph without its owner is never accepted. Every other lookup the
  census makes (a package's targets, the packages that reach the owner, the pinned script, the
  package a compiler message names) is keyed by the package's id in cargo's resolve graph, or is
  unique, or is refused.
- **One admission.** Section 12 calls progression's pinned script admitted and says no other
  admission exists, and it also calls kernel's script admitted. The pin is the only admission: it
  admits progression's script at `PROGRESSION_BUILD_SHA256`, and nothing else is admitted. A build
  script of a package that cannot name `settle`, kernel's among them, is not refused, because its
  cfg governs only code that cannot reach `settle`; it is not admitted by anything.
- **An empty target for each census.** Each census compiles in an empty target directory made for
  it alone and removed when it ends, so nothing an earlier compile built, fingerprints and
  build-script output included, reaches a later verdict. The killer's workers keep one workspace
  path, and each tree they judge still compiles in a target of its own. Section 12's Cost bullet,
  which keeps the census's target between CI runs under a cache key of its own and has each worker
  recompile only a case's own files, no longer holds: no cache serves the census, and every census
  compile is cold.
- **The environment, by name.** The census's cargo starts from an empty environment and inherits
  only `PATH`, `HOME`, `CARGO_HOME`, `RUSTUP_HOME` and `RUSTUP_TOOLCHAIN` (where the programs, the
  homes and the pinned toolchain are), then sets `CARGO_INCREMENTAL`, its own flags and
  `SETTLE_CENSUS`. No other variable of the machine reaches a build script, a macro or rustc in the
  census's build.
- **Cargo's configuration, wherever cargo reads it.** A `.cargo/config` or `.cargo/config.toml` in
  the tree's root or in any folder above it, and one in cargo's home, are refused by name. Section
  12 refused the root's alone and disclosed the others under "the machine that runs the census";
  that bullet now names the toolchain alone, which `rust-toolchain.toml` pins.
- **Reads beyond the tree.** Code of a workspace member that reads a file outside the tree, or a
  variable the host sets, as the compiler records it in its dependency information, is refused by
  name. Registry and git sources are read from cargo's home, and the lock file pins them: a
  registry package by its checksum, a git package by its revision.
- **Still disclosed by kind** (main's round-6 ruling; issue 445 tracks each, with pull request 425
  named):
  - progression's own code reading the census's cfg, which the pin does not cover (section 12's
    bullet on a proc-macro is one instance of this kind, not the whole of it);
  - build-time code of a package that cannot name `settle`, its build script or a proc-macro it
    provides, which cargo runs during the census's compile: what it reads beyond the tree is not
    part of the verdict's rule, and a cfg it sets is section 12's disclosed kind;
  - every other kind section 12 discloses, unchanged.
- **The window between the read and the passes.** Section 12's disclosed window stands: the census
  reads the graph once and then compiles, so an actor that changes a manifest or the lock file
  between the two can change what the census compiled but not what it read, and the `--locked` of
  every pass makes a changed lock file a refusal.
- **The property issue 481 proves.** It is stated over what the census refuses, not over uses
  alone: a graph where the owner cannot be told by path, where its script is not the pinned one, or
  where a package other than the owner that has a build script, or that comes from a registry or
  git, reaches the owner by a normal, build or dev edge at any depth, is refused whether or not any
  code uses `settle`; on any other graph that section 12 does not refuse by design, the census
  refuses exactly the uses of `settle` in code outside progression that the cause rule does not
  allow coordination.
- **The killers.** Round 7's review population (46 trees) is judged in the census's own tests, each
  tree on an empty target, the owner found by path; the same trees are judged again, in a seeded
  order, each on a target another tree of the population used, and each verdict must equal the
  fresh one. Two trees that differ only in which crate a member calls are judged in both orders on
  one target, a target copied from another tree's census is judged against a fresh one, and a
  variable set and unset around the census must not move its verdict. The killer's population and
  round 7's are each pinned by their count and a digest of their trees, together.
- **Cost.** Every census compile is cold, so the `rust` job pays each compile in full. The pull
  request's body carries the `rust` job's wall time on `dev` without this change, at this change,
  and the increment, each read from CI.
- **Rows.** S07290 to S07301: the owner found by path, the decoy refusal, the renamed owner, the
  owner without a script, the ambiguous owner, the empty target, the cleared environment, the
  configuration above the tree, the configuration in cargo's home, a member reading a variable
  the host sets, a member reading a file outside the tree, and a registry or git package that
  reaches the owner. Each is killed by a test of `xp_census.rs` and proved KILLED by its full id
  on a committed tree.
- **Files.** `crates/progression/tests/xp_census.rs`, `.github/workflows/ci.yml` (the census's
  cache removed), `docs/schematics/ci-jobs-and-caches.md`,
  `scripts/mutation-rows.d/S07200-S07299.json`, `scripts/mutation-rows.d/S07300-S07399.json`,
  `docs/decisions/ADR-197-*.md`,
  `docs/red-first/SPEC-072.md` and `changelog.d/settle-census-397.md`.

## 15. Acceptance criteria of section 14's amendment

| id | criterion | decided by |
|---|---|---|
| A36 | the census's verdict depends only on the tree it judges: the owner is the member at progression's manifest path, and a graph where it cannot be told is refused by name; every lookup is keyed by id, unique or refused; each census compiles in an empty target, with an environment it names; a cargo configuration above the tree or in cargo's home, and a member's read beyond the tree, are refused by name; and a tree's verdict on a target another tree used equals its verdict on a fresh one | progression `xp_census` tests: `verify_round_seven_population_is_judged_as_each_case_expects_on_any_target`, `the_order_pair_p3_is_judged_alike_in_both_orders_on_one_target`, `a_target_copied_from_another_trees_census_does_not_move_the_verdict`, `a_git_package_carrying_the_owners_name_beside_a_members_script_is_refused_by_name`, `a_git_package_carrying_the_owners_name_beside_an_edited_pin_is_refused_by_name`, `a_git_package_carrying_the_owners_name_beside_a_disarmed_owner_is_refused_by_name`, `the_owners_manifest_under_another_package_name_is_refused_by_name`, `an_owner_without_a_build_script_is_refused_by_name`, `the_owner_is_the_member_at_its_manifest_and_every_other_lookup_is_unique_or_refused`, `a_variable_the_tree_does_not_set_does_not_move_the_verdict`, `a_cargo_configuration_above_the_tree_is_refused_by_name`, `a_cargo_configuration_in_cargos_home_is_refused_by_name`, `a_members_code_reading_a_variable_the_host_sets_is_refused_by_name`, `a_git_url_reused_with_other_content_is_judged_as_a_fresh_url` and `main_round_seven_generation_is_refused_by_name` |

```acceptance
A36: cargo test -p deck-streak-progression --test xp_census -- --exact verify_round_seven_population_is_judged_as_each_case_expects_on_any_target the_order_pair_p3_is_judged_alike_in_both_orders_on_one_target a_target_copied_from_another_trees_census_does_not_move_the_verdict a_git_package_carrying_the_owners_name_beside_a_members_script_is_refused_by_name a_git_package_carrying_the_owners_name_beside_an_edited_pin_is_refused_by_name a_git_package_carrying_the_owners_name_beside_a_disarmed_owner_is_refused_by_name the_owners_manifest_under_another_package_name_is_refused_by_name an_owner_without_a_build_script_is_refused_by_name the_owner_is_the_member_at_its_manifest_and_every_other_lookup_is_unique_or_refused a_variable_the_tree_does_not_set_does_not_move_the_verdict a_cargo_configuration_above_the_tree_is_refused_by_name a_cargo_configuration_in_cargos_home_is_refused_by_name a_members_code_reading_a_variable_the_host_sets_is_refused_by_name a_git_url_reused_with_other_content_is_judged_as_a_fresh_url main_round_seven_generation_is_refused_by_name
```

## 16. Amendment, 2026-10-03: the owner's own operation, and the census reads joined names

SPEC-324 amends the settle census twice, and ADR-197 round 9 and ADR-325 decide it.

- **The owner's own operation is refused.** Section 12's first out-of-reach bullet, a wrapper
  function, a function pointer or a generic in progression's own code that calls `settle`, is no
  longer disclosed: a use that rustc reports in progression's own package, by a target that is not
  a test, a bench or an example, is refused by name unless it is written inside a `use` declaration
  or in a file of `OWNER_ADMITS`, which admits none. An import and a re-export stay accepted, and
  round 6 follows them to their callers as before.
- **The table's name is read joined.** Beside its own reader, the census adds the shared reader of
  `tools/table-census/table_census.rs`: every crate's literals outside progression, decoded as rustc
  decodes them, pooled workspace-wide and refused where they can assemble `xp_settlement` in any
  case; an unread `concat!` argument beside part of the name and an include it cannot name or read
  are refused by name.
- **Still disclosed by kind**, now tracked by issue 586 in place of issue 445: progression's own code
  reading the census's cfg; a doctest and a compile test; code a procedural macro writes; another
  package's build script; an `include!` of a recompute file; a registry or git package's own code;
  and builds in mixed debug-assertion states. Each stays as sections 12 and 14 state it.

The criteria are SPEC-324's: A2 (the joined name), A6 (the owner's own operation, and an admitted
file accepted) and A7 (a re-export progression's macro writes), each decided by its fenced command
there.
