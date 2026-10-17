# SPEC-101: the digest and the weekly report tell what closed, and the debrief asks how it felt

- **Wave:** W5. **Issues:** #129 (the daily digest), #131 (the session debrief), #130 (the weekly
  report) and the lapse digest of #123, in epic #6. **Context(s):** `deck-streak-notifications` (the
  digest's, the weekly report's and the debrief's texts, the kind `weekly`, the debrief's rule and
  `debrief_ratings`); `deck-streak-coordination` (the two jobs, the inputs they gather and the
  schedule that reads the digest hour); `deck-streak-bot` (the `de:` and `ha:deal` callbacks);
  `deck-streak-daemon` (the wiring); the Mini App (`web/app`, the holdout card and the startapp
  tokens `progress` and `insights`).
- **Decided by:** ADR-108 (this SPEC's: the weekly report names this week's stored instrument
  reports and links to them, splicing none), ADR-041 (the router core), ADR-085 (charts are drawn on
  the client), ADR-094 (coordination stores each instrument's latest report), ADR-052 (deep links
  are URL buttons), ADR-011 and ADR-027 (the job minutes), ADR-053 (the private rail's reserved
  slots) and ADR-012 (the parity oracle).
- **Prerequisites:** SPEC-041 (the router), SPEC-100 (the button rows, the recent rollups' read, the
  holdout's settle and readout and its route), SPEC-071 (the stored rollup), SPEC-072 (the level and
  a study day's XP), SPEC-076 (the streak state and the governor's lapse), SPEC-077 (the law block's
  view and each course's progress), SPEC-078 (the habit summary), SPEC-079 (the focus progress),
  SPEC-094 (the stored instrument reports and the insights screen), SPEC-098 (`/hand`), SPEC-105
  (the window occurrences), SPEC-107 (the Oracle's block and weekly line), SPEC-020 (the digest
  hour), SPEC-027 (the job table), SPEC-021 (the six files of a table) and SPEC-029 (the goldens).
  **Mutation band:** `S10100-S10199`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-101.md` (ADR-016).

## 1. The problem, measured

- **What exists.** The kernel resolves the digest hour: an explicit one before the rollover hour is
  refused, and an unset one is the larger of 9 and the rollover hour (SPEC-020,
  `crates/kernel/src/settings.rs`). The policy declares the kind `digest` behind `digest_enabled`.
  The job table holds the sync, the upkeep and the liveness watch, and its schedules are a daily
  minute after the rollover hour, a daily local time and an hourly minute; none reads the digest
  hour, and none is weekly. SPEC-071, SPEC-076, SPEC-077, SPEC-078, SPEC-079 and SPEC-107 each leave
  their digest and weekly lines to #129 and #130; SPEC-100 leaves the holdout's weekly settle and
  readout to #130; the W4 instruments store their reports and leave the weekly report to #130
  (SPEC-094). No digest, weekly report or debrief exists.
- **What is ported** (at `27ee2bc`):
  - the digest, `pipeline_layers/digests.py:DigestsLayer.run_daily_digest`,
    `pipeline_layers/base.py:PipelineBase._render_daily_from_rollup`,
    `telegram.py:render_daily_digest`, `motivational_line`, `render_law_block`,
    `render_habits_block`, `render_focus_block` and `progress.py:render_progress_block`;
  - the weekly report, `DigestsLayer.run_weekly_report` and `telegram.py:render_weekly`;
  - the debrief, `bot.py:CommandBot._callback_debrief`, `database.py:GamifyStore`'s
    `record_session_debrief` and `session_debrief_readout_line`.
- **Traps a hand port falls into.**
  - The digest is a hybrid: the body, the Oracle block, the windows line and the debrief row read
    the just-closed study day, while the law block, Road to C2, the habits, the focus, the streak
    and the level read the current study day. The golden holds the hybrid as it is.
  - The stored streak carries no transient flag, so the body's freeze and reset lines never render
    from it; the body passes no badge, so no badge block renders in the digest or the weekly report.
  - A day whose card state was never recorded renders no card line and no queue line, never zeros;
    the learning count is not stored, so a recorded day's card line reads 0 learning.
  - The weekly report reads the 8 most recent rollup rows, newest first, under a label of the 7 days
    before the current study day; its average retention is over the rows with reviews only, its
    average score over every row, both rounded to one decimal.
  - The debrief accepts a tap on its day and up to 3 days after it; a fourth day's tap, a future
    day's and a malformed one are refused and write nothing. A second tap on the same day replaces
    the first.
  - The readout names a count and the rated days still needed until 42 exist, and never a
    percentage.
- **Deviations from the predecessor, each with its reason.**
  - The digest and the weekly report go through the one router (SPEC-041); a failed push releases
    the claim, so neither latches a day it did not deliver, and a later run can send it.
  - The weekly report names this week's stored instrument reports in one line and links to the
    insights screen, and splices no instrument's text (ADR-108). Its Road to C2 chart is a button to
    the progress screen, whose chart the client draws (ADR-085).
  - The digest carries no coaching block, which is the agent's duty (#53), and no offsite-backup
    line, since a failed backup already pages through the one alert path (#44).
  - #129's 09:05 is the digest hour's minute 6, and #130's 09:10 the digest hour's minute 11 on
    Sundays: the predecessor's own minutes stay its own while both run (ADR-011).

## 2. Requirements

The texts (#129, #130, #131)

R1. `crates/notifications/src/digest.rs` renders the digest's body from notifications' own input: a
    study day, its metrics, its card state or none, its score, the day's XP, the level and the
    streak's heat, current, longest and freezes. The body equals the golden `daily_digest_body`
    (`PipelineBase._render_daily_from_rollup` over `telegram.py:render_daily_digest`), and its
    closing line the golden `motivational_line` (`telegram.py:motivational_line`), over its outcome
    set: not studied or a score of 0 or less, at least 90, at least 60, and the rest.
R2. A day whose card state is none renders neither the card line nor the queue line.
R3. The law block equals the golden `digest_law_block` (`telegram.py:render_law_block`), over
    SPEC-077's view, and is empty when that view's shown rule says so (SPEC-077 R13).
R4. The Road to C2 block equals the golden `progress_block` (`progress.py:render_progress_block`
    with `telegram.py:bar`), over each course's progress (SPEC-077 R4).
R5. The habits block equals the golden `habits_block` (`telegram.py:render_habits_block`), over
    SPEC-078 R12's summary.
R6. The focus block equals the golden `focus_digest_block` (`telegram.py:render_focus_block`), over
    SPEC-079 R13's progress, and is empty when today's minutes, the week's minutes and the streak
    are all 0.
R7. The weekly body equals the golden `weekly_body` (`telegram.py:render_weekly`), over the week's
    label, the rows (each with its study day, score, reviews, retention, seconds and XP), the level
    and the streak. Its sums over floats are compensated as CPython's `sum` is (`python_sum`).
R8. The debrief's readout names, below 42 rated days, the count and the days still needed, and at 42
    or more the count banked; it never names a percentage. It equals the golden
    `debrief_readout_line` (`database.py:GamifyStore.session_debrief_readout_line`, over
    `SESSION_DEBRIEF_READOUT_THRESHOLD`).
R9. The research line (ADR-108) names by its id, in the registry's order, each weekly instrument
    whose stored report's study day falls in the report's week (R20's label, both ends included),
    the id of a report with a failed read followed by a space and `(incomplete)`: `🔬 <b>Research
    this week</b>: <id> · <id>`. It is empty when no report falls in that week.
R10. The constants (the debrief's accept window of 3 days, its ratings `1` smooth, `2` work and `3`
    brutal, and the readout's threshold of 42) equal the golden `digests.constants`.

The digest (#129, #123)

R11. The job `daily_digest` runs at the digest hour, minute 6, with catch-up. Its target is the
    study day before the current one. When the target has no stored rollup, it raises nothing.
R12. Otherwise it raises one occasion of kind `digest`, key `digest`. Outside a lapse its text is,
    in this order and each empty block omitted: the law block, the body, the Road to C2 block, the
    habits block, the focus block, the Oracle's block for the target (SPEC-107 R20), the windows
    line, then the debrief's readout. The windows line, `🪟 Windows: <kept> kept · <rest> quiet`,
    counts the target's judged window occurrences by verdict, `kept` against every other verdict
    (SPEC-105 R6), and is omitted when there are none.
R13. The digest carries one row: `😌 Smooth`, `💪 Work` and `🔥 Brutal`, with the callbacks
    `de:1:<day>`, `de:2:<day>` and `de:3:<day>`, where `<day>` is the target as 8 digits.
R14. In a lapse (the governor's open lapse, SPEC-076 R16) the text is the lapse line alone, naming
    streaks' relight count (`constants.RELIGHT_CARDS`, SPEC-076 R20, golden `streaks.constants`),
    and it carries the debrief's row and a second row, `🃏 Deal today's hand` with the callback
    `ha:deal`.
R15. The digest's reads are, each that context's own and a read only: analytics' stored rollup of
    the target (SPEC-071); progression's level and the target's XP (SPEC-072 R10, R13); streaks'
    language streak and the governor's lapse (SPEC-076); coordination's law block view (SPEC-077);
    curriculum's course progress (SPEC-077 R4); habits' summary (SPEC-078 R12); focus's progress
    (SPEC-079 R13); markets' Oracle block (SPEC-107 R20); discipline's judged occurrences of the
    target (SPEC-105 R6); and notifications' count of rated days.
R16. The message equals the golden `daily_digest_message` (`DigestsLayer.run_daily_digest`), and a
    second run on the same study day raises the same key, so the router's claim (SPEC-041) sends
    nothing more.

The debrief (#131)

R17. `crates/notifications/src/debrief.rs` accepts a tap of a rating in `1`, `2` or `3` and a day of
    8 digits naming a study day no later than the current one and at most 3 days before it, and
    refuses any other: its outcome set is accepted, and refused when the rating or the day is
    malformed, the age is below 0 or the age is above 3 (`bot.py:CommandBot._callback_debrief`,
    `_DEBRIEF_ACCEPT_WINDOW_DAYS`).
R18. `de:<rating>:<day>`, from the owner only, has the bot record the rating for its day in
    `debrief_ratings` and answer `Logged — thanks for the honesty.`; a refused tap writes nothing
    and answers `That debrief tap has gone stale.`; a tap from anyone else writes nothing. A second
    tap on the same day replaces the first.
R19. `ha:deal`, from the owner only, runs `/hand` (SPEC-098 R17) and answers as it does.

The weekly report (#130)

R20. The job `weekly_report` runs on Sundays at the digest hour, minute 11, without catch-up. It
    raises one occasion of kind `weekly`, key `weekly`, for the report's week, from the current
    study day minus 7 to the current study day, labelled `<first> – <last>` with both days.
R21. Its text is, in this order and each empty block omitted: the law block, the weekly body over
    the 8 most recent stored rollups up to the current study day, newest first, the Road to C2
    block, the habits block, the focus block, the Oracle's weekly line (SPEC-107 R20), the holdout's
    readout (SPEC-100 R23), and the research line (R9). It carries one row: `📈 Road to C2` with the
    token `progress`, and `🔬 Research` with the token `insights` when the research line shows.
R22. It runs the holdout's settle (SPEC-100 R22) before it reads the readout.
R23. Each block's read runs on its own: a read that fails omits its block, is logged by the block's
    name, and never stops the report.
R24. The weekly report's reads are, each that context's own and a read only: analytics' 8 recent
    rollups (`RollupStore::recent_before` at the day after the current study day and a count of 8,
    SPEC-100 R9); progression's level and each row's XP (SPEC-072); streaks' language streak
    (SPEC-076); the law block view, the course progress, the habit summary and the focus progress
    (as R15); markets' weekly line (SPEC-107 R20); the holdout's settle and readout (SPEC-100 R22,
    R23); and coordination's stored instrument reports (SPEC-094 R9).
R25. The message equals the golden `weekly_report_message` (`DigestsLayer.run_weekly_report`, with
    each instrument's block empty). A failed push releases the claim, so the week stays unsent and a
    second run on the same study day sends it.

The kinds, the table and the jobs

R26. The policy gains the kind `weekly` (class `digest`, tiers `["T2"]`, budget null,
    `per-study-day`, setting `weekly_enabled`), with its deviation (ADR-108).
R27. Notifications owns `debrief_ratings` (study day as primary key, rating `smooth`, `work` or
    `brutal`, `created_at`, `updated_at`), `STRICT`, created by
    `migrations/010101_notifications_debrief_ratings.sql`. It owes SPEC-021's six files;
    notifications' data-rights port exports and erases it.
R28. `migrations/010102_notifications_digests_side_by_side_defaults.sql` seeds `digest_enabled` and
    `weekly_enabled` as `"0"` with `INSERT OR IGNORE` (ADR-011), so nothing speaks twice while the
    predecessor runs.
R29. `Schedule` gains `DailyAtDigestHour { minute }` and `WeeklyAtDigestHour { weekday, minute }`,
    each at the kernel's digest hour, as `DailyAtRollover` reads the rollover hour. The two jobs
    join the job table, each with its timer at the default digest hour and its calendar key in the
    rail contract; none shares a minute with the predecessor's schedule, a reserved minute (0, 25,
    39), the private rail's reserved slots or the sync's slot (SPEC-027 R2, SPEC-053 R2).

The Mini App

R30. The startapp map gains `progress`, opening `/progress` (SPEC-077), and `insights`, opening
    `/insights` (SPEC-094 R13).
R31. The insights screen gains the holdout card over `GET /api/nudges/holdout` (SPEC-100 R25): each
    kind with both arms, sorted, with each arm's n and studied rate and `low n` when the smaller arm
    is under 10, and no card when no kind has both arms.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the body equals the golden `daily_digest_body`: a recorded and a never-recorded card state, scores 0, 59, 60, 89, 90 and 100, a day with no review, and a retention that rounds at one decimal | `the_digest_body_matches_the_parity_golden` |
| A2 | the closing line equals the golden `motivational_line` at scores 0, 1, 59, 60, 89 and 90, studied and not | `the_motivational_line_matches_the_parity_golden` |
| A3 | a day never recorded renders no card line and no queue line, and no zero stands in for them | `a_day_never_recorded_renders_no_card_or_queue_line` |
| A4 | the law block equals the golden `digest_law_block`, shown and omitted | `the_law_block_matches_the_parity_golden` |
| A5 | the Road to C2 block equals the golden `progress_block` | `the_progress_block_matches_the_parity_golden` |
| A6 | the habits block equals the golden `habits_block` | `the_habits_block_matches_the_parity_golden` |
| A7 | the focus block equals the golden `focus_digest_block`, empty at all zeros | `the_focus_digest_block_matches_the_parity_golden` |
| A8 | the weekly body equals the golden `weekly_body`, with a row of no reviews and 8 rows | `the_weekly_body_matches_the_parity_golden` |
| A9 | the readout equals the golden `debrief_readout_line` at 0, 41, 42 and 43 rated days | `the_debrief_readout_matches_the_parity_golden` |
| A10 | the window, the ratings and the threshold equal the golden `digests.constants` | `the_digest_constants_equal_the_predecessors` |
| A11 | the digest's message equals the golden `daily_digest_message`: every block present and each empty, the windows line with and without occurrences, and the lapse path | `the_digest_message_matches_the_parity_golden` |
| A12 | the digest's sections keep the order of R12, and its row carries the three callbacks for the target | `the_sections_keep_the_predecessors_order` |
| A13 | a block with nothing to say is omitted, never rendered as zero | `a_block_with_nothing_to_say_is_omitted_never_zero` |
| A14 | the weekly message equals the golden `weekly_report_message`, every block present and each empty | `the_weekly_message_matches_the_parity_golden` |
| A15 | the research line names this week's reports in the registry's order, marks an incomplete one, names a report on the week's first day and skips one the day before it, and is empty with none | `the_research_line_names_this_weeks_reports` |
| A16 | a rating other than `1`, `2` or `3`, and a day that is not 8 digits, are refused | `only_the_three_ratings_are_accepted` |
| A17 | a tap is accepted on its day and 3 days after, and refused a day ahead and 4 days after | `a_tap_is_accepted_up_to_three_days_back_and_refused_after` |
| A18 | a second tap on the same day replaces the first, and the table holds one row for the day | `a_second_tap_on_the_same_day_replaces_the_first` |
| A19 | `debrief_ratings` is exported and erased by notifications' port | `the_debrief_ratings_are_exported_and_erased` |
| A20 | the kind `weekly` is declared with its class, tier, dedupe and setting, a recorded deviation | `the_weekly_kind_is_a_recorded_deviation` |
| A21 | `digest_enabled` and `weekly_enabled` are seeded off, and an existing value is kept | `the_digest_switches_start_off_beside_the_predecessor` |
| A22 | the digest reports the study day before the current one, and raises nothing when that day has no stored rollup | `the_digest_reports_the_just_closed_day_and_needs_its_rollup` |
| A23 | a second run on the same study day raises the same key, and the transport receives one push | `a_second_digest_run_on_a_study_day_sends_nothing` |
| A24 | in a lapse the digest is the lapse line with the debrief's row and the hand's row | `in_a_lapse_the_digest_is_the_lapse_line_with_both_rows` |
| A25 | the digest reads each of R15's ten inputs, and a stub that drops one changes the message | `the_digest_reads_every_input_it_names` |
| A26 | a failed weekly push leaves the week unsent, and a second run on the same study day sends it | `a_failed_weekly_push_leaves_the_week_unsent_and_a_rerun_sends_it` |
| A27 | a block whose read fails is omitted and logged by name, and the rest of the report is sent | `a_failing_block_is_omitted_and_never_stops_the_report` |
| A28 | the weekly settles the holdout before it reads the readout | `the_weekly_settles_the_holdout_before_its_readout` |
| A29 | the weekly reads each of R24's inputs, asks for 8 rollups at the day after the current study day, and a stub that drops one changes the message | `the_weekly_reads_every_input_it_names` |
| A30 | the two jobs fire at the digest hour, the weekly on Sundays, keep off every predecessor, reserved and private-rail minute and the sync's slot, and their timers and the rail contract hold the calendars | `the_digest_jobs_keep_off_every_reserved_minute` |
| A31 | the digest catches up and the weekly does not | `the_digest_catches_up_and_the_weekly_does_not` |
| A32 | an owner's tap records the rating for its day and answers `Logged — thanks for the honesty.` | `an_owner_tap_records_the_rating_for_its_day` |
| A33 | a stale or malformed tap writes nothing and answers `That debrief tap has gone stale.` | `a_stale_or_malformed_tap_writes_nothing` |
| A34 | a tap from anyone but the owner writes nothing | `a_tap_from_anyone_but_the_owner_writes_nothing` |
| A35 | `ha:deal` from the owner answers as `/hand` does | `deal_todays_hand_answers_as_the_hand_command` |
| A36 | the holdout card shows only kinds with both arms, marks a low n, and shows nothing without one | `shows only kinds with both arms` |
| A37 | the tokens `progress` and `insights` open their screens | `opens the progress and insights screens from their tokens` |

```acceptance
A1: cargo test -p deck-streak-notifications --test digest_texts -- --exact the_digest_body_matches_the_parity_golden
A2: cargo test -p deck-streak-notifications --test digest_texts -- --exact the_motivational_line_matches_the_parity_golden
A3: cargo test -p deck-streak-notifications --test digest_texts -- --exact a_day_never_recorded_renders_no_card_or_queue_line
A4: cargo test -p deck-streak-notifications --test digest_texts -- --exact the_law_block_matches_the_parity_golden
A5: cargo test -p deck-streak-notifications --test digest_texts -- --exact the_progress_block_matches_the_parity_golden
A6: cargo test -p deck-streak-notifications --test digest_texts -- --exact the_habits_block_matches_the_parity_golden
A7: cargo test -p deck-streak-notifications --test digest_texts -- --exact the_focus_digest_block_matches_the_parity_golden
A8: cargo test -p deck-streak-notifications --test digest_texts -- --exact the_weekly_body_matches_the_parity_golden
A9: cargo test -p deck-streak-notifications --test digest_texts -- --exact the_debrief_readout_matches_the_parity_golden
A10: cargo test -p deck-streak-notifications --test digest_texts -- --exact the_digest_constants_equal_the_predecessors
A11: cargo test -p deck-streak-notifications --test digest_compose -- --exact the_digest_message_matches_the_parity_golden
A12: cargo test -p deck-streak-notifications --test digest_compose -- --exact the_sections_keep_the_predecessors_order
A13: cargo test -p deck-streak-notifications --test digest_compose -- --exact a_block_with_nothing_to_say_is_omitted_never_zero
A14: cargo test -p deck-streak-notifications --test digest_compose -- --exact the_weekly_message_matches_the_parity_golden
A15: cargo test -p deck-streak-notifications --test digest_compose -- --exact the_research_line_names_this_weeks_reports
A16: cargo test -p deck-streak-notifications --test debrief -- --exact only_the_three_ratings_are_accepted
A17: cargo test -p deck-streak-notifications --test debrief -- --exact a_tap_is_accepted_up_to_three_days_back_and_refused_after
A18: cargo test -p deck-streak-notifications --test debrief -- --exact a_second_tap_on_the_same_day_replaces_the_first
A19: cargo test -p deck-streak-notifications --test debrief -- --exact the_debrief_ratings_are_exported_and_erased
A20: cargo test -p deck-streak-notifications --test digest_kinds -- --exact the_weekly_kind_is_a_recorded_deviation
A21: cargo test -p deck-streak-notifications --test digest_kinds -- --exact the_digest_switches_start_off_beside_the_predecessor
A22: cargo test -p deck-streak-coordination --test digest_job -- --exact the_digest_reports_the_just_closed_day_and_needs_its_rollup
A23: cargo test -p deck-streak-coordination --test digest_job -- --exact a_second_digest_run_on_a_study_day_sends_nothing
A24: cargo test -p deck-streak-coordination --test digest_job -- --exact in_a_lapse_the_digest_is_the_lapse_line_with_both_rows
A25: cargo test -p deck-streak-coordination --test digest_job -- --exact the_digest_reads_every_input_it_names
A26: cargo test -p deck-streak-coordination --test weekly_job -- --exact a_failed_weekly_push_leaves_the_week_unsent_and_a_rerun_sends_it
A27: cargo test -p deck-streak-coordination --test weekly_job -- --exact a_failing_block_is_omitted_and_never_stops_the_report
A28: cargo test -p deck-streak-coordination --test weekly_job -- --exact the_weekly_settles_the_holdout_before_its_readout
A29: cargo test -p deck-streak-coordination --test weekly_job -- --exact the_weekly_reads_every_input_it_names
A30: cargo test -p deck-streak-coordination --test digest_jobs -- --exact the_digest_jobs_keep_off_every_reserved_minute
A31: cargo test -p deck-streak-coordination --test digest_jobs -- --exact the_digest_catches_up_and_the_weekly_does_not
A32: cargo test -p deck-streak-bot --test debrief_callbacks -- --exact an_owner_tap_records_the_rating_for_its_day
A33: cargo test -p deck-streak-bot --test debrief_callbacks -- --exact a_stale_or_malformed_tap_writes_nothing
A34: cargo test -p deck-streak-bot --test debrief_callbacks -- --exact a_tap_from_anyone_but_the_owner_writes_nothing
A35: cargo test -p deck-streak-bot --test debrief_callbacks -- --exact deal_todays_hand_answers_as_the_hand_command
A36: pnpm exec vitest run web/app/src/lib/insights/HoldoutCard.test.ts -t "shows only kinds with both arms"
A37: pnpm exec vitest run web/app/src/lib/startapp-reports.test.ts -t "opens the progress and insights screens from their tokens"
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr, notifications-policy and
telegram-platform packs stay enforced, and no row is deferred or lifted for this delivery, so the
private wiring does not change when it merges. The nudge-duties pack stays pending until #53 decides
it, and the message metadata stays deferred (#257): this delivery claims neither.

| id | criterion | decided by |
|---|---|---|
| B1 | `debrief_ratings` is declared under notifications' categories with data, purpose, basis, retention and erase mode, over `privacy.json`, `migrations/010101_notifications_debrief_ratings.sql` and `crates/notifications/src/data_rights.rs` | the privacy-gdpr pack |
| B2 | the kind `weekly` carries its deviation, over `notifications-policy.json` | the notifications-policy pack |
| B3 | the digest and the weekly report go through the one router, over the three files under `crates/coordination/src/digests/` | the notifications-policy pack |
| B4 | every `de:` and `ha:deal` button's data fits the platform's limit, every callback is answered, and every deep link is a URL button, over `crates/notifications/src/digest.rs` and `crates/bot/src/debrief_commands.rs` | the telegram-platform pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/notifications/src/digest.rs` | `deck-streak-notifications` | added: the body, the closing line, the law, Road to C2, habits and focus blocks, the weekly body, the research line, the lapse line, the windows line, the rows and the order |
| `crates/notifications/src/debrief.rs` | `deck-streak-notifications` | added: the accept rule, the ratings, the readout and the table's store |
| `crates/notifications/src/lib.rs` | `deck-streak-notifications` | changed: the two modules |
| `crates/notifications/src/policy.rs` | `deck-streak-notifications` | changed: the kind `weekly` |
| `crates/notifications/src/data_rights.rs` | `deck-streak-notifications` | changed: `debrief_ratings`, exported and erased |
| `crates/notifications/tests/digest_texts.rs` | `deck-streak-notifications` | added: A1 to A10 |
| `crates/notifications/tests/digest_compose.rs` | `deck-streak-notifications` | added: A11 to A15 |
| `crates/notifications/tests/debrief.rs` | `deck-streak-notifications` | added: A16 to A19 |
| `crates/notifications/tests/digest_kinds.rs` | `deck-streak-notifications` | added: A20, A21 |
| `crates/coordination/src/digests/mod.rs` | `deck-streak-coordination` | added: the two use cases |
| `crates/coordination/src/digests/daily.rs` | `deck-streak-coordination` | added: the digest's reads, the lapse path and the occasion |
| `crates/coordination/src/digests/weekly.rs` | `deck-streak-coordination` | added: the weekly's reads, each block's isolation, the settle and the occasion |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the digests module |
| `crates/coordination/src/jobs.rs` | `deck-streak-coordination` | changed: the two schedules and the jobs `daily_digest` and `weekly_report`, which `job_table.rs`'s test, as SPEC-100 changes it, holds equal to their timers on the sending template |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: `debrief_ratings` under notifications' port |
| `crates/coordination/tests/digest_job.rs` | `deck-streak-coordination` | added: A22 to A25 |
| `crates/coordination/tests/weekly_job.rs` | `deck-streak-coordination` | added: A26 to A29 |
| `crates/coordination/tests/digest_jobs.rs` | `deck-streak-coordination` | added: A30, A31 |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row for `debrief_ratings` |
| `crates/bot/src/debrief_commands.rs` | `deck-streak-bot` | added: the `de:` and `ha:deal` callbacks and their answers |
| `crates/bot/src/lib.rs` | `deck-streak-bot` | changed: the module and the callbacks' routing |
| `crates/bot/tests/debrief_callbacks.rs` | `deck-streak-bot` | added: A32 to A35 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the two jobs and their reads |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: the new ports named |
| `web/app/src/lib/insights/HoldoutCard.svelte` | miniapp | added: the holdout card |
| `web/app/src/lib/insights/HoldoutCard.test.ts` | miniapp | added: A36 |
| `web/app/src/lib/startapp.ts` | miniapp | changed: the tokens `progress` and `insights` |
| `web/app/src/lib/startapp-reports.test.ts` | miniapp | added: A37 |
| `migrations/010101_notifications_debrief_ratings.sql` | `deck-streak-notifications` | added |
| `migrations/010102_notifications_digests_side_by_side_defaults.sql` | `deck-streak-notifications` | added |
| `notifications-policy.json` | repo | changed: the kind `weekly` and its deviation |
| `deploy/systemd/deck-streak-job-send@daily_digest.timer` | deploy | added: on the sending template (SPEC-100 R28) |
| `deploy/systemd/deck-streak-job-send@weekly_report.timer` | deploy | added: on the sending template (SPEC-100 R28) |
| `deploy/rail-contract.json` | deploy | changed: the two calendar keys |
| `tools/parity-oracle/registry/spec_101.py` | tools | added: the adapters |
| `tools/parity-oracle/goldens/` | tools | added: the goldens of section 7 |
| `scripts/mutation-rows.d/S10100-S10199.json` | scripts | added: the rows of section 9 |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `debrief_ratings` |
| `privacy.json` | repo | changed: the table's category |
| `PRIVACY.md` | repo | changed: one line for the category |
| `docs/specs/SPEC-101-the-digest-and-the-weekly-report-tell-what-closed-and-the-debrief-asks-how-it-felt.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-108-the-weekly-report-names-this-weeks-stored-instrument-reports-and-splices-none.md` | docs | changed: status accepted |
| `docs/red-first/SPEC-101.md` | docs | added |

## 5. What this does NOT do

- It writes no coaching block: the goal, the weak spots and the time to the next band are the
  agent's duty (#53).
- It writes no offsite-backup line: a failed backup already pages through the one alert path (#44).
- It splices no instrument's text into the weekly report and runs no instrument: the instruments
  store their reports, and the insights screen shows them (#137, #142).
- It sends no chart image: the weekly's Road to C2 button opens the progress screen, whose chart the
  client draws (#85).
- It builds no per-day report screen and no shortened push that links to one: both are surfaces for
  the wave that polishes them (#8).
- It analyses no debrief rating: the readout names a count until 42 days are rated, and the analysis
  is its own later item (#131).
- It flushes no held message when quiet hours end: the router's flush is (#291).

## 6. Risks

- **A day claimed that was not delivered.** A failed push releases the claim, so a later run sends
  the day or the week (A23, A26).
- **A zero standing in for an unknown.** A day never recorded renders no card or queue line, and an
  empty block is omitted (A3, A13).
- **A block that takes the report down.** Each block's read is isolated and logged by name (A27).
- **A rating written to the wrong day.** The callback carries its day, a stale or malformed tap
  writes nothing, and only the owner writes (A17, A33, A34).
- **The weekly average drifts from the predecessor's by a bit.** Its sums are compensated as
  CPython's are, and its golden holds a row with no reviews (A8).
- **Double messages beside the predecessor.** Both switches are seeded off while it runs (A21).

## 7. Parity goldens

All are registered in `tools/parity-oracle/registry/spec_101.py` and generated on the owner's
checkout of the predecessor at `27ee2bc` (SPEC-029). Every day is an epoch day number and every
name synthetic; no golden holds a calendar date or a personal value.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `goldens/daily_digest_body.json` | `pipeline_layers/base.py:PipelineBase._render_daily_from_rollup`, `telegram.py:render_daily_digest` | adapter | stub rollup rows; a card state recorded and never recorded, scores 0, 59, 60, 89, 90 and 100, 0 reviews, retentions 84.95 and 84.96, and a stored streak |
| `goldens/motivational_line.json` | `telegram.py:motivational_line` | pure | scores -1, 0, 1, 59, 60, 89, 90 and 100, studied and not |
| `goldens/digest_law_block.json` | `telegram.py:render_law_block` | pure | the view's fields all zero, each alone above zero, dues of 0 and 1, and active leeches |
| `goldens/progress_block.json` | `progress.py:render_progress_block`, `telegram.py:bar` | pure | synthetic courses at 0, a band's edge and every band achieved |
| `goldens/habits_block.json` | `telegram.py:render_habits_block` | pure | synthetic summaries: no course, a goal met and not, writing done and not, streaks of 0 and 2 |
| `goldens/focus_digest_block.json` | `telegram.py:render_focus_block` | pure | all zeros, each field alone, and goals of 0 |
| `goldens/weekly_body.json` | `telegram.py:render_weekly` | pure | 0, 1 and 8 synthetic rows, a row with no reviews, seconds that round at one decimal |
| `goldens/debrief_readout_line.json` | `database.py:GamifyStore.session_debrief_readout_line` | adapter | a temporary store holding 0, 1, 41, 42 and 43 rated days |
| `goldens/daily_digest_message.json` | `pipeline_layers/digests.py:DigestsLayer.run_daily_digest` | adapter | a recording notifier and stub blocks, the coaching and offsite reads empty; each block present and empty, window events kept and not, a lapse, no rollup and a day already sent |
| `goldens/weekly_report_message.json` | `DigestsLayer.run_weekly_report` | adapter | a recording notifier and stub blocks, every instrument's block and the chart empty; each block present and empty, and a failed send |
| `goldens/digests.constants.json` | `bot.py` and `database.py` | constants | `_DEBRIEF_ACCEPT_WINDOW_DAYS`, `_DEBRIEF_RATINGS` and `SESSION_DEBRIEF_READOUT_THRESHOLD` |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `debrief_ratings` | `notifications` | `migrations/010101_notifications_debrief_ratings.sql` (SPEC-101) | `session_debrief`, its day as an epoch day | exported and erased |

## 9. Mutation rows

The band is `S10100-S10199`, in `scripts/mutation-rows.d/S10100-S10199.json`. Each row's killer
selects exactly one test, and each is proved with its file restored byte for byte (SPEC-039). No
row's mutant makes a loop or a wait unbounded.

| row | target | what it guards | killer |
|---|---|---|---|
| `S10100-LEGENDARY-AT-90` | `crates/notifications/src/digest.rs` | a score of exactly 90 is legendary | `digest_texts::the_motivational_line_matches_the_parity_golden` |
| `S10101-STRONG-AT-60` | `crates/notifications/src/digest.rs` | a score of exactly 60 is strong | `digest_texts::the_motivational_line_matches_the_parity_golden` |
| `S10102-NO-CARD-LINE-WHEN-UNRECORDED` | `crates/notifications/src/digest.rs` | an unrecorded day has no card or queue line | `digest_texts::a_day_never_recorded_renders_no_card_or_queue_line` |
| `S10103-RETENTION-OVER-STUDIED-ROWS` | `crates/notifications/src/digest.rs` | the weekly retention averages rows with reviews only | `digest_texts::the_weekly_body_matches_the_parity_golden` |
| `S10104-FOCUS-EMPTY-AT-ZERO` | `crates/notifications/src/digest.rs` | the focus block is empty at all zeros | `digest_texts::the_focus_digest_block_matches_the_parity_golden` |
| `S10105-READOUT-AT-42` | `crates/notifications/src/debrief.rs` | exactly 42 rated days read as banked | `digest_texts::the_debrief_readout_matches_the_parity_golden` |
| `S10106-THREE-DAYS-ACCEPTED` | `crates/notifications/src/debrief.rs` | a tap 3 days back is accepted | `debrief::a_tap_is_accepted_up_to_three_days_back_and_refused_after` |
| `S10107-NO-FUTURE-TAP` | `crates/notifications/src/debrief.rs` | a tap a day ahead is refused | `debrief::a_tap_is_accepted_up_to_three_days_back_and_refused_after` |
| `S10108-ONLY-THREE-RATINGS` | `crates/notifications/src/debrief.rs` | a rating outside 1 to 3 is refused | `debrief::only_the_three_ratings_are_accepted` |
| `S10109-THE-LAW-BLOCK-LEADS` | `crates/notifications/src/digest.rs` | the law block comes before the body | `digest_compose::the_sections_keep_the_predecessors_order` |
| `S10110-RESEARCH-LINE-THIS-WEEK` | `crates/notifications/src/digest.rs` | a report older than the week is not named | `digest_compose::the_research_line_names_this_weeks_reports` |
| `S10111-THE-JUST-CLOSED-DAY` | `crates/coordination/src/digests/daily.rs` | the target is the day before the current one | `digest_job::the_digest_reports_the_just_closed_day_and_needs_its_rollup` |
| `S10112-EIGHT-ROWS` | `crates/coordination/src/digests/weekly.rs` | the weekly asks for 8 rows | `weekly_job::the_weekly_reads_every_input_it_names` |
| `S10113-SETTLE-BEFORE-READOUT` | `crates/coordination/src/digests/weekly.rs` | the settle runs before the readout | `weekly_job::the_weekly_settles_the_holdout_before_its_readout` |
| `S10114-THE-WEEKLY-DOES-NOT-CATCH-UP` | `crates/coordination/src/jobs.rs` | the weekly has no catch-up | `digest_jobs::the_digest_catches_up_and_the_weekly_does_not` |
