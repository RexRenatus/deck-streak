# SPEC-079: a focus block runs on the service's clock, ends once, and deep work earns its XP once

- **Wave:** W3. **Issue:** #98, #99 (epic #4). **Context(s):** `deck-streak-focus` (the timer, its
  claim, the focus log, focus XP, the focus streak, the board, the badge conditions and the nudge
  rule); `deck-streak-coordination` (the start, stop and fire use cases, the settle of focus XP, the
  badge awards, the block-end celebration, and the recompute's focus step); `deck-streak-api` and
  `deck-streak-bot` (the routes, the commands, and the timer each role arms); the Mini App
  (`web/app`).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-041 (the one router and its
  origin rule), ADR-071 (the recompute settles each study day once, in order), ADR-072 (derived XP
  is settled, and a closed day's never falls), ADR-079 (a block ends on the service's clock, is
  claimed once by its generation, and is announced as a celebration) and ADR-087 (the focus subjects
  are the owner's configured courses and subjects).
- **Prerequisites:** SPEC-071 (the recompute, and the day's backlog-zero grant the combo reads),
  SPEC-072 (progression's `settle` port), SPEC-073 (the badge award port), SPEC-026 (the bot's
  command table, callbacks and transport), SPEC-041 (the router). **Mutation band:**
  `S07900-S07999`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-079.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists.** `crates/focus/src/` holds only `lib.rs` (read at `dev` c3d769b), and the Mini
  App has no focus screen (`web/app/src/lib/routes.ts` lists `/` and `/about`).
- **What is ported.** The predecessor's `focus-timer` and `focus-xp-badges-stats` features: the pure
  rules of `focus.py` (block XP, the day's cap, the focus streak, the board, the breakdown, the
  badges, the countdown cadence, the clamp), the layer rules of
  `pipeline_layers/focus.py:FocusLayer` (the entry clamp and force-abandon, the weekly goal and the
  combo, the nudge's condition, the daily minutes), and the timer's transitions in
  `bot.py:CommandBot` (start, cycle, pause, resume, add five minutes, stop, complete, the cycle's
  advance, and the restart's re-arm), at predecessor `27ee2bc`.
- **What changes, and why.** The predecessor's only surface is the bot, which edits a countdown
  message every few seconds from one process. DeckStreak has two surfaces and two long-running roles
  (`api` and `bot`): the Mini App counts down on the client from the server's end instant, the bot
  keeps its countdown for blocks it started, and either role can hold a timer for the same block.
  So the predecessor's claim, one fired flag on the timer's row, would let a timer armed for a
  replaced block claim the new block's end, and its order (the claim committed before the block's
  row) leaves a claimed block uncredited after a crash. ADR-079 keys the claim by the block's
  generation and commits the claim, the block's row and the timer's next state in one write.
- **Corrections to the issues, from the predecessor's code.**
  - #99 says the XP curve is floored. `focus.py:focus_xp` rounds with Python's `round`, half to
    even, so a block whose interpolated XP lands exactly on a half earns its even neighbour, which a
    floor would not always give. The golden's `tie` class holds those blocks.
  - #99 says the combo needs a qualifying block.
    `pipeline_layers/focus.py:FocusLayer._recompute_focus_xp` pays it when the day's completed
    blocks total at least the qualifying floor (`constants.FOCUS_MIN_QUALIFY_MIN`) and the day
    carries a backlog-zero grant; the predecessor also re-derives today's combo at each recompute,
    after the day's backlog-zero grant is settled, so a queue cleared after the block still pays it.
  - #99 says the rest day between two qualifying days is bridged. `focus.py:focus_streak` bridges
    at most one rest day in the whole run, and only one with a counted day before it.
  - #99's nudge also requires today to be under the daily goal
    (`pipeline_layers/focus.py:FocusLayer._focus_nudge_body`).
  - The predecessor's weekly bonus is the source `focusgoal:` on the week's first day, not a
    week-suffixed source; this SPEC keeps that key.
- **Traps a hand port falls into.** Rust's `f64::round` sends a tie away from zero, and Python's
  `round` sends it to the even neighbour (`f64::round_ties_even`). The stop's minutes are the
  elapsed seconds over 60, rounded the same way, never more than planned. The cycle counts work
  blocks, and the long break follows the block whose count is a multiple of
  `constants.FOCUS_CYCLE_LONG_EVERY`.
- **What the parity oracle proves.** Seventeen goldens (§7): every rule above, over seeded synthetic
  blocks, days and subjects, and the constants the port uses verbatim.
- **Prerequisites.** SPEC-071, SPEC-072, SPEC-073, SPEC-026 and SPEC-041, each named in the header.

## 2. Requirements

The focus timer (#98)

R1. `deck-streak-focus` owns the timer as one row, `focus_timer`. Its state is `idle`, `running` or
    `paused`, and its `block` is a generation number that every start increments: a one-off block, a
    cycle's first block and a cycle's next block. The row holds the subject, the kind (`work` or
    `break`), the planned minutes, the end instant in epoch milliseconds while running, the
    remaining seconds while paused, the cycle's count of finished work blocks, whether the block
    belongs to a cycle, whether its end has been claimed, and, for a block started from the bot, its
    countdown message.
R2. A start takes a subject, minutes and a kind. The minutes are clamped to the predecessor's range
    (`goldens/focus_clamp_minutes.json`, `focus.py:clamp_minutes` with
    `constants.FOCUS_MAX_ENTRY_MIN` in `goldens/focus.constants.json`). The subject resolves among
    the configured focus subjects, the course codes and the courses file's `focus_subjects` read
    from the kernel's `Courses` (ADR-087), and an unknown one falls back to the predecessor's
    default subject (`goldens/focus.constants.json`, `curriculum.DEFAULT_FOCUS_SUBJECT`). The
    pickers offer the predecessor's work, extra and break presets (`constants.FOCUS_WORK_PRESETS`,
    `FOCUS_EXTRA_PRESETS`, `FOCUS_BREAK_PRESETS` in `goldens/focus.constants.json`). A start while
    another block is active first ends that block as a stop does (R5), so a replaced block is never
    lost.
R3. A cycle starts a work block of the predecessor's cycle length, and each time a block ends it
    starts the next: after a work block a short break, or a long break after every fourth work
    block; after a break a work block. The lengths and the rule equal the golden of
    `bot.py:CommandBot._cycle_advance` (`goldens/focus_cycle_advance.json`; the constants
    `FOCUS_CYCLE_WORK_MIN`, `FOCUS_CYCLE_SHORT_BREAK_MIN`, `FOCUS_CYCLE_LONG_BREAK_MIN` and
    `FOCUS_CYCLE_LONG_EVERY` in `goldens/focus.constants.json`). A stop ends the whole cycle.
R4. Pause keeps the remaining seconds, and resume sets a new end from them. Adding five minutes
    raises the planned minutes up to the entry cap and moves the end, or the paused remainder, by
    exactly the minutes it added (`goldens/focus_add_minutes.json`,
    `bot.py:CommandBot._focus_add_locked`).
R5. A stop, or a block that a new start replaces, records the minutes actually run: the elapsed
    seconds over 60 rounded half to even, never more than planned
    (`goldens/focus_stop_minutes.json`, `bot.py:CommandBot._focus_stop_locked`). Every entry is
    clamped and force-abandoned as the predecessor's is (`goldens/focus_record_entry.json`,
    `pipeline_layers/focus.py:FocusLayer._record_focus_impl`): the planned and actual minutes are
    clamped to their ranges, and a block under the qualifying floor
    (`constants.FOCUS_MIN_QUALIFY_MIN`) is recorded abandoned and uncertified and earns nothing. A
    work block that runs to its end is recorded at its planned minutes and certified.
R6. A block's end is claimed once (ADR-079). The timer's claim is one conditional update that
    succeeds only for the block's own generation, while the row is running, unclaimed and past its
    end; a stop or a replacement claims by the generation alone. The claim, the block's `focus_log`
    row and the timer's next state (idle, or the cycle's next block) commit in one write through the
    kernel's repository base (SPEC-020 R16). A timer armed for an older generation, a paused block,
    or an end that adding five minutes moved claims nothing. The claim's SQL is a constant string in
    the focus repository, so a hand-proved row can mutate it (§9).
R7. Each long-running role, `api` and `bot`, arms an in-process timer (`tokio::time::sleep_until`)
    for the running block when the role starts and whenever it writes the row, fires through R6's
    claim, and stops its timer at shutdown. At start a role decides from the row as the
    predecessor's restart does (`goldens/focus_rearm.json`, `bot.py:CommandBot._rearm_focus_timer`):
    an idle row does nothing, a paused row stays paused, a running row past its end fires once (the
    catch-up), and a running row before its end is armed for the rest. The predecessor's stale-claim
    branch (a claim committed before its credit) cannot arise under R6, so no interrupted-cycle
    notice is sent.
R8. The Mini App counts down on the client from the server's end instant and the server's own
    current instant in the same response, so a phone whose clock is off counts to the same end; when
    its countdown reaches zero it asks the server again.
R9. A block started from the bot gets one countdown message with Pause, Resume, add-five and Stop
    controls, which the bot role edits at the adaptive interval `focus.py:tick_interval` returns
    (`goldens/focus_tick_interval.json`), re-reading the row at each edit
    so a change made in the Mini App shows at the next one. Every `fo:` callback's data fits
    Telegram's 64 bytes. The bot keeps `/focus` (the picker, or a typed `/focus <minutes>
    [subject]`), `/cycle [subject]` and `/focusstats`, in the owner's menu only.
R10. A block that ends on its timer (a one-off block's end, a cycle's advance, or a restart's
    catch-up) is announced through the one router as the celebration event `focus_block` at T2
    (ADR-079), once per block: its dedupe key names the generation, never a date. Quiet hours defer
    it, as every celebration. It names the minutes, today's focus XP and the focus streak, and in a
    cycle the block that started next. A stop is answered where it was made: the bot edits its
    countdown message, and the Mini App shows the result in its response.
    `notifications-policy.json`'s ladder gains `"focus_block": "T2"`, and its `deviations` gain the
    key `ladder.events.focus_block` with ADR-079.

Deep-work XP, the streak, the board and the nudge (#99)

R11. A completed block's XP equals the golden of `focus.py:focus_xp` (`goldens/focus_xp.json`):
    interpolation on the predecessor's curve (`constants.FOCUS_XP_CURVE`), rounded half to even as
    Python's `round` rounds (the golden's `tie` class), continued at the last segment's slope past
    its last point, and nothing below the qualifying floor. The day's focus XP is the sum over the
    day's completed blocks, capped (`goldens/focus_day_xp.json`, `focus.py:focus_day_xp`,
    `constants.FOCUS_XP_DAILY_CAP`).
R12. Focus XP is derived XP (ADR-072), and this delivery adds its three sources to ADR-072's closed
    registry of derived sources. Coordination settles it through progression's `settle` port
    with the predecessor's three sources, equal to the golden of
    `pipeline_layers/focus.py:FocusLayer._recompute_focus_xp` (`goldens/focus_recompute_xp.json`):
    `focus` on the block's study day (the day's sum); `focusgoal:` on the week's first study day
    (`constants.FOCUS_WEEKLY_GOAL_XP` when the week's completed minutes reach
    `constants.FOCUS_WEEKLY_GOAL_MIN`, weeks Monday to Sunday); and `focus_combo` on the day
    (`constants.FOCUS_COMBO_XP` when the day carries a backlog-zero grant and its completed blocks
    total at least the qualifying floor). They are settled after each recorded block, and at each
    recompute by a step registered in phase 4 of SPEC-071's fold, after phase 2 settles the day's
    backlog-zero grant (ADR-071), so a queue cleared after the block still pays the combo when the
    day is settled. A second settle of the same inputs writes nothing new.
R13. A day counts toward the focus streak when its completed minutes reach the daily goal
    (`constants.FOCUS_DAILY_GOAL_MIN`). The streak runs back from today or yesterday and bridges at
    most one rest day in the whole run, and only a rest day with a counted day before it
    (`goldens/focus_streak.json`, `focus.py:focus_streak`, `constants.FOCUS_STREAK_REST_GRACE`).
    Today's and the week's minutes against their goals, the streak, today's completed blocks and
    today's abandoned blocks equal `goldens/focus_progress.json` (`focus.py:compute_focus`); the
    minutes and blocks per subject equal `goldens/focus_subject_breakdown.json`
    (`focus.py:subject_breakdown`, over synthetic subjects).
R14. The eight focus badges' conditions equal the golden of `focus.py:evaluate_focus_badges`
    (`goldens/focus_badges.json`, with their thresholds in `goldens/focus.constants.json`). They are
    evaluated after each recorded block and at each recompute by a step registered in phase 7 of
    SPEC-071's fold, and coordination awards each through progression's badge port (SPEC-073), which
    keeps every badge once. Only a timer-completed block is certified; a block
    without the certificate (a manual back-log) never counts toward the longest certified block, so
    it never earns the deep-diver badge.
R15. The focus nudge's eligibility equals the golden of
    `pipeline_layers/focus.py:FocusLayer._focus_nudge_body` (`goldens/focus_nudge_eligible.json`): a
    focus streak of at least `constants.FOCUS_NUDGE_MIN_STREAK` with today still under the daily
    goal. The focus context exposes the rule; the evening coordinator sends the nudge (#117).
R16. `GET /api/focus` returns the timer, the server's current instant, today and the week against
    their goals, the focus streak, the subject breakdown and today's focus XP. `GET
    /api/focus/days?weeks=N` returns each study day's completed minutes over the last N weeks, equal
    to the golden of `pipeline_layers/focus.py:FocusLayer.focus_daily_minutes`
    (`goldens/focus_daily_minutes.json`). `POST /api/focus/start`, `/api/focus/cycle`,
    `/api/focus/pause`, `/api/focus/resume`, `/api/focus/add` and `/api/focus/stop` change the
    timer. Every route answers the owner's session only (SPEC-024).
R17. The Mini App's `/focus` screen shows the picker, the countdown with its controls, today's and
    the week's rings against their goals, the focus streak and the subject breakdown. It joins the
    route table, which the accessibility coverage test holds equal to the screens. The heatmap and
    the minutes-over-time line are drawn from `GET /api/focus/days` by the charts (SPEC-085).

Data rights

R18. `focus_log` and `focus_timer` are `STRICT`, each with `created_at`, created by
    `migrations/007901_focus_timer_and_log.sql`; `focus_timer` holds exactly one row (`CHECK (id =
    1)`). Focus's data-rights port exports and erases `focus_log` and resets `focus_timer` in place
    to idle. Both are registered in the context map's register of DeckStreak's own tables,
    coordination's data-rights registry, `privacy.json` and `PRIVACY.md`.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a completed block's XP equals the golden of `focus.py:focus_xp` for every case, the ties and the floor included | `the_block_xp_matches_the_predecessors_golden` |
| A2 | the day's focus XP equals the golden of `focus.py:focus_day_xp`, the cap included | `the_day_xp_matches_the_predecessors_golden` |
| A3 | the focus streak equals the golden of `focus.py:focus_streak`, one bridged rest day at most | `the_focus_streak_matches_the_predecessors_golden` |
| A4 | today's and the week's progress, the streak and the day's counts equal the golden of `focus.py:compute_focus` | `the_focus_progress_matches_the_predecessors_golden` |
| A5 | the minutes and blocks per subject equal the golden of `focus.py:subject_breakdown` | `the_subject_breakdown_matches_the_predecessors_golden` |
| A6 | every focus constant the port uses equals the constants golden | `the_focus_constants_equal_the_predecessors` |
| A7 | an entry is clamped and force-abandoned as `FocusLayer._record_focus_impl` does | `a_block_entry_is_clamped_as_the_predecessor_clamps_it` |
| A8 | an early stop at fourteen minutes records an abandoned, uncertified block that earns nothing | `an_early_stop_at_fourteen_minutes_earns_nothing` |
| A9 | a cycle over five blocks runs as the golden of `CommandBot._cycle_advance`, the long break after the fourth | `a_cycle_advances_as_the_predecessors_cycle_does` |
| A10 | adding five minutes stops at the entry cap and moves the end by the minutes added | `adding_five_minutes_stops_at_the_entry_cap` |
| A11 | at start a role decides from the row as the golden of `CommandBot._rearm_focus_timer` | `a_restart_decides_as_the_predecessor_re_arms` |
| A12 | the bot's countdown cadence equals the golden of `focus.py:tick_interval` | `the_bot_countdown_cadence_matches_the_predecessors_golden` |
| A13 | a second claim of one block's end wins nothing | `a_second_claim_of_one_block_wins_nothing` |
| A14 | a timer armed for a replaced, paused or extended block claims nothing | `a_stale_timer_claims_nothing` |
| A15 | a timer claims nothing before its block's end | `a_timer_claims_nothing_before_its_end` |
| A16 | the timer table refuses a second row | `the_timer_is_one_row` |
| A17 | the focus badges equal the golden of `focus.py:evaluate_focus_badges` | `the_focus_badges_match_the_predecessors_golden` |
| A18 | an uncertified block never earns the deep-diver badge, however long | `a_manual_block_never_earns_deep_diver` |
| A19 | the nudge's eligibility equals the golden of `FocusLayer._focus_nudge_body` | `the_nudge_eligibility_matches_the_predecessors_golden` |
| A20 | the focus tables are exported, and an erase empties the log and resets the timer to idle | `the_focus_tables_are_exported_and_erased` |
| A21 | two roles armed for one block both fire after a restart, and the block is credited exactly once | `a_restart_mid_block_credits_the_block_exactly_once` |
| A22 | a block that ends on its timer raises one `focus_block` celebration at T2 through the router | `a_finished_block_raises_one_focus_block_celebration` |
| A23 | the weekly goal and the combo settle as the golden of `FocusLayer._recompute_focus_xp`, and a second settle writes nothing new | `the_weekly_goal_and_combo_settle_once` |
| A24 | the daily minutes series equals the golden of `FocusLayer.focus_daily_minutes` | `the_focus_days_series_matches_the_predecessors_golden` |
| A25 | every focus route answers the owner's session only | `the_focus_routes_answer_only_the_owner` |
| A26 | `/focus`, `/cycle` and `/focusstats` are in the owner's menu, and a typed `/focus 47` starts a 47-minute block | `the_focus_commands_are_in_the_owners_menu` |
| A27 | every `fo:` callback's data fits 64 bytes, over every subject and preset | `every_focus_callback_fits_sixty_four_bytes` |
| A28 | the policy declares the ladder event `focus_block` at T2 with a deviation that names ADR-079 | `test_the_focus_block_event_is_declared_with_its_deviation` |
| A29 | the Mini App counts down to the server's end whatever the phone's clock says | `counts down to the server's end whatever the phone's clock says` |

```acceptance
A1: cargo test -p deck-streak-focus --test focus_xp -- --exact the_block_xp_matches_the_predecessors_golden
A2: cargo test -p deck-streak-focus --test focus_xp -- --exact the_day_xp_matches_the_predecessors_golden
A3: cargo test -p deck-streak-focus --test focus_progress -- --exact the_focus_streak_matches_the_predecessors_golden
A4: cargo test -p deck-streak-focus --test focus_progress -- --exact the_focus_progress_matches_the_predecessors_golden
A5: cargo test -p deck-streak-focus --test focus_progress -- --exact the_subject_breakdown_matches_the_predecessors_golden
A6: cargo test -p deck-streak-focus --test focus_constants -- --exact the_focus_constants_equal_the_predecessors
A7: cargo test -p deck-streak-focus --test focus_timer -- --exact a_block_entry_is_clamped_as_the_predecessor_clamps_it
A8: cargo test -p deck-streak-focus --test focus_timer -- --exact an_early_stop_at_fourteen_minutes_earns_nothing
A9: cargo test -p deck-streak-focus --test focus_timer -- --exact a_cycle_advances_as_the_predecessors_cycle_does
A10: cargo test -p deck-streak-focus --test focus_timer -- --exact adding_five_minutes_stops_at_the_entry_cap
A11: cargo test -p deck-streak-focus --test focus_timer -- --exact a_restart_decides_as_the_predecessor_re_arms
A12: cargo test -p deck-streak-focus --test focus_timer -- --exact the_bot_countdown_cadence_matches_the_predecessors_golden
A13: cargo test -p deck-streak-focus --test focus_claim -- --exact a_second_claim_of_one_block_wins_nothing
A14: cargo test -p deck-streak-focus --test focus_claim -- --exact a_stale_timer_claims_nothing
A15: cargo test -p deck-streak-focus --test focus_claim -- --exact a_timer_claims_nothing_before_its_end
A16: cargo test -p deck-streak-focus --test focus_claim -- --exact the_timer_is_one_row
A17: cargo test -p deck-streak-focus --test focus_badges -- --exact the_focus_badges_match_the_predecessors_golden
A18: cargo test -p deck-streak-focus --test focus_badges -- --exact a_manual_block_never_earns_deep_diver
A19: cargo test -p deck-streak-focus --test focus_badges -- --exact the_nudge_eligibility_matches_the_predecessors_golden
A20: cargo test -p deck-streak-focus --test focus_rights -- --exact the_focus_tables_are_exported_and_erased
A21: cargo test -p deck-streak-coordination --test focus_fire -- --exact a_restart_mid_block_credits_the_block_exactly_once
A22: cargo test -p deck-streak-coordination --test focus_fire -- --exact a_finished_block_raises_one_focus_block_celebration
A23: cargo test -p deck-streak-coordination --test focus_fire -- --exact the_weekly_goal_and_combo_settle_once
A24: cargo test -p deck-streak-coordination --test focus_fire -- --exact the_focus_days_series_matches_the_predecessors_golden
A25: cargo test -p deck-streak-api --test focus_routes -- --exact the_focus_routes_answer_only_the_owner
A26: cargo test -p deck-streak-bot --test focus_commands -- --exact the_focus_commands_are_in_the_owners_menu
A27: cargo test -p deck-streak-bot --test focus_commands -- --exact every_focus_callback_fits_sixty_four_bytes
A28: python3 -m unittest discover -s scripts/tests -p test_focus_policy_event.py -k test_the_focus_block_event_is_declared_with_its_deviation
A29: pnpm exec vitest run web/app/src/lib/focus/countdown.test.ts -t "counts down to the server's end whatever the phone's clock says"
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The notifications-policy, privacy-gdpr, accessibility,
telegram-platform and rust-service packs stay enforced; no row of theirs is deferred for this
delivery, and none changes state when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `notifications-policy.json`, examining every kind and every ladder event, `focus_block` included, the policy is well formed, and the one value that differs from the baseline is recorded by a deviation whose ADR names its key | the notifications-policy pack |
| B2 | over the shipped source under `crates/`, examining every delivery call, no call is made outside the router module, so a block's end reaches the owner only through `route` | the notifications-policy pack |
| B3 | over `privacy.json`, `PRIVACY.md` and `migrations/007901_focus_timer_and_log.sql`, examining every table the migration creates, `focus_log` and `focus_timer` each belong to a category with its basis, retention, export and erase | the privacy-gdpr pack |
| B4 | over `web/app/src/routes/focus/` and `web/app/src/lib/focus/`, examining the new screen and its components, the accessibility checks pass | the accessibility pack |
| B5 | over `crates/bot/src/focus_commands.rs` and `crates/bot/src/commands.rs`, examining every focus command and callback, the menu is the owner's and every callback's data fits the Bot API's limit | the telegram-platform pack |
| B6 | over `crates/focus/src/` and `crates/coordination/src/focus/`, examining every task the roles spawn, each timer stops at shutdown and no buffer is unbounded | the rust-service pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/focus/src/lib.rs` | `deck-streak-focus` | changed: the modules below |
| `crates/focus/src/constants.rs` | `deck-streak-focus` | added: the constants the golden proves |
| `crates/focus/src/xp.rs` | `deck-streak-focus` | added: block XP and the day's capped sum |
| `crates/focus/src/streak.rs` | `deck-streak-focus` | added: the focus streak, today and the week, the subject breakdown |
| `crates/focus/src/timer.rs` | `deck-streak-focus` | added: the timer's transitions, the cycle, add five minutes, the stop's minutes, the restart decision, the countdown cadence |
| `crates/focus/src/repository.rs` | `deck-streak-focus` | added: `focus_log` and `focus_timer` through the kernel's base; the claim's SQL as constant strings |
| `crates/focus/src/badges.rs` | `deck-streak-focus` | added: the focus badge conditions and the nudge's eligibility |
| `crates/focus/src/data_rights.rs` | `deck-streak-focus` | added: the data-rights port (SPEC-021's rule) |
| `crates/focus/Cargo.toml` | `deck-streak-focus` | changed: the kernel, `sqlx`, `tokio` (`time`), `thiserror`; dev-dependencies `serde`, `serde_json` (the golden reader, SPEC-029 R8), `tempfile`, and `tokio`'s `test-util` |
| `crates/focus/tests/focus_xp.rs` | `deck-streak-focus` | added: A1, A2 |
| `crates/focus/tests/focus_progress.rs` | `deck-streak-focus` | added: A3 to A5 |
| `crates/focus/tests/focus_constants.rs` | `deck-streak-focus` | added: A6 |
| `crates/focus/tests/focus_timer.rs` | `deck-streak-focus` | added: A7 to A12 |
| `crates/focus/tests/focus_claim.rs` | `deck-streak-focus` | added: A13 to A16 |
| `crates/focus/tests/focus_badges.rs` | `deck-streak-focus` | added: A17 to A19 |
| `crates/focus/tests/focus_rights.rs` | `deck-streak-focus` | added: A20 |
| `migrations/007901_focus_timer_and_log.sql` | `deck-streak-focus` | added: `focus_log` and `focus_timer`, `STRICT`, with `created_at` (SPEC-020 R15, R18) |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `crates/coordination/src/focus/mod.rs` | `deck-streak-coordination` | added: start, cycle, pause, resume, add, stop and the fire path, the settle of focus XP, the badge awards, the block-end celebration |
| `crates/coordination/src/focus/timer.rs` | `deck-streak-coordination` | added: the timer a role arms, and the restart decision applied |
| `crates/coordination/src/recompute/focus.rs` | `deck-streak-coordination` | added: the recompute's focus XP step, registered in phase 4 of SPEC-071's fold, and its focus badge step, registered in phase 7 |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the focus module |
| `crates/coordination/tests/focus_fire.rs` | `deck-streak-coordination` | added: A21 to A24 |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: focus's port joins the registry |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded `focus_log` and `focus_timer` rows |
| `crates/api/src/focus_routes.rs` | `deck-streak-api` | added: the focus routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the focus routes mounted |
| `crates/api/tests/focus_routes.rs` | `deck-streak-api` | added: A25 |
| `crates/bot/src/focus_commands.rs` | `deck-streak-bot` | added: the focus, cycle and focus-stats commands, the `fo:` callbacks and the countdown message |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the command table and the owner's menu |
| `crates/bot/tests/focus_commands.rs` | `deck-streak-bot` | added: A26, A27 |
| `crates/daemon/src/role_api.rs` | `deck-streak-daemon` | changed: the api role arms the focus timer at start |
| `crates/daemon/src/role_bot.rs` | `deck-streak-daemon` | changed: the bot role arms the focus timer at start (the role SPEC-026 adds) |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the focus use cases joined to progression's settle and badge ports and to the router |
| `web/app/src/routes/focus/+page.svelte` | miniapp | added: the focus screen |
| `web/app/src/lib/focus/countdown.ts` | miniapp | added: the countdown from the server's end and the server's instant |
| `web/app/src/lib/focus/countdown.test.ts` | miniapp | added: A29 |
| `web/app/src/lib/focus/Countdown.svelte` | miniapp | added: the countdown and its controls |
| `web/app/src/lib/focus/FocusRings.svelte` | miniapp | added: today's and the week's rings, the streak, the breakdown |
| `web/app/src/lib/routes.ts` | miniapp | changed: the focus screen joins the route table |
| `notifications-policy.json` | repo | changed: `ladder.events.focus_block` at T2, and its deviation naming ADR-079 |
| `scripts/tests/test_focus_policy_event.py` | repo | added: A28 |
| `docs/CONTEXT-MAP.md` | docs | changed: `focus_log` and `focus_timer` in the register of DeckStreak's own tables |
| `privacy.json` | repo | changed: the `focus-log` and `focus-timer` categories |
| `PRIVACY.md` | repo | changed: one line per category |
| `tools/parity-oracle/registry/spec_079.py` | repo | added: this SPEC's registrations (§7) |
| `tools/parity-oracle/goldens/focus_xp.json` | repo | added: the golden of `focus.py:focus_xp` (function) |
| `tools/parity-oracle/goldens/focus_day_xp.json` | repo | added: the golden of `focus.py:focus_day_xp` (function) |
| `tools/parity-oracle/goldens/focus_streak.json` | repo | added: the golden of `focus.py:focus_streak` (adapter: dates from epoch days) |
| `tools/parity-oracle/goldens/focus_progress.json` | repo | added: the golden of `focus.py:compute_focus` (adapter: focus rows from JSON) |
| `tools/parity-oracle/goldens/focus_subject_breakdown.json` | repo | added: the golden of `focus.py:subject_breakdown` (adapter: synthetic subjects) |
| `tools/parity-oracle/goldens/focus_tick_interval.json` | repo | added: the golden of `focus.py:tick_interval` (function) |
| `tools/parity-oracle/goldens/focus_clamp_minutes.json` | repo | added: the golden of `focus.py:clamp_minutes` (function) |
| `tools/parity-oracle/goldens/focus_badges.json` | repo | added: the golden of `focus.py:evaluate_focus_badges` (adapter: a badge context from JSON) |
| `tools/parity-oracle/goldens/focus_record_entry.json` | repo | added: the golden of `pipeline_layers/focus.py:FocusLayer._record_focus_impl` (adapter: a stand-in layer with a recording store) |
| `tools/parity-oracle/goldens/focus_recompute_xp.json` | repo | added: the golden of `pipeline_layers/focus.py:FocusLayer._recompute_focus_xp` (adapter: a stand-in layer with a stub store) |
| `tools/parity-oracle/goldens/focus_nudge_eligible.json` | repo | added: the golden of `pipeline_layers/focus.py:FocusLayer._focus_nudge_body` (adapter: a patched summary) |
| `tools/parity-oracle/goldens/focus_daily_minutes.json` | repo | added: the golden of `pipeline_layers/focus.py:FocusLayer.focus_daily_minutes` (adapter: a stub store) |
| `tools/parity-oracle/goldens/focus_stop_minutes.json` | repo | added: the golden of `bot.py:CommandBot._focus_stop_locked` (adapter: a stand-in bot and a manual clock) |
| `tools/parity-oracle/goldens/focus_cycle_advance.json` | repo | added: the golden of `bot.py:CommandBot._cycle_advance` (adapter: a stand-in bot recording the next block) |
| `tools/parity-oracle/goldens/focus_add_minutes.json` | repo | added: the golden of `bot.py:CommandBot._focus_add_locked` (adapter: a stand-in bot and a manual clock) |
| `tools/parity-oracle/goldens/focus_rearm.json` | repo | added: the golden of `bot.py:CommandBot._rearm_focus_timer` (adapter: a stand-in bot and a stub timer row) |
| `tools/parity-oracle/goldens/focus.constants.json` | repo | added: the constants golden (constants) |
| `scripts/mutation-rows.d/S07900-S07999.json` | repo | added: the hand-proved rows (§9) |
| `Cargo.lock` | workspace | changed |
| `docs/specs/SPEC-079-a-focus-block-runs-on-the-services-clock-and-deep-work-earns-once.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-079-a-focus-block-ends-on-the-services-clock-armed-by-each-long-running-role-and-claimed-once.md` | docs | changed: accepted |
| `docs/red-first/SPEC-079.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It sends no evening focus nudge: the evening coordinator does, and this SPEC builds only its
  eligibility (#117).
- It adds no focus block to the daily digest or the weekly report (#129, #130).
- It offers no manual back-log of a block; the agent's tools bring it, and such a block is
  uncertified (#157).
- It draws no focus heatmap and no minutes-over-time line; the charts draw them from this SPEC's
  series (#152).
- It builds no settings screen for the focus subjects or the nudge's switch (#57).
- It keeps no all-time focus totals for the public page (#156).
- It imports none of the predecessor's focus rows (#61).

## 6. Risks

- **Two roles credit one block twice.** Detected by A13, A14 and A21, and by the rows on the claim's
  SQL (§9); prevented by the claim's generation and the one write (R6).
- **A tie rounds away from zero.** Detected by A1's `tie` cases and A8's stop minutes; the port uses
  `f64::round_ties_even`.
- **A phone's clock moves the end.** Detected by A29, which runs the countdown with a phone clock
  set apart from the server's.
- **A block that ends in quiet hours is announced later than the predecessor announced it.**
  Recorded by ADR-079; the Mini App shows the end on the client meanwhile, and the router's decision
  ledger shows the deferral.
- **A role that armed the only timer is down at the block's end.** The block ends at that role's
  next start, as a catch-up (A11, A21); each role's unit restarts it on failure and its watchdog
  catches a hung one (SPEC-032).
- **A stand-in adapter drifts from the predecessor's code.** Each adapter's note names what it
  builds, and each golden records the predecessor's commit; review holds both to `27ee2bc`.
- **The bot's countdown edits meet Telegram's rate limits.** The cadence is the predecessor's (A12),
  and the bot honours a 429's wait (SPEC-026).

## 7. Parity goldens

Every golden is registered in `tools/parity-oracle/registry/spec_079.py` and generated from the
predecessor at `27ee2bc` over seeded synthetic inputs. Days are epoch day numbers and instants epoch
milliseconds; no golden carries a date string (SPEC-029 R3).

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `focus_xp` | `focus.py:focus_xp` | function | nothing; cases at the floor, the anchors, the ties and past the last point |
| `focus_day_xp` | `focus.py:focus_day_xp` | function | nothing; lists of block minutes around the cap |
| `focus_streak` | `focus.py:focus_streak` | adapter | the qualifying days and today as dates from epoch days; classes for a bridged rest day, a second rest day and a two-day gap |
| `focus_progress` | `focus.py:compute_focus` | adapter | focus rows from JSON, today as a date; returns the progress fields |
| `focus_subject_breakdown` | `focus.py:subject_breakdown` | adapter | patches the private subject table with synthetic subjects, recorded in the note |
| `focus_tick_interval` | `focus.py:tick_interval` | function | nothing; remaining seconds on both sides of each threshold |
| `focus_clamp_minutes` | `focus.py:clamp_minutes` | function | nothing; the entry cap as `hi` |
| `focus_badges` | `focus.py:evaluate_focus_badges` | adapter | a badge context from JSON; returns the keys |
| `focus_record_entry` | `pipeline_layers/focus.py:FocusLayer._record_focus_impl` | adapter | a stand-in layer holding a store that records the entry it is given, with the XP, badge and summary steps patched to do nothing |
| `focus_recompute_xp` | `pipeline_layers/focus.py:FocusLayer._recompute_focus_xp` | adapter | a stand-in layer holding a stub store with the case's day blocks, week rows and backlog-zero amount; returns the three grants |
| `focus_nudge_eligible` | `pipeline_layers/focus.py:FocusLayer._focus_nudge_body` | adapter | patches the summary to the case's streak, today and goal, and the renderer to a marker; returns whether a body is produced |
| `focus_daily_minutes` | `pipeline_layers/focus.py:FocusLayer.focus_daily_minutes` | adapter | a stub store with the case's rows and today; returns minutes by epoch day |
| `focus_stop_minutes` | `bot.py:CommandBot._focus_stop_locked` | adapter | a stand-in bot holding only the focus fields, a stub pipeline that grants the claim and records the block, and a manual monotonic clock |
| `focus_cycle_advance` | `bot.py:CommandBot._cycle_advance` | adapter | a stand-in bot whose block start and message edit record the next block |
| `focus_add_minutes` | `bot.py:CommandBot._focus_add_locked` | adapter | a stand-in bot and a manual clock; returns the planned minutes and the seconds added |
| `focus_rearm` | `bot.py:CommandBot._rearm_focus_timer` | adapter | a stand-in bot whose pipeline returns the case's timer row, with the end as an instant relative to a fixed now; records whether it fires, re-arms with the remaining seconds, stays paused or clears |
| `focus.constants` | `constants.FOCUS_*` and `curriculum.DEFAULT_FOCUS_SUBJECT` | constants | nothing |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `focus_log` | `focus` | `migrations/007901_focus_timer_and_log.sql` | its focus log: one row per block with its day, subject, planned and actual minutes, and whether it was abandoned or certified, plus the owner's note | exported and erased |
| `focus_timer` | `focus` | `migrations/007901_focus_timer_and_log.sql` | its one-row focus timer; whether a live block is carried over is the import's decision (#61) | reset in place to idle |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S07901-QUALIFY-FLOOR-IS-FIFTEEN` | `crates/focus/src/constants.rs` | a block under the floor is abandoned and earns nothing | `focus_timer::an_early_stop_at_fourteen_minutes_earns_nothing` |
| `S07902-THE-DAY-CAP` | `crates/focus/src/constants.rs` | the day's focus XP stops at the cap | `focus_xp::the_day_xp_matches_the_predecessors_golden` |
| `S07903-THE-ENTRY-CAP` | `crates/focus/src/constants.rs` | an entry and add-five stop at the entry cap | `focus_timer::adding_five_minutes_stops_at_the_entry_cap` |
| `S07904-THE-LONG-BREAK-EVERY-FOURTH` | `crates/focus/src/constants.rs` | the long break follows every fourth work block | `focus_timer::a_cycle_advances_as_the_predecessors_cycle_does` |
| `S07905-A-CLAIM-NEEDS-AN-UNCLAIMED-ROW` | `crates/focus/src/repository.rs` | the claim's SQL requires an unclaimed row | `focus_claim::a_second_claim_of_one_block_wins_nothing` |
| `S07906-A-CLAIM-NAMES-ITS-GENERATION` | `crates/focus/src/repository.rs` | the claim's SQL requires the block's own generation | `focus_claim::a_stale_timer_claims_nothing` |
| `S07907-A-CLAIM-WAITS-FOR-THE-END` | `crates/focus/src/repository.rs` | the timer's claim requires the end to have passed | `focus_claim::a_timer_claims_nothing_before_its_end` |
| `S07908-THE-TIMER-IS-ONE-ROW` | `migrations/007901_focus_timer_and_log.sql` | the timer table's one-row check (a script row with a cargo killer) | `focus_claim::the_timer_is_one_row` |
| `S07909-THE-NUDGE-NEEDS-A-STREAK-OF-THREE` | `crates/focus/src/constants.rs` | the nudge's minimum streak | `focus_badges::the_nudge_eligibility_matches_the_predecessors_golden` |
| `S07910-DEEP-DIVER-IS-CERTIFIED-ONLY` | `crates/focus/src/badges.rs` | only a certified block counts toward the longest certified block | `focus_badges::a_manual_block_never_earns_deep_diver` |
