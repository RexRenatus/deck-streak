# SPEC-102: landmarks and milestones are celebrated once, and the widget is one pinned message a study day

- **Wave:** W5. **Issues:** #127 (the historical landmarks), #128 (the milestone pings and queue
  zero) and #121 (the pinned daily widget), in epic #6. **Context(s):** `deck-streak-notifications`
  (the landmarks' rule and texts, the milestone and queue-zero texts, an occasion's dice, the
  widget's text and mood, the router's widget refresh and T5 re-pin, the kind `widget`,
  `widget_messages`); `deck-streak-ingest` (the study days of the whole scoped log);
  `deck-streak-coordination` (the landmarks and queue-zero steps of the recompute, the milestone
  occasions' texts, the widget's inputs, the sync cycle's widget step and the job `widget`);
  `deck-streak-bot` (the widget's send, edit, pin and unpin calls); `deck-streak-daemon` (the
  wiring).
- **Decided by:** ADR-109 (this SPEC's: the widget is one silent pinned message a study day, edited
  in place through the router), ADR-071 (a closed study day is settled as the current day it was, so
  a today-only rule runs at its settle), ADR-037 (one scheduled sync a study day), ADR-095 (a read
  of what was ever studied reads the whole scoped log), ADR-041 (the router core and the surface
  rule), ADR-052 (deep links are URL buttons), ADR-011 and ADR-027 (the job minutes), ADR-053 (the
  private rail's reserved slots) and ADR-012 (the parity oracle).
- **Prerequisites:** SPEC-041 (the router), SPEC-084 (the ladder, its dice and pin calls and the
  streak-break facts), SPEC-100 (the button rows and the token `today`), SPEC-071 (the fold and the
  stored rollup), SPEC-072 (level info, the level-up occasion, `backlog_zero` and the Ascendant
  day), SPEC-073 (the badge occasion and the catalog), SPEC-076 (the language streak, its heat, the
  governor's strength and lapse), SPEC-080 (the day's quests and the weekly quest), SPEC-081 (the
  token's window), SPEC-083 (the skip set), SPEC-090 (the daily goal), SPEC-106 (the active wager),
  SPEC-023 (the reader, its scope and the study event), SPEC-020 (the owner's zone), SPEC-027 (the
  job table), SPEC-021 (the six files of a table) and SPEC-029 (the goldens); and #291, the flush at
  the quiet window's end, which delivers what the morning's recompute raises. **Mutation band:**
  `S10200-S10299`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-102.md` (ADR-016).

## 1. The problem, measured

- **What exists.** The ladder declares the events `landmark_anniversary` and `landmark_study_day` at
  T2, `badge` and `level_up` at T2 and `queue_zero` at T4 (`notifications-policy.json`,
  `ladder.events`). SPEC-073 R4 raises a badge's occasion and SPEC-072 R14 a level-up's, each with
  the router's line only, and each leaves its wording to #128; SPEC-072 and SPEC-084 leave the
  queue-zero ping to #128, SPEC-084 leaves the landmarks to #127, and SPEC-076, SPEC-072 and
  SPEC-084 leave the widget, its footer and its re-pin to #121. An occasion carries no dice emoji,
  so a T4 throws the default one. The transport port has no call that edits a sent message or
  unpins one: SPEC-084's reveal edits inside `push_message`. No landmark, milestone text, queue-zero
  ping or widget exists. SPEC-073 leaves a record's wording to #128 too, but the predecessor's pings
  name badges and level-ups only (`GamifyPipeline._notify_milestones`), so a record keeps the
  router's line.
- **What is ported** (at `27ee2bc`):
  - the landmarks, `landmarks.py:compute_landmarks`, `due_today`, `render_landmark`,
    `_ordinal_label`, `run_landmarks` and `_gap_honest`;
  - the milestone pings, `pipeline.py:GamifyPipeline._notify_milestones`,
    `telegram.py:render_milestone` and `_streak_line`, and the queue-zero ping in
    `GamifyPipeline._run_sync_cycle_impl`;
  - the widget, `pipeline_layers/showcase.py:ShowcaseLayer._update_widget`, `_widget_payload` and
    `_repin_widget`, and `telegram.py:render_widget` and `widget_mood`.
- **Traps a hand port falls into.**
  - A Feb 29 first study day has its anniversary on Feb 28 in a year that is not a leap year. An
    anniversary that falls on the day evaluated is kept, and one after it ends the list. The list is
    sorted by day and then by key.
  - The ordinal's suffix is `th` for every value whose last two digits are 11, 12 or 13, so 111 is
    `111th` and 121 is `121st`.
  - The first run seeds the high-water mark and delivers at most one due landmark. After it, the
    router's once-ever dedupe is the only guard: the mark never suppresses a replay.
  - The anniversary's honest variant reads whether the language streak's last study day is the day
    evaluated, not the lapse rule.
  - A milestone renders the level-up lines, then the badge lines, when both are given; the reward
    line only for a reward above 0; and the streak line with `🔥` when the heat is empty.
  - The widget leads with the strength line, not the streak, on a day the streak broke. Its lapse
    mood names 3 cards as a literal. `Radiant` needs reviews at least the goal and at least 1. An
    unset goal reads as 30. A day with no rollup reads 0 done of 0.
  - The strength's percent is formatted as Python's `.0f`, which rounds half to even on the binary
    value. A quest's "to go" is the whole part of what is left and shows only above a target of 1.
    The token's time is the window's end in UTC, as hours and minutes, the seconds cut off. The
    wager's day and length count both ends. The weekly quest shows only while it is not completed.
  - An edit whose text equals the message's answers "message is not modified", which is a success.
- **Deviations from the predecessor, each with its reason.**
  - #127's evening job becomes a step of the recompute's awards phase, run for each study day the
    fold settles or evaluates (ADR-071). Under one sync a study day (ADR-037), a study day's
    landmark is first read by the next morning's sync; a job reading the last sync's log in the
    evening would find it dated the day before, and it would never be due. A landmark is raised at
    its day's settle, or the same day after an owner's sync.
  - The honest variant therefore reads the streak as of the day evaluated: at the morning's
    evaluation of the current day, which has no study yet, an anniversary carries it.
  - Queue zero is likewise a step of the awards phase: a closed day is settled with its end-of-day
    snapshot, so a queue cleared that day is celebrated at its settle (ADR-071).
  - What the morning's recompute raises falls in quiet hours and is held, and #291's flush delivers
    it when the window ends.
  - The widget is the kind `widget`, one message a study day, edited in place through the router,
    held by quiet hours like every class but the alert, and refreshed after each sync cycle and
    every hour (ADR-109).
  - The widget carries an Open app button (the token `today`), as #121's note on the Mini App asks.
    Its change test is the sha256 of its text, which the predecessor's per-process hash is not.
  - `widget_enabled` and `widget_mood` turn off only at `"0"`, as every kind's switch does
    (SPEC-041); the predecessor turns them off at any value but `"1"`. `widget_enabled` is seeded
    `"0"` while the predecessor runs side by side (ADR-011), and the cutover sets it (#62).
  - The predecessor's `notify_milestones` gates every celebration (`CelebrationsLayer.celebrate`),
    which the kind `celebration`'s switch `celebrations_enabled` already does.
  - #128's in-app activity feed and #127's timeline are Mini App screens, for the wave that polishes
    the surfaces (#8).

## 2. Requirements

The landmarks (#127)

R1. `crates/ingest/src/study_days.rs` reads, under the shared collection lock on the kernel's
    `Offload` (SPEC-023 R1), every study event in scope of the whole log (SPEC-023 R2) and answers
    their distinct study days, oldest first. It keeps one value per study day as it reads, so its
    memory grows with the days studied and not with the reviews. It reads the whole scoped log as
    ADR-095 reads the ids ever reviewed.
R2. `crates/notifications/src/landmarks.rs` computes the landmarks from those study days and the day
    evaluated, equal to the golden `landmarks` (`landmarks.py:compute_landmarks`): each anniversary
    of the first study day up to the day evaluated, with key `landmark:anniv:<n>` and event
    `landmark_anniversary`, and each study day whose count is a multiple of `LANDMARK_DAY_STEP` (25)
    up to the day evaluated, with key `landmark:day:<n>` and event `landmark_study_day`, sorted by
    day and then key. No study day answers no landmark.
R3. The landmarks due on a day are those whose day is that day (`landmarks.py:due_today`).
R4. A landmark's text equals the golden `landmark_text` (`landmarks.py:render_landmark` and
    `_ordinal_label`): the anniversary's, its honest variant's and the study day's, over the
    ordinals of every suffix. The step and the three templates equal the golden
    `landmarks.constants`.
R5. The landmarks step runs in the awards phase, the seventh of the fold's step order (ADR-071),
    after the badges. It reads the study days once a recompute, and for each study day the fold
    settles or evaluates, in order, it raises each landmark due that day as one celebration through
    the router, with its event, its key and its text. An anniversary carries the honest variant when
    the language streak's last study day, as of the day evaluated, is not that day
    (`landmarks.py:_gap_honest`, SPEC-076).
R6. When `notification_settings` holds no `landmark_high_water`, the step first stores it as the
    predecessor's JSON (`seeded`, and the highest anniversary and study-day ordinals among the
    landmarks, keys sorted), then raises at most the first due landmark of that recompute. The run
    equals the golden `landmarks_run` (`landmarks.py:run_landmarks`). The mark is never read again
    to suppress a landmark.

The milestone pings and queue zero (#128)

R7. `crates/notifications/src/milestone.rs` renders a milestone from notifications' own input: a
    badge's emoji, name and description, a level's number, title, emoji and XP to the next level, a
    reward, and the language streak's heat, current length, longest and freezes. The text equals the
    golden `milestone_text` (`telegram.py:render_milestone` and `_streak_line`), over a badge, a
    level-up, both, neither, a reward of 0 and above, and an empty heat.
R8. The badge occasion of SPEC-073 R4 carries the milestone text of its badge, and the level-up
    occasion of SPEC-072 R14 the text of the level reached (SPEC-072 R13), each with a reward of 0
    and the language streak as of the day evaluated (SPEC-076), as
    `GamifyPipeline._notify_milestones` renders them.
R9. The queue-zero step runs in the awards phase, after the landmarks. For each study day the fold
    settles or evaluates that holds a study review in scope and a `backlog_zero` grant above 0
    (SPEC-072 R12), it raises one celebration with the event `queue_zero`, the key
    `queue_zero:<epoch day>`, the rarity `epic`, the dice `🎯` and the text
    `🎯 <b>QUEUE ZERO!</b> Every card cleared — all decks at rest.`, equal to the golden
    `queue_zero_ping` (`GamifyPipeline._run_sync_cycle_impl`). A day with no study review, a
    declared skip day among them, raises none.
R10. An occasion may carry a dice emoji. At T4 and T5 the router throws it in place of the golden
    `celebration_tier`'s default, and an occasion without one throws the default (amending SPEC-084
    R8 by addition). A deferred occasion keeps it in `notification_queue.dice`, `NULL` by default,
    created by `migrations/010202_notifications_queue_dice.sql`, and the flush throws it.

The pinned widget (#121)

R11. `crates/notifications/src/widget.rs` renders the widget from notifications' own input: the
    language streak's length, the governor's strength, whether the streak broke on the current study
    day (SPEC-084 R5's rule), the mood, the current study day's reviews and due cards, the day's
    quest lines and the footer. The text equals the golden `widget_text`
    (`telegram.py:render_widget`), empty lines dropped.
R12. The mood equals the golden `widget_mood` (`telegram.py:widget_mood`) over its outcome set, the
    first that holds, in this order: an open lapse,
    `✨ The phoenix waits in the ashes — 3 cards relight it.`; a skip day,
    `🧊 Rest day — the flame is banked, not out.`; reviews at least the larger of 1 and the goal,
    `🐦‍🔥 Radiant — goal met.`; reviews above 0, `🙂 Warming up.`; a local hour of 22 or later,
    `🪶 Embers... the day is almost gone.`; a local hour of 18 or later,
    `😐 The phoenix dims — nothing yet today.`; otherwise `🌅 A fresh day.`. A goal not yet stored
    reads as 30. The mood line is empty when `widget_mood` is `"0"`.
R13. The quest lines and the footer equal the golden `widget_payload`
    (`ShowcaseLayer._widget_payload`). A sealed quest reads `✉️ sealed quest`; any other reads `✅`
    when completed or `◻️`, then its emoji, then ` · <n> to go` when open with a target above 1.
    The footer follows, each line only while it holds, in this order: an Ascendant day,
    `🔱 Ascendant: +25% XP today`; a token's window open, `⚡ 2× XP until <HH:MM> UTC`; a wager
    active, `🎲 Wager: day <n>/<length> · <stake> 🪙`; the week's quest not completed,
    `📅 Week: <progress>/<target>`.
R14. The widget's inputs are, each that context's own and a read only: analytics' stored rollup of
    the current study day, its reviews and its due-today count (SPEC-071 R6 and R8; a missing
    rollup or a NULL card state reads as 0); the language streak's length, last study day and
    longest (SPEC-076, SPEC-084 R5); the governor's strength and open lapse (SPEC-076 R13, R16); the
    daily goal (SPEC-090 R4); whether the current study day is in the skip set (SPEC-083 R4); the
    day's quests, each with its emoji, target and progress and whether it is sealed or completed,
    and the week's quest with its progress, target and completion (SPEC-080 R3, R25); whether the
    day is an Ascendant day (SPEC-072 R22); the open token window's end (SPEC-081 R12); the active
    wager's first and last days and stake (SPEC-106 R1); and the refresh's local hour in the
    owner's zone (SPEC-020).
    `crates/coordination/src/widget.rs` gathers them and passes plain values.
R15. Coordination refreshes the widget by routing one occasion of the kind `widget` for the current
    study day, with the key `widget` and the text R11 renders, so `route` stays the one entry point
    (SPEC-041 R1). For that kind `route` decides in this order: when `widget_messages` holds the
    study day with the text's sha256, it answers unchanged, pushing and recording nothing; when
    `widget_enabled` is `"0"` it withholds with `nudges_disabled`; inside the quiet window it
    withholds with `quiet_hours`; with no bot transport joined, or while the outage breaker is open
    (SPEC-041), it withholds with `no_notifier`. Each withhold is recorded under `widget:withheld`,
    and nothing is pushed.
R16. Otherwise, with no row for the study day, `route` claims the kind for the study day (SPEC-041's
    claim), sends the text silently with one row, Open app (the token `today`), pins it silently,
    stores its message id and sha256, unpins the widget of the study day before when one is stored,
    and records one send. A failed send releases the claim, records `no_notifier` and opens the
    outage breaker, as a failed send of any kind does (SPEC-041).
R17. With a row whose sha256 differs, `route` edits the stored message to the text with the same
    row. An edit answered "message is not modified" counts as edited. An edited message stores the
    new sha256 and records one send. A failed edit sends the text anew with the row and pins it, and
    stores the new message id and sha256. When that send fails too, it records `no_notifier`, opens
    the outage breaker and keeps the row, so a later refresh tries again.
R18. After a T5 pins its message, the router unpins and pins the current study day's widget, when
    one is stored, so the widget stays the newest pin (`ShowcaseLayer._repin_widget`); the flush's
    T5 does the same (amending SPEC-084 R8 by addition). The widget's calls equal the golden
    `widget_update`'s (`ShowcaseLayer._update_widget`): the first refresh, an unchanged one, a
    changed one and a failed edit.
R19. The bot transport gains `push_widget` (a silent send with its row, answering the message id or
    a failure), `push_edit` (answering edited, unchanged or failed) and `push_unpin`, each given the
    message id it acts on; the widget pins with the `push_pin` of SPEC-084 R8, given the stored
    message id. `router.transport.bot` in the policy names the three, so the policy's delivery calls
    are eight where SPEC-041 R1 counts five, and SPEC-041 R1's rule holds for all eight: only the
    router module calls them. The census of SPEC-041's A15 names each call's one site in the bot
    transport.
R20. The sync cycle refreshes the widget after its recompute, and a cycle whose sync failed
    refreshes nothing. The job `widget` refreshes it every hour at minute 44, without catch-up, off
    every minute of the predecessor's schedule, the reserved minutes (0, 25, 39), the private rail's
    reserved slots and the sync's slot (SPEC-027 R2, SPEC-053 R2). Its timer
    `deck-streak-job@widget.timer` and the rail contract hold its calendar equal to the table.

Surfaces, tables and names

R21. The policy gains the kind `widget` (class `digest`, tiers `["T2"]`, budget null, dedupe
    `per-study-day`, setting `widget_enabled`), recorded in its deviations with ADR-109.
R22. Notifications owns `widget_messages` (the study day as primary key, the message id, the text's
    sha256, `created_at`, `updated_at`), `STRICT`, created by
    `migrations/010201_notifications_widget_messages.sql`. It owes SPEC-021's six files;
    notifications' data-rights port exports and erases it.
R23. `migrations/010203_notifications_widget_side_by_side_default.sql` seeds `widget_enabled` as
    `"0"` with `INSERT OR IGNORE` (ADR-011), so no second widget is pinned while the predecessor
    runs; `widget_mood` is left unset, which is on.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the landmarks equal the golden `landmarks`: a Feb 29 origin in a year that is not a leap year, an anniversary on the day evaluated and one the day after, the 24th, 25th and 50th study days, and no study day | `the_landmarks_match_the_parity_golden` |
| A2 | the due landmarks are exactly those dated the day | `only_the_landmarks_dated_the_day_are_due` |
| A3 | the texts equal the golden `landmark_text` at the ordinals 1, 2, 3, 4, 11, 12, 13, 21, 111 and 121, both anniversary variants and the study day's | `the_landmark_text_matches_the_parity_golden` |
| A4 | the step and the templates equal the golden `landmarks.constants` | `the_landmark_constants_equal_the_predecessors` |
| A5 | the milestone text equals the golden `milestone_text` | `the_milestone_text_matches_the_parity_golden` |
| A6 | a T4 and a T5 throw the occasion's dice, one without a dice throws the default, and a deferred occasion keeps its dice through the flush | `an_occasions_dice_is_thrown_and_kept_through_a_deferral` |
| A7 | the widget text equals the golden `widget_text`, the broke-today line included | `the_widget_text_matches_the_parity_golden` |
| A8 | the mood equals the golden `widget_mood` in each of its seven outcomes, at reviews equal to the goal and one below, a goal of 0, and the hours 17, 18, 21 and 22 | `the_widget_mood_matches_the_parity_golden` |
| A9 | the mood line is absent at `widget_mood` `"0"` and present when it is unset or any other value | `the_mood_switch_turns_the_mood_line_off_only_at_zero` |
| A10 | the quest lines and the footer equal the golden `widget_payload`: a sealed quest, a completed one, targets of 1 and 15, each footer line alone and all four, an unset goal and a day with no rollup | `the_widget_payload_matches_the_parity_golden` |
| A11 | the first refresh of a study day sends silently with the Open app row, pins it, unpins the day before's widget, stores the row and records one send | `the_first_refresh_of_a_day_sends_pins_and_unpins_the_day_before` |
| A12 | a refresh whose text is unchanged pushes nothing, re-pins nothing and records nothing | `an_unchanged_widget_is_never_sent_again_or_re_pinned` |
| A13 | a changed refresh edits the message with the same row, and a "message is not modified" answer counts as edited | `a_changed_widget_is_edited_in_place_and_not_modified_is_success` |
| A14 | a failed edit sends anew and pins, replacing the message id; a failed send records `no_notifier`, keeps the row and opens the breaker, and a refresh while it is open pushes nothing | `a_failed_edit_sends_anew_and_a_failed_send_keeps_the_row` |
| A15 | at `widget_enabled` `"0"` a first or changed refresh is withheld with `nudges_disabled`, and nothing is pushed | `the_widget_switch_stops_every_push` |
| A16 | a first or changed refresh inside the quiet window is withheld with `quiet_hours`, and the first refresh after the window sends it | `quiet_hours_hold_the_widget_until_the_window_ends` |
| A17 | the refreshes make the golden `widget_update`'s calls, in order | `the_refreshes_make_the_parity_goldens_calls` |
| A18 | a T5 unpins and pins the day's widget after its own pin, and a T5 with no widget stored re-pins nothing | `a_t5_re_pins_the_days_widget` |
| A19 | the kind `widget` is declared with its class, tier, dedupe and setting, a recorded deviation, and the transport names its three calls | `the_widget_kind_and_its_calls_are_declared` |
| A20 | `widget_enabled` is seeded off, an existing value is kept, and `widget_mood` is left unset | `the_widget_switch_starts_off_beside_the_predecessor` |
| A21 | `widget_messages` is exported and erased by notifications' port | `the_widget_messages_are_exported_and_erased` |
| A22 | `push_widget` sends silently with its row and answers the message id, `push_pin` pins that id silently and `push_unpin` unpins it | `the_widget_calls_send_silently_and_pin_and_unpin_by_id` |
| A23 | `push_edit` answers unchanged for "message is not modified", and failed for any other refusal | `an_edit_reads_not_modified_as_unchanged` |
| A24 | the study days are every study day of a scoped study event of the whole log, once each and oldest first: a review out of scope, a review that is not a study event, and two reviews on one day | `the_study_days_are_every_scoped_study_event_day_once` |
| A25 | each study day the fold settles or evaluates raises the landmarks due that day, with their events, keys and texts, and a second recompute raises none | `each_evaluated_day_raises_its_due_landmarks_once` |
| A26 | the first run stores the mark and raises at most one landmark, equal to the golden `landmarks_run`, and a later run with the mark raises every due one | `the_first_run_seeds_the_mark_and_raises_at_most_one` |
| A27 | an anniversary is the honest variant when the streak's last study day is not the day evaluated, and the plain one when it is | `the_anniversary_is_honest_on_a_day_without_study` |
| A28 | a study day with a `backlog_zero` grant above 0 raises one `queue_zero` occasion, epic, with the dice and the golden `queue_zero_ping`'s text, and a second recompute raises none | `queue_zero_is_raised_once_for_a_cleared_study_day` |
| A29 | a declared skip day with no review, and a study day whose `backlog_zero` is 0, raise no `queue_zero` | `a_skip_day_or_an_open_queue_raises_no_queue_zero` |
| A30 | the badge occasion carries its badge's milestone text with the streak as of the day evaluated | `the_badge_occasion_carries_the_milestone_text` |
| A31 | the level-up occasion carries the reached level's milestone text | `the_level_up_occasion_carries_the_milestone_text` |
| A32 | the widget reads the rollup's reviews and due-today count, the streak's length, last study day and longest, the strength, the open lapse, the goal, the skip set, the day's quests, the week's quest, the Ascendant day, the token's end, the wager's days and stake and the local hour, and a fixture that changes any one of them changes the text | `the_widget_reads_every_input_it_names` |
| A33 | the sync cycle refreshes the widget after its recompute, and a failed sync refreshes nothing | `the_sync_cycle_refreshes_the_widget_after_the_recompute` |
| A34 | the job `widget` fires hourly at minute 44 without catch-up, off every predecessor, reserved and private-rail minute and the sync's slot, and its timer and the rail contract hold the calendar | `the_widget_job_keeps_off_every_reserved_minute` |

```acceptance
A1: cargo test -p deck-streak-notifications --test landmarks -- --exact the_landmarks_match_the_parity_golden
A2: cargo test -p deck-streak-notifications --test landmarks -- --exact only_the_landmarks_dated_the_day_are_due
A3: cargo test -p deck-streak-notifications --test landmarks -- --exact the_landmark_text_matches_the_parity_golden
A4: cargo test -p deck-streak-notifications --test landmarks -- --exact the_landmark_constants_equal_the_predecessors
A5: cargo test -p deck-streak-notifications --test milestones -- --exact the_milestone_text_matches_the_parity_golden
A6: cargo test -p deck-streak-notifications --test milestones -- --exact an_occasions_dice_is_thrown_and_kept_through_a_deferral
A7: cargo test -p deck-streak-notifications --test widget -- --exact the_widget_text_matches_the_parity_golden
A8: cargo test -p deck-streak-notifications --test widget -- --exact the_widget_mood_matches_the_parity_golden
A9: cargo test -p deck-streak-notifications --test widget -- --exact the_mood_switch_turns_the_mood_line_off_only_at_zero
A10: cargo test -p deck-streak-notifications --test widget -- --exact the_widget_payload_matches_the_parity_golden
A11: cargo test -p deck-streak-notifications --test widget_router -- --exact the_first_refresh_of_a_day_sends_pins_and_unpins_the_day_before
A12: cargo test -p deck-streak-notifications --test widget_router -- --exact an_unchanged_widget_is_never_sent_again_or_re_pinned
A13: cargo test -p deck-streak-notifications --test widget_router -- --exact a_changed_widget_is_edited_in_place_and_not_modified_is_success
A14: cargo test -p deck-streak-notifications --test widget_router -- --exact a_failed_edit_sends_anew_and_a_failed_send_keeps_the_row
A15: cargo test -p deck-streak-notifications --test widget_router -- --exact the_widget_switch_stops_every_push
A16: cargo test -p deck-streak-notifications --test widget_router -- --exact quiet_hours_hold_the_widget_until_the_window_ends
A17: cargo test -p deck-streak-notifications --test widget_router -- --exact the_refreshes_make_the_parity_goldens_calls
A18: cargo test -p deck-streak-notifications --test widget_router -- --exact a_t5_re_pins_the_days_widget
A19: cargo test -p deck-streak-notifications --test widget_router -- --exact the_widget_kind_and_its_calls_are_declared
A20: cargo test -p deck-streak-notifications --test widget_router -- --exact the_widget_switch_starts_off_beside_the_predecessor
A21: cargo test -p deck-streak-notifications --test widget_router -- --exact the_widget_messages_are_exported_and_erased
A22: cargo test -p deck-streak-bot --test widget_transport -- --exact the_widget_calls_send_silently_and_pin_and_unpin_by_id
A23: cargo test -p deck-streak-bot --test widget_transport -- --exact an_edit_reads_not_modified_as_unchanged
A24: cargo test -p deck-streak-ingest --test study_days -- --exact the_study_days_are_every_scoped_study_event_day_once
A25: cargo test -p deck-streak-coordination --test landmarks_step -- --exact each_evaluated_day_raises_its_due_landmarks_once
A26: cargo test -p deck-streak-coordination --test landmarks_step -- --exact the_first_run_seeds_the_mark_and_raises_at_most_one
A27: cargo test -p deck-streak-coordination --test landmarks_step -- --exact the_anniversary_is_honest_on_a_day_without_study
A28: cargo test -p deck-streak-coordination --test queue_zero_step -- --exact queue_zero_is_raised_once_for_a_cleared_study_day
A29: cargo test -p deck-streak-coordination --test queue_zero_step -- --exact a_skip_day_or_an_open_queue_raises_no_queue_zero
A30: cargo test -p deck-streak-coordination --test milestone_texts -- --exact the_badge_occasion_carries_the_milestone_text
A31: cargo test -p deck-streak-coordination --test milestone_texts -- --exact the_level_up_occasion_carries_the_milestone_text
A32: cargo test -p deck-streak-coordination --test widget_step -- --exact the_widget_reads_every_input_it_names
A33: cargo test -p deck-streak-coordination --test widget_step -- --exact the_sync_cycle_refreshes_the_widget_after_the_recompute
A34: cargo test -p deck-streak-coordination --test widget_job -- --exact the_widget_job_keeps_off_every_reserved_minute
```

A11 to A18 run the router over a recording transport on a manual clock; A22 and A23 run the bot
transport against a recording Bot API stand-in. A25 to A31 run the recompute over a temporary
deployment with synthetic reviews.

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr, notifications-policy and
telegram-platform packs stay enforced, and no row is deferred or lifted for this delivery, so the
private wiring does not change when it merges. The message metadata stays deferred (#257): this
delivery claims none of it.

| id | criterion | decided by |
|---|---|---|
| B1 | `widget_messages` is declared under notifications' categories with data, purpose, basis, retention and erase mode, over `privacy.json`, `migrations/010201_notifications_widget_messages.sql` and `crates/notifications/src/data_rights.rs` | the privacy-gdpr pack |
| B2 | the kind `widget` carries its deviation, and its class is one the pack admits and not exempt from quiet hours, over `notifications-policy.json` | the notifications-policy pack |
| B3 | `push_widget`, `push_edit` and `push_unpin` are called only in the router module, over the shipped sources under `crates/` | the notifications-policy pack |
| B4 | the widget's send, edit, pin and unpin calls stay within the Bot API's rules and its Open app button is a URL button, over `crates/bot/src/transport.rs` and `crates/notifications/src/widget.rs` | the telegram-platform pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/ingest/src/study_days.rs` | `deck-streak-ingest` | added: the study days of the whole scoped log |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the module |
| `crates/ingest/tests/study_days.rs` | `deck-streak-ingest` | added: A24 |
| `crates/notifications/src/landmarks.rs` | `deck-streak-notifications` | added: the landmarks, the due rule, the texts and the high-water mark |
| `crates/notifications/src/milestone.rs` | `deck-streak-notifications` | added: the milestone text, the streak line and the queue-zero text |
| `crates/notifications/src/widget.rs` | `deck-streak-notifications` | added: the widget's text, mood, quest lines and footer, and the table's store |
| `crates/notifications/src/occasion.rs` | `deck-streak-notifications` | changed: an occasion's dice |
| `crates/notifications/src/router.rs` | `deck-streak-notifications` | changed: the kind `widget`'s arm of `route`, the dice at T4 and T5 and through a deferral, and the T5 re-pin |
| `crates/notifications/tests/one_router.rs` | `deck-streak-notifications` | changed: A15's census names the widget's send, edit, pin and unpin at their one site in the bot transport |
| `crates/notifications/src/transport.rs` | `deck-streak-notifications` | changed: `push_widget`, `push_edit` and `push_unpin` |
| `crates/notifications/src/ledger.rs` | `deck-streak-notifications` | changed: the queue's dice |
| `crates/notifications/src/policy.rs` | `deck-streak-notifications` | changed: the kind `widget` and the three calls |
| `crates/notifications/src/lib.rs` | `deck-streak-notifications` | changed: the three modules |
| `crates/notifications/src/data_rights.rs` | `deck-streak-notifications` | changed: `widget_messages`, exported and erased |
| `crates/notifications/tests/landmarks.rs` | `deck-streak-notifications` | added: A1 to A4 |
| `crates/notifications/tests/milestones.rs` | `deck-streak-notifications` | added: A5, A6 |
| `crates/notifications/tests/widget.rs` | `deck-streak-notifications` | added: A7 to A10 |
| `crates/notifications/tests/widget_router.rs` | `deck-streak-notifications` | added: A11 to A21 |
| `crates/coordination/src/recompute/landmarks.rs` | `deck-streak-coordination` | added: the landmarks step and the first run |
| `crates/coordination/src/recompute/queue_zero.rs` | `deck-streak-coordination` | added: the queue-zero step |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: the two steps join the awards phase of the fold |
| `crates/coordination/src/recompute/badges.rs` | `deck-streak-coordination` | changed: the badge occasion's milestone text |
| `crates/coordination/src/progression/level_view.rs` | `deck-streak-coordination` | changed: the level-up occasion's milestone text |
| `crates/coordination/src/widget.rs` | `deck-streak-coordination` | added: the widget's reads and the refresh |
| `crates/coordination/src/sync_cycle.rs` | `deck-streak-coordination` | changed: the widget step after the recompute |
| `crates/coordination/src/jobs.rs` | `deck-streak-coordination` | changed: the job `widget`, which `job_table.rs`'s existing test holds equal to its timer |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the widget module |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: `widget_messages` under notifications' port |
| `crates/coordination/tests/landmarks_step.rs` | `deck-streak-coordination` | added: A25 to A27 |
| `crates/coordination/tests/queue_zero_step.rs` | `deck-streak-coordination` | added: A28, A29 |
| `crates/coordination/tests/milestone_texts.rs` | `deck-streak-coordination` | added: A30, A31 |
| `crates/coordination/tests/widget_step.rs` | `deck-streak-coordination` | added: A32, A33 |
| `crates/coordination/tests/widget_job.rs` | `deck-streak-coordination` | added: A34 |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row for `widget_messages` |
| `crates/bot/src/transport.rs` | `deck-streak-bot` | changed: the widget's send, edit and unpin calls |
| `crates/bot/tests/widget_transport.rs` | `deck-streak-bot` | added: A22, A23 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the job `widget`, the sync cycle's widget step and the study-days read |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: the new ports named |
| `migrations/010201_notifications_widget_messages.sql` | `deck-streak-notifications` | added |
| `migrations/010202_notifications_queue_dice.sql` | `deck-streak-notifications` | added |
| `migrations/010203_notifications_widget_side_by_side_default.sql` | `deck-streak-notifications` | added |
| `notifications-policy.json` | repo | changed: the kind `widget`, its deviation and the three calls |
| `deploy/systemd/deck-streak-job@widget.timer` | deploy | added |
| `deploy/rail-contract.json` | deploy | changed: the calendar key |
| `tools/parity-oracle/registry/spec_102.py` | tools | added: the adapters |
| `tools/parity-oracle/goldens/` | tools | added: the goldens of section 7 |
| `scripts/mutation-rows.d/S10200-S10299.json` | scripts | added: the rows of section 9 |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `widget_messages` |
| `privacy.json` | repo | changed: the table's category |
| `PRIVACY.md` | repo | changed: one line for the category |
| `docs/schematics/landmarks-milestones-and-the-pinned-widget.md` | docs | added by the W5 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/specs/SPEC-102-landmarks-and-milestones-are-celebrated-once-and-the-widget-is-one-pinned-message-a-study-day.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-109-the-widget-is-one-silent-pinned-message-a-study-day-edited-in-place-through-the-router.md` | docs | changed: status accepted |
| `docs/red-first/SPEC-102.md` | docs | added |

## 5. What this does NOT do

- It builds no in-app activity feed of past celebrations and no landmark timeline: both are Mini App
  screens for the wave that polishes the surfaces (#8).
- It flushes no held celebration when quiet hours end: the router's flush does (#291).
- It adds no sync: a landmark or a queue zero is raised at the next sync's recompute, the owner's
  `/sync` included (#286).
- It builds no settings screen for `widget_enabled`, `widget_mood` or the celebrations' switch
  (#57).
- It sends no share card for a landmark or a level: the share card is its own item (#125).
- It renders no band-up: the band-up celebrates its own badge (#85).

## 6. Risks

- **A landmark read a day late and never due.** The step runs at each day's settle, so a study day
  first read by the next morning's sync is due at its own settle (A25).
- **A backlog flood on the first run.** The first run delivers at most one landmark (A26).
- **A widget that flickers or spams.** An unchanged text pushes nothing, and a change is an edit of
  the one message a study day (A12, A13).
- **A widget lost to a deleted message.** A failed edit sends anew and pins (A14).
- **Two widgets beside the predecessor.** The switch is seeded off while it runs (A20).
- **A widget that wakes the owner.** Its sends are silent, and quiet hours hold it (A16, A22).
- **A whole-log read that outgrows the host.** The read keeps one value per study day, never the
  reviews (R1, A24).
- **A ledger row an hour while the widget is off.** While the switch is `"0"`, each refresh with no
  row for the day records a withheld decision, as every withhold does (SPEC-041): one an hour and
  one a sync at most. The kind has no budget (R21), so the rows spend none.

## 7. Parity goldens

All are registered in `tools/parity-oracle/registry/spec_102.py` and generated on the owner's
checkout of the predecessor at `27ee2bc` (SPEC-029). Every day is an epoch day number and every
name synthetic; no golden holds a calendar date or a personal value.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `goldens/landmarks.json` | `landmarks.py:compute_landmarks`, `landmarks.py:due_today` | adapter | synthetic reviews: none, a Feb 29 origin, an anniversary on the day and a day before it, 24, 25 and 50 study days, and reviews that are not study events |
| `goldens/landmark_text.json` | `landmarks.py:render_landmark`, `landmarks.py:_ordinal_label` | adapter | landmarks at the ordinals 1, 2, 3, 4, 11, 12, 13, 21, 111 and 121, each event, and both anniversary variants |
| `goldens/landmarks_run.json` | `landmarks.py:run_landmarks` | adapter | a temporary store with and without the mark, a stub reader, a stored streak and a recording celebrator; no due landmark, one and two |
| `goldens/landmarks.constants.json` | `landmarks.py` | constants | `LANDMARK_DAY_STEP`, `LANDMARK_HIGH_WATER_KEY` and the three templates |
| `goldens/milestone_text.json` | `telegram.py:render_milestone`, `telegram.py:_streak_line` | pure | a badge, a level-up, both and neither; rewards 0 and 25; heats empty and set |
| `goldens/queue_zero_ping.json` | `pipeline.py:GamifyPipeline._run_sync_cycle_impl` | adapter | a stub pipeline whose other steps are patched to do nothing, a study-day set and a stored `backlog_zero`, and a recording celebrator; a study day at 0 and 1, and a day not studied at 50 |
| `goldens/widget_text.json` | `telegram.py:render_widget` | pure | streaks 0 and 12, strengths 0, 0.125, 0.135 and 1, broke and not, an empty mood, 0 done of 0 due, quest lines and a footer |
| `goldens/widget_mood.json` | `telegram.py:widget_mood` | pure | each outcome; reviews 0, 1, 29 and 30 at goals 0 and 30; the hours 0, 17, 18, 21 and 22 |
| `goldens/widget_payload.json` | `pipeline_layers/showcase.py:ShowcaseLayer._widget_payload` | adapter | a stub store and governor: quests sealed, completed and open at targets 1 and 15; each footer line; a token ending at 13:07:59; a wager on its first and last day; an unset goal; no rollup |
| `goldens/widget_update.json` | `ShowcaseLayer._update_widget` | adapter | a temporary store and a recording notifier: the first refresh with and without the day before's widget, an unchanged one, a changed one, a failed edit and a disabled switch |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `widget_messages` | `notifications` | `migrations/010201_notifications_widget_messages.sql` (SPEC-102) | `widget_state`, its day as an epoch day and its message id; its hash differs by process, so the sha256 is left empty and the first refresh edits once | exported and erased |
| `notification_queue` (the column `dice`) | `notifications` | `migrations/010202_notifications_queue_dice.sql` (SPEC-102) | a held celebration's `dice_emoji` | exported and erased with its row |

The mark `landmark_high_water` is a key of `notification_settings`, imported from the predecessor's
setting of the same name.

## 9. Mutation rows

The band is `S10200-S10299`, in `scripts/mutation-rows.d/S10200-S10299.json`. Each row's killer
selects exactly one test, and each is proved with its file restored byte for byte (SPEC-039). No
row's mutant makes a loop or a wait unbounded.

| row | target | what it guards | killer |
|---|---|---|---|
| `S10200-DAY-STEP-25` | `crates/notifications/src/landmarks.rs` | a study-day landmark every 25 days | `landmarks::the_landmarks_match_the_parity_golden` |
| `S10201-LEAP-DAY-TO-28` | `crates/notifications/src/landmarks.rs` | a Feb 29 origin falls on Feb 28 | `landmarks::the_landmarks_match_the_parity_golden` |
| `S10202-ANNIVERSARY-ON-THE-DAY` | `crates/notifications/src/landmarks.rs` | an anniversary on the day evaluated is kept | `landmarks::the_landmarks_match_the_parity_golden` |
| `S10203-TEENS-TAKE-TH` | `crates/notifications/src/landmarks.rs` | 11, 12 and 13 take `th` | `landmarks::the_landmark_text_matches_the_parity_golden` |
| `S10204-DUE-ON-THE-DAY` | `crates/notifications/src/landmarks.rs` | only the day's landmarks are due | `landmarks::only_the_landmarks_dated_the_day_are_due` |
| `S10205-FIRST-RUN-AT-MOST-ONE` | `crates/coordination/src/recompute/landmarks.rs` | the first run raises one landmark at most | `landmarks_step::the_first_run_seeds_the_mark_and_raises_at_most_one` |
| `S10206-EVERY-EVALUATED-DAY` | `crates/coordination/src/recompute/landmarks.rs` | a settled day's landmarks are raised, not only the current day's | `landmarks_step::each_evaluated_day_raises_its_due_landmarks_once` |
| `S10207-HONEST-WITHOUT-STUDY` | `crates/coordination/src/recompute/landmarks.rs` | the honest variant when the day was not studied | `landmarks_step::the_anniversary_is_honest_on_a_day_without_study` |
| `S10208-REWARD-LINE-ABOVE-ZERO` | `crates/notifications/src/milestone.rs` | the reward line only above 0 | `milestones::the_milestone_text_matches_the_parity_golden` |
| `S10209-QUEUE-ZERO-ABOVE-ZERO` | `crates/coordination/src/recompute/queue_zero.rs` | a `backlog_zero` of 0 raises nothing | `queue_zero_step::a_skip_day_or_an_open_queue_raises_no_queue_zero` |
| `S10210-QUEUE-ZERO-NEEDS-STUDY` | `crates/coordination/src/recompute/queue_zero.rs` | a day with no study review raises nothing | `queue_zero_step::a_skip_day_or_an_open_queue_raises_no_queue_zero` |
| `S10211-QUEUE-ZERO-IS-EPIC` | `crates/coordination/src/recompute/queue_zero.rs` | queue zero is epic | `queue_zero_step::queue_zero_is_raised_once_for_a_cleared_study_day` |
| `S10212-DICE-KEPT-IN-THE-QUEUE` | `crates/notifications/src/router.rs` | a deferred occasion keeps its dice | `milestones::an_occasions_dice_is_thrown_and_kept_through_a_deferral` |
| `S10213-RADIANT-AT-THE-GOAL` | `crates/notifications/src/widget.rs` | reviews equal to the goal are radiant | `widget::the_widget_mood_matches_the_parity_golden` |
| `S10214-EMBERS-AT-22` | `crates/notifications/src/widget.rs` | the hour 22 is embers | `widget::the_widget_mood_matches_the_parity_golden` |
| `S10215-DIMS-AT-18` | `crates/notifications/src/widget.rs` | the hour 18 dims | `widget::the_widget_mood_matches_the_parity_golden` |
| `S10216-TO-GO-ABOVE-ONE` | `crates/notifications/src/widget.rs` | a target of 1 shows no "to go" | `widget::the_widget_payload_matches_the_parity_golden` |
| `S10217-UNCHANGED-PUSHES-NOTHING` | `crates/notifications/src/router.rs` | an unchanged widget is not pushed | `widget_router::an_unchanged_widget_is_never_sent_again_or_re_pinned` |
| `S10218-UNPIN-THE-DAY-BEFORE` | `crates/notifications/src/router.rs` | the day before's widget is unpinned | `widget_router::the_first_refresh_of_a_day_sends_pins_and_unpins_the_day_before` |
| `S10219-QUIET-HOLDS-THE-WIDGET` | `crates/notifications/src/router.rs` | quiet hours withhold the widget | `widget_router::quiet_hours_hold_the_widget_until_the_window_ends` |
| `S10220-REPIN-AFTER-T5` | `crates/notifications/src/router.rs` | a T5 re-pins the day's widget | `widget_router::a_t5_re_pins_the_days_widget` |
| `S10221-NOT-MODIFIED-IS-SUCCESS` | `crates/bot/src/transport.rs` | "message is not modified" is unchanged | `widget_transport::an_edit_reads_not_modified_as_unchanged` |
| `S10222-WIDGET-AT-MINUTE-44` | `crates/coordination/src/jobs.rs` | the job fires at minute 44 | `widget_job::the_widget_job_keeps_off_every_reserved_minute` |
