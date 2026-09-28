# SPEC-078: reading minutes and writing earn XP once, and feed one habit board and honest analytics

- **Wave:** W3. **Issue:** #93, #94, #95, #96 (epic #4). **Context(s):** `deck-streak-habits` (the
  minutes log of book reading and its XP, the writing confirmations and their XP, the goal and writing
  streaks, the board, freshness, the reading analytics, the habit badge conditions, `minutes_log`,
  `writing_log`); `deck-streak-coordination` (settling habit XP, correcting it on an undo or a
  toggle, the recompute's habit step, awarding habit badges, joining analytics' per-course daily
  stats); `deck-streak-api`, `deck-streak-bot` and the Mini App (the Habits tab).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-041 (every celebration passes
  through the one router), ADR-071 (the recompute reaches every study day once, in order), ADR-072
  (habit XP is settled, and the owner's own correction may lower it) and ADR-087 (the configured
  courses name the reading and the writing courses).
- **Prerequisites:** SPEC-026 (the bot's command table), SPEC-040 (the XP ledger), SPEC-041 (the
  router), SPEC-071 (the courses configuration and the per-course daily stats), SPEC-072 (the settle
  port and the level-up occasion) and SPEC-073 (the badge award port). **Mutation band:**
  `S07800-S07899`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-078.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists yet.** At `dev` c3d769b `crates/habits/src/` holds only `lib.rs`, and the crate's
  manifest names the kernel alone. No habit entry, writing confirmation, habit XP, board or reading
  analytics exists.
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
  - An entry of 0 minutes or over 600 is refused, not capped, and a note is cut at 200 characters.
  - Undo removes the newest entry in the order entries were logged, whatever its day.
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

The minutes log of book reading (#93)

R1. A minutes log entry names a course, a number of minutes and an optional note. The course token
    resolves as the predecessor's `curriculum.py:resolve_lang` does, over the configured courses of the
    kernel's `Courses` (SPEC-071 loads it, and habits never loads the file): a course code passes
    first, then a course's one-letter alias, and any other token or a course that is not configured is
    refused (`goldens/habit_course_token.json`). Minutes from 1 to 600 are accepted and any other
    number is refused (`constants.READING_MAX_ENTRY_MIN`); the note is cut to 200 characters; the
    entry's day is the current study day.
R2. `/read <course> <minutes> [note]` logs an entry. `/read <minutes> [note]` logs it for the most-used
    course, the course with the most minutes this week, else of all time
    (`goldens/habit_most_used.json`, `HabitsLayer.most_used_reading_lang`), and asks which course when
    there is none. Bare `/read` opens a course picker and then the minute presets of 10, 15, 20, 30,
    45 and 60 (`bot._READ_PRESETS` in `goldens/habits.constants.json`); a logged entry offers an Undo
    button.
R3. A course's XP for a study day is `min(240, 2 × that day's minutes)` (`goldens/habit_reading_xp.json`,
    `habits.py:reading_xp`; `constants.READING_XP_PER_MIN`, `READING_XP_DAILY_CAP_PER_LANG`), settled
    (ADR-072) as `read:<code>` on that day, track language; this delivery adds `read:<code>`,
    `readgoal:<code>`, `write:<code>` and `write:all` to ADR-072's closed registry of derived sources. The week's bonus is 150 when the course's
    minutes in the Monday-to-Sunday week reach 210, settled as `readgoal:<code>` on the week's Monday,
    and 0 otherwise (`constants.READING_GOAL_XP_BONUS`, `READING_WEEKLY_GOAL_MIN`,
    `WEEK_START_WEEKDAY`). Both are re-derived after every entry and undo and equal
    `goldens/habit_reading_settled.json` (`HabitsLayer._recompute_reading_xp`).
R4. `/undo` removes the newest entry, whatever its day, and re-derives that day's XP and that week's
    bonus. An amount it lowers, a closed day's included, is the owner's own correction (ADR-072); the
    settled amounts after an entry and its undo equal those before the entry.
R5. An entry, an undo and a recompute each write through the kernel's write base, so they serialise
    (`BEGIN IMMEDIATE`). The habit step registers in phase 4 of SPEC-071's fold, before the derived
    bonuses and the coin mint, which count habit XP in the day's base, and re-derives the habit XP of
    every study day of the fold that holds an entry: a failure between an entry and its settle heals
    at the next recompute, with the same rule's value.

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

R10. The board's week is the Monday-to-Sunday week of the current study day. Each reading course
    shows its minutes against the 210-minute goal, its percent, the minutes remaining, whether it met
    the goal, the days left and the minutes a day it needs (`goldens/habit_weekly_bars.json`,
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
    `MARATHON_READER_WEEK_MIN`).
R18. When a habit write raises the level, SPEC-072's level-up occasion is raised through the router.

The screens and the data

R19. The routes under `/api/habits` (the summary, an entry, an undo, a writing toggle, the analytics,
    the correlation and the weekly series) serve the owner only (SPEC-024); any other caller gets 401
    or 403 and no data.
R20. The Mini App's `/habits` tab holds the minutes logger (course chips, the minute presets and a
    stepper, an optional note), the week's bars, the writing toggle chips, the streak counters and each
    course's freshness; `/habits/analytics` shows the rollup, the effort, the library, each course's
    verdict and the anomaly as text and tables. Both join `ROUTES`.
R21. Habits owns `minutes_log` (id, code, study day, minutes, note, `created_at`) and `writing_log`
    (code, study day, `created_at`, unique on code and study day), both `STRICT`, created by
    `migrations/007801_habits_minutes_and_writing.sql` (SPEC-020 R15, R18). Each is registered in the
    context map's register of DeckStreak's own tables, declared in `privacy.json` (the categories
    `minutes-log`, which holds the owner's free-text notes, and `writing-log`), given a line per
    category in `PRIVACY.md`, and listed as exported and erased by habits' new data-rights port, which
    joins coordination's registry; the symmetry test seeds each.
R22. Every constant this SPEC uses equals `goldens/habits.constants.json`, held by a test.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a course's per-day XP of every case equals the golden of `habits.py:reading_xp` (examined count reported, zero refused) | `the_minutes_xp_matches_the_predecessors_golden` |
| A2 | every course token resolves as the golden of `resolve_lang` says, and minutes outside 1 to 600 are refused | `an_entry_resolves_its_course_and_refuses_minutes_outside_the_bounds` |
| A3 | a minutes-only entry is logged for the course the golden of `most_used_reading_lang` names | `minutes_only_logs_the_most_used_course` |
| A4 | the settled `read:` and `readgoal:` amounts after each case's entries equal the golden of `_recompute_reading_xp` | `the_minutes_xp_settles_as_the_predecessors_golden` |
| A5 | an entry and its undo leave every settled amount as it was, a closed day's included | `undo_restores_the_prior_settled_xp` |
| A6 | a settle left undone by a failure after an entry is made by the next recompute, with the same value | `the_recompute_heals_a_habit_settle_left_undone` |
| A7 | the writing XP of every case equals the golden of `habits.py:writing_day_xp` | `the_writing_xp_matches_the_predecessors_golden` |
| A8 | the writing streaks of every case equal the golden of `writing_streak` and `writing_streaks_by_lang` | `the_writing_streaks_match_the_predecessors_golden` |
| A9 | with no writing course configured, no writing XP is settled and no writing streak counts | `no_writing_xp_is_settled_without_a_writing_course` |
| A10 | toggling a confirmation twice leaves the day's settled writing XP as it was | `toggling_twice_restores_the_days_writing_xp` |
| A11 | the week's bars of every case equal the golden of `habits.py:compute_reading` | `the_weekly_bars_match_the_predecessors_golden` |
| A12 | the goal streaks of every case equal the golden of `reading_goal_streak` and `reading_goal_streaks_by_lang`, and none counts with no course | `the_goal_streaks_match_the_predecessors_golden` |
| A13 | each course's days since and label equal the goldens of `_reading_days_since_by_lang` and `_reading_freshness_suffix` | `the_freshness_label_matches_the_predecessors_rule` |
| A14 | every habit constant equals `goldens/habits.constants.json` | `the_habit_constants_equal_the_predecessors` |
| A15 | `/habits` and the summary route report the same minutes, streaks and confirmations for one study day | `habits_and_the_route_report_the_same_summary` |
| A16 | the rollup, the effort and the library of every case equal their goldens | `the_rollup_effort_and_library_match_the_predecessors_goldens` |
| A17 | the correlation and its interval equal their goldens for every case, every verdict band's wording included | `the_correlation_matches_the_predecessors_golden` |
| A18 | the anomaly of every case equals the golden of `weekly_volume_anomaly`, a z of exactly 3.0 unflagged | `the_volume_anomaly_matches_the_predecessors_golden` |
| A19 | the habit badges of every case equal the golden of `evaluate_habit_badges`, each awarded and celebrated once | `habit_badges_are_awarded_once_as_the_predecessors_golden` |
| A20 | every habit route answers the owner and refuses every other caller with no data | `the_habit_routes_answer_only_the_owner` |
| A21 | the minutes logger logs a preset of minutes for the chosen course | `logs a preset of minutes for the chosen course` |
| A22 | habits' data-rights port lists both tables as exported and erased, and an erase leaves them empty | `the_habit_tables_are_exported_and_erased` |

```acceptance
A1: cargo test -p deck-streak-habits --test habits_minutes -- --exact the_minutes_xp_matches_the_predecessors_golden
A2: cargo test -p deck-streak-habits --test habits_minutes -- --exact an_entry_resolves_its_course_and_refuses_minutes_outside_the_bounds
A3: cargo test -p deck-streak-coordination --test habits_minutes -- --exact minutes_only_logs_the_most_used_course
A4: cargo test -p deck-streak-coordination --test habits_minutes -- --exact the_minutes_xp_settles_as_the_predecessors_golden
A5: cargo test -p deck-streak-coordination --test habits_minutes -- --exact undo_restores_the_prior_settled_xp
A6: cargo test -p deck-streak-coordination --test habits_minutes -- --exact the_recompute_heals_a_habit_settle_left_undone
A7: cargo test -p deck-streak-habits --test writing_goldens -- --exact the_writing_xp_matches_the_predecessors_golden
A8: cargo test -p deck-streak-habits --test writing_goldens -- --exact the_writing_streaks_match_the_predecessors_golden
A9: cargo test -p deck-streak-habits --test writing_goldens -- --exact no_writing_xp_is_settled_without_a_writing_course
A10: cargo test -p deck-streak-coordination --test writing_toggle -- --exact toggling_twice_restores_the_days_writing_xp
A11: cargo test -p deck-streak-habits --test habits_board -- --exact the_weekly_bars_match_the_predecessors_golden
A12: cargo test -p deck-streak-habits --test habits_board -- --exact the_goal_streaks_match_the_predecessors_golden
A13: cargo test -p deck-streak-habits --test habits_board -- --exact the_freshness_label_matches_the_predecessors_rule
A14: cargo test -p deck-streak-habits --test habits_board -- --exact the_habit_constants_equal_the_predecessors
A15: cargo test -p deck-streak-bot --test habits_commands -- --exact habits_and_the_route_report_the_same_summary
A16: cargo test -p deck-streak-habits --test habits_analytics -- --exact the_rollup_effort_and_library_match_the_predecessors_goldens
A17: cargo test -p deck-streak-habits --test habits_analytics -- --exact the_correlation_matches_the_predecessors_golden
A18: cargo test -p deck-streak-habits --test habits_analytics -- --exact the_volume_anomaly_matches_the_predecessors_golden
A19: cargo test -p deck-streak-coordination --test habits_badges -- --exact habit_badges_are_awarded_once_as_the_predecessors_golden
A20: cargo test -p deck-streak-api --test habits_routes -- --exact the_habit_routes_answer_only_the_owner
A21: pnpm exec vitest run web/app/src/lib/habits/MinutesLogger.test.ts -t "logs a preset of minutes for the chosen course"
A22: cargo test -p deck-streak-habits --test habits_store -- --exact the_habit_tables_are_exported_and_erased
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr, telegram-platform, notifications-policy
and accessibility packs stay enforced; no check is deferred or lifted for this delivery, so the private
wiring does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md` and `crates/habits/src/data_rights.rs`: the `minutes-log` category declares the owner's free-text notes with purpose, basis and retention, the `writing-log` category its confirmations, and export and erase cover both tables | the privacy-gdpr pack |
| B2 | over `crates/bot/src/habits_commands.rs` and `crates/bot/src/commands.rs`: every habit command is registered for the owner's chat only, and every callback's data stays inside Telegram's 64-byte bound | the telegram-platform pack |
| B3 | over every file under `crates/coordination/src/`: each habit badge's celebration goes through the one router | the notifications-policy pack |
| B4 | over `web/app/src/routes/habits/+page.svelte`, `web/app/src/routes/habits/analytics/+page.svelte` and `web/app/src/lib/habits/`: both screens pass the accessibility audit in both Telegram colour schemes | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/habits/src/minutes.rs` | `deck-streak-habits` | added: the minutes log's entries, the course token, the per-day XP and the weekly bonus |
| `crates/habits/src/writing.rs` | `deck-streak-habits` | added: the writing confirmations, their XP and streaks |
| `crates/habits/src/board.rs` | `deck-streak-habits` | added: the week's bars, the goal streaks, freshness and the one summary |
| `crates/habits/src/analytics.rs` | `deck-streak-habits` | added: the rollup, the effort, the library, the correlation and its interval, the anomaly |
| `crates/habits/src/badges.rs` | `deck-streak-habits` | added: the habit badge conditions |
| `crates/habits/src/store.rs` | `deck-streak-habits` | added: the repository over `minutes_log` and `writing_log` |
| `crates/habits/src/data_rights.rs` | `deck-streak-habits` | added: habits' data-rights port |
| `crates/habits/src/lib.rs` | `deck-streak-habits` | changed: the modules above |
| `crates/habits/Cargo.toml` | `deck-streak-habits` | changed: `sqlx`, `thiserror` and `serde_json` (ADR-003, ADR-029), and the golden reader's `serde` and `serde_json` as dev-dependencies (SPEC-029 R8) |
| `crates/habits/tests/habits_minutes.rs` | `deck-streak-habits` | added: A1, A2 |
| `crates/habits/tests/writing_goldens.rs` | `deck-streak-habits` | added: A7 to A9 |
| `crates/habits/tests/habits_board.rs` | `deck-streak-habits` | added: A11 to A14 |
| `crates/habits/tests/habits_analytics.rs` | `deck-streak-habits` | added: A16 to A18 |
| `crates/habits/tests/habits_store.rs` | `deck-streak-habits` | added: A22 |
| `migrations/007801_habits_minutes_and_writing.sql` | `deck-streak-habits` | added: both tables, `STRICT`, with `created_at` and their keys |
| `crates/coordination/src/habits/mod.rs` | `deck-streak-coordination` | added: entries, undo and toggles with their settles, the badge awards and the analytics join |
| `crates/coordination/src/recompute/habits.rs` | `deck-streak-coordination` | added: the habit step, which re-derives the habit XP of the fold's days that hold an entry |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: the habit step in phase 4 and the habit badges in phase 7 of SPEC-071's fold |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the habits module |
| `crates/coordination/tests/habits_minutes.rs` | `deck-streak-coordination` | added: A3 to A6 |
| `crates/coordination/tests/writing_toggle.rs` | `deck-streak-coordination` | added: A10 |
| `crates/coordination/tests/habits_badges.rs` | `deck-streak-coordination` | added: A19 |
| `crates/api/src/habits_routes.rs` | `deck-streak-api` | added: the routes under /api/habits |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the habit routes |
| `crates/api/tests/habits_routes.rs` | `deck-streak-api` | added: A20 |
| `crates/bot/src/habits_commands.rs` | `deck-streak-bot` | added: /read, /undo, /write, /unwrite, /habits, /readstats, /readtrend, /correlate and their callbacks |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the command table and the owner's menu gain the eight commands |
| `crates/bot/tests/habits_commands.rs` | `deck-streak-bot` | added: A15 |
| `web/app/src/routes/habits/+page.svelte` | miniapp | added: the Habits tab |
| `web/app/src/routes/habits/analytics/+page.svelte` | miniapp | added: the analytics screen |
| `web/app/src/lib/habits/MinutesLogger.svelte` | miniapp | added: the minutes logger |
| `web/app/src/lib/habits/WeekBars.svelte` | miniapp | added: the week's bars with freshness |
| `web/app/src/lib/habits/WritingChips.svelte` | miniapp | added: the writing toggle chips |
| `web/app/src/lib/habits/habits.ts` | miniapp | added: the routes' client and types |
| `web/app/src/lib/habits/MinutesLogger.test.ts` | miniapp | added: A21 |
| `web/app/src/lib/routes.ts` | miniapp | changed: /habits and /habits/analytics join `ROUTES` |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `minutes_log` and `writing_log` |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the registry gains habits' port |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded rows for both tables |
| `privacy.json` | repo | changed: the `minutes-log` and `writing-log` categories |
| `PRIVACY.md` | repo | changed: one line per category |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `tools/parity-oracle/registry/spec_078.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/habit_reading_xp.json` | repo | added: the golden of `habits.py:reading_xp` (function) |
| `tools/parity-oracle/goldens/habit_course_token.json` | repo | added: the golden of `curriculum.py:resolve_lang` (adapter; synthetic courses and aliases) |
| `tools/parity-oracle/goldens/habit_most_used.json` | repo | added: the golden of `pipeline_layers/habits.py:HabitsLayer.most_used_reading_lang` (adapter; a stub store) |
| `tools/parity-oracle/goldens/habit_reading_settled.json` | repo | added: the golden of `pipeline_layers/habits.py:HabitsLayer._recompute_reading_xp` (adapter; a stub store) |
| `tools/parity-oracle/goldens/habit_writing_xp.json` | repo | added: the golden of `habits.py:writing_day_xp` (adapter; synthetic writing courses) |
| `tools/parity-oracle/goldens/habit_writing_streak.json` | repo | added: the golden of `habits.py:writing_streak` and `writing_streaks_by_lang` (adapter; epoch days) |
| `tools/parity-oracle/goldens/habit_weekly_bars.json` | repo | added: the golden of `habits.py:compute_reading` (adapter; synthetic courses) |
| `tools/parity-oracle/goldens/habit_goal_streaks.json` | repo | added: the golden of `habits.py:reading_goal_streak` and `reading_goal_streaks_by_lang` (adapter; epoch days) |
| `tools/parity-oracle/goldens/habit_days_since.json` | repo | added: the golden of `pipeline_layers/habits.py:HabitsLayer._reading_days_since_by_lang` (adapter; a stub store) |
| `tools/parity-oracle/goldens/habit_freshness.json` | repo | added: the golden of `telegram.py:_reading_freshness_suffix` (function) |
| `tools/parity-oracle/goldens/habit_rollup.json` | repo | added: the golden of `habit_analytics.py:reading_rollup` (adapter; epoch days) |
| `tools/parity-oracle/goldens/habit_effort.json` | repo | added: the golden of `habit_analytics.py:combined_effort` (adapter; epoch days) |
| `tools/parity-oracle/goldens/habit_library.json` | repo | added: the golden of `habit_analytics.py:reading_library` (adapter; epoch days for its last day) |
| `tools/parity-oracle/goldens/habit_correlation.json` | repo | added: the golden of `habit_analytics.py:reading_retention_correlation` (adapter; epoch days, every verdict band) |
| `tools/parity-oracle/goldens/habit_fisher_z.json` | repo | added: the golden of `habit_analytics.py:fisher_z_ci` (function) |
| `tools/parity-oracle/goldens/habit_volume_anomaly.json` | repo | added: the golden of `habit_analytics.py:weekly_volume_anomaly` (adapter; epoch days for its week start) |
| `tools/parity-oracle/goldens/habit_badges.json` | repo | added: the golden of `habits.py:evaluate_habit_badges` (adapter; a synthetic badge context over synthetic courses) |
| `tools/parity-oracle/goldens/habits.constants.json` | repo | added: the constants this SPEC uses (constants) |
| `scripts/mutation-rows.d/S07800-S07899.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-078-the-reading-and-writing-habits-the-habit-board-and-reading-analytics.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-078.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It sends no evening habit check-in, the predecessor's home of the "gone cold" label (#97).
- It writes no habit block into the daily digest or the weekly report (#129, #130).
- It draws no chart: the eight-week trend and the correlation's scatter are SPEC-085's (#152).
- It exposes no habit entry, summary or analytics through the agent's tools (#157).
- It writes no habit number into the vault's stats file (#153).
- It publishes no habit total on the public page (#156).
- It imports none of the predecessor's entries or confirmations (#61).

## 6. Risks

- **A course's alias or code leaks the owner's language set into a golden.** Prevented by the
  adapters, which patch the predecessor's constants with synthetic courses, and detected by the public
  scrub over the tree.
- **The coefficients round differently from Python's `round`.** Detected by A17, whose golden's cases
  include values that sit on a half at the second decimal, compared exactly.
- **An undo or a toggle lowers a closed day's XP where ADR-072 forbids a fall.** Bounded by ADR-072's
  exception, which admits only the owner's own correction of a manual entry; A5 and A10 pin it.
- **A failure between an entry and its settle leaves the XP behind the log.** Detected by A6, and
  healed by the recompute's habit step.
- **An empty course set pays or counts for nothing done.** Detected by A9 and A12.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_078.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic; no golden carries the owner's language
codes, aliases or names, a note of the owner's, or a calendar string.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `habit_reading_xp` | `habits.py:reading_xp` | function | none: 0, negative, below and at the cap, past it |
| `habit_course_token` | `curriculum.py:resolve_lang` | adapter | patches `curriculum.LANGUAGES_BY_CODE` and `LANG_ALIASES` with synthetic courses and aliases; codes, aliases, case and space, unknown tokens |
| `habit_most_used` | `pipeline_layers/habits.py:HabitsLayer.most_used_reading_lang` | adapter | a stub store returning the week's and all-time minutes per course in code order; ties, an empty week and no entries |
| `habit_reading_settled` | `pipeline_layers/habits.py:HabitsLayer._recompute_reading_xp` | adapter | a stub store holding the case's entries; it returns each `read:` amount on its day and each `readgoal:` amount on its week's Monday, as epoch days |
| `habit_writing_xp` | `habits.py:writing_day_xp` | adapter | synthetic writing courses passed as `codes`; none, some and all confirmed |
| `habit_writing_streak` | `habits.py:writing_streak`, `writing_streaks_by_lang` | adapter | synthetic writing courses and confirmations on epoch days, with gaps, today and yesterday |
| `habit_weekly_bars` | `habits.py:compute_reading` | adapter | patches `habits.LANGUAGES_BY_CODE` with synthetic courses; each weekday, met and unmet goals |
| `habit_goal_streaks` | `habits.py:reading_goal_streak`, `reading_goal_streaks_by_lang` | adapter | synthetic courses and entries on epoch days across met, unmet and current weeks |
| `habit_days_since` | `pipeline_layers/habits.py:HabitsLayer._reading_days_since_by_lang` | adapter | a stub store holding entries on epoch days; courses read today, long ago and never |
| `habit_freshness` | `telegram.py:_reading_freshness_suffix` | function | none: a missing key, none, zero, a few days, exactly 21 and more |
| `habit_rollup` | `habit_analytics.py:reading_rollup` | adapter | synthetic courses and entries on epoch days, inside and outside the window |
| `habit_effort` | `habit_analytics.py:combined_effort` | adapter | synthetic courses, entries, confirmations and per-course daily stats on epoch days |
| `habit_library` | `habit_analytics.py:reading_library` | adapter | synthetic entries with synthetic material notes; its last day returned as an epoch day |
| `habit_correlation` | `habit_analytics.py:reading_retention_correlation` | adapter | synthetic courses, entries and per-course daily stats on epoch days; every verdict band, too few weeks, and values on a half at the second decimal |
| `habit_fisher_z` | `habit_analytics.py:fisher_z_ci` | function | none: n below 4, at 4, rho at and past the clamp |
| `habit_volume_anomaly` | `habit_analytics.py:weekly_volume_anomaly` | adapter | synthetic per-course daily stats on epoch days; too few weeks, zero variance, z at exactly 3.0 and past it both ways; its week start returned as an epoch day |
| `habit_badges` | `habits.py:evaluate_habit_badges` | adapter | patches `habits.READING_LANG_CODES` with synthetic courses and builds a badge context from the case |
| `habits.constants` | `constants.READING_XP_PER_MIN`, `READING_XP_DAILY_CAP_PER_LANG`, `READING_GOAL_XP_BONUS`, `READING_WEEKLY_GOAL_MIN`, `WEEK_START_WEEKDAY`, `READING_MAX_ENTRY_MIN`, `READING_COLD_DAYS`, `WRITING_XP_PER_DAY`, `WRITING_XP_ALL_THREE_BONUS`, `WRITING_STREAK_WEEK`, `WRITING_STREAK_MONTH`, `WRITING_STREAK_CENTURY`, `MARATHON_READER_WEEK_MIN`; `habit_analytics.DEFAULT_ANALYTICS_WINDOW_DAYS`, `DEFAULT_CORR_WEEKS`, `MIN_CORR_WEEKS`, `FISHER_Z_95`, `SPEARMAN_SE_FACTOR`, `VOLUME_ANOMALY_TRAILING_WEEKS`, `VOLUME_ANOMALY_MIN_PRIOR_WEEKS`, `VOLUME_ANOMALY_Z_THRESHOLD`; `bot._READ_PRESETS` | constants | none |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `minutes_log` | `deck-streak-habits` | `migrations/007801_habits_minutes_and_writing.sql` | its `reading_log`, one row per entry with its language code as the course code and its day as an epoch day | exported and erased |
| `writing_log` | `deck-streak-habits` | `migrations/007801_habits_minutes_and_writing.sql` | its `writing_log`, one row per confirmed language and day | exported and erased |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S07801-TWO-XP-A-MINUTE` | `crates/habits/src/minutes.rs` | two XP a minute | `habits_minutes::the_minutes_xp_matches_the_predecessors_golden` |
| `S07802-DAILY-CAP-240` | `crates/habits/src/minutes.rs` | the per-course daily cap of 240 | `habits_minutes::the_minutes_xp_matches_the_predecessors_golden` |
| `S07803-ENTRY-BOUND-600` | `crates/habits/src/minutes.rs` | an entry refused above 600 minutes | `habits_minutes::an_entry_resolves_its_course_and_refuses_minutes_outside_the_bounds` |
| `S07804-WEEKLY-GOAL-210` | `crates/habits/src/minutes.rs` | the weekly goal of 210 minutes | `habits_board::the_habit_constants_equal_the_predecessors` |
| `S07805-WRITING-ALL-BONUS` | `crates/habits/src/writing.rs` | the bonus only when every writing course is confirmed | `writing_goldens::the_writing_xp_matches_the_predecessors_golden` |
| `S07806-NO-WRITING-COURSE-PAYS-NOTHING` | `crates/habits/src/writing.rs` | no writing XP with no writing course | `writing_goldens::no_writing_xp_is_settled_without_a_writing_course` |
| `S07807-COLD-AT-21` | `crates/habits/src/board.rs` | the "gone cold" label at 21 days | `habits_board::the_freshness_label_matches_the_predecessors_rule` |
| `S07808-SE-FACTOR` | `crates/habits/src/analytics.rs` | the Fisher z standard error's factor of 1.06 | `habits_analytics::the_correlation_matches_the_predecessors_golden` |
| `S07809-ANOMALY-STRICTLY-ABOVE-THREE` | `crates/habits/src/analytics.rs` | a flag only when |z| exceeds 3.0 | `habits_analytics::the_volume_anomaly_matches_the_predecessors_golden` |
| `S07810-ONE-CONFIRMATION-A-DAY` | `migrations/007801_habits_minutes_and_writing.sql` | the key on `writing_log (code, study day)` (a script row; the cargo killer) | `writing_toggle::toggling_twice_restores_the_days_writing_xp` |
