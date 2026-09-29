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
| `crates/progression/src/ascendant.rs` | `deck-streak-progression` | added: the arming, the bonus, the repository over `buffs` |
| `crates/progression/src/economy_config.rs` | `deck-streak-progression` | added: `economy.json`'s `xp` section, embedded at build time |
| `crates/progression/src/ledger.rs` | `deck-streak-progression` | changed: the total sums both XP tables |
| `crates/progression/src/data_rights.rs` | `deck-streak-progression` | changed: `xp_settlement` and `buffs`, exported and erased |
| `crates/progression/src/lib.rs` | `deck-streak-progression` | changed: the modules above |
| `crates/progression/Cargo.toml` | `deck-streak-progression` | changed: the workspace dependencies the modules use |
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
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `Cargo.lock` | workspace | changed |
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
| `changelog.d/` fragment | repo | added |

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
| `S07201-ROUND-HALF-TO-EVEN` | `crates/progression/src/review_xp.rs` | rounding half to even, not half away from zero | `xp_review::review_xp_matches_the_parity_golden_for_every_combination` |
| `S07202-MATURITY-FROM-THE-NEW-INTERVAL` | `crates/progression/src/review_xp.rs` | a review is mature at 21 days or more of its new interval | `xp_review::review_xp_matches_the_parity_golden_for_every_combination` |
| `S07203-THE-TIER-ON-LAW-ONLY` | `crates/coordination/src/recompute/xp.rs` | the tier map holds law-track cards only | `xp_steps::an_untagged_law_card_and_a_tagged_language_card_earn_the_base_rate` |
| `S07204-A-CLOSED-DAY-ONLY-RISES` | `crates/progression/src/settle.rs` | a recompute keeps the larger of the stored and the recomputed amount of a closed day | `xp_settle::a_closed_days_settled_xp_is_raised_and_never_lowered_by_a_recompute` |
| `S07205-THE-SETTLEMENT-KEY` | `migrations/007201_progression_xp_settlement.sql` | the unique key is study day, source and track | `xp_settle::settle_keeps_one_row_per_study_day_source_and_track` |
| `S07206-THE-REGISTRY-REFUSES` | `crates/progression/src/settle.rs` | a source outside the derived registry is refused at construction | `xp_settle::settle_refuses_a_source_outside_the_derived_registry` |
| `S07207-THE-DAY-BASE-EXCLUSIONS` | `crates/progression/src/consistency.rs` | the day base leaves chest XP out, as the predecessor does | `xp_consistency::the_day_base_matches_the_parity_golden_over_both_tables` |
| `S07208-THE-READINGS-LEAVE-THE-BASE` | `crates/progression/src/consistency.rs` | the day base leaves the readings' grants out | `xp_consistency::the_day_base_leaves_out_the_readings_grants` |
| `S07209-THE-ON-PACE-WINDOW` | `economy.json` | the run folds the 90 most recent rollups | `xp_consistency::the_consistency_run_and_multiplier_match_the_parity_goldens` |
| `S07210-THE-STREAK-BONUS-CAP` | `economy.json` | the streak bonus stops at 250 | `xp_bonus::daily_bonus_grants_match_the_parity_golden` |
