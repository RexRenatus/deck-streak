# SPEC-607: focus blocks in the study clients run on one timer that carries the focus parity rows

- **Campaign:** focus blocks in the study clients. **Issue:** #924, #98, #99 (epic #923).
  **Context(s):** `deck-streak-focus` (the timer, its claim, the focus log, focus XP, the focus
  streak, the board, the badge conditions and the nudge rule); `deck-streak-coordination` (the
  start, stop, reach and fire use cases, the settle of focus XP and the badge awards);
  `deck-streak-api` and `deck-streak-daemon` (the routes and the role that arms the timer); and the
  study clients, the web app (`web/app`) and the iPhone and iPad app (`ios/`).
- **Decided by:** ADR-628 (one timer serves a block and the focus rows, and a block shows only its
  own time left), ADR-629 (the rest guard is opt-in and keeps its readings on the device), ADR-012
  (the parity oracle proves the math) and ADR-079 (a block ends on the service's clock and is
  claimed once by its generation).
- **Supersedes:** SPEC-079 (planned, never built; its bot and Mini App parts fall under D5).
- **Mutation band:** `S60700-S60799`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to `docs/specs/` with its tests and `docs/red-first/SPEC-607.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists.** `crates/focus/src/` holds only `lib.rs`, whose header names the timer and
  deep-work accounting as the context's charge, and the web app's `/study` route opens the deck
  list and the review with no block around it (`web/app/src/routes/study/+page.svelte`). The
  iPhone and iPad app's review (`ios/App/Sources/ReviewView.swift`) has no block either.
- **What #924 asks.** The study clients run a review as a focus block, and the focus parity rows of
  #98 and #99 become that block's timer, so focus minutes and study minutes are never counted
  twice.
- **What SPEC-079 planned, and why it is superseded.** SPEC-079 ported the predecessor's focus
  timer and deep-work XP onto the Mini App and the bot, with a timer armed by the `api` and `bot`
  roles. The study clients are now the web app and the iPhone and iPad app, and the Mini App and the
  bot gain no feature (D5). This SPEC keeps SPEC-079's numbers, goldens and claim, drops its bot and
  Mini App parts, and adds the block that #924 describes.
- **The decisions this design records**, each in plain words. D1, D2, D5 and D6 are the owner's
  decisions; D3 and D4 are the DeckStreak architect's design rulings.
  - D1 (the owner's), the block's timer: a block may show its own time left in the block and its
    progress, the minutes or cards done of the block's length; never a date, a day count or days
    left. The PRD's third non-goal, as the app-surfaces ruling amends it, does not forbid a block's
    own time left (ADR-628).
  - D2 (the owner's), the economy: overlapping focus minutes and study minutes pay XP once.
  - D3 (the architect's), the rest guard's data: a declared on-device store, opt-in, which the
    learner can withdraw and erase at any time, with a data-protection impact note; it reads
    nothing from a health store or from any other app's data, and it is never sent to the service
    (ADR-629).
  - D4 (the architect's), streaks: a completed block changes no existing streak rule; the focus
    streak of #99 is that parity row's own rule, added beside the others.
  - D5 (the owner's), surfaces: the study clients are the web app and the iPhone and iPad app; the
    Mini App and the bot are deprecated, and nothing in this SPEC is built on them (the owner's
    decision on surfaces, kept under `docs/rulings/`).
  - D6 (the owner's), the design principle vivid in session and calm out of session: outside a
    session nothing pressures, shames or fabricates, and at most one nudge a day; the timer's copy
    never urges.
- **Corrections SPEC-079 made to the issues, kept.** The XP curve rounds half to even, never
  floors; the combo pays when the day's completed blocks total at least the qualifying floor and
  the day carries a backlog-zero grant; the focus streak bridges at most one rest day in the whole
  run; the focus nudge also needs today under the daily goal.

## 2. Requirements

The block (#924's behaviour)

R1. A block's length comes from the presets or is typed, and counts minutes or cards (#924). The
    presets are the predecessor's: work 25, 50 and 90 minutes, extra 15, 40 and 60, breaks 5 and 10
    (#98); a typed length is clamped at the entry cap of 240 minutes (#98,
    `goldens/focus_clamp_minutes.json`). A block counted in cards holds its card count and its start
    instant, and its fallback end instant is the entry cap after its start, so a block left open
    still ends.
R2. The block's end is soft (#924): when the minutes reach the length, or the learner answers the
    block's last card, the client finishes the card in hand and offers to continue; it never takes
    a card off the screen mid-answer. To continue starts the next block (R4).
R3. Work and break cycles are the learner's own to set (#924): the work length, the short and long
    break lengths and how many work blocks come before a long break, each from the presets or typed
    and clamped. The defaults and the advance rule are the predecessor's: work 25, short break 5,
    long break 15 after every fourth work block (#98, `goldens/focus_cycle_advance.json`). A break
    counts as elapsed time for the cards waiting on a learning step: no block holds back the
    scheduler's clock, so a card whose step falls due during a break is due when the next work
    block opens, as it would be with no block.
R4. Blocks run back to back (#924): each block starts from the card states the last one left, and
    the due cards a block did not reach stay due for the next one. No block keeps its own list of
    cards.
R5. Between blocks the client suggests a pause of the learner's break length, which the learner may
    skip (#924). An optional daily study-time limit, off until the learner sets it, counts the day's
    block minutes on the one timer (R7); when it is reached the client finishes the card in hand,
    ends the block, and opens no new block that day until the learner changes the limit (#924).
R6. The opt-in rest guard keeps its readings on the device (#924, D3): its data and rules are R22
    and ADR-629.
R7. One timer serves the block and the focus rows (#924, D2): every block in a study client is a
    focus block on the service's one timer row, its minutes are the focus log's minutes, and
    overlapping focus minutes and study minutes pay XP once (D2). How the overlap is paid once is
    not in this SPEC.

The timer (#98, under ADR-079's rule and D5)

R8. `deck-streak-focus` owns the timer as one row, `focus_timer`, as SPEC-079 R1 has it, less the
    bot's countdown message (D5): its state is `idle`, `running` or `paused`, and its `block` is a
    generation that every start increments. The row holds the subject, the kind (`work` or
    `break`), the length in minutes or in cards, the end instant in epoch milliseconds while
    running, the remaining seconds while paused, the cycle's count of finished work blocks, whether
    the block belongs to a cycle, and whether its end has been claimed.
R9. A start, pause, resume, add of five minutes and stop behave as SPEC-079 R2, R4 and R5 have them
    (#98): the subject resolves as ADR-087 has it, a start while a block is active first ends that
    block as a stop does, adding five minutes stops at the entry cap and moves the end or the
    paused remainder by the minutes added (`goldens/focus_add_minutes.json`), and a stop records
    the elapsed seconds over 60 rounded half to even, never more than planned
    (`goldens/focus_stop_minutes.json`). Every entry is clamped and force-abandoned as the
    predecessor's is (`goldens/focus_record_entry.json`): a block under the qualifying floor of 15
    minutes is recorded abandoned and uncertified and earns nothing, so an early stop at fourteen
    minutes earns nothing (#98).
R10. A block's end is claimed once by its generation in one write (ADR-079's rule, #98): the claim,
    the block's `focus_log` row and the timer's next state commit together, across a restart. A
    block counted in cards is claimed when the client reports its count reached, by the same
    conditional update keyed by the generation, and is recorded at its elapsed minutes, clamped. A
    timer armed for an older generation, a paused block, or an end that an added five minutes moved
    claims nothing.
R11. The long-running `api` role, which serves both study clients, arms an in-process timer for the
    running block when it starts and whenever it writes the row, fires through R10's claim, and
    stops its timer at shutdown; at start it decides from the row as the predecessor's restart does
    (`goldens/focus_rearm.json`). The deprecated bot role arms no focus timer and gains no focus
    command (D5).
R12. Both clients count down from the service's end instant and the service's own current instant
    in the same response, never from the device clock alone, and ask the service again when their
    count reaches zero; the service's answer carries the block's result, shown in the session.
R13. A block shows its own time left and its progress, the minutes or cards done of its length, and
    never a date, a day count or days left (D1). The block's copy never urges (D6): it holds none of
    the urging words the ux-laws pack's fake-urgency row lists, and its time-left element is that
    row's advisory `countdown-pressure` reading, recorded in ADR-628, never a deadline.
R14. A block writes nothing to a card beyond the learner's own answers (#924): it never changes
    which cards are due, any card field, the scheduler or its weights.

The focus rows (#99)

R15. A completed block's XP equals the golden of the predecessor's block XP
    (`goldens/focus_xp.json`): the curve's points are 0 minutes 0 XP, 25 minutes 30, 50 minutes 70
    and 90 minutes 140, interpolated and rounded half to even, continued at 1.75 XP a minute past
    90, and nothing under 15 minutes. The day's focus XP is the sum over the day's completed blocks,
    capped at 200 (`goldens/focus_day_xp.json`).
R16. Focus XP is derived XP (ADR-072), settled through progression's settle port with the
    predecessor's three sources (`goldens/focus_recompute_xp.json`): `focus` on the block's study
    day; `focusgoal:` on the week's first study day, 150 XP once a week when the week's completed
    minutes reach 300; and `focus_combo`, 50 XP once a day when the day carries a backlog-zero grant
    and its completed blocks total at least the qualifying floor (#99). A second settle of the same
    inputs writes nothing new.
R17. A day counts toward the focus streak when its completed minutes reach the daily goal of 50, and
    the streak bridges at most one rest day in the whole run, only one with a counted day before it
    (#99, `goldens/focus_streak.json`). Today's and the week's progress equal
    `goldens/focus_progress.json`, and the minutes and blocks per subject equal
    `goldens/focus_subject_breakdown.json`. A completed block changes no existing streak rule (D4).
R18. The eight focus badges' conditions equal `goldens/focus_badges.json` (#99). A block that
    reaches its end on the timer, or its card count, is certified; a manually back-logged block is
    uncertified and never earns the deep-diver badge (#99).
R19. The focus nudge's eligibility equals `goldens/focus_nudge_eligible.json`: a focus streak of at
    least 3 with today under the daily goal (#99). The focus context exposes the rule; no message is
    sent by this SPEC.
R20. `GET /api/focus` returns the timer, the service's current instant, today and the week against
    their goals, the focus streak, the subject breakdown and today's focus XP; `GET
    /api/focus/days?weeks=N` returns each study day's completed minutes, equal to
    `goldens/focus_daily_minutes.json`; `POST /api/focus/start`, `/cycle`, `/pause`, `/resume`,
    `/add`, `/stop` and `/reached` change the timer. Every route answers the owner's session only.
R21. Every number in R1, R3, R9 and R15 to R19 equals the constants golden
    (`goldens/focus.constants.json`), copied from #98, #99 and SPEC-079's requirements and proved
    by a parity-oracle golden of the predecessor's own function (ADR-012), never re-derived.

Data rights

R22. The rest guard (D3, ADR-629) is off until the learner turns it on. Its readings are the open
    session's own timings on this device: the minutes studied today, the minutes since the last
    break and each answer's time in the open block. It reads nothing from a health store or from
    any other app's data. Its store is the device's own, apart from the collection that syncs, and
    is declared in `privacy.json` as a device store with consent as its basis, the switch that
    turns it off as its withdrawal, and a retention of the current study day. Turning it off erases
    the store at once. No request either client sends carries a reading. What it does with the
    readings is an in-session suggestion to rest, which never stops a review.
R23. `focus_log` and `focus_timer` are `STRICT`, each with `created_at`, created by
    `migrations/060701_focus_timer_and_log.sql`; `focus_timer` holds exactly one row. Focus's
    data-rights port exports and erases `focus_log` and resets `focus_timer` in place to idle, and
    both are declared in `privacy.json` and `PRIVACY.md`.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a completed block's XP equals the golden, the ties and the floor included (R15) | `the_block_xp_matches_the_predecessors_golden` |
| A2 | the day's focus XP equals the golden, the cap included (R15) | `the_day_xp_matches_the_predecessors_golden` |
| A3 | the focus streak equals the golden, one bridged rest day at most (R17) | `the_focus_streak_matches_the_predecessors_golden` |
| A4 | today's and the week's progress equal the golden (R17) | `the_focus_progress_matches_the_predecessors_golden` |
| A5 | the minutes and blocks per subject equal the golden (R17) | `the_subject_breakdown_matches_the_predecessors_golden` |
| A6 | every focus constant equals the constants golden, and an entry is clamped and force-abandoned as the predecessor's is (R1, R3, R9, R21) | `the_focus_constants_equal_the_predecessors` and `a_block_entry_is_clamped_as_the_predecessor_clamps_it` |
| A8 | an early stop at fourteen minutes records an abandoned block that earns nothing (R9) | `an_early_stop_at_fourteen_minutes_earns_nothing` |
| A9 | the default cycle over five blocks advances as the golden, the long break after the fourth (R3) | `a_cycle_advances_as_the_predecessors_cycle_does` |
| A10 | a cycle of the learner's own lengths advances by the same rule (R3) | `a_cycle_of_set_lengths_advances_by_the_same_rule` |
| A11 | adding five minutes stops at the entry cap and moves the end by the minutes added (R9) | `adding_five_minutes_stops_at_the_entry_cap` |
| A12 | a stop's minutes equal the golden (R9) | `a_stop_records_the_minutes_as_the_predecessor_does` |
| A13 | at start the role decides from the row as the golden (R11) | `a_restart_decides_as_the_predecessor_re_arms` |
| A14 | a second claim of one block's end wins nothing (R10) | `a_second_claim_of_one_block_wins_nothing` |
| A15 | a timer armed for a replaced, paused or extended block claims nothing (R10) | `a_stale_timer_claims_nothing` |
| A16 | a timer claims nothing before its block's end (R10) | `a_timer_claims_nothing_before_its_end` |
| A17 | the timer table refuses a second row (R23) | `the_timer_is_one_row` |
| A18 | a block counted in cards is claimed once when its count is reached (R1, R10) | `a_cards_block_is_claimed_once_when_its_count_is_reached` |
| A19 | a restart mid-block credits the block exactly once (R10, R11) | `a_restart_mid_block_credits_the_block_exactly_once` |
| A20 | only the `api` role arms the focus timer (R11) | `only_the_api_role_arms_the_focus_timer` |
| A21 | a minute both the block and the study count pays XP once (R7) | `a_minute_both_rows_count_pays_xp_once` |
| A22 | the weekly goal and the combo settle as the golden, and a second settle writes nothing new (R16) | `the_weekly_goal_and_combo_settle_once` |
| A23 | a completed block changes no existing streak rule (R17) | `a_completed_block_changes_no_streak_rule` |
| A24 | the focus badges equal the golden (R18) | `the_focus_badges_match_the_predecessors_golden` |
| A25 | an uncertified block never earns the deep-diver badge, however long (R18) | `a_manual_block_never_earns_deep_diver` |
| A26 | the nudge's eligibility equals the golden (R19) | `the_nudge_eligibility_matches_the_predecessors_golden` |
| A27 | the daily minutes series equals the golden (R20) | `the_focus_days_series_matches_the_predecessors_golden` |
| A28 | every focus route answers the owner's session only (R20) | `the_focus_routes_answer_only_the_owner` |
| A29 | the focus tables are exported, and an erase empties the log and resets the timer to idle (R23) | `the_focus_tables_are_exported_and_erased` |
| A30 | the web app counts down to the service's end whatever the device clock says (R12) | `counts down to the service's end whatever the device clock says` |
| A31 | the web app offers the presets and a typed length in minutes or in cards (R1) | `offers the presets and a typed length in minutes or in cards` |
| A32 | the web app finishes the card in hand and offers to continue at the block's end (R2) | `finishes the card in hand and offers to continue` |
| A33 | the next block starts from the card states the last one left, and unreached due cards stay due (R3, R4) | `the next block starts where the last one left the cards` |
| A34 | the pause between blocks is suggested and skippable, and the daily limit ends the block after the card in hand (R5) | `suggests a pause and holds the daily limit after the card in hand` |
| A35 | a block in the web app writes nothing to a card beyond the learner's own answers (R14) | `a block writes nothing to a card beyond the answers` |
| A36 | the web app's block shows its time left and progress, never a date or a day count, and its copy holds no urging word (R13) | `shows only the block's time left and holds no urging word` |
| A37 | the web app's rest guard is off until turned on, and turning it off erases its store (R22) | `the rest guard is opt-in and its withdrawal erases it` |
| A38 | the web app never sends a rest guard reading to the service (R22) | `never sends a rest guard reading` |
| A39 | the iPhone and iPad app counts down to the service's end whatever the device clock says (R12) | `test_counts_down_to_the_services_end_whatever_the_device_clock_says` |
| A40 | the iPhone and iPad app finishes the card in hand, offers to continue, and writes nothing to a card beyond the answers (R2, R14) | `test_a_block_ends_softly_and_writes_only_the_answers` |
| A41 | the iPhone and iPad app's block shows only its time left and holds no urging word (R13) | `test_the_block_shows_only_its_time_left_and_no_urging_word` |
| A42 | the iPhone and iPad app's rest guard is opt-in, erased on withdrawal and never sent (R22) | `test_the_rest_guard_is_opt_in_erased_on_withdrawal_and_never_sent` |
| A43 | the rest guard's store is a declared device store with consent, a withdrawal and a bounded retention, and neither client reads a health store (R22) | `test_the_rest_guard_store_is_declared_and_reads_no_health_store` |

```acceptance
A1: cargo test -p deck-streak-focus --test focus_xp -- --exact the_block_xp_matches_the_predecessors_golden
A2: cargo test -p deck-streak-focus --test focus_xp -- --exact the_day_xp_matches_the_predecessors_golden
A3: cargo test -p deck-streak-focus --test focus_progress -- --exact the_focus_streak_matches_the_predecessors_golden
A4: cargo test -p deck-streak-focus --test focus_progress -- --exact the_focus_progress_matches_the_predecessors_golden
A5: cargo test -p deck-streak-focus --test focus_progress -- --exact the_subject_breakdown_matches_the_predecessors_golden
A6: cargo test -p deck-streak-focus --test focus_constants -- --exact the_focus_constants_equal_the_predecessors
A6: cargo test -p deck-streak-focus --test focus_timer -- --exact a_block_entry_is_clamped_as_the_predecessor_clamps_it
A8: cargo test -p deck-streak-focus --test focus_timer -- --exact an_early_stop_at_fourteen_minutes_earns_nothing
A9: cargo test -p deck-streak-focus --test focus_timer -- --exact a_cycle_advances_as_the_predecessors_cycle_does
A10: cargo test -p deck-streak-focus --test focus_timer -- --exact a_cycle_of_set_lengths_advances_by_the_same_rule
A11: cargo test -p deck-streak-focus --test focus_timer -- --exact adding_five_minutes_stops_at_the_entry_cap
A12: cargo test -p deck-streak-focus --test focus_timer -- --exact a_stop_records_the_minutes_as_the_predecessor_does
A13: cargo test -p deck-streak-focus --test focus_timer -- --exact a_restart_decides_as_the_predecessor_re_arms
A14: cargo test -p deck-streak-focus --test focus_claim -- --exact a_second_claim_of_one_block_wins_nothing
A15: cargo test -p deck-streak-focus --test focus_claim -- --exact a_stale_timer_claims_nothing
A16: cargo test -p deck-streak-focus --test focus_claim -- --exact a_timer_claims_nothing_before_its_end
A17: cargo test -p deck-streak-focus --test focus_claim -- --exact the_timer_is_one_row
A18: cargo test -p deck-streak-focus --test focus_claim -- --exact a_cards_block_is_claimed_once_when_its_count_is_reached
A19: cargo test -p deck-streak-coordination --test focus_fire -- --exact a_restart_mid_block_credits_the_block_exactly_once
A20: cargo test -p deck-streak-daemon --test focus_roles -- --exact only_the_api_role_arms_the_focus_timer
A21: cargo test -p deck-streak-coordination --test focus_fire -- --exact a_minute_both_rows_count_pays_xp_once
A22: cargo test -p deck-streak-coordination --test focus_fire -- --exact the_weekly_goal_and_combo_settle_once
A23: cargo test -p deck-streak-coordination --test focus_fire -- --exact a_completed_block_changes_no_streak_rule
A24: cargo test -p deck-streak-focus --test focus_badges -- --exact the_focus_badges_match_the_predecessors_golden
A25: cargo test -p deck-streak-focus --test focus_badges -- --exact a_manual_block_never_earns_deep_diver
A26: cargo test -p deck-streak-focus --test focus_badges -- --exact the_nudge_eligibility_matches_the_predecessors_golden
A27: cargo test -p deck-streak-coordination --test focus_fire -- --exact the_focus_days_series_matches_the_predecessors_golden
A28: cargo test -p deck-streak-api --test focus_routes -- --exact the_focus_routes_answer_only_the_owner
A29: cargo test -p deck-streak-focus --test focus_rights -- --exact the_focus_tables_are_exported_and_erased
A30: pnpm exec vitest run web/app/src/lib/focus/countdown.test.ts -t "counts down to the service's end whatever the device clock says"
A31: pnpm exec vitest run web/app/src/lib/focus/block.test.ts -t "offers the presets and a typed length in minutes or in cards"
A32: pnpm exec vitest run web/app/src/lib/focus/block.test.ts -t "finishes the card in hand and offers to continue"
A33: pnpm exec vitest run web/app/src/lib/focus/block.test.ts -t "the next block starts where the last one left the cards"
A34: pnpm exec vitest run web/app/src/lib/focus/block.test.ts -t "suggests a pause and holds the daily limit after the card in hand"
A35: pnpm exec vitest run web/app/src/lib/focus/block-writes.test.ts -t "a block writes nothing to a card beyond the answers"
A36: pnpm exec vitest run web/app/src/lib/focus/focus-words.test.ts -t "shows only the block's time left and holds no urging word"
A37: pnpm exec vitest run web/app/src/lib/focus/rest-guard.test.ts -t "the rest guard is opt-in and its withdrawal erases it"
A38: pnpm exec vitest run web/app/src/lib/focus/rest-guard.test.ts -t "never sends a rest guard reading"
A39: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/FocusCountdownTests/test_counts_down_to_the_services_end_whatever_the_device_clock_says
A40: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/FocusBlockModelTests/test_a_block_ends_softly_and_writes_only_the_answers
A41: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/FocusWordsTests/test_the_block_shows_only_its_time_left_and_no_urging_word
A42: xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" -disable-concurrent-destination-testing -only-testing:DeckStreakTests/RestGuardTests/test_the_rest_guard_is_opt_in_erased_on_withdrawal_and_never_sent
A43: python3 -m unittest discover -s scripts/tests -p test_rest_guard_declared.py -k test_the_rest_guard_store_is_declared_and_reads_no_health_store
```

## 3a. What the box run judges

The box run judges these over the committed tree and posts its verdict on the pull request. They
have no line in the fence above, because no public test can run a pack's row.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json` and `PRIVACY.md`, examining every category, `focus_log`, `focus_timer` and the rest guard's device store each carry a basis, a retention, an export and an erase, and the rest guard's consent names its withdrawal | the privacy-gdpr pack |
| B2 | over `web/app/src/lib/focus/` and the iPhone and iPad app's focus views, examining every element and string, the fake-urgency row reads the block's time-left element as its advisory `countdown-pressure` reading and finds no urging word | the ux-laws pack |
| B3 | over `economy.json`, examining every value, no economy number changes | the game-economy pack |

## 3b. What is recorded by hand

At the release that ships this SPEC's build, #98 and #99 each receive an evidence comment naming
the tests above that prove their numbers, and they close with the release (#924).

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/focus/src/lib.rs` | `deck-streak-focus` | changed: the modules below |
| `crates/focus/src/constants.rs` | `deck-streak-focus` | added: the constants the golden proves |
| `crates/focus/src/xp.rs` | `deck-streak-focus` | added: block XP and the day's capped sum |
| `crates/focus/src/streak.rs` | `deck-streak-focus` | added: the focus streak, today and the week, the subject breakdown |
| `crates/focus/src/timer.rs` | `deck-streak-focus` | added: the transitions, the cycle of set lengths, add five minutes, the stop's minutes, the cards count, the restart decision |
| `crates/focus/src/repository.rs` | `deck-streak-focus` | added: `focus_log` and `focus_timer`; the claim's SQL as constant strings |
| `crates/focus/src/badges.rs` | `deck-streak-focus` | added: the badge conditions and the nudge's eligibility |
| `crates/focus/src/data_rights.rs` | `deck-streak-focus` | added: the data-rights port |
| `crates/focus/Cargo.toml` | `deck-streak-focus` | changed: the kernel, `sqlx`, `tokio`, `thiserror`, and the golden reader's dev-dependencies |
| `crates/focus/tests/focus_xp.rs` | `deck-streak-focus` | added: A1, A2 |
| `crates/focus/tests/focus_progress.rs` | `deck-streak-focus` | added: A3 to A5 |
| `crates/focus/tests/focus_constants.rs` | `deck-streak-focus` | added: A6 |
| `crates/focus/tests/focus_timer.rs` | `deck-streak-focus` | added: A6's clamp, A8 to A13 |
| `crates/focus/tests/focus_claim.rs` | `deck-streak-focus` | added: A14 to A18 |
| `crates/focus/tests/focus_badges.rs` | `deck-streak-focus` | added: A24 to A26 |
| `crates/focus/tests/focus_rights.rs` | `deck-streak-focus` | added: A29 |
| `migrations/060701_focus_timer_and_log.sql` | `deck-streak-focus` | added: `focus_log` and `focus_timer`, `STRICT`, with `created_at` |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `crates/coordination/src/focus/mod.rs` | `deck-streak-coordination` | added: start, cycle, pause, resume, add, stop, reach and fire, the settle of focus XP, the badge awards |
| `crates/coordination/src/focus/timer.rs` | `deck-streak-coordination` | added: the timer the `api` role arms, and the restart decision applied |
| `crates/coordination/src/recompute/focus.rs` | `deck-streak-coordination` | added: the recompute's focus XP and focus badge steps |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the focus module |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: focus's port joins the registry |
| `crates/coordination/tests/focus_fire.rs` | `deck-streak-coordination` | added: A19, A21 to A23, A27 |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded `focus_log` and `focus_timer` rows |
| `crates/api/src/focus_routes.rs` | `deck-streak-api` | added: the focus routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the focus routes mounted |
| `crates/api/tests/focus_routes.rs` | `deck-streak-api` | added: A28 |
| `crates/daemon/src/role_api.rs` | `deck-streak-daemon` | changed: the `api` role arms the focus timer at start |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the focus use cases joined to progression's ports |
| `crates/daemon/tests/focus_roles.rs` | `deck-streak-daemon` | added: A20 |
| `web/app/src/lib/focus/countdown.ts` | web app | added: the count from the service's end and the service's instant |
| `web/app/src/lib/focus/countdown.test.ts` | web app | added: A30 |
| `web/app/src/lib/focus/block.ts` | web app | added: the block's length, soft end, cycles, pause and daily limit |
| `web/app/src/lib/focus/block.test.ts` | web app | added: A31 to A34 |
| `web/app/src/lib/focus/block-writes.test.ts` | web app | added: A35 |
| `web/app/src/lib/focus/focus-words.ts` | web app | added: the block's copy |
| `web/app/src/lib/focus/focus-words.test.ts` | web app | added: A36 |
| `web/app/src/lib/focus/rest-guard.ts` | web app | added: the rest guard and its device store |
| `web/app/src/lib/focus/rest-guard.test.ts` | web app | added: A37, A38 |
| `web/app/src/lib/focus/FocusBlockBar.svelte` | web app | added: the block's time left, progress and controls |
| `web/app/src/lib/study/ReviewScreen.svelte` | web app | changed: the review runs inside a block |
| `web/app/src/routes/study/+page.svelte` | web app | changed: the block picker opens before the review |
| `ios/App/Sources/FocusBlockModel.swift` | iPhone and iPad app | added: the block's length, soft end, cycles, pause and daily limit |
| `ios/App/Sources/FocusCountdown.swift` | iPhone and iPad app | added: the count from the service's end |
| `ios/App/Sources/FocusBlockView.swift` | iPhone and iPad app | added: the block's time left, progress and controls |
| `ios/App/Sources/FocusWords.swift` | iPhone and iPad app | added: the block's copy |
| `ios/App/Sources/RestGuard.swift` | iPhone and iPad app | added: the rest guard and its device store |
| `ios/App/Sources/ReviewView.swift` | iPhone and iPad app | changed: the review runs inside a block |
| `ios/AppTests/FocusCountdownTests.swift` | iPhone and iPad app | added: A39 |
| `ios/AppTests/FocusBlockModelTests.swift` | iPhone and iPad app | added: A40 |
| `ios/AppTests/FocusWordsTests.swift` | iPhone and iPad app | added: A41 |
| `ios/AppTests/RestGuardTests.swift` | iPhone and iPad app | added: A42 |
| `scripts/tests/test_rest_guard_declared.py` | repo | added: A43 |
| `privacy.json` | repo | changed: the `focus-log`, `focus-timer` and `rest-guard` categories |
| `PRIVACY.md` | repo | changed: one line per category |
| `tools/parity-oracle/registry/spec_607.py` | repo | added: this SPEC's registrations (section 8) |
| `tools/parity-oracle/goldens/focus_xp.json` | repo | added: the golden of the predecessor's block XP |
| `tools/parity-oracle/goldens/focus_day_xp.json` | repo | added: the golden of the day's capped sum |
| `tools/parity-oracle/goldens/focus_streak.json` | repo | added: the golden of the focus streak |
| `tools/parity-oracle/goldens/focus_progress.json` | repo | added: the golden of today's and the week's progress |
| `tools/parity-oracle/goldens/focus_subject_breakdown.json` | repo | added: the golden of the subject breakdown |
| `tools/parity-oracle/goldens/focus_clamp_minutes.json` | repo | added: the golden of the clamp |
| `tools/parity-oracle/goldens/focus_badges.json` | repo | added: the golden of the badge conditions |
| `tools/parity-oracle/goldens/focus_record_entry.json` | repo | added: the golden of the entry's clamp and force-abandon |
| `tools/parity-oracle/goldens/focus_recompute_xp.json` | repo | added: the golden of the three settled sources |
| `tools/parity-oracle/goldens/focus_nudge_eligible.json` | repo | added: the golden of the nudge's eligibility |
| `tools/parity-oracle/goldens/focus_daily_minutes.json` | repo | added: the golden of the daily minutes series |
| `tools/parity-oracle/goldens/focus_stop_minutes.json` | repo | added: the golden of the stop's minutes |
| `tools/parity-oracle/goldens/focus_cycle_advance.json` | repo | added: the golden of the cycle's advance |
| `tools/parity-oracle/goldens/focus_add_minutes.json` | repo | added: the golden of add five minutes |
| `tools/parity-oracle/goldens/focus_rearm.json` | repo | added: the golden of the restart decision |
| `tools/parity-oracle/goldens/focus.constants.json` | repo | added: the constants golden |
| `scripts/mutation-rows.d/S60700-S60799.json` | repo | added: the hand-proved rows (section 9) |
| `Cargo.lock` | workspace | changed |
| `docs/specs/SPEC-607-focus-blocks-in-the-study-clients-run-on-one-timer-that-carries-the-focus-parity-rows.md` | docs | moved from `docs/specs/planned/` |
| `docs/specs/planned/SPEC-079-a-focus-block-runs-on-the-services-clock-and-deep-work-earns-once.md` | docs | unchanged by this build: its status already names this SPEC as superseding it |
| `docs/decisions/ADR-079-a-focus-block-ends-on-the-services-clock-armed-by-each-long-running-role-and-claimed-once.md` | docs | changed: a note that ADR-628 replaces its role list |
| `docs/decisions/ADR-628-one-timer-serves-a-block-and-the-focus-rows-and-a-block-shows-only-its-own-time-left.md` | docs | changed: accepted |
| `docs/decisions/ADR-629-the-rest-guard-is-opt-in-and-keeps-its-readings-on-the-device.md` | docs | changed: accepted |
| `docs/schematics/focus-timer-state-machine.md` | docs | changed: the bot role leaves, and the block's states point to `docs/schematics/focus-blocks.md` |
| `docs/PRD.md` | docs | changed: an amendment note that the third non-goal does not forbid a block's own time left (D1) |
| `docs/red-first/SPEC-607.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. Risks

- **A block credits twice.** Detected by A14, A15, A18 and A19, and by the rows on the claim's SQL
  (section 9); prevented by the generation and the one write (R10).
- **A device clock moves the end.** Detected by A30 and A39, which count with a device clock set
  apart from the service's.
- **A minute pays XP twice.** Detected by A21 over a seeded day where a block and its reviews share
  every minute.
- **A block changes what is due.** Detected by A33, A35 and A40, which compare the card states after
  a block with the states the same answers leave with no block.
- **The rest guard's readings leave the device.** Detected by A38 and A42, which record every
  request a seeded block sends, and by B1.
- **The time left reads as pressure.** Detected by A36, A41 and B2; the element stays an advisory
  reading and the copy names no deadline.
- **The `api` role is down at a block's end.** The block ends at the role's next start, as a
  catch-up (A13, A19), and the client shows the end meanwhile from the service's end instant.
- **The iPhone and iPad app's device store needs a declared reason in its privacy manifest.** The
  ios-swift pack's privacy-manifest row reads it before the build lands.
- **A stand-in adapter drifts from the predecessor's code.** Each golden records the predecessor's
  commit, and review holds every adapter to it.

## 6. What this does NOT do

- How a minute that both the block and the study count is paid once is not in this SPEC (#924).
- Any push, notification or nudge outside an open session is not in this SPEC, the block-end push
  of #98 included (#98).
- Sending the evening focus nudge is not in this SPEC; it builds only the eligibility (#117).
- The block's card order, and anything a block's screen hides of the scheduler's output, are not in
  this SPEC (#924).
- No economy number changes in this SPEC (#924).
- It builds no Mini App screen and no bot command (#924).
- Committed study windows (#109, SPEC-105) and the session debrief (#131, SPEC-101) are related but
  not in this SPEC: a block books no study window and asks no rating of how the session felt.

## 7. Formal

The build's formal surface is the claim once across a restart (R10, R11) and the block's state
machine (`docs/schematics/focus-blocks.md`). No model is owed: the claim is one conditional update
keyed by the generation inside one write, the only racers on the one row are the `api` role's
timer, a stop, a reached count and a restart's catch-up, and A14 to A19 enumerate each pair. A model
becomes owed if a second role arms the timer again or the claim ever spans two writes.

## 8. Parity goldens

Every golden is registered in `tools/parity-oracle/registry/spec_607.py` and generated from the
predecessor over seeded synthetic inputs; where SPEC-079 names a golden file, this SPEC names the
same file. Days are epoch day numbers and instants epoch milliseconds; no golden carries a date
string. SPEC-079's bot countdown cadence golden is not carried, because no bot countdown is built
(D5).

| golden | the predecessor's rule | proves |
|---|---|---|
| `focus_xp` | block XP | R15 |
| `focus_day_xp` | the day's capped sum | R15 |
| `focus_streak` | the focus streak | R17 |
| `focus_progress` | today's and the week's progress | R17 |
| `focus_subject_breakdown` | the subject breakdown | R17 |
| `focus_clamp_minutes` | the entry clamp | R1 |
| `focus_badges` | the badge conditions | R18 |
| `focus_record_entry` | the entry's clamp and force-abandon | R9 |
| `focus_recompute_xp` | the three settled sources | R16 |
| `focus_nudge_eligible` | the nudge's eligibility | R19 |
| `focus_daily_minutes` | the daily minutes series | R20 |
| `focus_stop_minutes` | the stop's minutes | R9 |
| `focus_cycle_advance` | the cycle's advance | R3 |
| `focus_add_minutes` | add five minutes | R9 |
| `focus_rearm` | the restart decision | R11 |
| `focus.constants` | every focus constant | R21 |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S60701-QUALIFY-FLOOR-IS-FIFTEEN` | `crates/focus/src/constants.rs` | a block under the floor is abandoned and earns nothing | `focus_timer::an_early_stop_at_fourteen_minutes_earns_nothing` |
| `S60702-THE-DAY-CAP` | `crates/focus/src/constants.rs` | the day's focus XP stops at the cap | `focus_xp::the_day_xp_matches_the_predecessors_golden` |
| `S60703-THE-ENTRY-CAP` | `crates/focus/src/constants.rs` | an entry and add-five stop at the entry cap | `focus_timer::adding_five_minutes_stops_at_the_entry_cap` |
| `S60704-THE-LONG-BREAK-EVERY-FOURTH` | `crates/focus/src/constants.rs` | the default long break follows every fourth work block | `focus_timer::a_cycle_advances_as_the_predecessors_cycle_does` |
| `S60705-A-CLAIM-NEEDS-AN-UNCLAIMED-ROW` | `crates/focus/src/repository.rs` | the claim's SQL requires an unclaimed row | `focus_claim::a_second_claim_of_one_block_wins_nothing` |
| `S60706-A-CLAIM-NAMES-ITS-GENERATION` | `crates/focus/src/repository.rs` | the claim's SQL requires the block's own generation | `focus_claim::a_stale_timer_claims_nothing` |
| `S60707-A-CLAIM-WAITS-FOR-THE-END` | `crates/focus/src/repository.rs` | the timer's claim requires the end to have passed | `focus_claim::a_timer_claims_nothing_before_its_end` |
| `S60708-THE-TIMER-IS-ONE-ROW` | `migrations/060701_focus_timer_and_log.sql` | the timer table's one-row check | `focus_claim::the_timer_is_one_row` |
| `S60709-DEEP-DIVER-IS-CERTIFIED-ONLY` | `crates/focus/src/badges.rs` | only a certified block counts toward the longest certified block | `focus_badges::a_manual_block_never_earns_deep_diver` |
