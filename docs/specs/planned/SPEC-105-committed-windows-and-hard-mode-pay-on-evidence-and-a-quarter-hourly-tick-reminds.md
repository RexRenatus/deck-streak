# SPEC-105: committed windows and hard mode pay on evidence, and a quarter-hourly tick reminds

- **Wave:** W5. **Issue:** #109 (committed windows), #115 (hard mode), in epic #6, and the standby
  notice SPEC-076 left to #109. **Context(s):** `deck-streak-discipline` (discipline's state row and
  its switches, the windows' occurrence, bounds, effort and verdict, the booking's refusals, the
  revision of a window and of a night, hard mode's booking, cancellation and settlement, and the
  pending standby notice); `deck-streak-coordination` (the discipline step of the sync cycle, the
  quarter-hourly `discipline_tick` job, the reminder and the notice); `deck-streak-streaks` (the
  writer of the last notice's study day); `deck-streak-notifications` (the policy's kind
  `discipline`); `deck-streak-bot` (`/windows`, `/hardmode`); `deck-streak-api` (discipline's
  routes); the Mini App (`web/app`, the `/discipline` screen).
- **Decided by:** ADR-105 (this SPEC's: discipline's clock is a quarter-hourly tick that reads no
  reviews, and the standby notice waits for it outside quiet hours), ADR-104 (a verdict settles
  after its evidence window and is revised within 7 closed study days; the kind `discipline`),
  ADR-037 (one sync a study day), ADR-027 (jobs are timers with a ledger), ADR-012 (the parity
  oracle), ADR-040 and ADR-071.
- **Prerequisites:** SPEC-100 (the reminder's estimate), SPEC-082 (the wallet's `credit`), SPEC-076
  (the governor, its stored state and the rule `standby_notice`), SPEC-083 (the skip set), SPEC-078
  and SPEC-079 (a study day's reading, writing and focus entries), SPEC-071 (the study day's reviews
  and its rollup's `due_today`), SPEC-041 (the router and its policy), SPEC-027 (the job table) and
  SPEC-023 (the obligation port and the sync cycle). **Mutation band:** `S10500-S10599`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-105.md` (ADR-016).

## 1. The problem, measured

- **What exists.** At `dev` 26263de, `crates/discipline/src/` holds only `lib.rs`, and no discipline
  table exists. `coordination::jobs::TABLE` holds three jobs (`sync` at the rollover hour's minute
  7, `maintenance` at its minute 28, `liveness` hourly at minute 14), and SPEC-027 R1 knows three
  schedule kinds, none shorter than an hour. The router core is built (SPEC-041), and its policy
  (`notifications-policy.json`) holds nine kinds, none for discipline.
- **This SPEC builds discipline's floor.** It is the first discipline SPEC to build, so it creates
  what SPEC-104 and SPEC-106 change: the state row `discipline_state`, discipline's data-rights
  port, the policy's kind `discipline` (ADR-104), the coordination module `discipline`, the bot's
  and the API's discipline modules, and the Mini App's `/discipline` screen.
- **What is ported** (at `27ee2bc`):
  - the occurrence and the bounds, `windows.py:occurs_on` and `bounds_ms` (a start before the
    rollover hour belongs to the next civil date);
  - the effort, `effort.py:measure` and `meets_window_floor`, and the verdict,
    `windows.py:evaluate`;
  - the booking, the evaluation, the reminder and the board,
    `pipeline_layers/committed_windows.py:CommittedWindowsLayer.book_window`, `_evaluate_windows`,
    `_window_reminders`, `_inside_committed_window` and `windows_board`;
  - hard mode, `pipeline_layers/discipline.py:DisciplineLayer.book_hardmode_tonight` and
    `_settle_hardmode`.
- **Why the effort is ported twice.** SPEC-081 ports `effort.py:measure` into quests, and the crate
  graph lets discipline depend on the kernel and ingest only. Discipline holds its own port of the
  same function, proved by its own golden, and each crate's copy is held to one predecessor.
- **The clock gap.** DeckStreak reads reviews once a study day plus the owner's `/sync` (ADR-037),
  so a verdict that reads reviews settles at the first sync cycle after its evidence window
  (ADR-104). Two duties read no review and cannot wait for a sync: the reminder, due 0 to 15 minutes
  before a window starts, and the standby notice. SPEC-076 built the notice's rule and recorded that
  it never falls due at the scheduled settle, which runs inside quiet hours by default; and the
  router withholds a nudge raised inside quiet hours (SPEC-041 R4, step 4), so a notice raised at
  the settle would be dropped, not held. ADR-105 adds a quarter-hourly tick for both.
- **Deviations from the predecessor, each with its reason.**
  - A kept verdict is never replaced, and its 5 coins are credited once; the predecessor upserts the
    day's grant to 0 when a recompute un-keeps a window. SPEC-082's `credit` writes a key once, and
    ADR-071 forbids a settled value falling without a recorded reason.
  - The evaluation covers the current study day and the 7 closed study days before it, where the
    predecessor covers yesterday and today (ADR-104); a broken hard-mode night is revised the same
    way.
  - A booked night may be cancelled until its window starts. The predecessor's table admits the
    outcome `cancelled` and no function of it writes one; #115 asks for each outcome.
  - The plan is one of a closed set (`desk`, `cafe`, `commute`, none), the predecessor's picker's
    set, never free text; its copy claims no effect a holdout has not measured (#132).
  - A window has no switch of its own: the predecessor's `enabled` and `reminder_on` columns have no
    writer (inert in v9).

## 2. Requirements

Discipline's floor

R1. `discipline_state` (one row, seeded) holds the engine switch (on), the scheduled panic's study
    day (none; SPEC-106 writes it), hard mode's switch (off), its start minute and duration (the
    defaults of the golden `hardmode_booking`'s unset case), and the pending standby notice's study
    day (none). SPEC-104 and SPEC-106 read the engine switch; this SPEC writes only hard mode's
    fields and the pending notice.
R2. The policy gains the kind `discipline` (ADR-104): class `nudge`, tiers `["T2"]`, budget null,
    dedupe `per-incident`, setting `discipline_notices_enabled`, and the recorded deviation
    `kinds.discipline` citing ADR-104. Every discipline message goes through the router under it
    (SPEC-041), with a dedupe key that holds an epoch day number, never a calendar date.

The windows (#109)

R3. A window occurs on a study day when its day mask's bit for that weekday is set (Monday bit 0);
    its bounds are `[start, end)` in epoch milliseconds under the kernel's study-day rule, a start
    before the rollover hour falling on the next civil date. Both equal the goldens `window_occurs`
    and `window_bounds`.
R4. `book_window(mask, start_min, dur_min, plan)` refuses, in the golden `book_window`'s order, with
    `max 2 windows` when two windows exist, `bad days` (a mask of 0 or above 127), `bad start` (off
    the 30-minute grid or outside the day) and `bad duration` (not 30, 60, 90 or 120). The plan is
    `desk`, `cafe`, `commute` or none. The booking records its study day, and no occurrence before
    that day is judged. `remove_window(id)` deletes the window and keeps its judged occurrences.
R5. A window's effort is the count of study events (SPEC-071's rule) in its bounds, their distinct
    cards, and their answer seconds, each capped at 60, summed in order and divided by 60; a window
    is kept at 10 distinct cards or 5 minutes. Both equal the golden `window_effort`.
R6. The verdict equals the golden `window_verdict`, over the outcome set `kept`, `token_effort`,
    `amber` and `red`: kept when the effort meets the floor; else `token_effort` when a review
    counted or the study day holds habit activity; else `red` when the rail's defections inside the
    window are above 0; else `amber`. It stores the reviews, the distinct cards, the minutes rounded
    to 2 places and the defections.
R7. At each sync cycle (SPEC-023 R12), the discipline step judges every occurrence of the current
    study day and of the 7 closed study days before it whose end plus 15 minutes has passed, from
    these inputs: the booked windows; the study day's reviews (SPEC-071); the skip set (SPEC-083),
    no occurrence of a skip day being judged; the governor's lapse (SPEC-076), no occurrence being
    judged while one is open; the study day's reading and writing entries (SPEC-078) and focus
    blocks (SPEC-079), any of which is habit activity; the rail's defections inside the window
    (SPEC-104 R23, 0 until it builds); and the cycle's instant. For the predecessor's two days the
    step equals the golden `window_evaluation`.
R8. A judged occurrence's verdict replaces its stored one, except that `kept` is never replaced.
    Each kept occurrence credits 5 coins once through SPEC-082's `credit(day, "window", "<window
    id>", 5)`. No verdict fines: a red window's defections were each judged by the rail (SPEC-104),
    so a fine of the window would fine them twice. A late review that meets the floor turns an
    amber, token-effort or red occurrence kept, and pays.
R9. Each occurrence's end plus 15 minutes is registered with the obligation port (SPEC-023 R10), so
    the first cycle after it recomputes.
R10. `inside_window(study day, instant)` answers whether the instant falls inside the bounds of an
    occurrence of any booked window on that study day; SPEC-104's verdict reads it.
R11. The board lists the booked windows and the current week's kept count and quiet count (`amber`
    and `token_effort`), the week starting on Monday, equal to the golden `windows_board`.

The reminder and the tick

R12. The job `discipline_tick` fires every 15 minutes at minutes 4, 19, 34 and 49 of each hour, a
    new quarter-hourly schedule kind of the job table that extends SPEC-027 R1 (ADR-105). None of
    its four minutes is a predecessor minute, a reserved minute (0, 25, 39) or the sync's slot
    (SPEC-027 R2), and a zone offset that is a multiple of 15 minutes maps the set onto itself. It
    does not catch up. Its timer `deck-streak-job-send@discipline_tick.timer` and the rail contract hold
    its calendar equal to the table. It reads no review and syncs nothing (ADR-037): it raises the
    reminders (R13) and the standby notice (R14), and later discipline SPECs add only messages a
    settle left pending (SPEC-106's stakes and their Sunday review).
R13. At each tick, a reminder is raised for each booked window whose occurrence today starts 0 to 15
    minutes after the tick's instant, both ends included, unless today is an active skip day or a
    lapse is open; its dedupe key is `window:<epoch day>:<window id>`. It carries the start as
    `HH:MM`, the study day's due count (SPEC-071's rollup `due_today`), the estimate in minutes
    (SPEC-100's `estimate_minutes` over its seconds per card) and the plan. Which windows are
    reminded at which instants, with those numbers, equals the golden `window_reminder`. The
    router's quiet hours and the setting decide delivery.
R14. When the governor's step of a settle finds SPEC-076's rule `standby_notice` true with its
    quiet-hours input taken as outside, the settled study day is stored as the pending notice. At a
    tick outside quiet hours, a pending notice is raised under `discipline` when the stored verdict
    is still standby and no lapse is open; in the same write the pending day is cleared and the
    notice's study day recorded through streaks' store, so a notice is raised once however many
    ticks run at once. A verdict out of standby, or an open lapse, clears the pending notice unsent.
    The copy says that no penalty can fire and that the engine re-arms as strength recovers.

Hard mode (#115)

R15. Hard mode is off until the owner turns it on: `/hardmode` while it is off turns it on and
    explains the morning booking; the Mini App's toggle turns it on or off.
R16. `book_hardmode_tonight` refuses, in the golden `hardmode_booking`'s order, with hard mode off,
    at a local hour of 12 or later, at 4 bookings since the week's Monday (a cancelled one counted),
    and when tonight is already booked; it books the stored start and duration.
R17. Tonight's booking may be cancelled until its window starts: its outcome becomes `cancelled`, it
    pays nothing, and it still counts toward the weekly cap and blocks a second booking that day.
R18. At each sync cycle, a booked night of the current study day or the 7 closed study days before
    it settles, for the predecessor's two days as the golden `hardmode_settle`, over the outcome set
    `booked`, `kept`, `broken`, `suppressed` and `cancelled`: a skip day makes it `suppressed`;
    before its end plus 15 minutes it stays booked; effort meeting R5's floor makes it `kept` and
    credits 15 coins once (`credit(day, "hardmode", "", 15)`); else 3 or more of the rail's
    defections inside it with 0 reviews, or an instant later than its end plus 6 hours, makes it
    `broken`; else it stays booked. Its end plus 15 minutes and plus 6 hours are registered with the
    obligation port.
R19. At each cycle, a `broken` night of the current study day or the 7 closed study days before it
    is judged again, and becomes `kept`, crediting once, when its effort now meets the floor
    (ADR-104). `kept`, `suppressed` and `cancelled` are final. A broken night fines nothing.

Surfaces and rights

R20. The bot's `/windows` shows the board with a remove button per window (`wn:del:<id>`) and a
    button that opens the Mini App's booking form; `/hardmode` turns hard mode on (R15) or books
    tonight (R16), and a booked night's message carries a cancel button (`hm:cancel`) until it
    starts. Every refusal names its reason.
R21. The API serves, behind the owner's session: `GET /api/discipline` (the switches, the board and
    tonight's night), `POST /api/discipline/windows`, `DELETE /api/discipline/windows/{id}`, `PUT
    /api/discipline/hardmode` (the switch), `POST /api/discipline/hardmode/tonight` and `DELETE
    /api/discipline/hardmode/tonight` (the cancellation).
R22. The Mini App's `/discipline` screen, listed in the route table, shows the windows card (the
    board and one booking form: the mask as daily, weekdays or weekend, a start from 17:00 to 22:30
    on the grid, the four durations and the plan) and the hard-mode card (the toggle, a book button
    showing the week's bookings against the cap of 4, and a cancel until the start).
R23. The migration `migrations/010501_discipline_windows_and_hard_mode.sql` creates
    `discipline_state` (one seeded row), `committed_windows`, `window_events` (unique on study day
    and window) and `hardmode_windows` (unique on study day), each `STRICT` with `created_at`, with
    `CHECK`s on the mask, the grid, the durations, the plan, the statuses and the outcomes (SPEC-020
    R15, R18).
R24. Discipline's data-rights port exports and erases `committed_windows`, `window_events` and
    `hardmode_windows`, and resets `discipline_state` in place; the four tables owe SPEC-021's six
    files.
R25. The constants (the masks, the grace, the cap of 2 windows, the lookahead, the effort floor, the
    answer cap and the kept window's coins) equal the constants golden `windows.constants`; hard
    mode's literals are proved by the goldens of R16 and R18.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | occurrence on each weekday of each mask, and the bounds across the rollover hour, equal the goldens `window_occurs` and `window_bounds` | `occurrence_and_bounds_match_the_parity_goldens` |
| A2 | every refusal and its order equal the golden `book_window`, and a plan outside the closed set is refused | `the_booking_matches_the_parity_golden` |
| A3 | the effort and its floor equal the golden `window_effort`, at 9 and 10 distinct cards, 5 minutes exactly, and an answer of 61 seconds | `the_effort_matches_the_parity_golden` |
| A4 | the verdict equals the golden `window_verdict` over its four outcomes | `the_window_verdict_matches_the_parity_golden` |
| A5 | the step equals the golden `window_evaluation` over the predecessor's two days, at the end plus 15 minutes and one millisecond before | `the_evaluation_matches_the_parity_golden` |
| A6 | the step reads each of R7's inputs from its source, each proved by flipping that source alone | `the_evaluation_gathers_every_input` |
| A7 | an amber occurrence turns kept when a late review arrives at a later cycle, and pays 5 coins once | `a_late_review_turns_an_amber_window_kept` |
| A8 | a kept occurrence is never replaced, and its coins are never taken back | `a_kept_window_is_never_replaced` |
| A9 | no occurrence is judged on a skip day, during a lapse, before its booking or more than 7 closed study days back | `the_evaluation_skips_what_it_must` |
| A10 | a red occurrence writes no fine and no coin movement | `a_red_window_fines_nothing` |
| A11 | each occurrence's and each night's deadlines are registered with the obligation port once | `the_discipline_deadlines_open_the_gate_once` |
| A12 | `inside_window` answers inside for the start's millisecond and outside for the end's | `inside_window_is_half_open` |
| A13 | the board equals the golden `windows_board` | `the_board_matches_the_parity_golden` |
| A14 | the tick's minutes are 4, 19, 34 and 49, none a predecessor minute, a reserved minute or the sync's slot, and its timer and the rail contract hold the calendar | `the_tick_keeps_off_every_reserved_and_predecessor_minute` |
| A15 | the reminders raised equal the golden `window_reminder`, at 15 minutes before a start, one millisecond more, the start itself and one millisecond after | `the_reminder_matches_the_parity_golden` |
| A16 | the reminder reads each of R13's inputs from its source, each flipped alone, and a second tick raises no second reminder of one occurrence | `the_reminder_gathers_every_input_once` |
| A17 | a notice pending from a settle inside quiet hours is raised by the first tick outside them, once, even with two ticks at once, and records its study day | `the_standby_notice_waits_for_the_tick_and_is_raised_once` |
| A18 | a pending notice is cleared unsent when the verdict leaves standby or a lapse opens | `a_stale_standby_notice_is_never_raised` |
| A19 | the notice's study day written by streaks' store is the day the rule reads next | `the_notice_day_is_read_back_by_the_rule` |
| A20 | the booking's refusals, their order and the defaults equal the golden `hardmode_booking`, at 11:59 and 12:00, and at the 4th and 5th booking of a week | `the_hard_mode_booking_matches_the_parity_golden` |
| A21 | a cancellation before the start makes the night cancelled, pays nothing, counts toward the cap and blocks a rebooking; one at the start is refused | `a_night_is_cancelled_only_before_it_starts` |
| A22 | the settlement equals the golden `hardmode_settle` over its outcomes, at 2 and 3 defections, and at the end plus 6 hours and one millisecond over | `the_hard_mode_settlement_matches_the_parity_golden` |
| A23 | a broken night turns kept when a late review meets the floor, and pays 15 coins once; kept, suppressed and cancelled nights are never re-judged | `a_broken_night_is_revised_to_kept` |
| A24 | the constants equal the constants golden | `the_window_constants_match_the_predecessors` |
| A25 | the kind `discipline` is a nudge of tier T2, per incident, behind its own setting, recorded as a deviation | `the_discipline_kind_is_a_recorded_deviation` |
| A26 | `/windows` and `/hardmode` call their use cases through a stub port and relay a refusal's reason unchanged | `windows_and_hardmode_commands_run_the_use_cases` |
| A27 | discipline's routes answer the owner's session only | `the_discipline_routes_answer_only_the_owner` |
| A28 | the screen books a window from the form and shows the week's bookings against the cap | `books a window and shows the hard mode cap` |
| A29 | an erase empties the three tables and resets the state row | `an_erase_empties_the_windows_and_resets_the_state` |

```acceptance
A1: cargo test -p deck-streak-discipline --test window_goldens -- --exact occurrence_and_bounds_match_the_parity_goldens
A2: cargo test -p deck-streak-discipline --test window_goldens -- --exact the_booking_matches_the_parity_golden
A3: cargo test -p deck-streak-discipline --test window_goldens -- --exact the_effort_matches_the_parity_golden
A4: cargo test -p deck-streak-discipline --test window_goldens -- --exact the_window_verdict_matches_the_parity_golden
A5: cargo test -p deck-streak-discipline --test window_goldens -- --exact the_evaluation_matches_the_parity_golden
A6: cargo test -p deck-streak-coordination --test discipline_windows -- --exact the_evaluation_gathers_every_input
A7: cargo test -p deck-streak-coordination --test discipline_windows -- --exact a_late_review_turns_an_amber_window_kept
A8: cargo test -p deck-streak-discipline --test window_revision -- --exact a_kept_window_is_never_replaced
A9: cargo test -p deck-streak-discipline --test window_revision -- --exact the_evaluation_skips_what_it_must
A10: cargo test -p deck-streak-coordination --test discipline_windows -- --exact a_red_window_fines_nothing
A11: cargo test -p deck-streak-coordination --test discipline_windows -- --exact the_discipline_deadlines_open_the_gate_once
A12: cargo test -p deck-streak-discipline --test window_revision -- --exact inside_window_is_half_open
A13: cargo test -p deck-streak-discipline --test window_goldens -- --exact the_board_matches_the_parity_golden
A14: cargo test -p deck-streak-coordination --test discipline_tick -- --exact the_tick_keeps_off_every_reserved_and_predecessor_minute
A15: cargo test -p deck-streak-discipline --test window_goldens -- --exact the_reminder_matches_the_parity_golden
A16: cargo test -p deck-streak-coordination --test discipline_tick -- --exact the_reminder_gathers_every_input_once
A17: cargo test -p deck-streak-coordination --test discipline_tick -- --exact the_standby_notice_waits_for_the_tick_and_is_raised_once
A18: cargo test -p deck-streak-coordination --test discipline_tick -- --exact a_stale_standby_notice_is_never_raised
A19: cargo test -p deck-streak-streaks --test standby_notice_day -- --exact the_notice_day_is_read_back_by_the_rule
A20: cargo test -p deck-streak-discipline --test window_goldens -- --exact the_hard_mode_booking_matches_the_parity_golden
A21: cargo test -p deck-streak-discipline --test window_revision -- --exact a_night_is_cancelled_only_before_it_starts
A22: cargo test -p deck-streak-discipline --test window_goldens -- --exact the_hard_mode_settlement_matches_the_parity_golden
A23: cargo test -p deck-streak-coordination --test discipline_windows -- --exact a_broken_night_is_revised_to_kept
A24: cargo test -p deck-streak-discipline --test window_goldens -- --exact the_window_constants_match_the_predecessors
A25: cargo test -p deck-streak-notifications --test discipline_kind -- --exact the_discipline_kind_is_a_recorded_deviation
A26: cargo test -p deck-streak-bot --test window_commands -- --exact windows_and_hardmode_commands_run_the_use_cases
A27: cargo test -p deck-streak-api --test window_routes -- --exact the_discipline_routes_answer_only_the_owner
A28: pnpm exec vitest run web/app/src/lib/discipline/windows.test.ts -t "books a window and shows the hard mode cap"
A29: cargo test -p deck-streak-discipline --test discipline_rights -- --exact an_erase_empties_the_windows_and_resets_the_state
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr, notifications-policy, ux-laws and
accessibility packs stay enforced, and no row is deferred or lifted for this delivery, so the
private wiring does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | the four tables are declared under their categories with data, purpose, basis, retention and erase mode, over `privacy.json`, `migrations/010501_discipline_windows_and_hard_mode.sql` and `crates/discipline/src/data_rights.rs` | the privacy-gdpr pack |
| B2 | the kind `discipline` declares its class, tier, dedupe and setting, and its deviation, over `notifications-policy.json` | the notifications-policy pack |
| B3 | the reminder's and the notice's copy carry no countdown pressure, no shame and no claimed effect, over every file under `web/app/src/lib/discipline/` and `crates/coordination/src/discipline/` | the ux-laws pack |
| B4 | the discipline screen passes the accessibility audit in both colour schemes, over the `/discipline` route that `web/app/src/lib/routes.ts` lists | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/discipline/src/lib.rs` | `deck-streak-discipline` | changed: the modules below |
| `crates/discipline/src/constants.rs` | `deck-streak-discipline` | added: the windows' constants, proved by the golden |
| `crates/discipline/src/state.rs` | `deck-streak-discipline` | added: `discipline_state`, its switches and the pending notice's claim |
| `crates/discipline/src/effort.rs` | `deck-streak-discipline` | added: the effort and its floor, pure |
| `crates/discipline/src/window.rs` | `deck-streak-discipline` | added: occurrence, bounds, `inside_window`, the verdict and the board, pure |
| `crates/discipline/src/windows.rs` | `deck-streak-discipline` | added: the booking, the evaluation's rules, the revision and the store |
| `crates/discipline/src/hardmode.rs` | `deck-streak-discipline` | added: the booking, the cancellation, the settlement and the revision |
| `crates/discipline/src/data_rights.rs` | `deck-streak-discipline` | added: discipline's data-rights port |
| `crates/discipline/Cargo.toml` | `deck-streak-discipline` | changed: the workspace dependencies it uses (`sqlx`, `thiserror`, `tokio`); `serde` and `serde_json` as dev-dependencies for the golden reader |
| `migrations/010501_discipline_windows_and_hard_mode.sql` | `deck-streak-discipline` | added: the four tables, `STRICT`, the state row seeded |
| `crates/discipline/tests/window_goldens.rs` | `deck-streak-discipline` | added: A1 to A5, A13, A15, A20, A22, A24 |
| `crates/discipline/tests/window_revision.rs` | `deck-streak-discipline` | added: A8, A9, A12, A21 |
| `crates/discipline/tests/discipline_rights.rs` | `deck-streak-discipline` | added: A29 |
| `crates/streaks/src/store.rs` | `deck-streak-streaks` | changed: the writer of the last notice's study day |
| `crates/streaks/tests/standby_notice_day.rs` | `deck-streak-streaks` | added: A19 |
| `crates/notifications/tests/discipline_kind.rs` | `deck-streak-notifications` | added: A25 |
| `notifications-policy.json` | repo | changed: the kind `discipline` and its deviation |
| `crates/coordination/src/discipline/mod.rs` | `deck-streak-coordination` | added: discipline's use cases |
| `crates/coordination/src/discipline/windows.rs` | `deck-streak-coordination` | added: the windows' and the nights' step, their inputs and their credits |
| `crates/coordination/src/discipline/tick.rs` | `deck-streak-coordination` | added: the tick's work, the reminder and the notice |
| `crates/coordination/src/recompute/streaks.rs` | `deck-streak-coordination` | changed: the governor's step stores the pending notice |
| `crates/coordination/src/jobs.rs` | `deck-streak-coordination` | changed: the quarter-hourly schedule kind and the job `discipline_tick` |
| `crates/coordination/src/runner.rs` | `deck-streak-coordination` | changed: the job runs its work by id |
| `crates/coordination/src/sync_cycle.rs` | `deck-streak-coordination` | changed: the discipline step after the recompute |
| `crates/coordination/src/obligations.rs` | `deck-streak-coordination` | changed: the windows' and the nights' deadline source |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module above |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: discipline's port joins the registry |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded rows for the four tables |
| `crates/coordination/tests/job_table.rs` | `deck-streak-coordination` | changed: a quarter-hourly job's four minutes each held to R2 |
| `crates/coordination/tests/discipline_windows.rs` | `deck-streak-coordination` | added: A6, A7, A10, A11, A23 |
| `crates/coordination/tests/discipline_tick.rs` | `deck-streak-coordination` | added: A14, A16 to A18 |
| `deploy/systemd/deck-streak-job-send@discipline_tick.timer` | deploy | added: the tick's calendar, in UTC, with its waivers, on the sending template (SPEC-100 R28) |
| `deploy/rail-contract.json` | deploy | changed: the tick's calendar key |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: the tick's id among the jobs run by id |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: discipline joined to coordination, the bot and the API |
| `crates/bot/src/discipline_commands.rs` | `deck-streak-bot` | added: /windows, /hardmode and the `wn:` and `hm:` buttons |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: both commands join the command table |
| `crates/bot/tests/window_commands.rs` | `deck-streak-bot` | added: A26 |
| `crates/api/src/discipline_routes.rs` | `deck-streak-api` | added: discipline's routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes, behind the owner's session |
| `crates/api/tests/window_routes.rs` | `deck-streak-api` | added: A27 |
| `web/app/src/lib/routes.ts` | miniapp | changed: /discipline joins the route table |
| `web/app/src/lib/discipline/discipline.ts` | miniapp | added: the client of discipline's routes |
| `web/app/src/lib/discipline/WindowsCard.svelte` | miniapp | added: the board and the booking form |
| `web/app/src/lib/discipline/HardModeCard.svelte` | miniapp | added: the toggle, the booking and the cancellation |
| `web/app/src/lib/discipline/windows.test.ts` | miniapp | added: A28 |
| `web/app/src/routes/discipline/+page.svelte` | miniapp | added: the discipline screen |
| `tools/parity-oracle/registry/spec_105.py` | repo | added: this SPEC's registrations |
| `tools/parity-oracle/goldens/window_occurs.json` | repo | added: the golden of `windows.py:occurs_on` (function) |
| `tools/parity-oracle/goldens/window_bounds.json` | repo | added: the golden of `windows.py:bounds_ms` (adapter) |
| `tools/parity-oracle/goldens/window_effort.json` | repo | added: the golden of `effort.py:measure` and `meets_window_floor` (adapter) |
| `tools/parity-oracle/goldens/window_verdict.json` | repo | added: the golden of `windows.py:evaluate` (adapter) |
| `tools/parity-oracle/goldens/book_window.json` | repo | added: the golden of `CommittedWindowsLayer.book_window` (adapter) |
| `tools/parity-oracle/goldens/window_evaluation.json` | repo | added: the golden of `CommittedWindowsLayer._evaluate_windows` (adapter) |
| `tools/parity-oracle/goldens/window_reminder.json` | repo | added: the golden of `CommittedWindowsLayer._window_reminders` (adapter) |
| `tools/parity-oracle/goldens/windows_board.json` | repo | added: the golden of `CommittedWindowsLayer.windows_board` (adapter) |
| `tools/parity-oracle/goldens/hardmode_booking.json` | repo | added: the golden of `DisciplineLayer.book_hardmode_tonight` (adapter) |
| `tools/parity-oracle/goldens/hardmode_settle.json` | repo | added: the golden of `DisciplineLayer._settle_hardmode` (adapter) |
| `tools/parity-oracle/goldens/windows.constants.json` | repo | added: the constants golden (constants) |
| `scripts/mutation-rows.d/S10500-S10599.json` | repo | added: the hand-proved rows (section 9) |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains the four tables |
| `privacy.json` | repo | changed: the categories `discipline-windows` and `discipline-state` |
| `PRIVACY.md` | repo | changed: one line for each category |
| `docs/schematics/discipline-tick-and-standby-notice.md` | docs | added by the W5 architect turn; this delivery corrects it only where the code proves it wrong |
| `.sqlx/` | workspace | changed: the offline query cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `docs/specs/SPEC-105-committed-windows-and-hard-mode-pay-on-evidence-and-a-quarter-hourly-tick-reminds.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-105.md` | docs | added |
| `changelog.d/feat-discipline-windows-105.md` | repo | added: the changelog fragment |

## 5. What this does NOT do

- It judges no ping and fines nothing: the rail does (#110), and passes its defections here.
- It writes no engine switch and no panic: the panic and the re-arm are the contracts' (#114).
- It sends no evening stakes line and no Sunday stake review of a booked night (#117, #113).
- It flushes no held celebration when quiet hours end: the router's flush is (#291).
- It carries no per-window switch: the predecessor's flags have no writer, and switches belong on
  the settings screen (#57).
- It changes no sync cadence: every verdict settles at whatever cadence the owner keeps (#164).
- It imports none of the predecessor's window or hard-mode rows (#61).

## 6. Risks

- **A reminder arrives late or twice.** Each start is inside exactly one tick's lookahead, and the
  dedupe key holds one reminder per occurrence (A15, A16); a tick that does not run is not caught
  up, because a reminder after the start is worthless.
- **Coins are paid twice or taken back.** `credit` writes a key once and kept is never replaced (A7,
  A8, A23).
- **The notice is lost or doubled.** It waits for a tick outside quiet hours and is claimed in one
  write (A17), and a stale one is dropped (A18).
- **A window reads as a fine.** No verdict fines (A10), and the copy carries no shame (B3).
- **The tick collides with the predecessor during side by side.** Its four minutes keep off every
  predecessor and reserved minute in any 15-minute zone offset (A14).

## 7. Parity goldens

All are registered in `tools/parity-oracle/registry/spec_105.py` and generated on the owner's
checkout of the predecessor at `27ee2bc` (SPEC-029). Every day is an epoch day number and every
instant epoch milliseconds; no golden holds a calendar date or a personal value.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `goldens/window_occurs.json` | `windows.py:occurs_on` | function | none; each weekday under the masks 127, 31, 96, 1 and 64 |
| `goldens/window_bounds.json` | `windows.py:bounds_ms` | adapter | a synthetic collection config at two offsets and two rollover hours; starts at the rollover minute, one grid step before it, and a window crossing midnight |
| `goldens/window_effort.json` | `effort.py:measure`, `meets_window_floor` | adapter | synthetic reviews: 9 and 10 distinct cards, 4.99 and 5 minutes, an answer of 60 and 61 seconds, a non-study event, and reviews at the start and at the end |
| `goldens/window_verdict.json` | `windows.py:evaluate` | adapter | measured effort with and without habit activity and with 0 and 1 defections, one case per outcome and each floor's boundary |
| `goldens/book_window.json` | `CommittedWindowsLayer.book_window` | adapter | a stand-in layer over a temporary store; masks 0, 1, 127 and 128, starts at 15, 30 and 1440 minutes, each duration and 45, and a third window |
| `goldens/window_evaluation.json` | `CommittedWindowsLayer._evaluate_windows` | adapter | the layer with the governor, the skip set and the habit store stubbed; instants at the end plus 15 minutes and one millisecond before, a skip day, a lapse, a booking after the day, and a late review flipping amber to kept |
| `goldens/window_reminder.json` | `CommittedWindowsLayer._window_reminders` | adapter | a recording notifier, a stubbed rollup and seconds per card; instants 15 minutes before a start, one millisecond more, at the start and one after, a skip day and a lapse |
| `goldens/windows_board.json` | `CommittedWindowsLayer.windows_board` | adapter | events of each status on the week's Monday, its Sunday and the Sunday before |
| `goldens/hardmode_booking.json` | `DisciplineLayer.book_hardmode_tonight` | adapter | a stubbed clock at 11:59 and 12:00 local, 3, 4 and 5 bookings in a week, a booking on the Monday, the switch off, the settings unset, and a second booking |
| `goldens/hardmode_settle.json` | `DisciplineLayer._settle_hardmode` | adapter | synthetic reviews and defections: 2 and 3 defections with 0 reviews, 3 with 1 review, the floor met, a skip day, and instants at the end plus 15 minutes, plus 6 hours and one millisecond over |
| `goldens/windows.constants.json` | `windows.py`, `constants.py` | constants | `MASK_DAILY`, `MASK_WEEKDAYS`, `MASK_WEEKEND`, `EVALUATION_GRACE_MIN`, `MAX_WINDOWS`, `REMINDER_LOOKAHEAD_MIN`; `REAL_EFFORT_WINDOW_MIN_DISTINCT`, `REAL_EFFORT_WINDOW_MIN_MINUTES`, `ANSWER_TIME_CAP_SECONDS`, `COIN_KEPT_WINDOW` |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `discipline_state` | `discipline` | `migrations/010501_discipline_windows_and_hard_mode.sql` (SPEC-105) | the discipline and hard-mode runtime settings (the switch, the scheduled panic, hard mode's switch, start and duration) | reset in place: the engine on, no panic, hard mode off, no pending notice |
| `committed_windows` | `discipline` | the same migration | `committed_windows`, its booking day as an epoch day and its plan's text mapped to the closed set (any other text reads as none) | exported and erased |
| `window_events` | `discipline` | the same migration | `window_events`, one row per study day and window | exported and erased |
| `hardmode_windows` | `discipline` | the same migration | `hardmode_windows`, one row per study day | exported and erased |

## 9. Mutation rows

The band is `S10500-S10599`, in `scripts/mutation-rows.d/S10500-S10599.json`. Each row's killer
selects exactly one test, and each is proved with its file restored byte for byte (SPEC-039).

| row | target | what it guards | killer |
|---|---|---|---|
| `S10501-TWO-WINDOWS-AT-MOST` | `crates/discipline/src/constants.rs` | the booking cap | `window_goldens::the_booking_matches_the_parity_golden` |
| `S10502-KEPT-AT-10-CARDS` | `crates/discipline/src/effort.rs` | the distinct cards' inclusive floor | `window_goldens::the_effort_matches_the_parity_golden` |
| `S10503-KEPT-AT-5-MINUTES` | `crates/discipline/src/effort.rs` | the minutes' inclusive floor | `window_goldens::the_effort_matches_the_parity_golden` |
| `S10504-AN-ANSWER-COUNTS-60-SECONDS-AT-MOST` | `crates/discipline/src/effort.rs` | the answer cap | `window_goldens::the_effort_matches_the_parity_golden` |
| `S10505-A-KEPT-WINDOW-PAYS-5` | `crates/discipline/src/constants.rs` | the kept window's coins | `window_goldens::the_window_constants_match_the_predecessors` |
| `S10506-KEPT-IS-NEVER-REPLACED` | `crates/discipline/src/windows.rs` | a paid verdict stands | `window_revision::a_kept_window_is_never_replaced` |
| `S10507-NO-VERDICT-ON-A-SKIP-DAY` | `crates/discipline/src/windows.rs` | a skip day is not judged | `window_revision::the_evaluation_skips_what_it_must` |
| `S10508-THE-LOOKAHEAD-IS-15-MINUTES` | `crates/discipline/src/window.rs` | the reminder's inclusive lookahead | `window_goldens::the_reminder_matches_the_parity_golden` |
| `S10509-HARD-MODE-BOOKS-BEFORE-NOON` | `crates/discipline/src/hardmode.rs` | the morning-only booking | `window_goldens::the_hard_mode_booking_matches_the_parity_golden` |
| `S10510-FOUR-NIGHTS-A-WEEK` | `crates/discipline/src/hardmode.rs` | the weekly cap | `window_goldens::the_hard_mode_booking_matches_the_parity_golden` |
| `S10511-A-KEPT-NIGHT-PAYS-15` | `crates/discipline/src/hardmode.rs` | the kept night's coins | `window_goldens::the_hard_mode_settlement_matches_the_parity_golden` |
| `S10512-BROKEN-AT-THREE-DEFECTIONS` | `crates/discipline/src/hardmode.rs` | the broken night's inclusive threshold | `window_goldens::the_hard_mode_settlement_matches_the_parity_golden` |
| `S10513-ONE-STANDBY-NOTICE` | `crates/discipline/src/state.rs` | the pending notice is claimed once | `discipline_tick::the_standby_notice_waits_for_the_tick_and_is_raised_once` |
| `S10514-ONE-VERDICT-PER-WINDOW-DAY` | `migrations/010501_discipline_windows_and_hard_mode.sql` | the unique key on (study day, window), a key held in the migration (a script-mutation row with a cargo killer) | `window_revision::a_kept_window_is_never_replaced` |
| `S10515-THE-TICK-KEEPS-OFF-THE-PREDECESSOR` | `crates/coordination/src/jobs.rs` | the tick's first minute | `discipline_tick::the_tick_keeps_off_every_reserved_and_predecessor_minute` |
