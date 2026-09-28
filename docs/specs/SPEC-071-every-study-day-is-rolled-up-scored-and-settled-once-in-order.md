# SPEC-071: every study day is rolled up, scored and settled once, in order

- **Wave:** W3. **Issue:** #66, #67, #68 (epic #4). **Context(s):** `deck-streak-analytics` (the
  daily rollup, the per-language statistics, the card snapshot, the five-pillar score, the raw
  streak and the volume baseline, the settle cursor); `deck-streak-coordination` (the fold in
  `crates/coordination/src/recompute/` and its phase order); `deck-streak-ingest` (each card's
  course, the collection day number); `deck-streak-kernel` (the owner's courses, ADR-087); the
  Mini App (`web/app`, the score screen); `deck-streak-api` and `deck-streak-bot` (the score reads).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-020 (the study day and the
  migrations' rule), ADR-037 (one scheduled sync per study day plus the owner's triggers), ADR-071
  (the recompute settles each study day once, in order, with its end-of-day state) and ADR-087
  (the owner's courses are private configuration).
- **Prerequisites:** SPEC-020 (the study day, settings, the SQLite base, the data-rights port),
  SPEC-021 (export and erase), SPEC-023 (the window read and the change gate), SPEC-024 (the
  owner's session), SPEC-026 (the bot's command table), SPEC-027 (the sync job), SPEC-029 (the
  golden reader); SPEC-045 when the readings taxonomy is configured (R3). **Mutation band:**
  `S07100-S07199`.
- **Status:** in delivery: moved to `docs/specs/` with its tests and `docs/red-first/SPEC-071.md`
  (ADR-016).

## 1. The problem, measured

- **Nothing rolls a day up yet.** `crates/analytics/src/` holds only `lib.rs` at `dev` c3d769b, and
  the sync cycle's recompute reads the window and writes the gate's anchor with no domain consumer
  (`crates/coordination/src/sync_cycle.rs`, its module documentation). Every W3 feature reads a
  day's rollup: the score, XP, the streaks, quests, chests, coins, charts and the home screen.
- **What is ported** (predecessor `27ee2bc`): the day's metrics (`analytics.py:compute_daily_metrics`),
  the card snapshot (`analytics.py:compute_card_snapshot`) at the collection day number
  (`analytics.py:today_day_number`), the per-language metrics
  (`cross_language.py:language_daily_metrics`), the five-pillar score (`scoring.py:compute_score`
  with `_consistency`, `_retention`, `_workload`, `_volume` and `_mastery`), the grade band
  (`scoring.py:grade_band`), the raw streak the consistency pillar reads (`analytics.py:raw_streak`)
  and the volume baseline (`analytics.py:volume_baseline` over the rows
  `pipeline.py:GamifyPipeline._baseline` selects).
- **Why the predecessor's pipeline cannot be ported as it stands.** It evaluates the card snapshot,
  the backlog-zero bonus, the streak's advance, chests, quests and the relight only for the study
  day that is current when a recompute runs (`pipeline.py:GamifyPipeline._recompute_day`, its
  `is_today`), and it clears and re-derives each past day's XP at every recompute. Under one
  scheduled sync per study day, just after the rollover (ADR-037), the current day at that
  recompute is the new, empty one. ADR-071 decides the fold this SPEC builds: each closed day is
  settled once, oldest first, with the state it had at its close.
- **The owner's courses are private.** The predecessor attributes a review to a language through a
  hard-coded table of the owner's deck names (`curriculum.py:LANGUAGE_DECKS`, read by
  `progress.py:language_of`). ADR-087 makes the courses private configuration; this SPEC builds
  its loader, because the per-language statistics are its first reader.
- **Traps a hand port falls into.**
  - Python's `round` sends a half to the even neighbour (`scoring.py:compute_score`'s total); the
    port uses `f64::round_ties_even`, and the golden carries tie cases.
  - The seconds are a float sum of capped answers in the reviews' order; another order can move the
    last bit and a rounded minute.
  - A language is the course whose deck root EQUALS the top-level name of the card's home deck
    (`progress.py:language_of`); the read's scope (SPEC-023) matches by prefix. A deck whose name
    only starts with a course's root is in scope and in no course.
  - The baseline window is a count of rollup rows before the day, not of active days
    (`pipeline.py:GamifyPipeline._baseline`); a day with no review takes a row.
  - The card snapshot's leech count excludes only suspended cards (queue -1); a buried card with
    enough lapses is a leech (`analytics.py:compute_card_snapshot`).
- **Corrections to the issues.** #68 says the volume baseline is the median over "at most 30 prior
  active days"; the predecessor takes the median over the active days among the most recent rollup
  rows before the day (`pipeline.py:GamifyPipeline._baseline`), which the golden `baseline_window`
  proves. #66 says a past day's card state is never overwritten; the predecessor also never
  re-scores a past day with it (its past-day score has no snapshot), which R14 keeps.
- **What the parity oracle proves.** Every function above over seeded synthetic reviews and cards,
  with the tie, empty-day and boundary classes; and every constant this SPEC names, read from the
  predecessor's `constants.py`.
- **Prerequisites.** Listed in the header. SPEC-072, SPEC-073, SPEC-074 and SPEC-076 to SPEC-082
  register their steps in the fold this SPEC builds, and cannot land before it.

## 2. Requirements

The owner's courses (ADR-087)

R1. The kernel loads the owner's courses once at start from the private file that
    `DECKSTREAK_COURSES_FILE` names (schema `deckstreak.courses.v1`) into `Courses`. Each course
    carries a `code`, a `name`, a `flag`, the top-level deck name it roots (`deck_root`), a
    one-letter `alias`, whether it carries the writing habit (`writing`), and its `unit_bands`
    (an inclusive range of unit numbers for each CEFR band, A1 to C2); a top-level `focus_subjects`
    list names further focus subjects with their aliases. Start is refused, with an error naming
    the setting and never a value, when the file is unreadable or malformed, when two entries share
    a code, an alias or a deck root, or when a course's unit bands overlap, run backwards or come
    out of order. With the setting unset there are no courses, and the start logs that once.
    `deploy/config/courses.example.json` shows the shape with neutral values; no course, deck name,
    alias or unit band of the owner's is a literal anywhere in the repository.
R2. Ingest publishes each card's course: the course whose `deck_root` equals, exactly, the
    top-level name of the card's home deck (the predecessor's `progress.py:language_of`), or none.
    Only the course code travels with the card.
R3. When the readings taxonomy (SPEC-045) is configured, a language deck that it maps to one code
    and the courses file to another refuses start, naming both settings and neither value.
R4. At start, when the courses file's digest differs from the digest recorded with the settings
    generation, the kernel bumps the settings generation and records the new digest in the same
    write (SPEC-020 R19), so the next cycle's change gate recomputes (SPEC-023).

The daily rollup and the card snapshot (#66)

R5. Analytics owns `daily_rollup`, created by `migrations/007101_analytics_daily_rollup.sql`
    (`STRICT`, `created_at`, `updated_at`): one row per study day, keyed by the kernel's
    `StudyDay`, holding the metrics of R6, the card state and its provenance (R8), the score and
    its five pillars (R12), the score the day closed with (R14), the instant the day was settled
    (R16) and the day's review fingerprint (R18).
R6. A day's metrics equal the golden of `analytics.py:compute_daily_metrics` for every case: study
    events only (SPEC-023's rule); reviews by type; the seconds, each answer's time capped at
    `constants.ANSWER_TIME_CAP_SECONDS` and summed in the reviews' order; true retention over the
    first review-type answer of each card that day, passed at ease 2 or more, split young and
    mature by the interval the card had going in against `constants.MATURE_IVL_DAYS`; graduations,
    the review-type answers whose interval crosses that boundary; and the distinct home decks
    answered. Every constant comes from `goldens/analytics.constants.json`.
R7. The collection day number of a study day is that day minus the study day of the collection's
    creation instant, under the configured study-day rule, and equals the golden of
    `analytics.py:today_day_number` for every case.
R8. A day's card snapshot equals the golden of `analytics.py:compute_card_snapshot` at that day's
    collection day number: suspended cards, leeches (lapses at or above the leech threshold, not
    suspended), mature and young review cards by interval, learning cards, the backlog (review-queue
    cards due before the day number) and the cards due on it. The leech threshold is the setting
    `DECKSTREAK_LEECH_THRESHOLD`, whose default is `constants.DEFAULT_LEECH_THRESHOLD`. The rollup
    stores the snapshot's mature, young, leech, backlog and due-today counts with a provenance:
    NULL when never recorded, `live:<epoch milliseconds>` when a recompute recorded it.
R9. A day's card state and its provenance are written only by the recompute that evaluates that day
    as the current day or settles it as the closing day (ADR-071); no other recompute overwrites
    them. A day never recorded keeps a NULL card state and a NULL provenance.
R10. The API renders a NULL card state, and the retention of a day with no answered review, as
    null, never as 0; the stored retention of such a day is the golden's value.

The per-language statistics (#67)

R11. Analytics owns `daily_lang_stats`, created by the same migration: one row per study day and
    course code, with the reviews, seconds, answered and passed of that course's cards that day.
    A day's rows equal the golden of `cross_language.py:language_daily_metrics` over the configured
    courses; a review whose card has no course contributes no row; a re-rolled day's rows replace
    all of that day's earlier rows in one write.

The five-pillar score and the grade (#68)

R12. A day's score and its five pillars equal the golden of `scoring.py:compute_score`:
    consistency, retention, workload, volume and mastery, each equal to its own golden
    (`scoring.py:_consistency`, `_retention`, `_workload`, `_volume`, `_mastery`), weighted by the
    constants' `WEIGHT_*`, clamped to 0 to 100 and rounded half to even; without a card snapshot,
    the three time-accurate pillars renormalised by their weights. The grade band and its emoji
    equal the golden of `scoring.py:grade_band`.
R13. The consistency pillar's streak is the raw run of consecutive study days ending on the day, or
    on the day before while the day has no review, and equals the golden of
    `analytics.py:raw_streak`. The volume pillar's baseline equals the golden of
    `analytics.py:volume_baseline` (the median reviews and minutes of the active rows, with its
    floors) over the rows the golden `baseline_window` proves `pipeline.py:GamifyPipeline._baseline`
    selects.
R14. The current study day is scored with its live card snapshot. A closed day is scored at its
    settle with its end-of-day snapshot when it has one, and that total is kept in
    `score_at_close`, because it decides the day's score-dependent grants (ADR-071, SPEC-072). From
    its settle on, the day's stored score is re-scored at every recompute in the predecessor's
    historical form: no snapshot, and the current day's baseline, as the predecessor stores a past
    day's score.

The fold (ADR-071)

R15. After every successful sync, scheduled or the owner's, the recompute settles each closed study
    day after the last settled one, oldest first, and then evaluates the current study day. A
    closed day is settled only by a recompute that follows a successful sync which started after
    the day closed; until then it stays owed, and only the current day is evaluated.
R16. Settling a day records the instant in its rollup's `settled_at`. The last settled day is the
    cursor, and a recompute never settles a day twice.
R17. A closing day's settle runs every registered step with that day as the day evaluated; when it
    is the most recently closed day, its card snapshot is taken at its own collection day number
    (R7, R8) and recorded as its card state. With no settled day at all (the first recompute),
    every study day of the window is rolled up and scored in the historical form, and no step runs
    a today-only rule for those days; the fold then settles from the most recently closed day on.
R18. A day's review fingerprint digests every field of its study reviews together with the courses
    file's digest. A recompute re-rolls a day's metrics and per-language rows only when the
    fingerprint changed, when it settles the day, or when the day is the current one; it re-scores
    every study day of the window and the settling and current days (R14).
R19. The fold runs its steps in a fixed phase order, declared once in
    `crates/coordination/src/recompute/mod.rs`: (1) the rollup and score, (2) base XP, (3) the
    streaks and the governor, (4) the other contexts' day steps, (5) the derived bonuses, (6) the
    coin mint, (7) awards. This SPEC registers phase 1's step; SPEC-072, SPEC-073, SPEC-074 and
    SPEC-076 to SPEC-082 each register theirs in its phase.

The surfaces

R20. `GET /api/analytics/days` returns the rollups of a range of study days, at most the window's
    length (SPEC-023), each with its metrics, card state, provenance and score; `GET /api/score`
    returns the current study day's score, its five pillars and its grade. Both answer the owner's
    session only (SPEC-024).
R21. `/score` answers with the current day's score, its grade, its reviews and its retention: the
    numbers `GET /api/score` returns for the same study day, with an absent retention said to be
    absent.
R22. The Mini App's `/score` screen shows the five pillars as a breakdown beside the total and the
    grade, and shows an absent retention as a gap, never as 0.
R23. Analytics' data-rights port lists `daily_rollup` and `daily_lang_stats` as exported and erased,
    and both are registered with the six files of SPEC-021's rule (§4).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a day's metrics equal the golden of `analytics.py:compute_daily_metrics` for every case (examined count reported, zero refused) | `the_daily_metrics_match_the_predecessors_golden` |
| A2 | a day's card snapshot equals the golden of `analytics.py:compute_card_snapshot` for every case | `the_card_snapshot_matches_the_predecessors_golden` |
| A3 | a day's per-language rows equal the golden of `cross_language.py:language_daily_metrics` over synthetic courses | `the_per_language_metrics_match_the_predecessors_golden` |
| A4 | a review whose card has no course, including one whose deck only starts with a course's root, adds no per-language row | `a_review_with_no_course_adds_no_language_row` |
| A5 | re-rolling a day with the same reviews writes identical rollup and per-language rows | `rerolling_a_day_with_the_same_reviews_writes_identical_rows` |
| A6 | re-rolling a settled day leaves its card state, its provenance and the score it closed with untouched, and a day never recorded keeps a NULL card state | `rerolling_a_settled_day_keeps_its_card_state_and_provenance` |
| A7 | every analytics constant equals the golden of the predecessor's constants | `the_analytics_constants_equal_the_predecessors` |
| A8 | the score and its pillars equal the golden of `scoring.py:compute_score`, with and without a card snapshot, ties rounded half to even | `the_score_matches_the_predecessors_golden` |
| A9 | each pillar equals its own golden (`_consistency`, `_retention`, `_workload`, `_volume`, `_mastery`) | `the_five_pillars_match_their_goldens` |
| A10 | the grade band, the raw streak, the volume baseline and the baseline's window equal their goldens | `the_grade_raw_streak_and_baseline_match_their_goldens` |
| A11 | the collection day number equals the golden of `analytics.py:today_day_number` | `the_collection_day_number_matches_the_predecessors_golden` |
| A12 | a card's course is the course whose deck root equals its home deck's top-level name, or none | `a_cards_course_is_the_course_whose_root_is_its_top_level_name` |
| A13 | the courses file loads the example, and refuses a file it cannot read, a malformed file, a duplicate code, alias or deck root and overlapping or unordered unit bands, each naming the setting and never a value | `the_courses_file_refuses_a_duplicate_or_overlapping_course` |
| A14 | a changed courses file bumps the settings generation once at start, and an unchanged one does not | `a_changed_courses_file_bumps_the_settings_generation_once` |
| A15 | a readings-taxonomy language deck mapped to another code than the courses file's refuses start | `a_taxonomy_language_mapped_to_another_code_refuses_start` |
| A16 | the fold settles each closed day after the cursor exactly once, oldest first, and a second recompute settles no day | `the_fold_settles_each_closed_day_once_oldest_first` |
| A17 | the most recently closed day is settled with its end-of-day card state at its own day number, and the score it closed with is kept | `the_closing_day_is_settled_with_its_end_of_day_card_state` |
| A18 | a closed day stays owed until a successful sync that started after its close | `a_closed_day_waits_for_a_successful_sync_after_its_close` |
| A19 | the first recompute rolls the window up in the historical form and runs no today-only rule for its past days | `the_first_recompute_backfills_the_window_without_today_only_rules` |
| A20 | the registered steps run in the declared phase order, and a step registered outside its phase is refused | `the_steps_run_in_their_phase_order` |
| A21 | a recompute re-rolls exactly the days whose fingerprint changed, the days it settles and the current day (counted) | `a_recompute_rerolls_only_changed_settling_and_current_days` |
| A22 | `GET /api/analytics/days` renders a day never recorded and a day with no answer as null card state and null retention | `a_day_never_recorded_and_a_day_with_no_answer_render_as_null` |
| A23 | the analytics and score routes answer only the owner's session: 401 or 403 and no data otherwise | `the_analytics_routes_answer_only_the_owner` |
| A24 | `/score` reports the numbers `GET /api/score` returns for the same study day | `score_reports_the_numbers_the_score_route_returns` |
| A25 | the score screen shows five pillars and an absent retention as a gap | `shows five pillars and an absent retention as a gap` |
| A26 | analytics' port exports and erases `daily_rollup` and `daily_lang_stats`, and an erase leaves both empty | `the_rollup_tables_are_exported_and_erased` |

```acceptance
A1: cargo test -p deck-streak-analytics --test rollup_metrics -- --exact the_daily_metrics_match_the_predecessors_golden
A2: cargo test -p deck-streak-analytics --test rollup_metrics -- --exact the_card_snapshot_matches_the_predecessors_golden
A3: cargo test -p deck-streak-analytics --test rollup_metrics -- --exact the_per_language_metrics_match_the_predecessors_golden
A4: cargo test -p deck-streak-analytics --test rollup_metrics -- --exact a_review_with_no_course_adds_no_language_row
A5: cargo test -p deck-streak-analytics --test rollup_store -- --exact rerolling_a_day_with_the_same_reviews_writes_identical_rows
A6: cargo test -p deck-streak-analytics --test rollup_store -- --exact rerolling_a_settled_day_keeps_its_card_state_and_provenance
A7: cargo test -p deck-streak-analytics --test score_pillars -- --exact the_analytics_constants_equal_the_predecessors
A8: cargo test -p deck-streak-analytics --test score_pillars -- --exact the_score_matches_the_predecessors_golden
A9: cargo test -p deck-streak-analytics --test score_pillars -- --exact the_five_pillars_match_their_goldens
A10: cargo test -p deck-streak-analytics --test score_pillars -- --exact the_grade_raw_streak_and_baseline_match_their_goldens
A11: cargo test -p deck-streak-ingest --test rollup_day_number -- --exact the_collection_day_number_matches_the_predecessors_golden
A12: cargo test -p deck-streak-ingest --test courses_cards -- --exact a_cards_course_is_the_course_whose_root_is_its_top_level_name
A13: cargo test -p deck-streak-kernel --test courses_config -- --exact the_courses_file_refuses_a_duplicate_or_overlapping_course
A14: cargo test -p deck-streak-kernel --test courses_config -- --exact a_changed_courses_file_bumps_the_settings_generation_once
A15: cargo test -p deck-streak-coordination --test courses_agree -- --exact a_taxonomy_language_mapped_to_another_code_refuses_start
A16: cargo test -p deck-streak-coordination --test settle_fold -- --exact the_fold_settles_each_closed_day_once_oldest_first
A17: cargo test -p deck-streak-coordination --test settle_fold -- --exact the_closing_day_is_settled_with_its_end_of_day_card_state
A18: cargo test -p deck-streak-coordination --test settle_fold -- --exact a_closed_day_waits_for_a_successful_sync_after_its_close
A19: cargo test -p deck-streak-coordination --test settle_fold -- --exact the_first_recompute_backfills_the_window_without_today_only_rules
A20: cargo test -p deck-streak-coordination --test settle_fold -- --exact the_steps_run_in_their_phase_order
A21: cargo test -p deck-streak-coordination --test settle_fold -- --exact a_recompute_rerolls_only_changed_settling_and_current_days
A22: cargo test -p deck-streak-api --test rollup_routes -- --exact a_day_never_recorded_and_a_day_with_no_answer_render_as_null
A23: cargo test -p deck-streak-api --test rollup_routes -- --exact the_analytics_routes_answer_only_the_owner
A24: cargo test -p deck-streak-bot --test score_commands -- --exact score_reports_the_numbers_the_score_route_returns
A25: pnpm exec vitest run web/app/src/lib/score/ScoreBreakdown.test.ts -t "shows five pillars and an absent retention as a gap"
A26: cargo test -p deck-streak-analytics --test rollup_rights -- --exact the_rollup_tables_are_exported_and_erased
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. No pack's state changes for this delivery: the
privacy-gdpr and accessibility packs stay enforced, and no row is deferred for it.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md`, `migrations/*.sql` and `crates/analytics/src/data_rights.rs`: a category declares `daily_rollup.*` and `daily_lang_stats.*` with its purpose, basis, retention, export and erase, every table the migrations create is covered, and the policy has a line for the category | the privacy-gdpr pack |
| B2 | over `web/app/src/routes/score/+page.svelte` and `web/app/src/lib/score/`: the score screen passes in both of Telegram's colour schemes, and each pillar's value has a text label as well as its bar | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/kernel/src/courses.rs` | `deck-streak-kernel` | added: `Courses`, its loader and its refusals (R1), the digest (R4) |
| `crates/kernel/src/lib.rs` | `deck-streak-kernel` | changed: the courses module |
| `crates/kernel/src/db.rs` | `deck-streak-kernel` | changed: the settings generation records the courses digest (R4) |
| `crates/kernel/src/data_rights.rs` | `deck-streak-kernel` | changed: the reset row clears the courses digest |
| `migrations/007102_kernel_courses_digest.sql` | `deck-streak-kernel` | added: the courses digest column of `settings_generation` (additive) |
| `crates/kernel/tests/courses_config.rs` | `deck-streak-kernel` | added: A13, A14 |
| `crates/ingest/src/reader.rs` | `deck-streak-ingest` | changed: each card's course (R2) |
| `crates/ingest/src/calendar.rs` | `deck-streak-ingest` | added: the collection day number (R7) |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the calendar module |
| `crates/ingest/tests/rollup_day_number.rs` | `deck-streak-ingest` | added: A11 |
| `crates/ingest/tests/courses_cards.rs` | `deck-streak-ingest` | added: A12 |
| `crates/analytics/Cargo.toml` | `deck-streak-analytics` | changed: the workspace dependencies it uses (`sqlx`, `thiserror`, `tracing`, `serde`, `serde_json`); `serde`, `serde_json`, `tempfile` and `tokio` as dev-dependencies for the golden reader and the store tests (SPEC-029 R8) |
| `crates/analytics/src/lib.rs` | `deck-streak-analytics` | changed: the modules below |
| `crates/analytics/src/constants.rs` | `deck-streak-analytics` | added: the constants the golden proves |
| `crates/analytics/src/metrics.rs` | `deck-streak-analytics` | added: the day's metrics and the per-language metrics |
| `crates/analytics/src/snapshot.rs` | `deck-streak-analytics` | added: the card snapshot |
| `crates/analytics/src/score.rs` | `deck-streak-analytics` | added: the pillars, the total, the grade, the raw streak, the baseline |
| `crates/analytics/src/rollup.rs` | `deck-streak-analytics` | added: the repository over `daily_rollup` and `daily_lang_stats`, the fingerprint, the settle cursor |
| `crates/analytics/src/settings.rs` | `deck-streak-analytics` | added: `DECKSTREAK_LEECH_THRESHOLD` |
| `crates/analytics/src/data_rights.rs` | `deck-streak-analytics` | added: the analytics data-rights port |
| `migrations/007101_analytics_daily_rollup.sql` | `deck-streak-analytics` | added: `daily_rollup` and `daily_lang_stats` |
| `crates/analytics/tests/rollup_metrics.rs` | `deck-streak-analytics` | added: A1 to A4 |
| `crates/analytics/tests/rollup_store.rs` | `deck-streak-analytics` | added: A5, A6 |
| `crates/analytics/tests/score_pillars.rs` | `deck-streak-analytics` | added: A7 to A10 |
| `crates/analytics/tests/rollup_rights.rs` | `deck-streak-analytics` | added: A26 |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | added: the fold, the phases, the step registry |
| `crates/coordination/src/recompute/analytics_step.rs` | `deck-streak-coordination` | added: phase 1's step |
| `crates/coordination/src/courses.rs` | `deck-streak-coordination` | added: the start-up agreement with the readings taxonomy (R3) |
| `crates/coordination/src/sync_cycle.rs` | `deck-streak-coordination` | changed: the recompute runs the fold |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the modules above |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the registry gains analytics' port |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded `daily_rollup` and `daily_lang_stats` rows |
| `crates/coordination/tests/settle_fold.rs` | `deck-streak-coordination` | added: A16 to A21 |
| `crates/coordination/tests/courses_agree.rs` | `deck-streak-coordination` | added: A15 |
| `crates/api/src/analytics_routes.rs` | `deck-streak-api` | added: `GET /api/analytics/days`, `GET /api/score` |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the analytics routes behind the owner's session |
| `crates/api/tests/rollup_routes.rs` | `deck-streak-api` | added: A22, A23 |
| `crates/bot/src/score_commands.rs` | `deck-streak-bot` | added: /score |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the command table and the owner's menu gain /score |
| `crates/bot/tests/score_commands.rs` | `deck-streak-bot` | added: A24 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the courses, the fold's analytics step, the routes' reader |
| `web/app/src/routes/score/+page.svelte` | miniapp | added: the score screen |
| `web/app/src/lib/score/ScoreBreakdown.svelte` | miniapp | added: the five pillars |
| `web/app/src/lib/score/score.ts` | miniapp | added: the score route's client and types |
| `web/app/src/lib/score/ScoreBreakdown.test.ts` | miniapp | added: A25 |
| `web/app/src/lib/routes.ts` | miniapp | changed: /score joins `ROUTES` |
| `deploy/config/courses.example.json` | deploy | added: neutral example courses |
| `.env.example` | repo | changed: `DECKSTREAK_COURSES_FILE`, `DECKSTREAK_LEECH_THRESHOLD` |
| `deploy/deck-streak.env.example` | deploy | changed: the same two settings |
| `docs/CONTEXT-MAP.md` | docs | changed: the own-tables rows for `daily_rollup` and `daily_lang_stats`; `settings_generation`'s reset clears the courses digest |
| `privacy.json` | repo | changed: the category of the two tables; `service-counters` names the courses digest |
| `PRIVACY.md` | repo | changed: the category's line, and the service counters' line |
| `.sqlx/` | workspace | changed: the offline query cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `tools/parity-oracle/registry/spec_071.py` | repo | added: this SPEC's registrations |
| `tools/parity-oracle/goldens/daily_metrics.json` | repo | added: the golden of `analytics.py:compute_daily_metrics` (adapter) |
| `tools/parity-oracle/goldens/card_snapshot.json` | repo | added: the golden of `analytics.py:compute_card_snapshot` (adapter) |
| `tools/parity-oracle/goldens/today_day_number.json` | repo | added: the golden of `analytics.py:today_day_number` (adapter) |
| `tools/parity-oracle/goldens/language_daily_metrics.json` | repo | added: the golden of `cross_language.py:language_daily_metrics` (adapter) |
| `tools/parity-oracle/goldens/raw_streak.json` | repo | added: the golden of `analytics.py:raw_streak` (adapter) |
| `tools/parity-oracle/goldens/volume_baseline.json` | repo | added: the golden of `analytics.py:volume_baseline` (function) |
| `tools/parity-oracle/goldens/baseline_window.json` | repo | added: the golden of `pipeline.py:GamifyPipeline._baseline` (adapter) |
| `tools/parity-oracle/goldens/compute_score.json` | repo | added: the golden of `scoring.py:compute_score` (adapter) |
| `tools/parity-oracle/goldens/score_consistency.json` | repo | added: the golden of `scoring.py:_consistency` (function) |
| `tools/parity-oracle/goldens/score_retention.json` | repo | added: the golden of `scoring.py:_retention` (function) |
| `tools/parity-oracle/goldens/score_workload.json` | repo | added: the golden of `scoring.py:_workload` (function) |
| `tools/parity-oracle/goldens/score_volume.json` | repo | added: the golden of `scoring.py:_volume` (function) |
| `tools/parity-oracle/goldens/score_mastery.json` | repo | added: the golden of `scoring.py:_mastery` (function) |
| `tools/parity-oracle/goldens/grade_band.json` | repo | added: the golden of `scoring.py:grade_band` (function) |
| `tools/parity-oracle/goldens/analytics.constants.json` | repo | added: the constants golden (§7) |
| `scripts/mutation-rows.d/S07100-S07199.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-071-every-study-day-is-rolled-up-scored-and-settled-once-in-order.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-071-the-recompute-settles-each-study-day-once-in-order-with-its-end-of-day-state.md` | docs | changed: accepted |
| `docs/decisions/ADR-087-the-owners-courses-are-private-configuration-passed-to-the-predecessors-own-codes.md` | docs | changed: accepted |
| `docs/red-first/SPEC-071.md` | docs | added |
| `changelog.d/` fragment | repo | added |
| `crates/coordination/src/score.rs` | `deck-streak-coordination` | added at delivery (§10): the score reads the API and the bot share |
| `crates/coordination/tests/score_reads.rs` | `deck-streak-coordination` | added at delivery (§10): the score reads' own tests, the ones cargo-mutants runs on this package |
| `crates/api/src/session_routes.rs` | `deck-streak-api` | changed at delivery (§10): the owner's study day, `OwnerAccess::study_day` |
| `crates/api/src/lib.rs` | `deck-streak-api` | changed at delivery (§10): the analytics routes' module |
| `crates/api/Cargo.toml` | `deck-streak-api` | changed at delivery (§10): `sqlx` as a dev-dependency |
| `crates/bot/src/lib.rs` | `deck-streak-bot` | changed at delivery (§10): the /score module |
| `crates/bot/Cargo.toml` | `deck-streak-bot` | changed at delivery (§10): `sqlx` as a dev-dependency |
| `crates/bot/tests/commands.rs` | `deck-streak-bot` | changed at delivery (§10): the menu of five, and the score's goldens covered |
| `crates/bot/tests/support/fake_bot_api.rs` | `deck-streak-bot` | changed at delivery (§10): the bench's clock |
| `crates/bot/tests/messages/start.msg.json` | `deck-streak-bot` | changed at delivery (§10): the command lines name /score |
| `crates/bot/tests/messages/help.msg.json` | `deck-streak-bot` | changed at delivery (§10): the command lines name /score |
| `crates/bot/tests/messages/score.msg.json` | `deck-streak-bot` | added at delivery (§10): /score's golden |
| `crates/bot/tests/messages/score-no-retention.msg.json` | `deck-streak-bot` | added at delivery (§10): /score's golden for an absent retention |
| `crates/bot/tests/messages/score-none.msg.json` | `deck-streak-bot` | added at delivery (§10): /score's golden for a day with no score |
| `crates/bot/tests/messages/score-failed.msg.json` | `deck-streak-bot` | added at delivery (§10): /score's golden for a failed read |
| `crates/daemon/src/role_job.rs` | `deck-streak-daemon` | changed at delivery (§10): the sync job loads its recompute; the new error's reason |
| `crates/daemon/src/role_bot.rs` | `deck-streak-daemon` | changed at delivery (§10): the recompute loaded at start; the bot's clock and rule |
| `crates/kernel/tests/data_rights.rs` | `deck-streak-kernel` | changed at delivery (§10): the pinned reset row names `courses_digest` |
| `crates/readings/tests/support/mod.rs` | `deck-streak-readings` | changed at delivery (§10): a card literal names its `course` |
| `web/app/src/lib/api.ts` | miniapp | changed at delivery (§10): `score()`, through the one session; a refusal stops the client in the call that met it, and a call with no answer reads null; its equivalent mutants recorded in place, each alone on its line |
| `web/app/src/lib/api.test.ts` | miniapp | changed at delivery (§10): `score()`'s tests, and the tests that kill the client's older survivors, the failed handshake's interleaving among them |
| `web/app/src/lib/score/score.test.ts` | miniapp | added at delivery (§10): the score body's reading |
| `web/app/src/routes/score.test.ts` | miniapp | added at delivery (§10): the score screen's states |
| `web/app/src/lib/startapp.ts` | miniapp | changed at delivery (§10): the `score` token; the shape check is `isToken`, tested on its own; the `today` entry's equivalent mutants recorded in place |
| `web/app/src/lib/startapp.test.ts` | miniapp | changed at delivery (§10): the token list names `score`; Today's own path; `isToken`'s own tests |
| `.cargo/mutants.toml` | repo | changed at delivery (§10): the one Rust equivalent's anchored `exclude_re` record (#295) |
| `web/app/messages/en.json` | miniapp | changed at delivery (§10): the score screen's messages |
| `web/app/tests/a11y.spec.ts` | miniapp | changed at delivery (§10): the score route answered, so the audit renders the breakdown |
| `crates/ingest/src/sync_runs.rs` | `deck-streak-ingest` | changed at delivery (§10, Continuation): `first_success_in`, the start of a study day's first successful sync |
| `crates/ingest/tests/gate.rs` | `deck-streak-ingest` | changed at delivery (§10, Continuation): `first_success_in`'s own test, the one cargo-mutants runs on this package |

## 5. What this does NOT do

- It draws no chart: the calendar heatmaps and trend lines over these rows are drawn on the client
  by the chart component (#152).
- It writes no stats file into the vault (#153).
- It adds no section to the daily digest or the weekly report (#129, #130).
- It imports none of the predecessor's rollups and recovers no past card state; the import decides
  how its rows meet the window this SPEC backfills (#61).
- It serves no agent read tool over the rollups (#157).
- It grants no XP, advances no streak and mints no coin: each of those steps joins the fold in its
  own SPEC (#70, #81, #106).
- It builds no settings screen for the leech threshold or the courses (#57).
- It computes no leech remediation; the snapshot's leech count is a count, not the workflow (#133).

## 6. Risks

- **A settle reads a card state that already moved.** A review made in the current study day
  before the settle's recompute lowers the closing day's backlog or due count. Accepted by ADR-071;
  visible in the day's provenance instant, and bounded to the reviews made between the rollover
  and the scheduled sync.
- **The float sums or the rounding drift from the predecessor's.** Detected by A1, A8 and A9, whose
  goldens carry tie and many-answer cases, and by the constant rows of §9.
- **The courses file and the readings taxonomy disagree.** Refused at start (A15); a course one
  file forgot is not detected, and ADR-087 names it.
- **A late review changes a settled day's past.** The fingerprint re-rolls the day (A21), its card
  state and closing score stay (A6), and its XP only rises (ADR-072); the change is visible in the
  day's `updated_at`.
- **The first recompute backfills a long window at once.** It reads the window once (SPEC-023),
  and its cost is measured by the recompute's own duration in the sync run's record; a repeat
  settles nothing (A16).
- **A step registers in the wrong phase and pays a derived bonus before its base.** Refused by A20.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_071.py` (SPEC-029), generated on the owner's
checkout of the predecessor at `27ee2bc`; every day is an epoch day number and every instant epoch
milliseconds (SPEC-029 R3).

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `daily_metrics` | `analytics.py:compute_daily_metrics` | adapter | `Review` objects from the case's rows, a `CollectionConfig` from its rollover hour and offset, the day as a date from its epoch day, and the card-to-home-deck map; it returns every `DailyMetrics` field, the day as its epoch day. Classes: `tie`, `empty`, `mature-boundary` |
| `card_snapshot` | `analytics.py:compute_card_snapshot` | adapter | `Card` objects from the case's rows and a `CollectionConfig` carrying its leech threshold; returns every `CardStateSnapshot` field. Classes: `due-boundary`, `buried-leech` |
| `today_day_number` | `analytics.py:today_day_number` | adapter | a `CollectionConfig` from the rollover hour, the offset and the creation instant in seconds; the case's instant. Class: `rollover` |
| `language_daily_metrics` | `cross_language.py:language_daily_metrics` | adapter | replaces the `LANGUAGE_DECKS` binding that `progress.py` imports with synthetic courses (recorded in the note), and builds the reviews, the config, the card-to-deck map and the deck names; returns rows keyed by epoch day and code. Class: `prefix-only-deck` |
| `raw_streak` | `analytics.py:raw_streak` | adapter | the day set and the day as dates from epoch days. Classes: `today-unfinished`, `gap` |
| `volume_baseline` | `analytics.py:volume_baseline` | function | JSON rows of reviews and seconds. Class: `floor` |
| `baseline_window` | `pipeline.py:GamifyPipeline._baseline` | adapter | a stand-in pipeline whose stub store returns the case's recent rollups, most recent first; returns the baseline pair |
| `compute_score` | `scoring.py:compute_score` | adapter | a `DailyMetrics` and a `CardStateSnapshot`, or none, from the case; returns every `ScoreBreakdown` field. Classes: `tie`, `historical` |
| `score_consistency` | `scoring.py:_consistency` | function | the reviews and the streak days |
| `score_retention` | `scoring.py:_retention` | function | the retention and the answered count. Class: `small-sample` |
| `score_workload` | `scoring.py:_workload` | function | the review count, the due-today count, the backlog and whether the day was studied |
| `score_volume` | `scoring.py:_volume` | function | the reviews, the minutes and the two baselines |
| `score_mastery` | `scoring.py:_mastery` | function | the graduations and the active leeches |
| `grade_band` | `scoring.py:grade_band` | function | the score, at and around every band's threshold |
| `analytics.constants` | `constants.py` | constants | `ANSWER_TIME_CAP_SECONDS`, `MATURE_IVL_DAYS`, `DEFAULT_LEECH_THRESHOLD`, `WEIGHT_CONSISTENCY`, `WEIGHT_RETENTION`, `WEIGHT_WORKLOAD`, `WEIGHT_VOLUME`, `WEIGHT_MASTERY`, `RETENTION_FLOOR_PCT`, `RETENTION_CEIL_PCT`, `RETENTION_MIN_SAMPLE`, `RETENTION_BLEND_TARGET`, `VOLUME_REVIEW_WEIGHT`, `VOLUME_TIME_WEIGHT`, `VOLUME_CAP_RATIO`, `MASTERY_GRADUATION_TARGET`, `MASTERY_LEECH_PENALTY`, `MASTERY_LEECH_PENALTY_CAP`, `WORKLOAD_BACKLOG_DIVISOR`, `WORKLOAD_BACKLOG_PENALTY_CAP`, `GRADE_BANDS` |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `daily_rollup` | `analytics` | `migrations/007101_analytics_daily_rollup.sql` | `daily_rollup`: the same metrics and score per study day, its card-state counts with their provenance (a zero it never recorded arrives as NULL), and nothing of its day XP or its track column, which progression and the courses now hold | exported and erased |
| `daily_lang_stats` | `analytics` | `migrations/007101_analytics_daily_rollup.sql` | `daily_lang_stats`: the same four numbers per study day and course code | exported and erased |
| `settings_generation` (a column) | `kernel` | `migrations/007102_kernel_courses_digest.sql` | nothing: the courses digest is new | reset in place: the generation back to 0 and the courses digest cleared |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S07101-ANSWER-CAP` | `crates/analytics/src/constants.rs` | each answer's time is capped at the predecessor's constant | `score_pillars::the_analytics_constants_equal_the_predecessors` |
| `S07102-MATURE-INTERVAL` | `crates/analytics/src/constants.rs` | the young and mature split and the graduation boundary | `score_pillars::the_analytics_constants_equal_the_predecessors` |
| `S07103-LEECH-THRESHOLD-DEFAULT` | `crates/analytics/src/settings.rs` | the leech threshold's default is the predecessor's | `rollup_metrics::the_card_snapshot_matches_the_predecessors_golden` |
| `S07104-SCORE-WEIGHTS` | `crates/analytics/src/constants.rs` | the five weights | `score_pillars::the_score_matches_the_predecessors_golden` |
| `S07105-RETENTION-SMALL-SAMPLE` | `crates/analytics/src/constants.rs` | the minimum sample and the blend target of the retention pillar | `score_pillars::the_five_pillars_match_their_goldens` |
| `S07106-ROUND-HALF-EVEN` | `crates/analytics/src/score.rs` | the total rounds half to even | `score_pillars::the_score_matches_the_predecessors_golden` |
| `S07107-COURSE-ROOT-EQUALS` | `crates/ingest/src/reader.rs` | a course is matched by equality of the top-level name, never by prefix | `courses_cards::a_cards_course_is_the_course_whose_root_is_its_top_level_name` |
| `S07108-SETTLE-ONCE` | `crates/coordination/src/recompute/mod.rs` | the fold settles only the days after the cursor | `settle_fold::the_fold_settles_each_closed_day_once_oldest_first` |
| `S07109-PHASE-ORDER` | `crates/coordination/src/recompute/mod.rs` | the declared order of the seven phases | `settle_fold::the_steps_run_in_their_phase_order` |
| `S07110-ROLLUP-KEY` | `migrations/007101_analytics_daily_rollup.sql` | one row per study day, held by the table's key (a script mutation of the migration with a cargo killer) | `rollup_store::rerolling_a_day_with_the_same_reviews_writes_identical_rows` |
| `S07111-SETTLE-ONE-WRITE` | `crates/coordination/src/recompute/mod.rs` | a settle records its cursor in the same write as its steps (added at delivery, §10) | `settle_fold::a_settle_whose_cursor_is_refused_commits_none_of_its_steps_work` |
| `S07112-SYNC-START-DAY` | `crates/coordination/src/sync_cycle.rs` | the fold is handed the study day the latest successful sync started in (added at delivery, §10) | `settle_fold::cycle::a_sync_across_the_rollover_leaves_the_day_it_started_in_owed` |
| `S07113-OWED-SETTLE-DUE` | `crates/coordination/src/sync_cycle.rs` | a cycle that runs the fold collects the settle a closed day is owed as an obligation (added at delivery, §10, Continuation) | `settle_fold::cycle::the_scheduled_sync_after_a_sync_across_the_rollover_settles_the_owed_day` |

## 10. Amendments at delivery

- **The score reads are one coordination use case** (`crates/coordination/src/score.rs`). The API's
  routes and the bot's `/score` both call `day_score`, and the days route calls `day_rollups`, so the
  two surfaces cannot show two scores for one study day (A23, A24). Chosen against each surface
  reading analytics' repository itself: `api` and `bot` depend on coordination and never on
  analytics (docs/CONTEXT-MAP.md), and two reads could drift. The use case says what is absent: a
  card state no recompute recorded, and, on a day with no answered review, both the day's true
  retention and its retention pillar (R10, R22). Chosen against rendering the pillar's stored 0,
  which would claim a measurement nobody made.
- **The routes (R20).** `GET /api/analytics/days?from=<ISO date>&to=<ISO date>` answers the rollups
  of the days that have one, oldest first, and refuses a range that ends before it starts
  (`range_invalid`) or spans more than the window's 400 study days (`range_too_long`) with 400.
  `GET /api/score` answers `{"study_day": ..., "score": ...}`, whose `score` is null while no
  recompute rolled the day up. The session is checked before anything else, so a request without
  the owner's live session learns nothing, not even whether the database is open (503
  `database_not_open` answers the owner alone). The owner's study day comes from
  `OwnerAccess::study_day`, which `GET /api/me` now reads too.
- **`/score` (R21).** The bot's handlers take the study-day rule and a clock, so `/score` names the
  current study day as `GET /api/score` does. It answers the total, the grade, the reviews and the
  true retention to one decimal, or says the retention is absent; a day with no rollup yet is said to
  have no score, naming `/sync`. `/score` joins the owner's menu and the command lines `/start` and
  the help answer list, so their goldens change, and each of its four messages has a golden.
- **The score screen (R22).** `/score` joins the route table and the startapp token map (token
  `score`), which `a11y-coverage.test.ts` and `startapp.test.ts` hold equal to the screens, and the
  audit answers `/api/score` with a day whose retention is absent, so both colour schemes audit the
  breakdown. The client reads the score through the one session (`api.ts`), sharing one read with
  `GET /api/me`. Each pillar shows its name, its value as text and a `<meter>` named by the text; an
  absent retention shows its name and a sentence, and no value or bar.
- **R15: the fold runs after a sync.** `CycleParts::with_fold` gives the cycle a fold shared by every
  cycle of a role, and the recompute runs it after the window read with the study day of the latest
  successful sync on record (`SyncRunStore::last_success`), then writes the anchor; a fold that fails
  is `CycleError::Recompute`, recorded as `recompute_failed`, and leaves the anchor as it was, so the
  next cycle recomputes. No test of A16 to A21 runs a sync cycle, so the cycle's own test,
  `settle_fold::cycle::a_sync_cycle_runs_the_fold_and_settles_only_after_a_successful_sync`, is
  recorded under A18 in `docs/red-first/SPEC-071.md`: a cycle whose sync fails evaluates the current
  day and settles nothing, and the next one, whose sync succeeds, settles the day that closed.
- **The courses load where a cycle runs.** The daemon's `RecomputeSetup` loads the courses (R1),
  checks them against the readings taxonomy (R3), records their digest (R4) and builds the fold, at
  the `bot` role's start and when the `sync` job runs, and hands the reader its courses. Chosen
  against loading them at every job's start, because the other jobs start without the sync's
  settings (SPEC-023 R12), and a malformed courses file would then refuse the maintenance jobs too.
- **How a step registers (R19).** A later SPEC implements `DayStep` in its own file under
  `crates/coordination/src/recompute/` and registers it in `crates/daemon/src/wiring.rs`'s
  `recompute_fold` with `Fold::register(phase, step)`; the fold refuses a step registered outside its
  phase (A20), and no later SPEC edits the fold. Chosen against a registry the fold discovers, which
  would hide the order of registration from the composition root that owns it.
- **Two privacy categories.** `daily_rollup` is the category `daily-rollups` and `daily_lang_stats`
  the category `daily-course-stats`, each with its line in `PRIVACY.md`; §3a B1's one category over
  both tables is refused by the data-rights symmetry census, whose planted check removes the first
  table's category and expects exactly one refusal. Chosen against one category, which that census
  cannot judge table by table.
- **No CHECK ties `score_at_close` to `settled_at`.** The closing day's step records the score it
  closed with before the fold records the settle, in the same write, and SQLite checks a CHECK at
  each statement, not at the commit. Chosen against recording the settle before the steps, because
  the cursor moves only once every step of the day ran (R16).
- **The first recompute's backfill (R17).** It rolls up, in the historical form, the past study days
  of the window before the most recently closed day; that day is then settled with its end-of-day
  state, the current day is evaluated, and a revisit re-rolls every past day whose fingerprint changed
  and re-scores every past study day of the window (R14, R18).
- **Two parity traps.** The predecessor's `sum` of floats is CPython's compensated summation
  (Neumaier's, since 3.12), so the seconds are summed by `metrics.rs`'s port of it, never by a
  running sum, which the bit-exact goldens reject in the last place; and serde_json's default float
  parser can land one unit in the last place away, so analytics' tests parse the goldens with its
  `float_roundtrip` feature.
- **The manifest.** Beside the files above, the pinned reset row of `crates/kernel/tests/data_rights.rs`
  names `courses_digest: null`, a card literal of `crates/readings/tests/support/mod.rs` names
  `course: None`, and the API's and the bot's tests write synthetic rollup rows through the kernel's
  `Db`, which hands out sqlx types, so both crates take `sqlx` as a dev-dependency: no new crate and
  no new edge of the context map.
- **Commit b822d12 carries code under a `docs(analytics)` subject.** Beside the amendments, the
  red-first lines and the changelog fragment, it refactors the score body's reader
  (`web/app/src/lib/score/score.ts`: the `isRecord` guard becomes destructuring over `?? {}`) and
  changes two tests: `ScoreBreakdown.test.ts` casts the meter it reads to `HTMLMeterElement`, a type
  alone, and `score.test.ts` gains the case of a study day in an array. The history is kept as it is.
  The refactor reads every body as before: a JSON value that is not an object answers `undefined`
  for any field, so `null` is the one value that needed the default, and the reader before it
  (2f51341) and the reader after it pass the same 24 tests of the score module, the score screen and
  the client; StrykerJS kills all 56 of its mutants.
- **The mutation jobs' survivors (SPEC-039).** The diff-scoped jobs mutate every file the delivery
  changed, whole, so the survivors of the three older client modules it touched are its own.
  `mutation-web` found 31 at 97823b2: 19 survived and 4 uncovered in `api.ts`, 7 in `startapp.ts`
  and 1 in `routes.ts`. Tests now kill every mutant of the three that a test can observe: a
  handshake and a call the network drops, a handshake refused with 403, two calls whose session
  ended together sharing one new handshake, a failed `/api/me` whose body reads as a session,
  `/api/me` bodies that are no JSON object or carry a malformed day, Today's own path, a failed
  handshake's interleaving, and the token's shape. Six mutants no test can observe are recorded in
  place as `EQUIVALENT (#294)`, each with its reason on its line: the `'open'` literal, which no
  caller compares with; `stopped` left unset after a refusal, since a refused session stays cached
  and a later call answers reopen and sends nothing either way; the `null` an unreadable body falls
  back to, which both parsers read as they read `undefined`; the `typeof` beside `body !== null`,
  since a JSON value that is no object reads no `study_day`; and the `today` entry removed or its
  key emptied, since `today` opens `TODAY` as every unlisted token does. At 8a12cf4 StrykerJS over
  the three modules kills 142 mutants, times out 1, and ignores 6, the records alone; none survives
  and none is uncovered.
- **A record hides only its own mutants.** StrykerJS disables a mutator for every mutant whose node
  starts on the line its comment names (instrumenter 10.0.0). At 47172da seven mutants the tests
  kill started where a recorded one did, since a condition's two forced values start at one node,
  as a regular expression's mutants do, and no line split can part them. Three places are
  restructured instead, each answering as before, and the seven are killed again:
  - a refusal stops the client in the call that met it (`get`), not in a callback on the shared
    handshake, and the refused session stays cached as before; the refusal's condition now decides
    the answer, so both its forced values are killed. The tests of a handshake refused with 403, a
    page opened outside Telegram and a call the new session refuses too pin it.
  - a call that got no answer reads `null`, not the string `'unavailable'`, so the read's refusal,
    `response === null || !response.ok`, holds no part that another implies. The tests of a
    handshake and a call the network drops, a server that fails and a failed `/api/me` whose body
    reads as a session pin it.
  - the token's shape is `isToken`, and `routeFor` reads the table through a conditional on it.
    The table is read by exact key, so no screen shows whether the shape holds; `isToken`'s own
    test pins its anchors, its characters and its length, beside the tests of unknown and
    malformed tokens.

  The mutant that forces the failed handshake's `session === used` true is killed by a test that
  sweeps the offsets of the microtask queue after a dropped handshake: at one offset, a call that
  joined the failing attempt resumes only after another call has opened a newer session, and the
  mutant then forgets that session, so a later call sends a handshake of its own. Against the mutant
  the test fails at that offset, the later call sending `POST /api/session` before its
  `GET /api/me`; it refuses a sweep that never reaches the interleaving.
- **`mutation-rust`** missed 51 mutants at 97823b2, and timed out 3, which it counts as examined.
  cargo-mutants runs only the mutated package's tests, and several functions were tested only
  through their callers in other packages, so 49 of them are now killed by a test in their own
  package: analytics' store reads and writes, fingerprint, conversions and leech setting, and
  its Python ports' ties and overflow (unit tests in `metrics.rs` and `score.rs`); coordination's
  score reads (`tests/score_reads.rs`) and the fold's evaluations, step order, `Debug` and
  `Display`, a clock stepped back, its error log and the cycle's fold; the courses file's
  boundaries and a course code's formatting; and the API's refusals. `seconds_of` caps each answer
  with the crate's one port of Python's `min`, `score.rs`'s `py_min`, which it matched line for line,
  so its tie is proved by `py_min`'s own test rather than standing as an equivalent. The last one
  is a mutant no test can observe: in `card_snapshot`, the test `card.interval < MATURE_IVL_DAYS`
  sits inside the branch that only a card younger than `MATURE_IVL_DAYS` reaches, so `<=` there
  admits no other card. It is recorded in the form SPEC-039 R5 holds in force: one `exclude_re`
  entry in `.cargo/mutants.toml`, with `EQUIVALENT: <reason> (#295)` on the line above, anchored to
  the whole name cargo-mutants' listing prints,
  `crates/analytics/src/snapshot.rs:75:58: replace < with <= in card_snapshot`. The listing no
  longer names it and still names the line's six other mutants. SPEC-057 (#277) replaces this
  form, and its delivery converts the entry. At 47172da the job read examined 465 (caught 461,
  missed 1, timeout 3), unviable 101, of 566; the missed one is this mutant.
- **Fix round 1: six defects no test observed.** The first review planted six defects that every
  test passed, on code that is right in all six places, so the round adds tests and no production
  code. Each test is green on the real code and red under its plant (`docs/red-first/SPEC-071.md`,
  "Fix round 1"), and each is recorded under the criterion it serves:
  - R9, under A6: `settle_fold::a_late_review_rerolls_a_settled_day_and_keeps_what_it_closed_with`.
    A late review re-rolls a settled day after its cards moved, and the day keeps the card state,
    the provenance and the score it closed with; a backfilled day re-rolled the same way keeps no
    card state. A6's own test holds analytics' store to this, and this one holds the fold's steps.
  - R16, under A16: `settle_fold::a_settle_whose_cursor_is_refused_commits_none_of_its_steps_work`.
    A trigger the test installs refuses the cursor's write, the one statement that names
    `settled_at`. The settle then commits none of its steps' work, and the recompute that settles
    the day at last commits it once. A failure inside a step could not tell the settle's one write
    from two.
  - R15, under A18: `settle_fold::cycle::a_sync_across_the_rollover_leaves_the_day_it_started_in_owed`.
    A successful sync that started before the rollover and finished after it leaves the day that
    closed owed, and the owner's next sync, which starts after the close, settles it.
  - R18, under A21: `settle_fold::a_change_to_any_review_field_or_to_the_courses_rerolls_the_day`.
    A change to any one of a review's eight fields, or to the courses file's digest, changes the
    day's fingerprint, and the next recompute rolls the day up again. cargo-mutants never removes
    one element of the list the fingerprint digests, so each field has its own row in the table.
  - R1, in A13's own test: every malformed file's refusal names the setting and quotes none of the
    file's values, as a contradiction's refusal already did. The malformed files grow from 11 to 19,
    so that every place the reader refuses a file's shape is reached by one of them. The one value a
    refusal may name is the schema's published name, `COURSES_SCHEMA`, as the shape it expects.
    A13's text in §3 grows to say so, because its test now observes it; R1 is unchanged.
  - R4, under A14: `settle_fold::cycle::cycles_with_an_unchanged_courses_file_leave_the_settings_generation_alone`.
    Two runs of the sync job with the courses file unchanged each record the digest at their start
    and run their cycle, and the settings generation stays where the first start put it.

  Two of the six hold a settlement invariant that cargo-mutants cannot produce, and they take rows
  (§9): S07111 holds R16's one write, and S07112 holds the start day R15 reads. The other four take
  none. R9's and R4's defects each add a call, which no constant, string, attribute or `new` method
  holds, and their tests hold them. A row for R18 would need its killer in analytics' own package,
  where no fold test runs. R1's defect leaks a value into a `&'static str` through `Box::leak`,
  which is contrived, and A13 itself now kills it.
- **Continuation: a day left owed by a sync across the rollover was settled late.** After the
  owner's sync that started before the rollover and finished after it, the anchor is written after
  the close. The next scheduled sync starts after the close, but nothing in the collection changed
  since that anchor, so the change gate skipped the recompute and the closed day stayed owed until
  some later recompute settled it with a later card state. R15 asks the recompute after every
  successful sync, scheduled or the owner's, to settle each closed day, so the code was short of R15
  and R15 stays as written. A cycle given the fold now registers an obligation, `owed_settle`
  (`crates/coordination/src/sync_cycle.rs`), whose one deadline is the start of the current study
  day's first successful sync (`SqliteSyncRuns::first_success_in`). While no recompute has run
  since that start, the gate runs one for it (`DeadlineDue`), and its fold settles the day that
  closed; once one has, the deadline has passed and the gate is back to its other terms. The tests,
  both recorded under A18 in `docs/red-first/SPEC-071.md`, "Continuation":
  - `settle_fold::cycle::the_scheduled_sync_after_a_sync_across_the_rollover_settles_the_owed_day`:
    after a sync across the rollover, the scheduled sync that starts after the close, with no
    rescore marked and nothing changed, runs one recompute for `owed_settle`, which settles the
    closed day with the card state at that sync; the next scheduled run that day is refused and
    the day is settled once. It is red at 83cceb7 and green at 2d76848.
  - `settle_fold::cycle::the_scheduled_sync_after_a_settle_still_skips_the_recompute`: after the
    owner's sync that started after the close settled the day, the day's scheduled sync, with
    nothing changed, costs no recompute, and the settled day is as it was.

  The S3 test of fix round 1 stays: its second cycle is the owner's sync, which marks a rescore, and
  it is S07112's killer; the new test adds the scheduled path. `first_success_in` has its own test in
  ingest's package, `gate::a_study_days_first_success_is_its_earliest_successful_start`. Row S07113
  (§9) holds the registration, which no function-level mutant removes. Chosen against a deadline
  per owed day, the start of the first successful sync after that day's close: it would read the
  fold's cursor as well as the sync record, and the one deadline already comes due at the cycle the
  first test observes. Chosen against the latest successful sync on record: it comes due again
  after every successful sync, so the day's scheduled sync after the owner's settle would recompute
  with nothing owed; planted in its place, the second test goes red.
