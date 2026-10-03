# SPEC-078: reading minutes and writing earn XP once, and feed one habit board and honest analytics

- **Wave:** W3. **Issue:** #93, #94, #95, #96 (epic #4). **Context(s):** `deck-streak-habits` (the
  minutes log of book reading and its XP, the writing confirmations and their XP, the goal and writing
  streaks, the board, freshness, the reading analytics, the habit badge conditions, `minutes_log`,
  `writing_log`); `deck-streak-coordination` (settling habit XP, correcting it on an undo or a
  toggle, the recompute's habit step, awarding habit badges, joining analytics' per-course daily
  stats); `deck-streak-progression` (the derived registry admits the habit sources);
  `deck-streak-api`, `deck-streak-bot` and the Mini App (the Habits tab).
- **Decided by:** ADR-078 (habit XP is settled in the write that changes its log), ADR-012 (the
  parity oracle proves the math), ADR-041 (every celebration passes through the one router), ADR-071
  (the recompute reaches every study day once, in order), ADR-072 (habit XP is settled, and the
  owner's own correction may lower it) and ADR-087 (the configured courses name the reading and the
  writing courses).
- **Prerequisites:** SPEC-026 (the bot's command table), SPEC-040 (the XP ledger), SPEC-041 (the
  router), SPEC-071 (the courses configuration, the fold's day steps and the per-course daily
  stats), SPEC-072 (the settle port and the level-up occasion) and SPEC-073 (the badge award port).
  **Mutation band:** `S07800-S07899`.
- **Status:** delivered in four pull requests. Part 078a (#93: the minutes log of book reading, its
  XP and its weekly bonus) is delivered: it moved this SPEC from `docs/specs/planned/` with its tests
  and `docs/red-first/SPEC-078.md` (ADR-016), and is proved by `formal/tla/HabitXpFollowsItsLog/`.
  Part 078b (#94: the writing habit and the habit badges), part 078c (#95: the board, the routes and
  the Habits tab) and part 078d (#96: the reading analytics) are pending; section 3c holds their
  criteria.

## 1. The problem, measured

- **Nothing exists yet.** At `dev` 02758d4 `crates/habits/src/` holds only `lib.rs`, and the crate's
  manifest names the kernel alone. No habit entry, writing confirmation, habit XP, board or reading
  analytics exists. `crates/progression/src/settle.rs` refuses every source outside its nine derived
  names before a write, so no habit source can be settled yet.
- **The words.** The context map holds "reading" apart: in `readings` it is one pre-study text; the
  book-reading habit is what `habits` calls a minutes log (`docs/CONTEXT-MAP.md`, overloaded words).
  This SPEC keeps that: its table is `minutes_log`, and the bot command keeps the predecessor's name,
  `/read`, as the brief for this wave requires.
- **What is ported.** The predecessor's `reading-log` (#93): `bot.py:CommandBot._handle_read` and
  `_callback_log_reading`, `pipeline_layers/habits.py:HabitsLayer.record_reading`,
  `undo_last_reading`, `_recompute_reading_xp` and `most_used_reading_lang`, `habits.py:reading_xp`,
  `curriculum.py:resolve_lang`. Its `writing-log` (#94): `bot.py:CommandBot._handle_write` and
  `_callback_writing`, `HabitsLayer.record_writing`, `toggle_writing` and `_recompute_writing_xp`,
  `habits.py:writing_day_xp` and `writing_streak`. Its `habit-board` (#95):
  `HabitsLayer.habits_summary`, `habits.py:compute_reading`, `compute_writing`,
  `reading_goal_streak` and `reading_goal_streaks_by_lang`, `telegram.py:render_habits_block`. Its
  `reading-analytics` (#96): `habit_analytics.py:reading_rollup`, `combined_effort`,
  `reading_retention_correlation`, `fisher_z_ci`, `weekly_volume_anomaly` and `reading_library`,
  `HabitsLayer.reading_analytics` and `reading_weekly_series`. And the habit badges:
  `habits.py:evaluate_habit_badges` over `HabitsLayer._habit_badge_context`.
- **The courses are configuration.** The predecessor's reading languages, writing languages, their
  one-letter aliases and their names are hard-coded constants of the owner's
  (`curriculum.READING_LANG_CODES`, `WRITING_LANG_CODES`, `LANG_ALIASES`). Here every configured
  course is a reading course, a course marked `writing` is a writing course, and each course names its
  alias (ADR-087). The predecessor's functions take the codes as a parameter, which is how their
  goldens drive them with synthetic courses.
- **Traps a hand port falls into.**
  - `habits.py` and `habit_analytics.py` import the language constants into their own namespaces, so
    an adapter patches those modules' names with synthetic courses.
  - The weekly bonus is keyed on the week's Monday, not the entry's day, and is re-derived after
    every entry and undo, so an undo that drops the week below the goal takes it back.
  - The predecessor's week is a calendar week. Here a day is the kernel's study day, which rolls over
    at 04:00 local time, so the week runs from the local Monday's 04:00 to the next and an entry at
    Monday 03:59 belongs to the week before.
  - An entry of 0 minutes or over 600 is refused, not capped, and a note is cut at 200 characters.
  - Undo removes the newest entry in the order entries were logged, whatever its day.
  - The predecessor's Undo button removes the newest entry whatever it was rendered for, so a stale
    button removes an entry the owner never saw on it.
  - The writing bonus pays when every writing course is confirmed (`set(codes) <= done`), which an
    empty set satisfies, and the goal streak holds when every course met the goal (`all(...)`), which
    an empty list satisfies: with no course of that kind configured, nothing is paid or counted here.
  - The correlation pairs a week's reading with the NEXT week's retention, never uses the current
    week, needs four paired weeks, and rounds r, rho and the interval to two decimals with Python's
    `round`, which rounds the float's exact value half to even.
  - The volume anomaly flags only when |z| exceeds 3.0, over the trailing complete weeks that had
    reviews, with the sample standard deviation.
  - The anomaly's week start and the library's last day are ISO dates in the predecessor's output;
    their goldens carry epoch days.
- **The settle and its rollover.** ADR-072 stores the larger amount when a recompute settles a
  closed day, and lets only the owner's own correction lower one. So an undo whose lowering settle
  committed apart from the log write, and failed, would leave a closed day over-paid for good: no
  later recompute can lower it. ADR-078 puts the log write and its settles in one write.
- **Corrections to the issues.**
  - #93 says entries "are capped at READING_MAX_ENTRY_MIN = 600"; the predecessor refuses an entry
    outside 1 to 600 minutes (`HabitsLayer._record_reading_impl`).
  - #95 says the board shows each language's freshness. In the predecessor the "gone cold" label
    appears only in the evening habit check-in (`telegram.py:render_habit_nudge`, through
    `_reading_freshness_suffix`, with `HabitsLayer._reading_days_since_by_lang`). Here the board shows
    each course's days since its last entry with the predecessor's label rule, and the check-in stays
    W5's (#97).
  - #96 says the anomaly flags "at |z| >= 3.0"; the predecessor flags |z| > 3.0.
  - #94's "all three" is every configured writing course.
- **What the parity oracle proves.** The per-day XP and its cap, the course token and its aliases, the
  settled amounts of an entry and the weekly bonus, the writing XP and streaks, the week's bars and the
  goal streaks, the days since and the freshness label, the rollup, the effort, the library, the
  correlation with its interval and every verdict band's wording, the anomaly, and the habit badges.
- **Prerequisites.** SPEC-026, SPEC-040, SPEC-041, SPEC-071, SPEC-072 and SPEC-073, as the header
  names them.

## 2. Requirements

Part 078a delivers R1 to R5, R18 for the minutes log, R21's `minutes_log` and R22 for the constants
it uses; 078b delivers R6 to R9, R17, R18 for the writing toggle and R21's `writing_log`; 078c
delivers R10 to R12, R19 and R20; 078d delivers R13 to R16.

The minutes log of book reading (#93)

R1. A minutes log entry names a course, a number of minutes and an optional note. The course token
    resolves as the predecessor's `curriculum.py:resolve_lang` does, over the configured courses of the
    kernel's `Courses` (SPEC-071 loads it, and habits never loads the file): a course code passes
    first, then a course's one-letter alias, and any other token or a course that is not configured is
    refused (`goldens/habit_course_token.json`). Minutes from 1 to 600 are accepted and any other
    number is refused (`constants.READING_MAX_ENTRY_MIN`); the note is cut to 200 characters; the
    entry's day is the current study day. These rules are pure functions of
    `crates/habits/src/minutes.rs`.
R2. `/read <course> <minutes> [note]` logs an entry. `/read <minutes> [note]`, whose first token is
    all digits, logs it for the most-used course, the course with the most minutes this week, else of
    all time (`goldens/habit_most_used.json`, `HabitsLayer.most_used_reading_lang`), and asks which
    course when there is none. Bare `/read` opens a course picker and then the minute presets of 10,
    15, 20, 30, 45 and 60 (`bot._READ_PRESETS` in `goldens/habits.constants.json`); a logged entry
    offers an Undo button. `read` and `undo` join the owner's chat menu. Every reply is HTML with the
    owner's note and token escaped, and none names a date, a time of day or a deadline.
R3. A course's XP for a study day is `min(240, 2 × that day's minutes)` (`goldens/habit_reading_xp.json`,
    `habits.py:reading_xp`; `constants.READING_XP_PER_MIN`, `READING_XP_DAILY_CAP_PER_LANG`), settled
    (ADR-072) as `read:<code>` on that day, track language. The week's bonus is 150 when the course's
    minutes in its study week reach 210, settled as `readgoal:<code>` on the week's first study day,
    and 0 otherwise (`constants.READING_GOAL_XP_BONUS`, `READING_WEEKLY_GOAL_MIN`,
    `WEEK_START_WEEKDAY`). A study week runs from the local Monday's 04:00 rollover to the next, so
    its first study day is `d − (d + 3) mod 7` over the kernel's study days (epoch day 0 was a
    Thursday). Both are re-derived after every entry and undo and equal
    `goldens/habit_reading_settled.json` (`HabitsLayer._recompute_reading_xp`). ADR-072's closed
    registry admits the habit sources by prefix: `read:` and `readgoal:` (078a), then `write:` (078b),
    each followed by a valid course code; its nine derived names are unchanged, and any other name
    stays refused. No `economy.json` key: the amounts are habits' own constants, held equal to the
    predecessor's by R22.
R4. `/undo` removes the newest entry, whatever its day, and re-derives that day's XP and that entry's
    week's bonus, never the current week's. The Undo button of a logged reply carries its entry's id
    and removes that entry only while it is still the newest; otherwise it removes nothing and says to
    send `/undo` (ADR-078). An amount an undo lowers, a closed day's included, is the owner's own
    correction (ADR-072); the settled amounts after an entry and its undo equal those before the
    entry.
R5. An entry and an undo each run in one write through the kernel's write base (`BEGIN
    IMMEDIATE`): the log write and its settles of `read:<code>` on the entry's day and
    `readgoal:<code>` on its week's first study day, with the owner's correction as their cause,
    commit together or not at all (ADR-078). The habit step registers in `Phase::DaySteps` of
    SPEC-071's fold, after the streaks and before the derived bonuses and the coin mint, which count
    habit XP in the day's base. Inside each evaluated day's write it settles `read:<code>` for every
    course with an entry or a held `read:` row on that day and, when the day is its week's first study
    day, `readgoal:<code>` from that week's minutes. A settle a failure left undone heals when the fold
    next evaluates its day, with the same rule's value.

The writing habit (#94)

R6. The writing courses are the configured courses marked `writing`. `/write <course>...` confirms
    today's writing for each named writing course, `/unwrite <course>` clears it, and a course that is
    not a writing course is refused. Bare `/write`, and `/habits`, show one toggle chip per writing
    course, and a tap flips today's confirmation.
R7. A study day's writing XP is 75 for each confirmed writing course, settled as `write:<code>`, and
    100 more when every writing course is confirmed, settled as `write:all`; an unconfirmed course
    settles 0 (`goldens/habit_writing_xp.json`, `habits.py:writing_day_xp` over synthetic writing
    courses; `constants.WRITING_XP_PER_DAY`, `WRITING_XP_ALL_THREE_BONUS`). With no writing course
    configured, no writing XP is settled and no writing streak or ink badge counts.
R8. The writing streak is the number of consecutive days, ending today or yesterday, on which every
    writing course was confirmed, and each writing course has its own streak
    (`goldens/habit_writing_streak.json`, `habits.py:writing_streak` and `writing_streaks_by_lang`).
R9. Toggling a confirmation off lowers that day's writing XP as the owner's own correction (ADR-072),
    and toggling it twice leaves the day's settled amounts as they were.

The habit board (#95)

R10. The board's week is the study week of the current study day. Each reading course shows its
    minutes against the 210-minute goal, its percent, the minutes remaining, whether it met the goal,
    the days left and the minutes a day it needs (`goldens/habit_weekly_bars.json`,
    `habits.py:compute_reading`), with its goal streak in weeks; the board shows the perfect reading
    weeks, the consecutive weeks ending this week or last in which every course met the goal
    (`goldens/habit_goal_streaks.json`, `habits.py:reading_goal_streak` and
    `reading_goal_streaks_by_lang`), today's writing checklist with each writing course's streak, and
    the writing streak. With no course configured, the board is empty and no streak counts.
R11. Each reading course shows its days since its last entry in the last 400 days
    (`goldens/habit_days_since.json`, `HabitsLayer._reading_days_since_by_lang`), labelled by the
    predecessor's rule: nothing when read today, "no reading logged yet" when there is none, the days
    since below 21, and "gone cold" with the days at 21 or more (`goldens/habit_freshness.json`,
    `telegram.py:_reading_freshness_suffix`; `constants.READING_COLD_DAYS`).
R12. One summary, the predecessor's `HabitsLayer.habits_summary`, backs `/habits`, `GET /api/habits`
    and the Habits tab: all three report the same minutes, streaks and confirmations for one study day.

Reading analytics (#96)

R13. `/readstats` and the analytics route report each course's trailing 28-day minutes, entries,
    average entry (one decimal), active days and daily reading streak (`goldens/habit_rollup.json`,
    `habit_analytics.py:reading_rollup`; `DEFAULT_ANALYTICS_WINDOW_DAYS`); this week's combined effort
    per course, the Anki minutes from SPEC-071's per-course daily stats, the minutes logged, the writing
    days and their total, ordered by the total (`goldens/habit_effort.json`,
    `habit_analytics.py:combined_effort`); and the library, the ten materials named by entries' notes,
    by minutes (`goldens/habit_library.json`, `habit_analytics.py:reading_library`).
R14. The correlation reports, for each course, a week's minutes against the next week's true retention
    over the last 12 complete weeks, counting a week only when its retention week has answers: Pearson
    r and Spearman rho to two decimals; rho's 95% interval by Fisher z, with the standard error
    `sqrt(1.06 / (n − 3))` and z 1.959964; whether the interval excludes 0; the verdict's wording,
    verbatim, for every band; and "not enough data yet" below 4 paired weeks
    (`goldens/habit_correlation.json`, `habit_analytics.py:reading_retention_correlation`;
    `goldens/habit_fisher_z.json`, `habit_analytics.py:fisher_z_ci`; `DEFAULT_CORR_WEEKS`,
    `MIN_CORR_WEEKS`, `FISHER_Z_95`, `SPEARMAN_SE_FACTOR`). No wording claims a cause.
R15. The volume anomaly compares this week's Anki reviews with the trailing 8 complete weeks that had
    reviews, needs 4 of them, uses the sample standard deviation, and flags when |z| exceeds 3.0, with
    its direction (`goldens/habit_volume_anomaly.json`, `habit_analytics.py:weekly_volume_anomaly`;
    `VOLUME_ANOMALY_TRAILING_WEEKS`, `VOLUME_ANOMALY_MIN_PRIOR_WEEKS`, `VOLUME_ANOMALY_Z_THRESHOLD`).
R16. `/readtrend` and `/correlate` answer in text with an Open button into the Mini App. The routes
    serve the eight-week series, each week's minutes per course and the goal line of 210 times the
    number of courses (the predecessor's `HabitsLayer.reading_weekly_series`), and the correlation's
    points, as JSON; SPEC-085 draws them.

The habit badges

R17. After an entry, an undo or a confirmation, and again in phase 7 of SPEC-071's fold, each habit
    badge whose condition holds is awarded once through SPEC-073's award port and celebrated through
    the router: `first_page`, `quill_initiate`,
    `ink_week`, `ink_month` and `ink_century` at a writing streak of 7, 30 and 100, `bookworm_week`
    when every course met the goal this week, `polyglot_reader` when every course was read this week,
    and `marathon_reader` at 600 minutes this week (`goldens/habit_badges.json`,
    `habits.py:evaluate_habit_badges` over `HabitsLayer._habit_badge_context`;
    `constants.WRITING_STREAK_WEEK`, `WRITING_STREAK_MONTH`, `WRITING_STREAK_CENTURY`,
    `MARATHON_READER_WEEK_MIN`). Part 078a awards no habit badge.
R18. When a habit write raises the level, the use case reads the level before and after its write
    and, after commit, raises SPEC-072's level-up occasion through the router; the router's once-ever
    key for the level keeps a sync cycle and a habit write from both celebrating it (ADR-078).

The screens and the data

R19. The routes under `/api/habits` (the summary, an entry, an undo, a writing toggle, the analytics,
    the correlation and the weekly series) serve the owner only (SPEC-024); any other caller gets 401
    or 403 and no data.
R20. The Mini App's `/habits` tab holds the minutes logger (course chips, the minute presets and a
    stepper, an optional note), the week's bars, the writing toggle chips, the streak counters and each
    course's freshness; `/habits/analytics` shows the rollup, the effort, the library, each course's
    verdict and the anomaly as text and tables. Both join `ROUTES`.
R21. Habits owns `minutes_log` (id, code, study day, minutes from 1 to 600, a note of at most 200
    characters, `created_at`), `STRICT`, created by `migrations/007801_habits_minutes_log.sql`
    (078a), and `writing_log` (code, study day, `created_at`, unique on code and study day),
    `STRICT`, created by `migrations/007802_habits_writing_log.sql` (078b) (SPEC-020 R15, R18). Each is
    registered in the context map's register of DeckStreak's own tables, declared in `privacy.json`
    (the categories `minutes-log`, which holds the owner's free-text notes, and `writing-log`), given a
    line per category in `PRIVACY.md`, and listed as exported and erased by habits' data-rights port,
    which joins coordination's registry; the symmetry test seeds each.
R22. Every constant this SPEC uses equals `goldens/habits.constants.json`, held by a test; each part
    adds the constants it uses.

## 3. Acceptance criteria

This part's criteria; section 3c holds the later parts'.

| id | criterion | decided by |
|---|---|---|
| A1 | a course's per-day XP of every case equals the golden of `habits.py:reading_xp`, a case above 120 minutes included (examined count reported, zero refused) | `the_minutes_xp_matches_the_predecessors_golden` |
| A2 | every course token resolves as the golden of `resolve_lang` says over synthetic courses; 0 and 601 minutes are refused and 1 and 600 accepted; a note of 201 characters is cut to 200 | `an_entry_resolves_its_course_and_refuses_minutes_outside_the_bounds` |
| A4c | a study day's week starts on its Monday, for days either side of epoch day 0 and below it | `a_study_days_week_starts_on_its_monday` |
| A3 | a minutes-only entry is logged for the course the golden of `most_used_reading_lang` names, and with no entry at all it is refused for want of a course | `minutes_only_logs_the_most_used_course` |
| A4 | the settled `read:` and `readgoal:` amounts after each case's entries equal the golden of `_recompute_reading_xp`, a week of exactly 210 minutes included | `the_minutes_xp_settles_as_the_predecessors_golden` |
| A4b | under a non-zero fixed offset, an entry at local Monday 03:59 counts in the previous study week and one at 04:00 in the new one | `the_weekly_bonus_is_keyed_on_the_study_weeks_first_day` |
| A5 | an entry and its undo leave every settled amount as it was, a closed day's and an earlier week's included | `undo_restores_the_prior_settled_xp` |
| A5b | an Undo button rendered for an entry removes nothing once a newer entry exists | `a_stale_undo_removes_nothing` |
| A6 | a `minutes_log` row left with no settle is settled by the fold's habit step, the `readgoal:` on its week's first study day included | `the_recompute_heals_a_habit_settle_left_undone` |
| A6b | entries and a recompute run concurrently on two connections, and the final settled amounts equal the rule over the final log | `entries_and_a_recompute_serialise` |
| A30 | a habit write that crosses a level raises one level-up decision through the router, and repeating the crossing raises none | `a_habit_write_that_raises_the_level_is_announced_once` |
| A14 | every habit constant this part uses equals `goldens/habits.constants.json`, the presets included | `the_habit_constants_equal_the_predecessors` |
| A22 | habits' data-rights port lists `minutes_log` as exported and erased, and an erase leaves it empty | `the_habit_tables_are_exported_and_erased` |
| A23 | `read:qaa` and `readgoal:qaa` settle; `read:`, `read:QAA`, `readgoal:a b` and `reading:read:r1` are refused as not derived | `the_habit_sources_are_derived_only_with_a_course_code` |
| A24 | the production fold registers the habit step in `Phase::DaySteps`, between the streaks and the derived bonuses | `wiring::tests::the_recompute_fold_registers_the_habit_step_in_the_day_steps_phase` |
| A25 | `/read` by code, by alias, by minutes only, and by the picker then a preset each log and answer their golden | `read_logs_minutes_and_answers_with_the_days_xp` |
| A26 | every refused `/read` answers its golden and logs nothing: 0, 601, a word for minutes, an unknown course, no courses configured, and a refused database | `a_refused_read_answers_and_logs_nothing` |
| A27 | `/undo` answers `undo-done` and `undo-nothing`; a stale button answers `undo-stale` and removes nothing | `undo_answers_for_the_newest_entry_and_a_stale_button_removes_nothing` |
| A28 | every habit callback's data is 1 to 64 bytes for the longest course code and the largest id, and every one is answered | `every_habit_callback_fits_telegrams_bound_and_is_answered` |
| A29 | `read` and `undo` join the owner's chat menu, each description 1 to 256 bytes | `the_reading_commands_join_the_owners_menu` |
| A31 | the existing data-rights symmetry tests pass with the new table: every table is declared by exactly one port, and `privacy.json` names every table a port exports or erases | `every_table_of_the_schema_is_declared_by_exactly_one_port`, `privacy_json_names_every_table_the_ports_export_or_erase` |

```acceptance
A1: cargo test -p deck-streak-habits --test habits_minutes -- --exact the_minutes_xp_matches_the_predecessors_golden
A2: cargo test -p deck-streak-habits --test habits_minutes -- --exact an_entry_resolves_its_course_and_refuses_minutes_outside_the_bounds
A4c: cargo test -p deck-streak-habits --test habits_minutes -- --exact a_study_days_week_starts_on_its_monday
A3: cargo test -p deck-streak-coordination --test habits_minutes -- --exact minutes_only_logs_the_most_used_course
A4: cargo test -p deck-streak-coordination --test habits_minutes -- --exact the_minutes_xp_settles_as_the_predecessors_golden
A4b: cargo test -p deck-streak-coordination --test habits_minutes -- --exact the_weekly_bonus_is_keyed_on_the_study_weeks_first_day
A5: cargo test -p deck-streak-coordination --test habits_minutes -- --exact undo_restores_the_prior_settled_xp
A5b: cargo test -p deck-streak-coordination --test habits_minutes -- --exact a_stale_undo_removes_nothing
A6: cargo test -p deck-streak-coordination --test habits_minutes -- --exact the_recompute_heals_a_habit_settle_left_undone
A6b: cargo test -p deck-streak-coordination --test habits_minutes -- --exact entries_and_a_recompute_serialise
A30: cargo test -p deck-streak-coordination --test habits_minutes -- --exact a_habit_write_that_raises_the_level_is_announced_once
A14: cargo test -p deck-streak-habits --test habits_constants -- --exact the_habit_constants_equal_the_predecessors
A22: cargo test -p deck-streak-habits --test habits_store -- --exact the_habit_tables_are_exported_and_erased
A23: cargo test -p deck-streak-progression --test xp_settle -- --exact the_habit_sources_are_derived_only_with_a_course_code
A24: cargo test -p deck-streak-daemon --lib -- --exact wiring::tests::the_recompute_fold_registers_the_habit_step_in_the_day_steps_phase
A25: cargo test -p deck-streak-bot --test habits_commands -- --exact read_logs_minutes_and_answers_with_the_days_xp
A26: cargo test -p deck-streak-bot --test habits_commands -- --exact a_refused_read_answers_and_logs_nothing
A27: cargo test -p deck-streak-bot --test habits_commands -- --exact undo_answers_for_the_newest_entry_and_a_stale_button_removes_nothing
A28: cargo test -p deck-streak-bot --test habits_commands -- --exact every_habit_callback_fits_telegrams_bound_and_is_answered
A29: cargo test -p deck-streak-bot --test habits_commands -- --exact the_reading_commands_join_the_owners_menu
A31: cargo test -p deck-streak-coordination --test data_rights_symmetry -- --exact every_table_of_the_schema_is_declared_by_exactly_one_port
A31: cargo test -p deck-streak-coordination --test data_rights_symmetry -- --exact privacy_json_names_every_table_the_ports_export_or_erase
```

Amended existing tests, which keep their names and are not red-first evidence:
`commands::the_menu_is_registered_for_the_owners_chat_only` (the two new commands joined to its
set's existing line, which rustfmt keeps as one line, eleven commands to
thirteen), `commands::every_golden_message_is_what_the_bot_sends` (its
`rendered()` gains the new replies), and
`wiring::tests::the_recompute_fold_registers_the_analytics_xp_and_streak_steps_in_their_phases` (one
line for the habit step).

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr, telegram-platform, notifications-policy
and accessibility packs stay enforced; no check is deferred or lifted for this delivery, so the private
wiring does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md` and `crates/habits/src/data_rights.rs`: the `minutes-log` category declares the owner's free-text notes with purpose, basis and retention, and export and erase cover `minutes_log` (078b adds `writing-log` and `writing_log`) | the privacy-gdpr pack |
| B2 | over `crates/bot/src/habits_commands.rs` and `crates/bot/src/commands.rs`: every habit command is registered for the owner's chat only, every callback is answered, and every callback's data stays inside Telegram's 64-byte bound | the telegram-platform pack |
| B3 | over every file under `crates/coordination/src/`: each habit badge's celebration goes through the one router (judged when 078b lands its badges) | the notifications-policy pack |
| B4 | over `web/app/src/routes/habits/+page.svelte`, `web/app/src/routes/habits/analytics/+page.svelte` and `web/app/src/lib/habits/`: both screens pass the accessibility audit in both Telegram colour schemes (judged when 078c lands them) | the accessibility pack |

## 3c. Delivered by the next pull requests

This SPEC lands in four pull requests, in order. This one (078a) delivers the minutes log of book
reading: the criteria of section 3's table and its fence. 078b delivers the writing habit and the
habit badges; 078c delivers the board, the routes and the Habits tab; 078d delivers the reading
analytics. The table below holds the criteria a later pull request delivers, each row naming that
pull request, and the lines under it are their fence lines, each prefixed with that pull request. A
later pull request moves each of its criteria back verbatim: the row into section 3's table, without
the `delivered by` column, and the fence line into the acceptance fence, without the prefix.

| id | criterion | decided by | delivered by |
|---|---|---|---|
| A7 | the writing XP of every case equals the golden of `habits.py:writing_day_xp` | `the_writing_xp_matches_the_predecessors_golden` | 078b |
| A8 | the writing streaks of every case equal the golden of `writing_streak` and `writing_streaks_by_lang` | `the_writing_streaks_match_the_predecessors_golden` | 078b |
| A9 | with no writing course configured, no writing XP is settled and no writing streak counts | `no_writing_xp_is_settled_without_a_writing_course` | 078b |
| A10 | toggling a confirmation twice leaves the day's settled writing XP as it was | `toggling_twice_restores_the_days_writing_xp` | 078b |
| A19 | the habit badges of every case equal the golden of `evaluate_habit_badges`, each awarded and celebrated once | `habit_badges_are_awarded_once_as_the_predecessors_golden` | 078b |
| A11 | the week's bars of every case equal the golden of `habits.py:compute_reading` | `the_weekly_bars_match_the_predecessors_golden` | 078c |
| A12 | the goal streaks of every case equal the golden of `reading_goal_streak` and `reading_goal_streaks_by_lang`, and none counts with no course | `the_goal_streaks_match_the_predecessors_golden` | 078c |
| A13 | each course's days since and label equal the goldens of `_reading_days_since_by_lang` and `_reading_freshness_suffix` | `the_freshness_label_matches_the_predecessors_rule` | 078c |
| A15 | `/habits` and the summary route report the same minutes, streaks and confirmations for one study day | `habits_and_the_route_report_the_same_summary` | 078c |
| A20 | every habit route answers the owner and refuses every other caller with no data | `the_habit_routes_answer_only_the_owner` | 078c |
| A21 | the minutes logger logs a preset of minutes for the chosen course | `logs a preset of minutes for the chosen course` | 078c |
| A16 | the rollup, the effort and the library of every case equal their goldens | `the_rollup_effort_and_library_match_the_predecessors_goldens` | 078d |
| A17 | the correlation and its interval equal their goldens for every case, every verdict band's wording included | `the_correlation_matches_the_predecessors_golden` | 078d |
| A18 | the anomaly of every case equals the golden of `weekly_volume_anomaly`, a z of exactly 3.0 unflagged | `the_volume_anomaly_matches_the_predecessors_golden` | 078d |

078b: A7: cargo test -p deck-streak-habits --test writing_goldens -- --exact the_writing_xp_matches_the_predecessors_golden
078b: A8: cargo test -p deck-streak-habits --test writing_goldens -- --exact the_writing_streaks_match_the_predecessors_golden
078b: A9: cargo test -p deck-streak-habits --test writing_goldens -- --exact no_writing_xp_is_settled_without_a_writing_course
078b: A10: cargo test -p deck-streak-coordination --test writing_toggle -- --exact toggling_twice_restores_the_days_writing_xp
078b: A19: cargo test -p deck-streak-coordination --test habits_badges -- --exact habit_badges_are_awarded_once_as_the_predecessors_golden
078c: A11: cargo test -p deck-streak-habits --test habits_board -- --exact the_weekly_bars_match_the_predecessors_golden
078c: A12: cargo test -p deck-streak-habits --test habits_board -- --exact the_goal_streaks_match_the_predecessors_golden
078c: A13: cargo test -p deck-streak-habits --test habits_board -- --exact the_freshness_label_matches_the_predecessors_rule
078c: A15: cargo test -p deck-streak-bot --test habits_commands -- --exact habits_and_the_route_report_the_same_summary
078c: A20: cargo test -p deck-streak-api --test habits_routes -- --exact the_habit_routes_answer_only_the_owner
078c: A21: pnpm exec vitest run web/app/src/lib/habits/MinutesLogger.test.ts -t "logs a preset of minutes for the chosen course"
078d: A16: cargo test -p deck-streak-habits --test habits_analytics -- --exact the_rollup_effort_and_library_match_the_predecessors_goldens
078d: A17: cargo test -p deck-streak-habits --test habits_analytics -- --exact the_correlation_matches_the_predecessors_golden
078d: A18: cargo test -p deck-streak-habits --test habits_analytics -- --exact the_volume_anomaly_matches_the_predecessors_golden

## 4. File manifest

This part's files:

| file | context | change |
|---|---|---|
| `crates/habits/src/minutes.rs` | `deck-streak-habits` | added: the course token, the entry's bounds and note, the per-day XP, the study week's first day and the weekly bonus, as pure functions |
| `crates/habits/src/store.rs` | `deck-streak-habits` | added: the repository over `minutes_log`, on the caller's connection |
| `crates/habits/src/data_rights.rs` | `deck-streak-habits` | added: habits' data-rights port |
| `crates/habits/src/lib.rs` | `deck-streak-habits` | changed: the modules above |
| `crates/habits/Cargo.toml` | `deck-streak-habits` | changed: `sqlx`, `thiserror` and `serde_json` (ADR-003, ADR-029), and the golden reader's `serde` and `serde_json`, `tempfile` and `tokio` as dev-dependencies (SPEC-029 R8) |
| `crates/habits/tests/habits_minutes.rs` | `deck-streak-habits` | added: A1, A2, A4c |
| `crates/habits/tests/habits_constants.rs` | `deck-streak-habits` | added: A14 |
| `crates/habits/tests/habits_store.rs` | `deck-streak-habits` | added: A22 |
| `migrations/007801_habits_minutes_log.sql` | `deck-streak-habits` | added: `minutes_log`, `STRICT`, with `created_at` and its checks |
| `crates/progression/src/settle.rs` | `deck-streak-progression` | changed: the derived prefixes `read:` and `readgoal:` and `is_derived` |
| `crates/progression/tests/xp_settle.rs` | `deck-streak-progression` | changed: A23 |
| `crates/coordination/src/habits/mod.rs` | `deck-streak-coordination` | added: the habits use cases' module |
| `crates/coordination/src/habits/minutes.rs` | `deck-streak-coordination` | added: `log_minutes`, `undo_newest` and `undo_entry`, each one write with its settles, and the level-up after commit |
| `crates/coordination/src/recompute/habits.rs` | `deck-streak-coordination` | added: the habit step |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: the habit step's module |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the habits module |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the registry gains habits' port |
| `crates/coordination/tests/habits_minutes.rs` | `deck-streak-coordination` | added: A3 to A6b, A30 |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded `minutes_log` row (A31) |
| `crates/coordination/tests/relight_order.rs` | `deck-streak-coordination` | changed: its census of statics names habits' data-rights port |
| `crates/notifications/tests/one_router.rs` | `deck-streak-notifications` | changed: the command handler's replies and their named callers gain `read`, `undo` and `habit_callback` |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the habit step in `Phase::DaySteps`, and A24 in its tests |
| `crates/daemon/src/role_bot.rs` | `deck-streak-daemon` | changed: the bot's commands get the courses and the router |
| `crates/bot/src/habits_commands.rs` | `deck-streak-bot` | added: /read, /undo and their callbacks |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the command table and the owner's menu gain `read` and `undo`; `with_habits` |
| `crates/bot/src/lib.rs` | `deck-streak-bot` | changed: the habits commands module |
| `crates/bot/tests/habits_commands.rs` | `deck-streak-bot` | added: A25 to A29 |
| `crates/bot/tests/commands.rs` | `deck-streak-bot` | changed: the menu's set and `rendered()` |
| `crates/bot/tests/messages/` | `deck-streak-bot` | added: the thirteen habit replies' goldens; changed: `help` and `start` |
| `tools/parity-oracle/registry/spec_078.py` | repo | added: this part's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/habit_reading_xp.json` | repo | added: the golden of `habits.py:reading_xp` (function) |
| `tools/parity-oracle/goldens/habit_course_token.json` | repo | added: the golden of `curriculum.py:resolve_lang` (adapter; synthetic courses and aliases) |
| `tools/parity-oracle/goldens/habit_most_used.json` | repo | added: the golden of `pipeline_layers/habits.py:HabitsLayer.most_used_reading_lang` (adapter; a stub store) |
| `tools/parity-oracle/goldens/habit_reading_settled.json` | repo | added: the golden of `pipeline_layers/habits.py:HabitsLayer._recompute_reading_xp` (adapter; a stub store) |
| `tools/parity-oracle/goldens/habits.constants.json` | repo | added: the constants this part uses (constants) |
| `formal/tla/HabitXpFollowsItsLog/` | formal | added: the model, its configuration and its witnesses |
| `formal/tla/MintReadsTheFinalBase/MintReadsTheFinalBase.tla` | formal | changed: the `settle` cover's digest and its re-read |
| `config/formal.json` | repo | changed: the new entry's budget |
| `docs/specs/SPEC-078-the-reading-and-writing-habits-the-habit-board-and-reading-analytics.md` | docs | moved from `docs/specs/planned/` and rewritten for part 078a |
| `docs/decisions/ADR-078-habit-xp-is-settled-in-the-write-that-changes-its-log.md` | docs | added |
| `docs/schematics/habit-xp-settles-from-its-log.md` | docs | added |
| `docs/red-first/SPEC-078.md` | docs | added |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `minutes_log`, and habits' external crates are named |
| `privacy.json` | repo | changed: the `minutes-log` category |
| `PRIVACY.md` | repo | changed: the `minutes-log` line |
| `scripts/mutation-rows.d/S07800-S07899.json` | repo | added: the rows of section 9 |
| `scripts/mutation-rows.d/S07200-S07299.json` | repo | changed: `S07206-DERIVED-ONLY` re-anchored on settle's `is_derived` check, its mutant and killer unchanged |
| `changelog.d/feat-habits-078.md` | repo | added |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `Cargo.lock` | workspace | changed |

The later parts add: 078b `crates/habits/src/writing.rs`, `crates/habits/src/badges.rs`,
`crates/habits/tests/writing_goldens.rs`, `migrations/007802_habits_writing_log.sql`,
`crates/coordination/tests/writing_toggle.rs`, `crates/coordination/tests/habits_badges.rs`, the
writing and badge goldens and the phase-7 badge step; 078c `crates/habits/src/board.rs`,
`crates/habits/tests/habits_board.rs`, `crates/api/src/habits_routes.rs`, `crates/api/src/router.rs`,
`crates/api/tests/habits_routes.rs`, the Habits tab under `web/app/src/routes/habits/` and
`web/app/src/lib/habits/`, `web/app/src/lib/routes.ts` and the board goldens; 078d
`crates/habits/src/analytics.rs`, `crates/habits/tests/habits_analytics.rs` and the analytics goldens.

## 5. What this does NOT do

- It sends no evening habit check-in, the predecessor's home of the "gone cold" label (#97).
- It writes no habit block into the daily digest or the weekly report (#129, #130).
- It draws no chart: the eight-week trend and the correlation's scatter are SPEC-085's (#152).
- It exposes no habit entry, summary or analytics through the agent's tools (#157).
- It writes no habit number into the vault's stats file (#153).
- It publishes no habit total on the public page (#156).
- It imports none of the predecessor's entries or confirmations (#61).
- Part 078a builds no writing habit, no `writing_log` and no habit badge (#94).
- Part 078a builds no board, no route and no Habits tab (#95). Those routes owe no formal model of
  their own only while they call `coordination::habits` use cases and nothing else: a route that
  writes `minutes_log` or a habit settle by any other path is a new actor on
  `formal/tla/HabitXpFollowsItsLog/` (#95).
- Part 078a builds no reading analytics (#96).
- An undo that lowers a closed day's `read:` amount re-derives neither the day's derived bonuses nor
  the coins minted from that day's base: those were settled when the day closed and keep the base
  they read, a residue this SPEC records rather than cures (#93; ADR-078 names what it was chosen
  against).

## 6. Risks

- **A course's alias or code leaks the owner's language set into a golden.** Prevented by the
  adapters, which patch the predecessor's constants with synthetic courses, and detected by the public
  scrub over the tree.
- **The coefficients round differently from Python's `round`.** Detected by A17, whose golden's cases
  include values that sit on a half at the second decimal, compared exactly.
- **An undo or a toggle lowers a closed day's XP where ADR-072 forbids a fall.** Bounded by ADR-072's
  exception, which admits only the owner's own correction of a manual entry; A5 and A10 pin it.
- **A failure between an entry and its settle leaves the XP behind the log.** Prevented by ADR-078's
  one write, detected by A6 and A6b, and healed by the recompute's habit step on the days it
  evaluates.
- **A week computed in UTC or from the calendar date puts an entry made before the rollover in the
  wrong week.** Detected by A4b and A4c.
- **A stale Undo button removes an entry the owner never saw on it.** Detected by A5b and A27.
- **A habit write and a sync cycle both celebrate one level.** Prevented by the router's once-ever
  key and detected by A30; checked by `ALevelUpIsCelebratedOnce` in the formal entry.
- **An empty course set pays or counts for nothing done.** Detected by A3, A9 and A12.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_078.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic; no golden carries the owner's language
codes, aliases or names, a note of the owner's, or a calendar string. Part 078a registers the first
five rows; each later part registers the goldens its criteria read.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `habit_reading_xp` | `habits.py:reading_xp` | function | none: 0, negative, below and at the cap, past it |
| `habit_course_token` | `curriculum.py:resolve_lang` | adapter | patches `curriculum.LANGUAGES_BY_CODE` and `LANG_ALIASES` with synthetic courses and aliases; codes, aliases, case and space, unknown tokens |
| `habit_most_used` | `pipeline_layers/habits.py:HabitsLayer.most_used_reading_lang` | adapter | a stub store returning the week's and all-time minutes per course in code order; ties, an empty week and no entries |
| `habit_reading_settled` | `pipeline_layers/habits.py:HabitsLayer._recompute_reading_xp` | adapter | a stub store holding the case's entries; it returns each `read:` amount on its day and each `readgoal:` amount on its week's Monday, as epoch days |
| `habits.constants` | the constants each part uses: 078a's `constants.READING_XP_PER_MIN`, `READING_XP_DAILY_CAP_PER_LANG`, `READING_GOAL_XP_BONUS`, `READING_WEEKLY_GOAL_MIN`, `WEEK_START_WEEKDAY`, `READING_MAX_ENTRY_MIN` and `bot._READ_PRESETS`; later parts add `READING_COLD_DAYS`, `WRITING_XP_PER_DAY`, `WRITING_XP_ALL_THREE_BONUS`, `WRITING_STREAK_WEEK`, `WRITING_STREAK_MONTH`, `WRITING_STREAK_CENTURY`, `MARATHON_READER_WEEK_MIN` and `habit_analytics.DEFAULT_ANALYTICS_WINDOW_DAYS`, `DEFAULT_CORR_WEEKS`, `MIN_CORR_WEEKS`, `FISHER_Z_95`, `SPEARMAN_SE_FACTOR`, `VOLUME_ANOMALY_TRAILING_WEEKS`, `VOLUME_ANOMALY_MIN_PRIOR_WEEKS`, `VOLUME_ANOMALY_Z_THRESHOLD` | constants | none |
| `habit_writing_xp` | `habits.py:writing_day_xp` | adapter | synthetic writing courses passed as `codes`; none, some and all confirmed (078b) |
| `habit_writing_streak` | `habits.py:writing_streak`, `writing_streaks_by_lang` | adapter | synthetic writing courses and confirmations on epoch days, with gaps, today and yesterday (078b) |
| `habit_badges` | `habits.py:evaluate_habit_badges` | adapter | patches `habits.READING_LANG_CODES` with synthetic courses and builds a badge context from the case (078b) |
| `habit_weekly_bars` | `habits.py:compute_reading` | adapter | patches `habits.LANGUAGES_BY_CODE` with synthetic courses; each weekday, met and unmet goals (078c) |
| `habit_goal_streaks` | `habits.py:reading_goal_streak`, `reading_goal_streaks_by_lang` | adapter | synthetic courses and entries on epoch days across met, unmet and current weeks (078c) |
| `habit_days_since` | `pipeline_layers/habits.py:HabitsLayer._reading_days_since_by_lang` | adapter | a stub store holding entries on epoch days; courses read today, long ago and never (078c) |
| `habit_freshness` | `telegram.py:_reading_freshness_suffix` | function | none: a missing key, none, zero, a few days, exactly 21 and more (078c) |
| `habit_rollup` | `habit_analytics.py:reading_rollup` | adapter | synthetic courses and entries on epoch days, inside and outside the window (078d) |
| `habit_effort` | `habit_analytics.py:combined_effort` | adapter | synthetic courses, entries, confirmations and per-course daily stats on epoch days (078d) |
| `habit_library` | `habit_analytics.py:reading_library` | adapter | synthetic entries with synthetic material notes; its last day returned as an epoch day (078d) |
| `habit_correlation` | `habit_analytics.py:reading_retention_correlation` | adapter | synthetic courses, entries and per-course daily stats on epoch days; every verdict band, too few weeks, and values on a half at the second decimal (078d) |
| `habit_fisher_z` | `habit_analytics.py:fisher_z_ci` | function | none: n below 4, at 4, rho at and past the clamp (078d) |
| `habit_volume_anomaly` | `habit_analytics.py:weekly_volume_anomaly` | adapter | synthetic per-course daily stats on epoch days; too few weeks, zero variance, z at exactly 3.0 and past it both ways; its week start returned as an epoch day (078d) |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `minutes_log` | `deck-streak-habits` | `migrations/007801_habits_minutes_log.sql` (078a) | its `reading_log`, one row per entry with its language code as the course code and its day as an epoch day | exported and erased |
| `writing_log` | `deck-streak-habits` | `migrations/007802_habits_writing_log.sql` (078b) | its `writing_log`, one row per confirmed language and day | exported and erased |

## 9. Mutation rows

Part 078a's rows are S07801 to S07816, in `scripts/mutation-rows.d/S07800-S07899.json`. S07817 to
S07819 stay free for a fix round; 078b takes S07820 to S07839; S07840 to S07899 stay with 078c and
078d.

| row | target | what it guards | killer |
|---|---|---|---|
| `S07801-TWO-XP-A-MINUTE` | `crates/habits/src/minutes.rs` | two XP a minute | `habits_minutes::the_minutes_xp_matches_the_predecessors_golden` |
| `S07802-DAILY-CAP-240` | `crates/habits/src/minutes.rs` | the per-course daily cap of 240 | `habits_minutes::the_minutes_xp_matches_the_predecessors_golden` |
| `S07803-ENTRY-BOUND-600` | `crates/habits/src/minutes.rs` | an entry refused above 600 minutes | `habits_minutes::an_entry_resolves_its_course_and_refuses_minutes_outside_the_bounds` |
| `S07804-NOTE-CUT-AT-200` | `crates/habits/src/minutes.rs` | a note cut at 200 characters | `habits_minutes::an_entry_resolves_its_course_and_refuses_minutes_outside_the_bounds` |
| `S07805-SIX-PRESETS` | `crates/habits/src/minutes.rs` | the minute presets | `habits_constants::the_habit_constants_equal_the_predecessors` |
| `S07806-WEEKLY-GOAL-210` | `crates/habits/src/minutes.rs` | the weekly goal of 210 minutes | `habits_constants::the_habit_constants_equal_the_predecessors` |
| `S07807-GOAL-BONUS-150` | `crates/habits/src/minutes.rs` | the weekly bonus of 150 | `habits_constants::the_habit_constants_equal_the_predecessors` |
| `S07808-WEEK-STARTS-ON-THE-STUDY-MONDAY` | `crates/habits/src/minutes.rs` | the study week's first day is its Monday | `habits_minutes::a_study_days_week_starts_on_its_monday` |
| `S07809-AN-UNDO-REDERIVES-ITS-ENTRYS-WEEK` | `crates/coordination/src/habits/minutes.rs` | an undo re-derives its entry's week, never the current one | `habits_minutes::undo_restores_the_prior_settled_xp` |
| `S07810-AN-UNDO-LOWERS-A-CLOSED-DAY-AS-THE-OWNER` | `crates/coordination/src/habits/minutes.rs` | a habit write settles as the owner's correction, so an undo lowers a closed day | `habits_minutes::undo_restores_the_prior_settled_xp` |
| `S07811-A-STALE-BUTTON-REMOVES-NOTHING` | `crates/coordination/src/habits/minutes.rs` | a button removes its entry only while it is the newest | `habits_minutes::a_stale_undo_removes_nothing` |
| `S07812-THE-STEP-SETTLES-THE-WEEKS-BONUS` | `crates/coordination/src/recompute/habits.rs` | the habit step settles the week's bonus on its first study day | `habits_minutes::the_recompute_heals_a_habit_settle_left_undone` |
| `S07813-THE-HABIT-STEP-IS-REGISTERED` | `crates/daemon/src/wiring.rs` | the production fold registers the habit step | `wiring::tests::the_recompute_fold_registers_the_habit_step_in_the_day_steps_phase` |
| `S07814-READGOAL-IS-A-DERIVED-PREFIX` | `crates/progression/src/settle.rs` | `readgoal:` is a derived prefix | `xp_settle::the_habit_sources_are_derived_only_with_a_course_code` |
| `S07815-A-PREFIX-NEEDS-A-COURSE-CODE` | `crates/progression/src/settle.rs` | a derived prefix admits only a valid course code after it | `xp_settle::the_habit_sources_are_derived_only_with_a_course_code` |
| `S07816-A-HABIT-LEVEL-UP-IS-ANNOUNCED` | `crates/coordination/src/habits/minutes.rs` | a habit write that crosses a level announces it | `habits_minutes::a_habit_write_that_raises_the_level_is_announced_once` |

S07810's mutant is also refused by
`xp_census::only_progression_writes_xp_settlement_and_only_coordination_settles`, which holds a
coordination caller outside the recompute to the owner's correction as its cause.

## 10. Amendments, 2026-10-03: the census admissions this part makes

Two census tests hold a population that this part grows. Each admission is recorded here,
insert-only; the body above is unchanged.

- **T1** (manifest, the census of statics). `crates/coordination/tests/relight_order.rs` admits
  habits' data-rights port, `static HABITS: HabitsDataRights = HabitsDataRights;` in
  `crates/coordination/src/data_rights_registry.rs`, into `STATICS`, whose count goes from 17 to 18.
  The census still requires every `static` item in the source of every crate coordination links to
  be written out in that table, and the new one is a registry entry, not a count per day.
- **T2** (manifest, the one-router census). `crates/notifications/tests/one_router.rs` admits the
  minutes log's replies. `COMMAND_REPLIES` gains `read`, `undo` and `habit_callback`, from 17 to 20.
  `COMMAND_CALLERS` gains six edges, from 31 to 37: `Commands::on_message` to `read` and to `undo`,
  `Commands::on_callback` to `habit_callback`, and each of `Commands::read`, `Commands::undo` and
  `Commands::habit_callback` to `send`. Every reply still reaches the chat through the router's port.
- **T3** (manifest, the formal settings' test). `scripts/tests/test_formal_config.py` is changed:
  its `EXPECTED` table names `tla/HabitXpFollowsItsLog`'s entry budget, 300, the value this part
  adds to `config/formal.json`, because that test holds the committed file equal to the table.

## 11. Amendments, 2026-10-03: the manifest's own records

- **T4** (manifest, the move). The planned copy,
  `docs/specs/planned/SPEC-078-the-reading-and-writing-habits-the-habit-board-and-reading-analytics.md`,
  is removed: this part moves the SPEC out of the planned directory and delivers it here.
- **T5** (manifest, a probe's reading). Section 4's row for `crates/bot/src/habits_commands.rs`
  names the commands /read and /undo in plain text rather than in backticks, because the SDD probe
  reads a backticked token that starts with a slash as a path outside the repository. It is the one
  edit above these amendments, and it changes no requirement, criterion or path.

## 12. Amendments, 2026-10-03: the update merge with dev

The update merge with `dev` meets exact lists and counts that this part and `dev` each grew. Each
now holds both sides' items: it grows by exactly `dev`'s items and stays exact, so each is an
addition and none is a weakening. Each is recorded here, insert-only. Above these amendments,
section 3's amended-tests paragraph now says that the two new commands join their set's existing
line.

- **T6** (A24, the fold's order). `wiring::tests::the_recompute_fold_registers_the_habit_step_in_the_day_steps_phase`
  reads the exact slice `steps.get(2..6)`, the streaks step, the progress step, the habit step and
  the day bonuses step, where it read `steps.get(2..5)` without the progress step. `dev`
  registers Road to C2's progress step in `Phase::DaySteps` right after the streaks step, and the
  habit step is registered after it. `dev`'s `the_recompute_fold_registers_road_to_c2s_steps` is
  unchanged. The progress step reads no XP that the habit step settles: its band-up grant reads back
  only its own key's row.
- **T7** (the fold's exact list).
  `wiring::tests::the_recompute_fold_registers_the_analytics_xp_and_streak_steps_in_their_phases`
  holds `dev`'s `(Phase::DaySteps, PROGRESS_STEP)` and `(Phase::Awards, BAND_BADGES_STEP)` beside
  this part's `(Phase::DaySteps, HABITS_STEP)`, which follows the progress step. The list is still
  the whole registration, in its order.
- **T8** (the owner's menu). `crates/bot/src/commands.rs`'s `MENU` holds 14 entries, where this part
  held 13: `dev`'s `progress` follows `records`, and `read` and `undo` stay last.
- **T9** (the menu's set). `commands::the_menu_is_registered_for_the_owners_chat_only` gains
  `progress` on the line that holds `read` and `undo`, and its message counts fourteen commands. At
  the merge the set goes from twelve commands to fourteen.
- **T10** (a golden). `crates/bot/tests/messages/help.msg.json` gains `dev`'s line for /progress
  after the line for /records, and the lines for /read and /undo stay last.
- **T11** (a golden). `crates/bot/tests/messages/start.msg.json` gains the same line in the same
  place.
- **T12** (the census of statics). `crates/coordination/tests/relight_order.rs`'s `STATICS` holds 19,
  where T1 made it 18: `dev`'s `static LADDER` in `crates/economy/src/tariff.rs` sits beside
  `static HABITS`. Its doc comment is `dev`'s, which names the parsed skip tariff ladder.
- **T13** (the one-router census). `crates/notifications/tests/one_router.rs`'s `COMMAND_REPLIES`
  holds 21, where T2 made it 20, because it gains `dev`'s `progress`. Its doc comment now counts
  seventeen replies that send one, where it counted sixteen, and `dev`'s `HANDLER` block stays above
  it.
- **T14** (the one-router census). `COMMAND_CALLERS` holds 39, where T2 made it 37, because it gains
  `dev`'s two edges: `Commands::on_message` to `progress`, and `Commands::progress` to `send`.
- **T15** (the data-rights seeds). `crates/coordination/tests/data_rights_symmetry.rs`'s `SEEDS`
  holds 41, where this part made it 39, because it gains `dev`'s seeds for `skip_days` and
  `skip_card_snapshot`.
- **T16** (the formal settings). `config/formal.json`'s entry budgets and the `EXPECTED` table of
  `scripts/tests/test_formal_config.py`, which T3 holds equal, now hold `dev`'s `tla/LandmarkOnce`
  (420) beside `tla/HabitXpFollowsItsLog` (300), in name order.

## 13. Amendments, 2026-10-03: part 078b (#94)

Part 078b delivers the writing habit and the habit badges: R6 to R9, R17 as amended here, R18 for
the writing toggle, R21's `writing_log` and R22 for the constants it uses. This section holds its
prose, its rows, its census admissions, its amended tests and its files; section 14 holds its
criteria and their fence. Everything above these amendments is unchanged, section 3c included.

Section 3c says that a later pull request moves each of its criteria back verbatim. For part 078b
that sentence is superseded: its criteria, section 3c's 078b rows among them, are delivered in
section 14, and neither section 3's table nor section 3c is edited.

R17 is amended (ruling 93 Q-h): a habit badge is awarded by the fold only, in phase 7, by a habit
badges step registered right before the badge step, and never by an owner's write. Each award goes
through SPEC-073's award port with its mark unset, and the offers that already drain every unmarked
badge celebrate it through the router, once, under `badge:<key>:<tier>`. The badges are judged for
the evaluated study day, never the wall clock's.

How part 078b builds R6 to R9 (ADR-078's amendment of this date decides each choice):

- **The log.** `writing_log` holds one row per confirmed writing course and study day: the code,
  the study day and `created_at`, `STRICT`, with the primary key on the code and the study day.
  A confirmation is a row, and clearing it deletes the row.
- **The writing courses** are the configured courses marked `writing`, never a fixed list. A study
  day pays 75 for each confirmed writing course as `write:<code>`, and 100 as `write:all` only when
  the writing set is non-empty and every writing course is confirmed. With no writing course
  configured nothing is settled, no writing streak counts and no ink badge is earned: this departs
  from the predecessor, whose subset test pays the 100 over an empty set.
- **The reserved code.** A writing course coded `all` is refused at start, naming the setting and no
  value: its `write:<code>` would be the bonus's own source.
- **The registry.** `DERIVED_PREFIXES` gains `write:`, followed by a valid course code as the two
  reading prefixes are.
- **The toggle.** `coordination::habits::writing::{confirm, clear, toggle}` each run ONE write
  holding the log change and the day's settles of `write:<code>` and `write:all`, as the owner's
  correction, and announce a level crossed after commit, as the minutes log's use cases do.
- **The chip carries its day.** A checklist chip's data is `hb:w:<code>:<epoch day>`, at most 34
  bytes. A tap whose day is no longer the current study day toggles nothing and answers with today's
  checklist. `/write` and `/unwrite` act on the current study day. A tap's answer is a new message.
- **The writing step.** A separate phase-4 step, registered right after the habit step, settles each
  evaluated day's `write:` sources from `writing_log` inside that day's write: every writing course,
  every code already held as a `write:<code>` row on that day, and `write:all`, a zero skipped where
  no row exists, as the recompute.
- **The streaks.** The writing streak counts the consecutive study days, ending on the evaluated day
  or the day before, on which every writing course was confirmed; each writing course has its own
  streak. One helper decides which days are all confirmed, and it returns none for an empty set.
- **The badge context** is built for the evaluated day from the logs through it: the entries of
  each log, the writing streak, the configured courses read in the day's week, the week's minutes
  and whether every configured course met the weekly goal. `polyglot_reader` and `bookworm_week`
  need at least one configured course, which departs from the predecessor's `0 >= 0` and `all([])`.
- **The bot.** `/write <course>...` resolves every token first and refuses the whole command when
  one names no writing course; `/unwrite <course>` clears; bare `/write` or `/unwrite` answers with
  the checklist: per writing course its mark, flag, name and own streak, then the writing streak. No
  reply names a course code. The owner's menu gains `write` and `unwrite`.
- **Privacy.** `privacy.json` gains the category `writing-log`, `PRIVACY.md` its line, habits'
  data-rights port declares `writing_log` exported and erased, and the context map's register of
  DeckStreak's own tables gains its row.

Rows S07820 to S07835, in `scripts/mutation-rows.d/S07800-S07899.json` (section 9 reserved S07820
to S07839 for this part):

| row | target | what it guards | killer |
|---|---|---|---|
| `S07820-SEVENTY-FIVE-A-WRITING-DAY` | `crates/habits/src/writing.rs` | 75 a confirmed writing course | `writing_goldens::the_writing_xp_matches_the_predecessors_golden` |
| `S07821-WRITING-ALL-BONUS` | `crates/habits/src/writing.rs` | the bonus of 100 | `writing_goldens::the_writing_xp_matches_the_predecessors_golden` |
| `S07822-NO-WRITING-COURSE-PAYS-NOTHING` | `crates/habits/src/writing.rs` | no day is all confirmed over no writing course | `writing_goldens::no_writing_xp_is_settled_without_a_writing_course` |
| `S07823-ONE-CONFIRMATION-A-DAY` | `migrations/007802_habits_writing_log.sql` | a course is confirmed at most once a study day | `habits_store::a_course_is_confirmed_once_a_day` |
| `S07824-STREAK-ENDS-TODAY-OR-YESTERDAY` | `crates/habits/src/writing.rs` | the streak ends on the day or the day before | `writing_goldens::the_writing_streaks_match_the_predecessors_golden` |
| `S07825-STREAK-NEEDS-EVERY-WRITING-COURSE` | `crates/habits/src/writing.rs` | a day counts only when every writing course is confirmed | `writing_goldens::the_writing_streaks_match_the_predecessors_golden` |
| `S07826-INK-WEEK-AT-7` | `crates/habits/src/writing.rs` | the ink week at 7 days | `habits_constants::the_habit_constants_equal_the_predecessors` |
| `S07827-MARATHON-AT-600` | `crates/habits/src/badges.rs` | the marathon at 600 minutes a week | `habits_constants::the_habit_constants_equal_the_predecessors` |
| `S07828-A-CHIP-TOGGLES-ONLY-ITS-OWN-DAY` | `crates/coordination/src/habits/writing.rs` | a chip drawn for a closed day toggles nothing | `writing_toggle::a_chip_for_a_closed_day_toggles_nothing` |
| `S07829-WRITE-IS-A-DERIVED-PREFIX` | `crates/progression/src/settle.rs` | `write:` is a derived prefix | `xp_settle::the_writing_sources_are_derived_only_with_a_course_code` |
| `S07830-A-WRITING-COURSE-IS-NOT-CODED-ALL` | `crates/kernel/src/courses.rs` | a writing course coded `all` is refused | `courses_config::a_writing_course_coded_all_is_refused` |
| `S07831-THE-HABIT-BADGES-STEP-IS-REGISTERED` | `crates/daemon/src/wiring.rs` | the production fold registers the habit badges step | `wiring::tests::the_recompute_fold_registers_the_habit_badges_step_in_the_awards_phase` |
| `S07832-READING-BADGES-NEED-A-COURSE` | `crates/habits/src/badges.rs` | the reading-set badges need a configured course | `habits_badges_rules::no_course_badge_is_earned_without_its_courses` |
| `S07833-INK-MONTH-AT-30` | `crates/habits/src/writing.rs` | the ink month at 30 days | `habits_constants::the_habit_constants_equal_the_predecessors` |
| `S07834-INK-CENTURY-AT-100` | `crates/habits/src/writing.rs` | the ink century at 100 days | `habits_constants::the_habit_constants_equal_the_predecessors` |
| `S07835-THE-WRITING-STEP-IS-REGISTERED` | `crates/daemon/src/wiring.rs` | the production fold registers the writing step | `wiring::tests::the_recompute_fold_registers_the_writing_step_after_the_habit_step` |

S07817 to S07819 stay free for a fix round of part 078a, and S07836 to S07839 for one of part 078b.

The exact lists, sets, arrays and counts that part 078b grows. Each grows by exactly this part's
items and stays exact, so each is an addition and none is a weakening (ruling 132 Q-14); each is
recorded here, insert-only.

- **T17** (A24, the fold's order). `wiring::tests::the_recompute_fold_registers_the_habit_step_in_the_day_steps_phase`
  reads the exact slice `steps.get(2..7)`: the streaks step, the progress step, the habit step, the
  writing step and the day bonuses step, where it read `steps.get(2..6)` without the writing step.
- **T18** (the fold's exact list).
  `wiring::tests::the_recompute_fold_registers_the_analytics_xp_and_streak_steps_in_their_phases`
  gains `(Phase::DaySteps, WRITING_STEP)` right after `(Phase::DaySteps, HABITS_STEP)` and
  `(Phase::Awards, HABIT_BADGES_STEP)` right before `(Phase::Awards, BADGES_STEP)`.
- **T19** (A23, the derived prefixes).
  `xp_settle::the_habit_sources_are_derived_only_with_a_course_code` asserts `DERIVED_PREFIXES`
  equal to `["read:", "readgoal:", "write:"]`, where it named the two reading prefixes.
- **T20** (A14, the constants). `habits_constants::the_habit_constants_equal_the_predecessors` gains
  six arms, one for each constant this part adds to `habits.constants`, and its examined count goes
  from 6 to 12, the count of the golden's cases.
- **T21** (A22, habits' data rights). `habits_store::the_habit_tables_are_exported_and_erased`
  asserts the declaration `[(MINUTES_LOG_TABLE, true), (WRITING_LOG_TABLE, true)]` and two exported
  tables, where it named `minutes_log` alone; its rows and its erase still judge `minutes_log`.
- **T22** (the owner's menu). `crates/bot/src/commands.rs`'s `MENU` holds 16 entries, where it held
  14: `write` and `unwrite` follow `undo`.
- **T23** (the menu's set). `commands::the_menu_is_registered_for_the_owners_chat_only` gains
  `write` and `unwrite`, and its message counts sixteen commands.
- **T24** (a golden). `crates/bot/tests/messages/help.msg.json` gains the lines for /write and
  /unwrite after the line for /undo.
- **T25** (a golden). `crates/bot/tests/messages/start.msg.json` gains the same lines in the same
  place.
- **T26** (the goldens the bot sends). `commands::every_golden_message_is_what_the_bot_sends`
  renders the seven writing replies beside the minutes log's.
- **T27** (the one-router census). `crates/notifications/tests/one_router.rs`'s `COMMAND_REPLIES`
  holds 23, where it held 21: it gains `write` and `unwrite`. The pull request's weakening table
  records this admission once, under ruling 104 Q-8, in ruling 98's shape.
- **T28** (the one-router census). `COMMAND_CALLERS` holds 43, where it held 39: it gains
  `Commands::on_message` to `write` and to `unwrite`, and each of `Commands::write` and
  `Commands::unwrite` to `send`. A chip tap is answered inside `habit_callback`, which already sends.
- **T29** (the data-rights seeds). `crates/coordination/tests/data_rights_symmetry.rs`'s `SEEDS`
  holds 42, where it held 41: it gains a seeded `writing_log` row.

Part 078b's files:

- Added: `migrations/007802_habits_writing_log.sql`, `crates/habits/src/writing.rs`,
  `crates/habits/src/badges.rs`, `crates/habits/tests/writing_goldens.rs`,
  `crates/habits/tests/habits_badges_rules.rs`, `crates/habits/tests/formal_vectors_habit_writing.rs`,
  `crates/coordination/src/habits/writing.rs`, `crates/coordination/src/recompute/writing.rs`,
  `crates/coordination/src/recompute/habit_badges.rs`, `crates/coordination/tests/writing_toggle.rs`,
  `crates/coordination/tests/habits_badges.rs`, `crates/bot/tests/messages/write-chips.msg.json`,
  `crates/bot/tests/messages/write-confirmed.msg.json`, `crates/bot/tests/messages/write-cleared.msg.json`,
  `crates/bot/tests/messages/write-not-a-writing-course.msg.json`,
  `crates/bot/tests/messages/write-no-writing-course.msg.json`,
  `crates/bot/tests/messages/write-day-closed.msg.json`, `crates/bot/tests/messages/write-failed.msg.json`,
  `tools/parity-oracle/goldens/habit_writing_xp.json`, `tools/parity-oracle/goldens/habit_writing_streak.json`,
  `tools/parity-oracle/goldens/habit_badges.json`, `formal/tla/HabitXpFollowsItsLog/MCHabitWriting.cfg`,
  `formal/tla/HabitXpFollowsItsLog/witness/a-toggle-whose-settle-is-a-write-of-its-own.cfg`,
  `formal/tla/HabitXpFollowsItsLog/witness/a-writing-bonus-paid-with-no-writing-course.cfg`,
  `formal/tla/HabitXpFollowsItsLog/witness/a-chip-tap-that-toggles-the-day-it-is-tapped-on.cfg`,
  `formal/lean/Formal/HabitWriting.lean`, `formal/lean/Formal/HabitWritingVectors.lean`,
  `formal/vectors/habit-writing.jsonl`, `changelog.d/feat-habits-078b.md` and the new `.sqlx/`
  query files.
- Changed: `crates/habits/src/lib.rs`, `crates/habits/src/store.rs`, `crates/habits/src/data_rights.rs`,
  `crates/habits/tests/habits_store.rs`, `crates/habits/tests/habits_constants.rs`,
  `crates/kernel/src/courses.rs`, `crates/kernel/tests/courses_config.rs`,
  `crates/progression/src/settle.rs`, `crates/progression/tests/xp_settle.rs`,
  `crates/coordination/src/habits/mod.rs`, `crates/coordination/src/habits/minutes.rs` (three
  helpers widened to `pub(super)`), `crates/coordination/src/recompute/mod.rs` (two `pub mod` lines),
  `crates/coordination/src/recompute/habits.rs` (one helper widened to `pub(super)`),
  `crates/coordination/tests/data_rights_symmetry.rs`, `crates/bot/src/habits_commands.rs`,
  `crates/bot/src/commands.rs`, `crates/bot/tests/habits_commands.rs`, `crates/bot/tests/commands.rs`,
  `crates/bot/tests/messages/help.msg.json`, `crates/bot/tests/messages/start.msg.json`,
  `crates/notifications/tests/one_router.rs`, `crates/daemon/src/wiring.rs`, `privacy.json`,
  `PRIVACY.md`, `docs/CONTEXT-MAP.md`, `tools/parity-oracle/registry/spec_078.py`, every golden that
  registry builds (its sha256 moves), `scripts/mutation-rows.d/S07800-S07899.json`,
  `formal/tla/HabitXpFollowsItsLog/HabitXpFollowsItsLog.tla`,
  `formal/tla/HabitXpFollowsItsLog/MCHabitXpFollowsItsLog.cfg`, `formal/tla/AwardOnce/AwardOnce.tla`,
  `formal/lean/Formal/Vectors.lean`, `docs/decisions/ADR-078-habit-xp-is-settled-in-the-write-that-changes-its-log.md`,
  `docs/schematics/habit-xp-settles-from-its-log.md`, `docs/red-first/SPEC-078.md` and this SPEC.
- `crates/coordination/src/recompute/badges.rs`: unchanged; the existing offers celebrate a habit badge.
- `crates/coordination/src/progression/badges_view.rs`: unchanged; the habit keys' progress is the board's.
- `crates/progression/src/badges/catalog.rs`: unchanged; it already holds the eight habit keys.
- `crates/bot/src/transport.rs`: unchanged; a chip tap's answer is a new message.
- `crates/daemon/src/role_bot.rs`: unchanged; the bot's courses loader is part 078a's.
- `deploy/config/courses.example.json`: unchanged.
- `config/formal.json`: unchanged; the TLA+ entry keeps its budget of 300 and a Lean entry takes no line.
- `scripts/tests/test_formal_config.py`: unchanged.
- `crates/habits/src/board.rs`: unchanged in this part; delivered by 078c.
- `crates/habits/tests/habits_board.rs`: unchanged in this part; delivered by 078c.
- `crates/api/src/habits_routes.rs`: unchanged in this part; delivered by 078c.
- `crates/api/src/router.rs`: unchanged in this part; delivered by 078c.
- `crates/api/tests/habits_routes.rs`: unchanged in this part; delivered by 078c.
- `web/app/src/routes/habits/`: unchanged in this part; delivered by 078c.
- `web/app/src/lib/habits/`: unchanged in this part; delivered by 078c.
- `web/app/src/lib/routes.ts`: unchanged in this part; delivered by 078c.
- `crates/habits/src/analytics.rs`: unchanged in this part; delivered by 078d.
- `crates/habits/tests/habits_analytics.rs`: unchanged in this part; delivered by 078d.

What part 078b does NOT do, each owned by a tracked item:

- It builds no Mini App writing chip, no habits route and no board streak counter (#95). Those
  routes owe no formal model of their own only while they call
  `coordination::habits::writing::{confirm, clear, toggle}` and nothing else (#95).
- It does not edit the checklist message in place after a chip tap: the edit carries no keyboard
  today, so a tap answers with a new message (#601).
- It imports none of the predecessor's writing confirmations (#61).
- It sends no evening nudge for an unconfirmed writing day (#97).
- It proves no Lean theorem of the reading rules, which the predecessor's goldens close (#93).

## 14. Acceptance criteria of the 2026-10-03 amendment (part 078b)

| id | criterion | decided by |
|---|---|---|
| A7 | each day's writing XP equals the golden of `writing_day_xp` over synthetic writing courses | `the_writing_xp_matches_the_predecessors_golden` |
| A8 | the writing streaks equal the goldens of `writing_streak` and `writing_streaks_by_lang` | `the_writing_streaks_match_the_predecessors_golden` |
| A9 | with no writing course, no `write:` source is settled, even over a planted row of a former writing course | `no_writing_xp_is_settled_without_a_writing_course` |
| A10 | toggling a course twice restores the day's settled writing XP | `toggling_twice_restores_the_days_writing_xp` |
| A10b | a course is confirmed at most once a study day | `a_course_is_confirmed_once_a_day` |
| A10c | a chip drawn for a closed day toggles nothing (planted rollover) | `a_chip_for_a_closed_day_toggles_nothing` |
| A10d | the writing step settles each evaluated day's writing from its log | `the_writing_step_settles_each_days_writing_from_its_log` |
| A14b | the writing and badge constants equal `habits.constants.json` | `the_habit_constants_equal_the_predecessors` (amended, T20) |
| A19a | the habit badge conditions equal the golden of `evaluate_habit_badges` | `the_habit_badge_conditions_match_the_predecessors_golden` |
| A19b | no ink badge without a writing course; no `polyglot_reader` or `bookworm_week` without a course | `no_course_badge_is_earned_without_its_courses` |
| A19 | the habit badges of every case are awarded once and celebrated once, by the fold | `habit_badges_are_awarded_once_as_the_predecessors_golden` |
| A22b | `writing_log` is exported and erased | `the_writing_log_is_exported_and_erased` |
| A23b | the `write:` sources are derived only with a course code | `the_writing_sources_are_derived_only_with_a_course_code` |
| A32 | a writing course coded `all` is refused, naming the setting and no value | `a_writing_course_coded_all_is_refused` |
| A33 | /write and /unwrite confirm, clear and refuse as their goldens say | `write_confirms_clears_and_refuses_as_its_goldens_say` |
| A34 | the writing commands join the owner's menu | `the_writing_commands_join_the_owners_menu` |
| A35 | the fold registers the habit badges step in the awards phase, right before the badge step | `the_recompute_fold_registers_the_habit_badges_step_in_the_awards_phase` |
| A35b | the fold registers the writing step in the day-steps phase, right after the habit step | `the_recompute_fold_registers_the_writing_step_after_the_habit_step` |
| A36 | every writing chip fits Telegram's 64 bytes and names its day | `every_writing_chip_fits_telegrams_bound_and_names_its_day` |
| A37 | the writing day's XP equals the Lean port's vectors | `the_writing_xp_equals_the_lean_ports_vectors` |

```acceptance
A7: cargo test -p deck-streak-habits --test writing_goldens -- --exact the_writing_xp_matches_the_predecessors_golden
A8: cargo test -p deck-streak-habits --test writing_goldens -- --exact the_writing_streaks_match_the_predecessors_golden
A9: cargo test -p deck-streak-habits --test writing_goldens -- --exact no_writing_xp_is_settled_without_a_writing_course
A10: cargo test -p deck-streak-coordination --test writing_toggle -- --exact toggling_twice_restores_the_days_writing_xp
A10b: cargo test -p deck-streak-habits --test habits_store -- --exact a_course_is_confirmed_once_a_day
A10c: cargo test -p deck-streak-coordination --test writing_toggle -- --exact a_chip_for_a_closed_day_toggles_nothing
A10d: cargo test -p deck-streak-coordination --test writing_toggle -- --exact the_writing_step_settles_each_days_writing_from_its_log
A14b: cargo test -p deck-streak-habits --test habits_constants -- --exact the_habit_constants_equal_the_predecessors
A19a: cargo test -p deck-streak-habits --test habits_badges_rules -- --exact the_habit_badge_conditions_match_the_predecessors_golden
A19b: cargo test -p deck-streak-habits --test habits_badges_rules -- --exact no_course_badge_is_earned_without_its_courses
A19: cargo test -p deck-streak-coordination --test habits_badges -- --exact habit_badges_are_awarded_once_as_the_predecessors_golden
A22b: cargo test -p deck-streak-habits --test habits_store -- --exact the_writing_log_is_exported_and_erased
A23b: cargo test -p deck-streak-progression --test xp_settle -- --exact the_writing_sources_are_derived_only_with_a_course_code
A32: cargo test -p deck-streak-kernel --test courses_config -- --exact a_writing_course_coded_all_is_refused
A33: cargo test -p deck-streak-bot --test habits_commands -- --exact write_confirms_clears_and_refuses_as_its_goldens_say
A34: cargo test -p deck-streak-bot --test habits_commands -- --exact the_writing_commands_join_the_owners_menu
A35: cargo test -p deck-streak-daemon --lib -- --exact wiring::tests::the_recompute_fold_registers_the_habit_badges_step_in_the_awards_phase
A35b: cargo test -p deck-streak-daemon --lib -- --exact wiring::tests::the_recompute_fold_registers_the_writing_step_after_the_habit_step
A36: cargo test -p deck-streak-bot --test habits_commands -- --exact every_writing_chip_fits_telegrams_bound_and_names_its_day
A37: cargo test -p deck-streak-habits --test formal_vectors_habit_writing -- --exact the_writing_xp_equals_the_lean_ports_vectors
```

Section 3c's `078b:`-prefixed lines are not fence lines; this fence holds part 078b's criteria
alone. A31 (section 3's two `data_rights_symmetry` lines) re-judges `writing_log` under the same
names once its seed is added (T29).
