# SPEC-073: badges, records and the next milestone are awarded once and shown with their progress

- **Wave:** W3. **Issue:** #74, #75, #76 (epic #4). **Context(s):** `deck-streak-progression` (the
  catalog of 40 badges, the award port, the study-badge conditions, band-badge keys, records, the
  milestone ladder); `deck-streak-coordination` (the badge context of each evaluated day, the records
  step, the records and milestone views, the celebrations); `deck-streak-api`, `deck-streak-bot` and
  the Mini App (`web/app`) (the badge, record and milestone views).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-041 (the one router and its
  surface rule), ADR-071 (a closing day is evaluated once, with its end-of-day state), and ADR-087
  (the configured courses name the band badges and the course-named descriptions).
- **Prerequisites:** SPEC-071 (the rollups, the card snapshot and the settle fold), SPEC-072 (progression's
  modules these sit beside, and the XP phases the awards follow), SPEC-076 (the language streak and its
  badge view), SPEC-041 (the router), SPEC-026 (the bot's command table). **Mutation band:**
  `S07300-S07399`.
- **Status:** delivered by build-073 (moved from `docs/specs/planned/` with its tests and
  `docs/red-first/SPEC-073.md`, ADR-016). Decided by ADR-303 (the celebration mark on the award row) and
  proved by `formal/tla/AwardOnce/` and `formal/lean/Formal/NextMilestone.lean`.
  The mutation rows of section 9 run from S07303: S07300 and S07301 belong to another delivery.

## 1. The problem, measured

- **What exists.** At `dev` c3d769b, `crates/progression/src/` holds only `lib.rs`. No badge, record or
  milestone exists, and nothing can award one.
- **What is ported** (predecessor `27ee2bc`, names only):
  - #74: the catalog `constants.py:BADGES` (40 badges: 24 study, 8 habit and 8 focus); the study
    conditions `gamification/badges.py:_conditions` and `gamification/badges.py:evaluate`; the context
    they read, built in `pipeline.py:GamifyPipeline._evaluate_and_award` (the week's and the 30 days'
    first-answer retention, the hour windows through `analytics.py:count_reviews_in_local_hours`,
    thirty study days in a row, the backlog cleared); the award `database.py:GamifyStore.award_badge`
    (one row per key and tier); the celebration in `pipeline.py:GamifyPipeline._notify_milestones`
    (event `badge`, key `badge:<key>:<tier>`); the band badges `band_<code>_<band>` of
    `pipeline.py:GamifyPipeline._celebrate_band_up`; and `/badges`
    (`bot.py:CommandBot.handle_command`, `badges`);
  - #75: `gamification/rewards.py:detect_records`, run over the most recent rollups in
    `pipeline.py:GamifyPipeline._compute_and_store_coaching`, which stores each new record with the
    value it beat (`database.py:GamifyStore.upsert_record`), seeds the first records silently, and
    celebrates (event `record`, key `pr:<kind>:<day>`); the board
    `pipeline_layers/celebrations.py:CelebrationsLayer.records_board` and its line naming the record
    to chase (`bot.py:CommandBot._render_records`);
  - #76: `gamification/rewards.py:next_milestone` over the lifetime reviews, the language streak and
    Road to C2's mature cards (`coaching.py:compute_all`).
- **Split by context.** The habit badges' conditions (`habits.py:evaluate_habit_badges`) and the focus
  badges' (`focus.py:evaluate_focus_badges`) are those contexts' rules: SPEC-078 and SPEC-079 evaluate
  them and award through this SPEC's port. The band badges are awarded by the band-up (SPEC-077).
  Progression owns the catalog, the table and the award, as the context map's register says.
- **One router for every badge.** The predecessor pings its habit and focus badges through its
  notifier directly, outside its ladder (`pipeline_layers/habits.py:HabitsLayer._award_habit_badges`).
  In DeckStreak every newly awarded badge is one celebration through the router (CHARTER 2).
- **Traps a hand port falls into.**
  - A closing day is judged at its settle with its end-of-day snapshot and its live score (ADR-071),
    not the historical score stored once it has closed; `legendary_day` and `perfect_week` read the
    score.
  - `comeback_kid` reads the streak's badge view (`gamification/streak.py:badge_view`, SPEC-076):
    armed only while the streak is alive.
  - A first detection of records finds every old best at once and must not celebrate them.
  - Five catalog descriptions count or name the predecessor's own courses (the three writing-streak
    badges, the weekly reading goal and the reading-in-every-course badge). With the courses as
    configuration (ADR-087), those five are rendered from the configured courses.
- **Corrections to the issues, read in the predecessor's code.**
  - #75: records are detected over the 370 most recent rollup rows, not 370 calendar days; the golden
    of the records step proves the limit.
  - #76: the mature ladder's input is Road to C2's mature cards summed over the courses (cards whose
    mastery is at least one half, `progress.py:compute_progress`), not the card snapshot's mature
    count. It arrives with Road to C2 (#85), so the milestone waits for it.
  - #74: `legendary_day` is a literal score of 100 in the predecessor's condition, and `leech_tamer`
    reads `gamification/badges.py:LEECH_TAMER_MIN_LIFETIME`; the conditions golden proves both.
- **What the parity oracle proves.** The catalog; every study condition over synthetic contexts; the
  context built from synthetic reviews, rollups and a snapshot; the hour windows; the thresholds; the
  records, their window, their silent seed and their celebration keys; the record to chase; the
  milestone and its ladders.
- **Prerequisites.** SPEC-071, SPEC-072, SPEC-076, SPEC-041 and SPEC-026, above. SPEC-077 supplies the
  milestone's mature-card sum; SPEC-078 and SPEC-079 award through the port this SPEC builds.

## 2. Requirements

The catalog and the award port (#74)

R1. Progression owns `badges_earned`, created by `migrations/007301_progression_badges_earned.sql`
    (`STRICT`, `created_at`, SPEC-020 R15 and R18): `badge_key`, `tier`, `name`, `emoji`,
    `study_day`, `celebrated_at` (nullable: set from the services clock after the router answers, and
    at insert for a band badge, which the band-up celebrates), primary key (`badge_key`, `tier`).
R2. The catalog holds the predecessor's 40 badges with their key, name, emoji and description, equal
    to `goldens/badge_catalog.json` (`constants.py:BADGES`), each at tier 0. The descriptions of
    `ink_week`, `ink_month`, `ink_century`, `bookworm_week` and `polyglot_reader` are rendered from
    the configured courses (ADR-087), and their golden cases are compared on key, name and emoji.
R3. The award port writes one row per key and tier in one `BEGIN IMMEDIATE` write and answers
    `Awarded` or `AlreadyAwarded`; it never updates or deletes a badge. It accepts a catalog key, or
    a band key `band_<code>_<band>` whose code is a configured course and whose band is one of A1 to
    C2; any other key is refused before a write, by an error that names the rule and never the value.
R4. A newly awarded catalog badge, study, habit or focus, is written with its `celebrated_at` unset, in
    the award's own write, and after that write commits it raises one celebration through the router
    (SPEC-041) with the event `badge` and the dedupe key `badge:<key>:<tier>`. `AlreadyAwarded` raises no
    new celebration: an award whose celebration is not yet marked is offered again at each evaluation
    until the router answers, and the router's once-ever dedupe key keeps each key to at most one send.
    A band badge raises none of its own: the band-up celebrates it (SPEC-077), so it is written marked.

The study badges (#74)

R5. For each day the recompute evaluates (a closing day at its settle, and the current day),
    coordination builds the badge context equal to `goldens/badge_context.json`
    (`pipeline.py:GamifyPipeline._evaluate_and_award`): the lifetime study reviews; the day's metrics
    and its card snapshot; the language streak's badge view (SPEC-076); the day's score (the live
    score for the current day, the rollup's `score_at_close` for a closing day, SPEC-071); the scores
    of the 7 most recent rollups on or before the day, oldest first, the day's own being that score;
    the reviews, the decks and the first-answer retention of the 7 days ending on the day; the
    mature first-answer retention and answers of the 30 days ending on it; the day's reviews between
    midnight and the rollover hour and between the rollover hour and the early-bird hour; the reviews
    cleared with no backlog, counted only at the backlog-slayer threshold; and whether each of the 30
    days ending on it is a study day.
R6. The study conditions equal `goldens/badge_conditions.json` (`gamification/badges.py:_conditions`),
    with every threshold from `goldens/badges.constants.json`: `constants.LIFETIME_REVIEWS_GRINDER`,
    `constants.LIFETIME_REVIEWS_MARATHONER`, `constants.CENTURION_DAY_REVIEWS`,
    `constants.SHARPSHOOTER_RETENTION`, `constants.SHARPSHOOTER_MIN_REVIEWS`,
    `constants.SNIPER_ELITE_RETENTION`, `constants.SNIPER_ELITE_MIN_MATURE`,
    `constants.BACKLOG_SLAYER_CLEARED`, `constants.NIGHT_OWL_REVIEWS`, `constants.EARLY_BIRD_REVIEWS`,
    `constants.EARLY_BIRD_END_HOUR`, `constants.MATURITY_MILESTONE_COUNT`,
    `constants.FOREST_GUARDIAN_COUNT`, `constants.POLYGLOT_DECKS_DAY`,
    `constants.GLOBETROTTER_DECKS_WEEK`, `constants.PERFECT_WEEK_SCORE`, `constants.SPEED_DEMON_REVIEWS`,
    `constants.SPEED_DEMON_AVG_SECONDS`, `constants.IRON_WILL_DAYS` and
    `gamification.badges.LEECH_TAMER_MIN_LIFETIME`.
R7. The hour windows equal `goldens/count_reviews_in_local_hours.json`
    (`analytics.py:count_reviews_in_local_hours`): the study reviews of the day whose local clock hour,
    at the configured offset, lies from the window's start hour up to, not including, its end hour.
R8. Coordination registers the badge step in phase 7 of SPEC-071's fold (the awards): each met study
    condition is awarded through R3's port and celebrated by R4's rule, on the day evaluated.

Records (#75)

R9. Progression owns `records`, created by `migrations/007302_progression_records.sql` (`STRICT`,
    `created_at`): `kind` (primary key: `best_score`, `most_reviews` or `most_minutes`), `value`,
    `study_day`, `previous`, `celebrated_at` (nullable, as R1's).
R10. Coordination registers the records step in phase 7 of SPEC-071's fold. For each evaluated day it
    equals `goldens/records_window.json` (`pipeline.py:GamifyPipeline._compute_and_store_coaching`):
    it runs `goldens/detect_records.json` (`gamification/rewards.py:detect_records`) over the 370 most
    recent rollups on or before the day, against the stored bests; the day itself counts with its
    score as R5 reads it, and every earlier day with its stored score. `best_score` is the highest score,
    `most_reviews` the most reviews and `most_minutes` the whole minutes of the most seconds, and only a
    value strictly above the stored best is a record.
R11. A new record is stored with the value it beat and its `celebrated_at` unset, and after the write
    commits it is celebrated through the router, with the event `record` and the dedupe key
    `pr:<kind>:<epoch day>` of the day that set it. A replay of the same day writes the same row and
    raises no new celebration: a record whose celebration is not yet marked is offered again at each
    evaluation until the router answers, and the router's once-ever dedupe key keeps each key to at most
    one send. A record row holds one kind, so a write that would replace the row of an earlier day whose
    mark is unset first offers that mark, and a record that is superseded before it could be offered is
    named in a log line with its kind and day.
R12. The first detection, with no record stored, stores each kind's best with `previous` equal to its
    value and celebrates none. The v9 import brings the predecessor's records first when it runs (#61).
R13. The records view carries each record's label, value, the day it was set, the value it beat and
    today's live value for its kind (the current day's live score, reviews and whole minutes),
    and names the record today is closest to: the smallest positive gap, the first on a tie, equal to
    `goldens/records_chase.json` (`bot.py:CommandBot._render_records`).

The next milestone (#76)

R14. The milestone equals `goldens/next_milestone.json` (`gamification/rewards.py:next_milestone`):
    across the review, streak and mature-card ladders (`goldens/rewards.constants.json`:
    `gamification.rewards.REVIEW_LADDER`, `gamification.rewards.STREAK_LADDER`,
    `gamification.rewards.MATURE_LADDER`), the nearest unreached rung by the remaining fraction of that
    rung, ties to reviews, then the streak, then mature cards. A ladder whose every rung is reached
    contributes nothing, and with all three complete the milestone is the top review rung at 100%.
R15. Its inputs are the lifetime study reviews (ingest's window base and the window's reviews,
    SPEC-023), the language streak's current length (SPEC-076) and Road to C2's mature cards summed over
    the configured courses (#85). Until Road to C2 supplies that sum, the milestone view answers
    `pending` and computes nothing from a stand-in.

The surfaces

R16. `GET /api/badges` answers the owner's session only (SPEC-024) with the earned badges (key, name,
    emoji and the day) and the locked catalog badges with their criteria; a locked study badge carries
    its input's value against its threshold, and a habit or focus badge carries its progress once its
    context supplies it.
R17. `GET /api/records` answers the owner only with R13's view, and `GET /api/milestone` with R14's
    milestone or `pending`.
R18. The bot's `/badges` lists the 20 most recently awarded badges, each its emoji and name, or the
    predecessor's line when none is earned; `/records` lists each record with its value, the day it was
    set and the value it beat, then the line naming the record to chase.
R19. The Mini App's `/badges` is a gallery of earned and locked badges, each locked one with its
    criteria and progress; its `/records` shows each record with a bar of today's distance to it. Both
    routes join the route table. The next-milestone card on the home screen is SPEC-086's, composed
    from `GET /api/milestone`.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the catalog's 40 badges equal the golden (examined count reported, zero refused) | `the_catalog_matches_the_parity_golden` |
| A2 | the five course-named descriptions are rendered from the configured courses | `the_course_named_descriptions_come_from_the_configured_courses` |
| A3 | awarding one badge twice writes one row, and the second answer is `AlreadyAwarded` | `awarding_a_badge_twice_writes_one_row` |
| A4 | a band key is accepted only for a configured course and a band from A1 to C2 | `a_band_badge_key_names_a_configured_course_and_a_band` |
| A5 | an unknown key is refused before a write, and the error names the rule | `an_unknown_badge_key_is_refused_before_a_write` |
| A6 | every study condition equals the golden | `the_study_badge_conditions_match_the_parity_golden` |
| A7 | every badge threshold equals the predecessor's constant | `the_badge_constants_equal_the_predecessors` |
| A8 | the hour windows equal the golden | `the_hour_counts_match_the_parity_golden` |
| A9 | the badge context built from synthetic reviews, rollups and a snapshot equals the golden | `the_badge_context_matches_the_parity_golden` |
| A10 | a newly awarded badge is sent once (an evaluation stopped after the write sends exactly once at the next, and a stop after the router answered but before the mark leaves the sends at one), and a replay sends nothing new | `a_new_badge_is_celebrated_once_and_a_replay_raises_nothing` |
| A11 | a closing day is judged with its end-of-day snapshot and its `score_at_close` | `a_closing_day_is_judged_with_its_end_of_day_state` |
| A12 | the record detection equals the golden | `detect_records_matches_the_parity_golden` |
| A13 | the record to chase equals the golden | `the_chase_record_matches_the_parity_golden` |
| A14 | the records step (its window, its seed and its celebration keys) equals the golden | `the_records_step_matches_the_parity_golden` |
| A15 | the first detection stores the bests with `previous` equal to their values and sends none | `the_first_detection_seeds_records_silently` |
| A16 | a record is sent once per kind and day, including when a later day's beat meets an earlier record whose mark is unset | `a_record_is_celebrated_once_per_kind_and_day` |
| A17 | the milestone equals the golden | `next_milestone_matches_the_parity_golden` |
| A18 | a complete ladder contributes nothing, and three complete ladders report the top review rung at 100% | `a_complete_ladder_contributes_nothing_and_all_complete_reports_the_top_review_rung` |
| A19 | the milestone view answers `pending` until Road to C2 supplies the mature cards | `the_milestone_is_pending_until_road_to_c2_supplies_the_mature_cards` |
| A20 | the badge, record and milestone routes answer only the owner (401 or 403, no data) | `the_badge_record_and_milestone_routes_answer_only_the_owner` |
| A21 | `/badges` lists the 20 most recently awarded badges | `badges_lists_the_twenty_most_recent` |
| A22 | `/records` names the record to chase | `records_names_the_record_to_chase` |
| A23 | the gallery shows a locked badge with its criteria and progress | `shows a locked badge with its criteria and progress` |
| A24 | the records screen shows each record's distance from today | `shows each record's distance from today` |

```acceptance
A1: cargo test -p deck-streak-progression --test badges_catalog -- --exact the_catalog_matches_the_parity_golden
A2: cargo test -p deck-streak-progression --test badges_catalog -- --exact the_course_named_descriptions_come_from_the_configured_courses
A3: cargo test -p deck-streak-progression --test badges_award -- --exact awarding_a_badge_twice_writes_one_row
A4: cargo test -p deck-streak-progression --test badges_award -- --exact a_band_badge_key_names_a_configured_course_and_a_band
A5: cargo test -p deck-streak-progression --test badges_award -- --exact an_unknown_badge_key_is_refused_before_a_write
A6: cargo test -p deck-streak-progression --test badges_conditions -- --exact the_study_badge_conditions_match_the_parity_golden
A7: cargo test -p deck-streak-progression --test badges_conditions -- --exact the_badge_constants_equal_the_predecessors
A8: cargo test -p deck-streak-progression --test badges_conditions -- --exact the_hour_counts_match_the_parity_golden
A9: cargo test -p deck-streak-coordination --test badges_context -- --exact the_badge_context_matches_the_parity_golden
A10: cargo test -p deck-streak-coordination --test badges_steps -- --exact a_new_badge_is_celebrated_once_and_a_replay_raises_nothing
A11: cargo test -p deck-streak-coordination --test badges_steps -- --exact a_closing_day_is_judged_with_its_end_of_day_state
A12: cargo test -p deck-streak-progression --test records_detect -- --exact detect_records_matches_the_parity_golden
A13: cargo test -p deck-streak-progression --test records_detect -- --exact the_chase_record_matches_the_parity_golden
A14: cargo test -p deck-streak-coordination --test records_steps -- --exact the_records_step_matches_the_parity_golden
A15: cargo test -p deck-streak-coordination --test records_steps -- --exact the_first_detection_seeds_records_silently
A16: cargo test -p deck-streak-coordination --test records_steps -- --exact a_record_is_celebrated_once_per_kind_and_day
A17: cargo test -p deck-streak-progression --test milestone_ladder -- --exact next_milestone_matches_the_parity_golden
A18: cargo test -p deck-streak-progression --test milestone_ladder -- --exact a_complete_ladder_contributes_nothing_and_all_complete_reports_the_top_review_rung
A19: cargo test -p deck-streak-coordination --test milestone_view -- --exact the_milestone_is_pending_until_road_to_c2_supplies_the_mature_cards
A20: cargo test -p deck-streak-api --test badges_routes -- --exact the_badge_record_and_milestone_routes_answer_only_the_owner
A21: cargo test -p deck-streak-bot --test badges_commands -- --exact badges_lists_the_twenty_most_recent
A22: cargo test -p deck-streak-bot --test badges_commands -- --exact records_names_the_record_to_chase
A23: pnpm exec vitest run web/app/src/lib/badges/BadgeGallery.test.ts -t "shows a locked badge with its criteria and progress"
A24: pnpm exec vitest run web/app/src/lib/records/RecordsScreen.test.ts -t "shows each record's distance from today"
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr and accessibility packs stay enforced;
no row is deferred for this delivery.

| id | criterion | decided by |
|---|---|---|
| B1 | the privacy checks pass over `privacy.json`, `PRIVACY.md` and `crates/progression/src/data_rights.rs`, examining the categories of `badges_earned` and `records` with their export and erase | the privacy-gdpr pack |
| B2 | the accessibility checks pass over `web/app/src/routes/badges/+page.svelte`, `web/app/src/routes/records/+page.svelte`, `web/app/src/lib/badges/*.svelte` and `web/app/src/lib/records/*.svelte`, examining every element of the two screens | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/progression/src/badges/catalog.rs` | `deck-streak-progression` | added: the 40 badges and the course-named descriptions |
| `crates/progression/src/badges/award.rs` | `deck-streak-progression` | added: the award port, the key rules, the repository over `badges_earned` |
| `crates/progression/src/badges/conditions.rs` | `deck-streak-progression` | added: the badge context and the study conditions |
| `crates/progression/src/badges/hours.rs` | `deck-streak-progression` | added: the hour windows |
| `crates/progression/src/badges/mod.rs` | `deck-streak-progression` | added |
| `crates/progression/src/records.rs` | `deck-streak-progression` | added: the detection, the seed, the record to chase, the repository over `records` |
| `crates/progression/src/milestone.rs` | `deck-streak-progression` | added: the three ladders and the nearest rung |
| `crates/progression/src/data_rights.rs` | `deck-streak-progression` | changed: `badges_earned` and `records`, exported and erased |
| `crates/progression/src/lib.rs` | `deck-streak-progression` | changed: the modules above |
| `crates/progression/tests/badges_catalog.rs` | `deck-streak-progression` | added: A1, A2 |
| `crates/progression/tests/badges_award.rs` | `deck-streak-progression` | added: A3 to A5 |
| `crates/progression/tests/badges_conditions.rs` | `deck-streak-progression` | added: A6 to A8 |
| `crates/progression/tests/records_detect.rs` | `deck-streak-progression` | added: A12, A13 |
| `crates/progression/tests/milestone_ladder.rs` | `deck-streak-progression` | added: A17, A18 |
| `crates/coordination/src/progression/badge_context.rs` | `deck-streak-coordination` | added: the context of each evaluated day, from the rollups, the reviews, the snapshot and the streak |
| `crates/coordination/src/recompute/badges.rs` | `deck-streak-coordination` | added: the badge step and its celebrations |
| `crates/coordination/src/recompute/records.rs` | `deck-streak-coordination` | added: the records step, its seed and its celebrations |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: the badge and records steps join phase 7 of SPEC-071's fold |
| `crates/coordination/src/progression/records_view.rs` | `deck-streak-coordination` | added: the records view with today's live values |
| `crates/coordination/src/progression/milestone_view.rs` | `deck-streak-coordination` | added: the milestone view, `pending` until the mature-card sum exists |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the modules above |
| `crates/coordination/tests/badges_context.rs` | `deck-streak-coordination` | added: A9 |
| `crates/coordination/tests/badges_steps.rs` | `deck-streak-coordination` | added: A10, A11 |
| `crates/coordination/tests/records_steps.rs` | `deck-streak-coordination` | added: A14 to A16 |
| `crates/coordination/tests/milestone_view.rs` | `deck-streak-coordination` | added: A19 |
| `crates/api/src/badges_routes.rs` | `deck-streak-api` | added: the badge, record and milestone routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the three routes |
| `crates/api/tests/badges_routes.rs` | `deck-streak-api` | added: A20 |
| `crates/bot/src/badges_commands.rs` | `deck-streak-bot` | added: the badges and records commands |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the two commands in the command table and the owner's menu |
| `crates/bot/tests/badges_commands.rs` | `deck-streak-bot` | added: A21, A22 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: progression's award and records ports built for the recompute |
| `web/app/src/routes/badges/+page.svelte` | miniapp | added |
| `web/app/src/routes/records/+page.svelte` | miniapp | added |
| `web/app/src/lib/badges/BadgeGallery.svelte` | miniapp | added |
| `web/app/src/lib/badges/badges.ts` | miniapp | added: the badge view's types and its fetch |
| `web/app/src/lib/badges/BadgeGallery.test.ts` | miniapp | added: A23 |
| `web/app/src/lib/records/RecordsScreen.svelte` | miniapp | added |
| `web/app/src/lib/records/records.ts` | miniapp | added: the records view's types and its fetch |
| `web/app/src/lib/records/RecordsScreen.test.ts` | miniapp | added: A24 |
| `web/app/src/lib/routes.ts` | miniapp | changed: the badges and records routes join `ROUTES` |
| `migrations/007301_progression_badges_earned.sql` | `deck-streak-progression` | added |
| `migrations/007302_progression_records.sql` | `deck-streak-progression` | added |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `badges_earned` and `records` |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | checked: progression's port is registered by SPEC-040; changed only if it is not |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded rows for `badges_earned` and `records` |
| `privacy.json` | repo | changed: the categories `badges` and `personal-records` |
| `PRIVACY.md` | repo | changed: one line for each of the two categories |
| `tools/parity-oracle/registry/spec_073.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/badge_catalog.json` | repo | added: the golden of `constants.py:BADGES` (adapter: each badge's key, name, emoji, description and tier) |
| `tools/parity-oracle/goldens/badge_conditions.json` | repo | added: the golden of `gamification/badges.py:_conditions` (adapter: a synthetic badge context) |
| `tools/parity-oracle/goldens/badge_context.json` | repo | added: the golden of `pipeline.py:GamifyPipeline._evaluate_and_award` (adapter: a stub store, synthetic reviews, the context recorded) |
| `tools/parity-oracle/goldens/count_reviews_in_local_hours.json` | repo | added: the golden of `analytics.py:count_reviews_in_local_hours` (adapter: synthetic reviews and a collection configuration) |
| `tools/parity-oracle/goldens/badges.constants.json` | repo | added: the golden of the badge thresholds (constants) |
| `tools/parity-oracle/goldens/detect_records.json` | repo | added: the golden of `gamification/rewards.py:detect_records` (adapter: each record as kind, value and label) |
| `tools/parity-oracle/goldens/records_window.json` | repo | added: the golden of `pipeline.py:GamifyPipeline._compute_and_store_coaching` (adapter: a stub store recording the rollup limit, a patched coaching payload) |
| `tools/parity-oracle/goldens/records_chase.json` | repo | added: the golden of `bot.py:CommandBot._render_records` (adapter: days as day tokens, the record to chase read back) |
| `tools/parity-oracle/goldens/next_milestone.json` | repo | added: the golden of `gamification/rewards.py:next_milestone` (adapter: the milestone's fields) |
| `tools/parity-oracle/goldens/rewards.constants.json` | repo | added: the golden of the three ladders and the record and ladder labels (constants) |
| `scripts/mutation-rows.d/S07300-S07399.json` | repo | added: the rows of section 9 |
| `docs/specs/SPEC-073-badges-records-and-the-next-milestone-are-awarded-once-and-shown-with-their-progress.md` | docs | moved from `docs/specs/planned/` |
| `formal/tla/AwardOnce/` | formal | added: the award-once model, its clean configuration and four witnesses |
| `formal/lean/Formal/NextMilestone.lean` | formal | added: the pick of the next milestone is least, with its tie order and its complete case |
| `docs/decisions/ADR-303-the-celebration-mark-lives-on-the-award-row.md` | docs | added |
| `docs/red-first/SPEC-073.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It evaluates no habit or focus badge: their contexts' rules do, and award through this port (#93,
  #94, #98).
- It awards no band badge by itself: the band-up awards it through this port and celebrates it (#85).
- It gives a new badge or record the router's line only; the milestone pings' wording is theirs
  (#128).
- It builds no personal leaderboard; the board joins the records screen with its own SPEC (#79).
- It renders no badge gallery image and publishes no badge (#156).
- It sends no streak share card (#125).
- It computes the milestone from no stand-in for Road to C2's mature cards: the card is pending until
  Road to C2 supplies them (#85).
- It serves no agent read tool for badges, records or the milestone (#157).
- It imports none of the predecessor's badges or records (#61).

## 6. Risks

- **A closing day is judged with its stored historical score.** `legendary_day` and `perfect_week`
  would then be missed on every day the owner never synced before its close. Detected by A11 and by
  the badge context's golden (A9).
- **A first run celebrates old records.** Every best is new against an empty table. Detected by A15
  and row S07310.
- **A first run with no import awards many badges at once.** The predecessor celebrates each newly
  awarded badge, and so does DeckStreak; the router's dedupe keeps each to one, and the v9 import
  brings the owner's earned badges before the first recompute (#61).
- **The milestone waits on Road to C2.** `GET /api/milestone` answers `pending` until SPEC-077 wires
  the mature-card sum, so the home screen's card (SPEC-086) shows it pending; held by A19.
- **A band key names an unconfigured course.** Refused before a write (A4, row S07306).
- **A course-named description drifts from the configuration.** Rendered from the configured courses
  (A2).

## 7. Parity goldens

Every golden is generated on the owner's machine from the predecessor at `27ee2bc` (SPEC-029), with
synthetic inputs only, and registered in `tools/parity-oracle/registry/spec_073.py`.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `badge_catalog` | `constants.py:BADGES` | adapter | each badge's key, name, emoji, description and tier, in catalog order |
| `badge_conditions` | `gamification/badges.py:_conditions` | adapter | a badge context from the case (daily metrics, a card snapshot, a streak state), each threshold on both sides |
| `badge_context` | `pipeline.py:GamifyPipeline._evaluate_and_award` | adapter | a stand-in pipeline over a stub store holding the case's rollups and badges, synthetic reviews, a collection configuration and a card-to-deck map; `badges.evaluate` is wrapped to record the context it receives, and the adapter returns that context's fields and the keys awarded |
| `count_reviews_in_local_hours` | `analytics.py:count_reviews_in_local_hours` | adapter | synthetic reviews around the window's edges and a collection configuration from the case's offset and rollover |
| `badges.constants` | `constants.py`, `gamification/badges.py` | constants | the thresholds R6 names |
| `detect_records` | `gamification/rewards.py:detect_records` | adapter | rollup rows and stored bests from the case; each record returned as its kind, value and label |
| `records_window` | `pipeline.py:GamifyPipeline._compute_and_store_coaching` | adapter | a stub store that records the rollup limit it is asked for and holds the case's stored records, with `coaching.compute_all` patched to return the case's new records; it returns the limit, the rows written, the celebration keys with each day as a day token, and whether the seed ran |
| `records_chase` | `bot.py:CommandBot._render_records` | adapter | a board from the case, each day passed as a day token; it returns the record to chase and its gap, read from the rendered text, or none |
| `next_milestone` | `gamification/rewards.py:next_milestone` | adapter | the milestone's label, emoji, current value, target, percentage and remainder |
| `rewards.constants` | `gamification/rewards.py` | constants | `REVIEW_LADDER`, `STREAK_LADDER`, `MATURE_LADDER`, `_LADDER_META` and `_RECORD_META` |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `badges_earned` | `progression` | `migrations/007301_progression_badges_earned.sql` | the earned badges, key and tier, with the day each was earned as its epoch day | exported and erased |
| `records` | `progression` | `migrations/007302_progression_records.sql` | the three records, each with the value it beat and the day it was set as its epoch day | exported and erased |

## 9. Mutation rows

A target outside a crate (a migration) is a cargo-killed script mutation (SPEC-039).

| row | target | what it guards | killer |
|---|---|---|---|
| `S07303-ONE-ROW-PER-BADGE` | `migrations/007301_progression_badges_earned.sql` | the primary key on the badge key and tier | `badges_award::awarding_a_badge_twice_writes_one_row` |
| `S07304-LEGENDARY-DAY-AT-ONE-HUNDRED` | `crates/progression/src/badges/conditions.rs` | a legendary day needs a score of 100 | `badges_conditions::the_study_badge_conditions_match_the_parity_golden` |
| `S07305-A-PERFECT-WEEK-NEEDS-SEVEN-SCORES` | `crates/progression/src/badges/conditions.rs` | a perfect week needs 7 scores | `badges_conditions::the_study_badge_conditions_match_the_parity_golden` |
| `S07306-THE-BAND-KEY-RULE` | `crates/progression/src/badges/award.rs` | a band key names a configured course and a band from A1 to C2 | `badges_award::a_band_badge_key_names_a_configured_course_and_a_band` |
| `S07307-A-RECORD-IS-STRICTLY-GREATER` | `crates/progression/src/records.rs` | only a value above the stored best is a record | `records_detect::detect_records_matches_the_parity_golden` |
| `S07308-MINUTES-ARE-WHOLE` | `crates/progression/src/records.rs` | the most minutes are the whole minutes of the most seconds | `records_detect::detect_records_matches_the_parity_golden` |
| `S07309-THE-RECORDS-WINDOW` | `crates/coordination/src/recompute/records.rs` | the detection reads the 370 most recent rollups | `records_steps::the_records_step_matches_the_parity_golden` |
| `S07310-THE-SILENT-SEED` | `crates/coordination/src/recompute/records.rs` | the first detection stores `previous` equal to the value and celebrates none | `records_steps::the_first_detection_seeds_records_silently` |
| `S07311-MILESTONE-TIES-TO-REVIEWS` | `crates/progression/src/milestone.rs` | a tie keeps the earlier ladder | `milestone_ladder::next_milestone_matches_the_parity_golden` |
| `S07312-THE-TOP-REVIEW-RUNG` | `crates/progression/src/milestone.rs` | three complete ladders report the top review rung at 100% | `milestone_ladder::a_complete_ladder_contributes_nothing_and_all_complete_reports_the_top_review_rung` |
| `S07313-THE-MARK-FOLLOWS-THE-ROUTER` | `crates/coordination/src/recompute/badges.rs` | `celebrated_at` is set only after the router answers | `badges_steps::a_router_that_did_not_answer_leaves_the_award_due` |
| `S07314-A-PENDING-AWARD-IS-OFFERED-AGAIN` | `crates/coordination/src/recompute/badges.rs` | an award whose mark is unset is offered at the next evaluation | `badges_steps::an_evaluation_stopped_after_the_write_sends_once_at_the_next` |
| `S07315-A-PENDING-RECORD-IS-OFFERED-BEFORE-ITS-ROW-IS-REPLACED` | `crates/coordination/src/recompute/records.rs` | a later day's beat offers the earlier record whose mark is unset before the row is replaced | `records_steps::a_later_beat_offers_the_unmarked_record_first` |
