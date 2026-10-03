# SPEC-102: landmarks and milestones are celebrated once, and the widget is one pinned message a study day

- **Wave:** W5. **Issues:** #127 (the historical landmarks), #128 (the milestone pings and queue
  zero) and #121 (the pinned daily widget), in epic #6. **Context(s):** `deck-streak-notifications`
  (the landmarks' rule and texts, the milestone and queue-zero texts, an occasion's dice, the
  widget's text and mood, the router's widget refresh and T5 re-pin, the kind `widget`,
  `widget_messages`); `deck-streak-ingest` (the study days of the whole scoped log);
  `deck-streak-coordination` (the landmarks and queue-zero steps of the recompute, the milestone
  occasions' texts, the widget's inputs, the sync cycle's widget step and the job `widget_refresh`);
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
- **Status:** delivered in part (moved from `docs/specs/planned/` with its tests and
  `docs/red-first/SPEC-102.md`, ADR-016): ART1a delivered #127's landmarks, their due rule and texts,
  and the study days of the whole scoped log (ADR-318); #127's second part delivers the landmarks step
  and the first run, #128 the milestone pings and queue zero, and #121 the pinned widget (section 3c).

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
    held by quiet hours like every class but the alert, and refreshed after each of the owner's sync cycles and every hour (ADR-109).
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
R12. The mood equals the golden `widget_mood` (`telegram.py:widget_mood`) over its outcome set of
    seven texts, each the golden's verbatim, the first that holds, in this order: an open lapse, the
    relight text (3 cards relight the streak); a skip day, the rest-day text; reviews at least the
    larger of 1 and the goal, the radiant text; reviews above 0, the warming text; a local hour of
    22 or later, the embers text; a local hour of 18 or later, the dimming text; otherwise the
    fresh-day text. A goal not yet stored reads as 30. The mood line is empty when `widget_mood` is
    `"0"`.
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
R20. The owner's sync cycle, the bot role's cycle that holds the router and its transport, refreshes
    the widget after its recompute, and a cycle whose sync failed refreshes nothing. The scheduled
    sync's cycle carries no router, so it refreshes nothing and attempts no send; whether it gains
    a router is #291's. The job `widget_refresh` refreshes it every hour at minute 44, without
    catch-up, off every minute of the predecessor's schedule, the reserved minutes (0, 25, 39), the
    private rail's reserved slots and the sync's slot (SPEC-027 R2, SPEC-053 R2). Its timer
    `deck-streak-job-send@widget_refresh.timer` and the rail contract hold its calendar equal to the
    table.

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
| A24 | the study days are every study day of a scoped study event of the whole log, once each and oldest first: a review out of scope, a review that is not a study event, and two reviews on one day | `the_study_days_are_every_scoped_study_event_day_once` |
| A25 | each study day the fold settles or evaluates raises the landmarks due that day, with their events, keys and texts, and a second recompute raises none | `each_evaluated_day_raises_its_due_landmarks_once` |
| A26 | the first run stores the mark and raises at most one landmark, equal to the golden `landmarks_run`, and a later run with the mark raises every due one | `the_first_run_seeds_the_mark_and_raises_at_most_one` |
| A27 | an anniversary is the honest variant when the streak's last study day is not the day evaluated, and the plain one when it is | `the_anniversary_is_honest_on_a_day_without_study` |
| A35 | a landmark the router did not answer keeps the cursor before its day and is offered again by the next recompute, and an answered one moves it | `an_unanswered_landmark_keeps_the_cursor_and_is_offered_again` |
| A36 | the first run never moves the cursor, so a seed-day landmark it did not raise is raised by a later run after that day settles | `the_first_run_never_moves_the_cursor_past_a_landmark_it_did_not_raise` |
| A37 | a recompute that finds the imported mark and no cursor stores the cursor at yesterday and raises every landmark due today and none dated before | `an_imported_mark_raises_todays_landmarks_and_no_history` |
| A38 | the mark a first run stores, read from `notification_settings` after the sync cycle, equals the golden `landmarks_run`'s bytes for every case | `the_high_water_mark_equals_the_predecessors_bytes` |
| A39 | the sync cycle offers the landmarks after the awards on every offer call, and a failed whole-log read offers none and moves no cursor | `the_sync_cycle_offers_the_landmarks_after_the_awards` |

```acceptance
A1: cargo test -p deck-streak-notifications --test landmarks -- --exact the_landmarks_match_the_parity_golden
A2: cargo test -p deck-streak-notifications --test landmarks -- --exact only_the_landmarks_dated_the_day_are_due
A3: cargo test -p deck-streak-notifications --test landmarks -- --exact the_landmark_text_matches_the_parity_golden
A4: cargo test -p deck-streak-notifications --test landmarks -- --exact the_landmark_constants_equal_the_predecessors
A24: cargo test -p deck-streak-ingest --test study_days -- --exact the_study_days_are_every_scoped_study_event_day_once
A25: cargo test -p deck-streak-coordination --test landmarks_step -- --exact each_evaluated_day_raises_its_due_landmarks_once
A26: cargo test -p deck-streak-coordination --test landmarks_step -- --exact the_first_run_seeds_the_mark_and_raises_at_most_one
A27: cargo test -p deck-streak-coordination --test landmarks_step -- --exact the_anniversary_is_honest_on_a_day_without_study
A35: cargo test -p deck-streak-coordination --test landmarks_step -- --exact an_unanswered_landmark_keeps_the_cursor_and_is_offered_again
A36: cargo test -p deck-streak-coordination --test landmarks_step -- --exact the_first_run_never_moves_the_cursor_past_a_landmark_it_did_not_raise
A37: cargo test -p deck-streak-coordination --test landmarks_step -- --exact an_imported_mark_raises_todays_landmarks_and_no_history
A38: cargo test -p deck-streak-coordination --test landmarks_step -- --exact the_high_water_mark_equals_the_predecessors_bytes
A39: cargo test -p deck-streak-coordination --test landmarks_step -- --exact the_sync_cycle_offers_the_landmarks_after_the_awards
```

A24 runs the reader over a synthetic collection built in a temporary directory; A1 to A4 run the
port over the parity goldens. A25 to A27 and A35 to A39 run the sync cycle with the real router over
a temporary deployment with synthetic reviews, and read `notification_settings` raw where they judge
the mark or the cursor. The remaining criteria are in section 3c, with the test setting each runs in.

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

## 3c. Delivered by the next pull requests

This SPEC lands in parts. ART1a (this pull request, #127's first part) delivers the landmarks' rule,
their due rule and texts, and the study days of the whole scoped log: the criteria of section 3's
table. #127's second part delivers the landmarks step, the first run and the mark; #128 delivers the
milestone pings and queue zero; #121 delivers the pinned widget. The table below holds the criteria a
later pull request delivers, each row naming that pull request, and the lines under it are their fence
lines, each prefixed with that pull request. A later pull request moves each of its criteria back
verbatim: the row into section 3's table, without the `delivered by` column, and the fence line into
the acceptance fence, without the prefix. A11 to A18 run the router over a recording transport on a
manual clock; A22 and A23 run the bot transport against a recording Bot API stand-in; A28 to A31 run
the recompute over a temporary deployment with synthetic reviews.

| id | criterion | decided by | delivered by |
|---|---|---|---|
| A5 | the milestone text equals the golden `milestone_text` | `the_milestone_text_matches_the_parity_golden` | #128 |
| A6 | a T4 and a T5 throw the occasion's dice, one without a dice throws the default, and a deferred occasion keeps its dice through the flush | `an_occasions_dice_is_thrown_and_kept_through_a_deferral` | #128 |
| A7 | the widget text equals the golden `widget_text`, the broke-today line included | `the_widget_text_matches_the_parity_golden` | #121 |
| A8 | the mood equals the golden `widget_mood` in each of its seven outcomes, at reviews equal to the goal and one below, a goal of 0, and the hours 17, 18, 21 and 22 | `the_widget_mood_matches_the_parity_golden` | #121 |
| A9 | the mood line is absent at `widget_mood` `"0"` and present when it is unset or any other value | `the_mood_switch_turns_the_mood_line_off_only_at_zero` | #121 |
| A10 | the quest lines and the footer equal the golden `widget_payload`: a sealed quest, a completed one, targets of 1 and 15, each footer line alone and all four, an unset goal and a day with no rollup | `the_widget_payload_matches_the_parity_golden` | #121 |
| A11 | the first refresh of a study day sends silently with the Open app row, pins it, unpins the day before's widget, stores the row and records one send | `the_first_refresh_of_a_day_sends_pins_and_unpins_the_day_before` | #121 |
| A12 | a refresh whose text is unchanged pushes nothing, re-pins nothing and records nothing | `an_unchanged_widget_is_never_sent_again_or_re_pinned` | #121 |
| A13 | a changed refresh edits the message with the same row, and a "message is not modified" answer counts as edited | `a_changed_widget_is_edited_in_place_and_not_modified_is_success` | #121 |
| A14 | a failed edit sends anew and pins, replacing the message id; a failed send records `no_notifier`, keeps the row and opens the breaker, and a refresh while it is open pushes nothing | `a_failed_edit_sends_anew_and_a_failed_send_keeps_the_row` | #121 |
| A15 | at `widget_enabled` `"0"` a first or changed refresh is withheld with `nudges_disabled`, and nothing is pushed | `the_widget_switch_stops_every_push` | #121 |
| A16 | a first or changed refresh inside the quiet window is withheld with `quiet_hours`, and the first refresh after the window sends it | `quiet_hours_hold_the_widget_until_the_window_ends` | #121 |
| A17 | the refreshes make the golden `widget_update`'s calls, in order | `the_refreshes_make_the_parity_goldens_calls` | #121 |
| A18 | a T5 unpins and pins the day's widget after its own pin, and a T5 with no widget stored re-pins nothing | `a_t5_re_pins_the_days_widget` | #121 |
| A19 | the kind `widget` is declared with its class, tier, dedupe and setting, a recorded deviation, and the transport names its three calls | `the_widget_kind_and_its_calls_are_declared` | #121 |
| A20 | `widget_enabled` is seeded off, an existing value is kept, and `widget_mood` is left unset | `the_widget_switch_starts_off_beside_the_predecessor` | #121 |
| A21 | `widget_messages` is exported and erased by notifications' port | `the_widget_messages_are_exported_and_erased` | #121 |
| A22 | `push_widget` sends silently with its row and answers the message id, `push_pin` pins that id silently and `push_unpin` unpins it | `the_widget_calls_send_silently_and_pin_and_unpin_by_id` | #121 |
| A23 | `push_edit` answers unchanged for "message is not modified", and failed for any other refusal | `an_edit_reads_not_modified_as_unchanged` | #121 |
| A28 | a study day with a `backlog_zero` grant above 0 raises one `queue_zero` occasion, epic, with the dice and the golden `queue_zero_ping`'s text, and a second recompute raises none | `queue_zero_is_raised_once_for_a_cleared_study_day` | #128 |
| A29 | a declared skip day with no review, and a study day whose `backlog_zero` is 0, raise no `queue_zero` | `a_skip_day_or_an_open_queue_raises_no_queue_zero` | #128 |
| A30 | the badge occasion carries its badge's milestone text with the streak as of the day evaluated | `the_badge_occasion_carries_the_milestone_text` | #128 |
| A31 | the level-up occasion carries the reached level's milestone text | `the_level_up_occasion_carries_the_milestone_text` | #128 |
| A32 | the widget reads the rollup's reviews and due-today count, the streak's length, last study day and longest, the strength, the open lapse, the goal, the skip set, the day's quests, the week's quest, the Ascendant day, the token's end, the wager's days and stake and the local hour, and a fixture that changes any one of them changes the text | `the_widget_reads_every_input_it_names` | #121 |
| A33 | the owner's sync cycle refreshes the widget after its recompute, a failed sync refreshes nothing, and the scheduled sync's cycle attempts no send | `the_owners_sync_cycle_refreshes_the_widget_and_the_scheduled_one_sends_nothing` | #121 |
| A34 | the job `widget_refresh` fires hourly at minute 44 without catch-up, off every predecessor, reserved and private-rail minute and the sync's slot, and its timer and the rail contract hold the calendar | `the_widget_job_keeps_off_every_reserved_minute` | #121 |

#128: A5: cargo test -p deck-streak-notifications --test milestones -- --exact the_milestone_text_matches_the_parity_golden
#128: A6: cargo test -p deck-streak-notifications --test milestones -- --exact an_occasions_dice_is_thrown_and_kept_through_a_deferral
#121: A7: cargo test -p deck-streak-notifications --test widget -- --exact the_widget_text_matches_the_parity_golden
#121: A8: cargo test -p deck-streak-notifications --test widget -- --exact the_widget_mood_matches_the_parity_golden
#121: A9: cargo test -p deck-streak-notifications --test widget -- --exact the_mood_switch_turns_the_mood_line_off_only_at_zero
#121: A10: cargo test -p deck-streak-notifications --test widget -- --exact the_widget_payload_matches_the_parity_golden
#121: A11: cargo test -p deck-streak-notifications --test widget_router -- --exact the_first_refresh_of_a_day_sends_pins_and_unpins_the_day_before
#121: A12: cargo test -p deck-streak-notifications --test widget_router -- --exact an_unchanged_widget_is_never_sent_again_or_re_pinned
#121: A13: cargo test -p deck-streak-notifications --test widget_router -- --exact a_changed_widget_is_edited_in_place_and_not_modified_is_success
#121: A14: cargo test -p deck-streak-notifications --test widget_router -- --exact a_failed_edit_sends_anew_and_a_failed_send_keeps_the_row
#121: A15: cargo test -p deck-streak-notifications --test widget_router -- --exact the_widget_switch_stops_every_push
#121: A16: cargo test -p deck-streak-notifications --test widget_router -- --exact quiet_hours_hold_the_widget_until_the_window_ends
#121: A17: cargo test -p deck-streak-notifications --test widget_router -- --exact the_refreshes_make_the_parity_goldens_calls
#121: A18: cargo test -p deck-streak-notifications --test widget_router -- --exact a_t5_re_pins_the_days_widget
#121: A19: cargo test -p deck-streak-notifications --test widget_router -- --exact the_widget_kind_and_its_calls_are_declared
#121: A20: cargo test -p deck-streak-notifications --test widget_router -- --exact the_widget_switch_starts_off_beside_the_predecessor
#121: A21: cargo test -p deck-streak-notifications --test widget_router -- --exact the_widget_messages_are_exported_and_erased
#121: A22: cargo test -p deck-streak-bot --test widget_transport -- --exact the_widget_calls_send_silently_and_pin_and_unpin_by_id
#121: A23: cargo test -p deck-streak-bot --test widget_transport -- --exact an_edit_reads_not_modified_as_unchanged
#128: A28: cargo test -p deck-streak-coordination --test queue_zero_step -- --exact queue_zero_is_raised_once_for_a_cleared_study_day
#128: A29: cargo test -p deck-streak-coordination --test queue_zero_step -- --exact a_skip_day_or_an_open_queue_raises_no_queue_zero
#128: A30: cargo test -p deck-streak-coordination --test milestone_texts -- --exact the_badge_occasion_carries_the_milestone_text
#128: A31: cargo test -p deck-streak-coordination --test milestone_texts -- --exact the_level_up_occasion_carries_the_milestone_text
#121: A32: cargo test -p deck-streak-coordination --test widget_step -- --exact the_widget_reads_every_input_it_names
#121: A33: cargo test -p deck-streak-coordination --test widget_step -- --exact the_owners_sync_cycle_refreshes_the_widget_and_the_scheduled_one_sends_nothing
#121: A34: cargo test -p deck-streak-coordination --test widget_job -- --exact the_widget_job_keeps_off_every_reserved_minute

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
| `crates/coordination/src/jobs.rs` | `deck-streak-coordination` | changed: the job `widget_refresh`, which `job_table.rs`'s test, as SPEC-100 changes it, holds equal to its timer on the sending template |
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
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the job `widget_refresh`, the sync cycle's widget step and the study-days read |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: the new ports named |
| `migrations/010201_notifications_widget_messages.sql` | `deck-streak-notifications` | added |
| `migrations/010202_notifications_queue_dice.sql` | `deck-streak-notifications` | added |
| `migrations/010203_notifications_widget_side_by_side_default.sql` | `deck-streak-notifications` | added |
| `notifications-policy.json` | repo | changed: the kind `widget`, its deviation and the three calls |
| `deploy/systemd/deck-streak-job-send@widget_refresh.timer` | deploy | added: on the sending template (SPEC-100 R28) |
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
- It gives the scheduled sync's recompute no router, so a landmark or a queue zero that recompute
  decides is not raised, like every awards-phase celebration of that recompute (SPEC-072 R14,
  SPEC-073 R4); whether the scheduled sync gains a router is #291's.
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
- **A whole-log read whose memory grows with the study days.** The read keeps one value per study day, never the
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

## 10. Amendments, 2026-10-03: what ART1a delivers, the scoped read, and the goldens' day token

Section 3's table now holds only the criteria this pull request delivers, A1 to A4 and A24: the rows of A5 to A23 and A25 to A34 moved out of it, and each stays verbatim in section 3c's table with its fence line there, as section 3c says a later pull request moves it back. The manifest in section 4 is otherwise unchanged. The Status line is the one body edit besides sections 3 and 3c.

**10.1, the manifest (section 4).** Old: `| `crates/notifications/src/landmarks.rs` | `deck-streak-notifications` | added: the landmarks, the due rule, the texts and the high-water mark |` New: `| `crates/notifications/src/landmarks.rs` | `deck-streak-notifications` | added: the landmarks, the due rule and the texts (ART1a); the high-water mark (#127's second part) |`

This part adds these files, which section 4 does not name:

- `docs/decisions/ADR-318-the-landmarks-are-ported-as-a-pure-rule-first-and-an-anniversary-is-read-from-the-study-days-own-calendar.md`: the landmarks are ported as a pure rule first, and an anniversary is read from the study days' own calendar.
- `changelog.d/landmarks-art1a-127.md`: the changelog fragment of this part.

**10.2, a deviation section 1 lacks** (insert-only; it adds a bullet to section 1's deviations). New: The study days are read from the scoped log (ADR-095, SPEC-023 R2): a review of a deck out of scope, or of a card since deleted, is not counted, where the predecessor's read of the whole revlog counts both. The first study day and the study-day ordinals can therefore differ from the predecessor's on one collection, and a key imported from its ledger can name another day; the router's once-ever key still raises each key once (#127).

**10.3, section 7.** Old: `| `goldens/landmark_text.json` | `landmarks.py:render_landmark`, `landmarks.py:_ordinal_label` | adapter | landmarks at the ordinals 1, 2, 3, 4, 11, 12, 13, 21, 111 and 121, each event, and both anniversary variants |` New: `| `goldens/landmark_text.json` | `landmarks.py:render_landmark`, `landmarks.py:_ordinal_label` | adapter | landmarks at the ordinals 1, 2, 3, 4, 11, 12, 13, 21, 111 and 121, each event, and both anniversary variants; each text writes its date as the token `{day:N}`, N its epoch day (the oracle's README), so the golden holds no calendar date |`

**10.4, section 7.** Old: `| `goldens/landmarks.constants.json` | `landmarks.py` | constants | `LANDMARK_DAY_STEP`, `LANDMARK_HIGH_WATER_KEY` and the three templates |` New: `| `goldens/landmarks.constants.json` | `landmarks.py`, `constants.py` | constants | `LANDMARK_DAY_STEP`, `LANDMARK_HIGH_WATER_KEY`, `ANNIVERSARY_EVENT_TYPE`, `STUDY_DAY_EVENT_TYPE` and the three templates, which `constants.py` defines and `landmarks.py` imports |`

**10.5, R5 and R9 under ADR-303** (insert-only; no Old is replaced here). New: ADR-303, accepted after this SPEC was written, keeps every celebration out of the fold's writes: the router opens its own write, and owed awards are offered between the writes. R5's and R9's "raises ... through the router" in the awards phase is read under it, and the pull request that delivers each amends it (#127's second part, #128).

**10.6, section 5** (#571 is filed for it; this amendment applies). Old: `- It gives the scheduled sync's recompute no router, so a landmark or a queue zero that recompute decides is not raised, like every awards-phase celebration of that recompute (SPEC-072 R14, SPEC-073 R4); whether the scheduled sync gains a router is #291's.` (the bullet of section 5, which wraps over three lines there) New: `- It gives the scheduled sync's recompute no router, so a landmark or a queue zero that recompute decides is not raised, like every awards-phase celebration of that recompute (SPEC-072 R14, SPEC-073 R4); #291 closed with the held flush as its own job and gave no recompute a router, so whether a recompute gains one is #571.`

Manifest rows this part does not touch (each delivered by a later part or left as it is):

- `crates/notifications/src/milestone.rs`: unchanged in this part.
- `crates/notifications/src/widget.rs`: unchanged in this part.
- `crates/notifications/src/occasion.rs`: unchanged in this part.
- `crates/notifications/src/router.rs`: unchanged in this part.
- `crates/notifications/tests/one_router.rs`: unchanged in this part.
- `crates/notifications/src/transport.rs`: unchanged in this part.
- `crates/notifications/src/ledger.rs`: unchanged in this part.
- `crates/notifications/src/policy.rs`: unchanged in this part.
- `crates/notifications/src/data_rights.rs`: unchanged in this part.
- `crates/notifications/tests/milestones.rs`: unchanged in this part.
- `crates/notifications/tests/widget.rs`: unchanged in this part.
- `crates/notifications/tests/widget_router.rs`: unchanged in this part.
- `crates/coordination/src/recompute/landmarks.rs`: unchanged in this part.
- `crates/coordination/src/recompute/queue_zero.rs`: unchanged in this part.
- `crates/coordination/src/recompute/mod.rs`: unchanged in this part.
- `crates/coordination/src/recompute/badges.rs`: unchanged in this part.
- `crates/coordination/src/progression/level_view.rs`: unchanged in this part.
- `crates/coordination/src/widget.rs`: unchanged in this part.
- `crates/coordination/src/sync_cycle.rs`: unchanged in this part.
- `crates/coordination/src/jobs.rs`: unchanged in this part.
- `crates/coordination/src/lib.rs`: unchanged in this part.
- `crates/coordination/src/data_rights_registry.rs`: unchanged in this part.
- `crates/coordination/tests/landmarks_step.rs`: unchanged in this part.
- `crates/coordination/tests/queue_zero_step.rs`: unchanged in this part.
- `crates/coordination/tests/milestone_texts.rs`: unchanged in this part.
- `crates/coordination/tests/widget_step.rs`: unchanged in this part.
- `crates/coordination/tests/widget_job.rs`: unchanged in this part.
- `crates/coordination/tests/data_rights_symmetry.rs`: unchanged in this part.
- `crates/bot/src/transport.rs`: unchanged in this part.
- `crates/bot/tests/widget_transport.rs`: unchanged in this part.
- `crates/daemon/src/wiring.rs`: unchanged in this part.
- `crates/daemon/tests/roles.rs`: unchanged in this part.
- `migrations/010201_notifications_widget_messages.sql`: unchanged in this part.
- `migrations/010202_notifications_queue_dice.sql`: unchanged in this part.
- `migrations/010203_notifications_widget_side_by_side_default.sql`: unchanged in this part.
- `notifications-policy.json`: unchanged in this part.
- `deploy/systemd/deck-streak-job-send@widget_refresh.timer`: unchanged in this part.
- `deploy/rail-contract.json`: unchanged in this part.
- `docs/CONTEXT-MAP.md`: unchanged in this part.
- `privacy.json`: unchanged in this part.
- `PRIVACY.md`: unchanged in this part.
- `docs/schematics/landmarks-milestones-and-the-pinned-widget.md`: unchanged in this part.
- `docs/decisions/ADR-109-the-widget-is-one-silent-pinned-message-a-study-day-edited-in-place-through-the-router.md`: unchanged in this part.

The row `crates/notifications/src/lib.rs` is changed in this part for the module `landmarks` only; the modules `milestone` and `widget` are #128's and #121's.

**10.7, the anniversary walk's cap (ruling 66)** (insert-only). The anniversary walk stops at 10,000 anniversaries, whatever its exits do. The predecessor's calendar holds the years 1 to 9999, so it holds no walk longer than 9,998 anniversaries, and the goldens span at most 22,424 days, so the cap binds no input the predecessor can hold. The test `the_anniversary_walk_stops_at_its_cap` observes it: one study day at 1970-01-01 and a today 10,005 years on yield exactly 10,000 anniversaries, the last `landmark:anniv:10000` dated `+11970-01-01`.

**10.8, the manifest (section 4)** (insert-only; ruling 52). This part also changes `scripts/mutation-equivalent.d/deck-streak-notifications.json`, which section 4 does not name: the two equivalent `ordinal_label` mutants (ruling-53 records, ruling 64).

## 11. Amendments, 2026-10-03: #127 part b, the landmarks' offers, the cursor and the seed

This part delivers #127's second part, decided by ADR-322: the landmarks are offered between the fold's writes from a cursor only the router's answers move, and the first run seeds the predecessor's mark. Sections 3 and 3c take 11.5's moves in place, because they are the live contract; they are this part's only body edits, and section 10 is unchanged. Each other item below is recorded here and applies as written, as section 10's items do. Every Old is quoted from this SPEC as it stood before this part, joined onto one line.

**11.1, R5 (section 2), under ADR-303 and ADR-322.** This discharges 10.5's "the pull request that delivers each amends it" for R5. Old: `R5. The landmarks step runs in the awards phase, the seventh of the fold's step order (ADR-071), after the badges. It reads the study days once a recompute, and for each study day the fold settles or evaluates, in order, it raises each landmark due that day as one celebration through the router, with its event, its key and its text. An anniversary carries the honest variant when the language streak's last study day, as of the day evaluated, is not that day (`landmarks.py:_gap_honest`, SPEC-076).` New: `R5. The landmarks are offered between the fold's writes, as the owed awards are (ADR-303, ADR-322). A sync cycle that holds a router reads the study days of the whole scoped log once a recompute (R1) and hands the fold the awards' offers and then the landmarks', both run on every offer call: before each settle write, before the current day's write and after the revisit. Each call owes, oldest first by day and then key, every landmark dated after the cursor `landmarks_offered_through` and at or before the settle cursor the call reads, and every landmark due on the day evaluated, less the keys the router answered earlier in the same recompute, and raises each as one celebration through the router, as an award is raised, with its event, its key and its text. An anniversary carries the honest variant when the language streak has no study day on the landmark's own day (`landmarks.py:_gap_honest`, SPEC-076); a day before the window's first day reads as not studied. A decision of the router (sent, held or withheld) answers the key, and a call the router did not answer leaves it owed. The call then moves the cursor, in a write of its own and never backwards, to the settle cursor it read, or, when an owed landmark at or before that day was not answered, to the day before the oldest such landmark. A recompute whose read of the study days fails offers no landmark and moves no cursor.`

**11.2, R6 (section 2), under ADR-322.** Old: `R6. When `notification_settings` holds no `landmark_high_water`, the step first stores it as the predecessor's JSON (`seeded`, and the highest anniversary and study-day ordinals among the landmarks, keys sorted), then raises at most the first due landmark of that recompute. The run equals the golden `landmarks_run` (`landmarks.py:run_landmarks`). The mark is never read again to suppress a landmark.` New: `R6. The first offer call of a recompute seeds in one write (BEGIN IMMEDIATE): when `notification_settings` holds no `landmark_high_water`, it stores the predecessor's JSON, exactly `{"anniversary": A, "seeded": true, "study_day": S}`, A and S the highest anniversary and study-day ordinals among the landmarks as of the day evaluated, 0 when there are none (`landmarks.py:run_landmarks`); and when it holds no `landmarks_offered_through`, it stores the day before the day evaluated. The recompute is the first run exactly when that write read no mark. A first run owes, on every offer call, only the first landmark due on the day evaluated, by day and then key, and never moves the cursor, so a landmark it did not raise is owed by a later run. The run equals the golden `landmarks_run` (`landmarks.py:run_landmarks`). A recompute that finds a mark and no cursor, as after the import (SPEC-141), is not a first run: it stores the cursor at the day before and raises every landmark due that day. Racing first recomputes serialize on the seed's write, so exactly one reads no mark, and the router answers a key both raise `already_recorded` (its once-ever key). The mark is never read again to suppress a landmark. The cursor is a key of the same table; the predecessor holds none, so nothing imports it.`

**11.3, R20 (section 2), ruling 65 Q5.** Old: `The scheduled sync's cycle carries no router, so it refreshes nothing and attempts no send; whether it gains a router is #291's.` New: `The scheduled sync's cycle carries the holding router with no bot transport (SPEC-319 R1, R2), so it refreshes nothing and attempts no send: a celebration its recompute raises, a landmark included, is held for the bot's senders (SPEC-319 R3).`

**11.4, section 5, ruling 65 Q5.** Old (10.6's New): `- It gives the scheduled sync's recompute no router, so a landmark or a queue zero that recompute decides is not raised, like every awards-phase celebration of that recompute (SPEC-072 R14, SPEC-073 R4); #291 closed with the held flush as its own job and gave no recompute a router, so whether a recompute gains one is #571.` New: `- It gives the scheduled sync's recompute no transport: its holding router holds a landmark or a queue zero that recompute raises for the bot's flush and the held_flush job (SPEC-041 R7, R14), as every awards-phase celebration of that recompute (SPEC-072 R14, SPEC-073 R4; #571).` Inserted after it (insert-only), two bullets: `- It raises none of the landmarks dated between the predecessor's last run and this part's first recompute: the first run considers only the day evaluated, as the predecessor's first run does, so those landmarks are history (#127).` and `- It raises nothing for a late review that moves a study-day ordinal onto a day the cursor has passed: that landmark's day is behind the cursor, and the predecessor never raises it either (#580).`

**11.5, sections 3 and 3c** (applied in place). (a) Section 3's table. Old: the row of A24 last. New: after it, the rows of A25, A26 and A27, verbatim from section 3c without its `delivered by` column, then `| A35 | a landmark the router did not answer keeps the cursor before its day and is offered again by the next recompute, and an answered one moves it | `an_unanswered_landmark_keeps_the_cursor_and_is_offered_again` |`, `| A36 | the first run never moves the cursor, so a seed-day landmark it did not raise is raised by a later run after that day settles | `the_first_run_never_moves_the_cursor_past_a_landmark_it_did_not_raise` |`, `| A37 | a recompute that finds the imported mark and no cursor stores the cursor at yesterday and raises every landmark due today and none dated before | `an_imported_mark_raises_todays_landmarks_and_no_history` |`, `| A38 | the mark a first run stores, read from `notification_settings` after the sync cycle, equals the golden `landmarks_run`'s bytes for every case | `the_high_water_mark_equals_the_predecessors_bytes` |` and `| A39 | the sync cycle offers the landmarks after the awards on every offer call, and a failed whole-log read offers none and moves no cursor | `the_sync_cycle_offers_the_landmarks_after_the_awards` |`. (b) Section 3's fence. Old: the line of A24 last. New: after it, the lines of A25, A26 and A27, verbatim from section 3c without the `#127b: ` prefix, then one line each for A35 to A39, each `cargo test -p deck-streak-coordination --test landmarks_step -- --exact` and the test its row names. A38 runs in the coordination package, where the sync cycle stores the mark, rather than in notifications' `landmarks` test file, so its member is behavioural (ruling 86 Q-d). (c) Section 3's paragraph after the fence. Old: `The remaining criteria are in section 3c, with the test setting each runs in.` New: `A25 to A27 and A35 to A39 run the sync cycle with the real router over a temporary deployment with synthetic reviews, and read `notification_settings` raw where they judge the mark or the cursor. The remaining criteria are in section 3c, with the test setting each runs in.` (d) Section 3c's table. Old: the rows of A25, A26 and A27, each ending `#127 part b`. New: none. (e) Section 3c's fence. Old: the three lines prefixed `#127b: `. New: none. (f) Section 3c's paragraph. Old: `A25 to A31 run the recompute over a temporary deployment with synthetic reviews.` New: `A28 to A31 run the recompute over a temporary deployment with synthetic reviews.`

**11.6, the manifest (section 4).** Rows of section 4 this part changes:

- `crates/notifications/src/landmarks.rs`: changed in this part: `LANDMARK_HIGH_WATER_KEY` and `high_water_mark`, the mark's bytes (10.1's "the high-water mark (#127's second part)").
- `crates/notifications/src/lib.rs`: changed in this part: the module `landmark_settings` only.
- `crates/notifications/tests/landmarks.rs`: changed in this part: `the_mark_is_the_predecessors_json_for_every_golden_run`, the mark's bytes against the golden `landmarks_run` in notifications' own package, S10226's killer.
- `crates/coordination/src/recompute/landmarks.rs`: added in this part: `LandmarkOffers`, the landmarks' offers, the cursor's owed set and its advance, and the first run; it is an offer between the fold's writes, not a step of the awards phase (ADR-322).
- `crates/coordination/src/recompute/mod.rs`: changed in this part: the module `landmarks` and `OffersInTurn`; the fold's phases, `run` and `offer_owed` are unchanged.
- `crates/coordination/src/sync_cycle.rs`: changed in this part: the cycle's offers, the awards' and then the landmarks' over the study days it reads.
- `crates/coordination/tests/landmarks_step.rs`: added in this part: A25 to A27 and A35 to A39.
- `tools/parity-oracle/registry/spec_102.py`: changed in this part: the adapter of `landmarks_run`.
- `tools/parity-oracle/goldens/`: changed in this part: `landmarks_run.json` added; the three other goldens of this SPEC move by `registry_sha256` only.
- `scripts/mutation-rows.d/S10200-S10299.json`: changed in this part: rows S10205 to S10207 and S10223 to S10228.
- `docs/schematics/landmarks-milestones-and-the-pinned-widget.md`: changed in this part: 11.9.
- `docs/specs/SPEC-102-landmarks-and-milestones-are-celebrated-once-and-the-widget-is-one-pinned-message-a-study-day.md`: changed in this part: this section and 11.5's moves.
- `docs/red-first/SPEC-102.md`: changed in this part: part b's record, appended.

This part adds these files, which section 4 does not name:

- `crates/coordination/src/recompute/streaks.rs` (`deck-streak-coordination`): changed: the language study days are built from the collection's data alone, so the landmarks' offers read them as the streaks step does.
- `crates/notifications/src/landmark_settings.rs` (`deck-streak-notifications`): added: the seed, the cursor's read and its monotone advance over `notification_settings`.
- `crates/notifications/tests/landmark_settings.rs` (`deck-streak-notifications`): added: the seed, the cursor and the advance in notifications' own package.
- `formal/tla/LandmarkOnce/LandmarkOnce.tla`, `formal/tla/LandmarkOnce/MCLandmarkOnce.cfg` and `formal/tla/LandmarkOnce/witness/` (formal): added: the entry `tla/LandmarkOnce` and its six witnesses (ADR-322).
- `formal/tla/AwardOnce/AwardOnce.tla`, `formal/tla/FoldSettlesOnce/FoldSettlesOnce.tla` and `formal/tla/RelightOrder/RelightOrder.tla` (formal): changed: each cover of `sync_cycle` re-stamped.
- `config/formal.json` (config): changed: the budget row of `tla/LandmarkOnce` only.
- `scripts/tests/test_formal_config.py` (scripts): changed: its declared copy of `config/formal.json` gains the same budget row, since the test holds the file to exactly that copy.
- `docs/decisions/ADR-322-a-landmark-is-offered-between-the-folds-writes-from-a-cursor-only-the-routers-answers-move.md` (docs): added.
- `changelog.d/landmarks-offers-127.md` (docs): added: the changelog fragment of this part.

Manifest rows this part does not touch:

- `crates/ingest/src/study_days.rs`: unchanged in this part.
- `crates/ingest/src/lib.rs`: unchanged in this part.
- `crates/ingest/tests/study_days.rs`: unchanged in this part.
- `crates/notifications/src/milestone.rs`: unchanged in this part.
- `crates/notifications/src/widget.rs`: unchanged in this part.
- `crates/notifications/src/occasion.rs`: unchanged in this part.
- `crates/notifications/src/router.rs`: unchanged in this part.
- `crates/notifications/tests/one_router.rs`: unchanged in this part.
- `crates/notifications/src/transport.rs`: unchanged in this part.
- `crates/notifications/src/ledger.rs`: unchanged in this part.
- `crates/notifications/src/policy.rs`: unchanged in this part.
- `crates/notifications/src/data_rights.rs`: unchanged in this part.
- `crates/notifications/tests/milestones.rs`: unchanged in this part.
- `crates/notifications/tests/widget.rs`: unchanged in this part.
- `crates/notifications/tests/widget_router.rs`: unchanged in this part.
- `crates/coordination/src/recompute/queue_zero.rs`: unchanged in this part.
- `crates/coordination/src/recompute/badges.rs`: unchanged in this part.
- `crates/coordination/src/progression/level_view.rs`: unchanged in this part.
- `crates/coordination/src/widget.rs`: unchanged in this part.
- `crates/coordination/src/jobs.rs`: unchanged in this part.
- `crates/coordination/src/lib.rs`: unchanged in this part.
- `crates/coordination/src/data_rights_registry.rs`: unchanged in this part.
- `crates/coordination/tests/queue_zero_step.rs`: unchanged in this part.
- `crates/coordination/tests/milestone_texts.rs`: unchanged in this part.
- `crates/coordination/tests/widget_step.rs`: unchanged in this part.
- `crates/coordination/tests/widget_job.rs`: unchanged in this part.
- `crates/coordination/tests/data_rights_symmetry.rs`: unchanged in this part.
- `crates/bot/src/transport.rs`: unchanged in this part.
- `crates/bot/tests/widget_transport.rs`: unchanged in this part.
- `crates/daemon/src/wiring.rs`: unchanged in this part: the sync cycle reads the study days through the reader it already holds, and every production cycle already holds a router (SPEC-319 R1), so the daemon wires nothing new.
- `crates/daemon/tests/roles.rs`: unchanged in this part.
- `migrations/010201_notifications_widget_messages.sql`: unchanged in this part.
- `migrations/010202_notifications_queue_dice.sql`: unchanged in this part.
- `migrations/010203_notifications_widget_side_by_side_default.sql`: unchanged in this part.
- `notifications-policy.json`: unchanged in this part.
- `deploy/systemd/deck-streak-job-send@widget_refresh.timer`: unchanged in this part.
- `deploy/rail-contract.json`: unchanged in this part.
- `docs/CONTEXT-MAP.md`: unchanged in this part: the cursor is a key of `notification_settings`, which the register already holds.
- `privacy.json`: unchanged in this part.
- `PRIVACY.md`: unchanged in this part.
- `docs/decisions/ADR-109-the-widget-is-one-silent-pinned-message-a-study-day-edited-in-place-through-the-router.md`: unchanged in this part.

**11.7, section 9** (insert-only). Section 9's table gains these rows, in the band file beside S10205 to S10207:

| row | target | what it guards | killer |
|---|---|---|---|
| `S10223-CURSOR-AFTER-THE-ANSWER` | `crates/coordination/src/recompute/landmarks.rs` | the cursor stops before a landmark the router did not answer | `landmarks_step::an_unanswered_landmark_keeps_the_cursor_and_is_offered_again` |
| `S10224-FIRST-RUN-KEEPS-THE-CURSOR` | `crates/coordination/src/recompute/landmarks.rs` | the first run never moves the cursor | `landmarks_step::the_first_run_never_moves_the_cursor_past_a_landmark_it_did_not_raise` |
| `S10225-SEED-AT-YESTERDAY` | `crates/coordination/src/recompute/landmarks.rs` | the cursor is seeded at the day before the day evaluated | `landmarks_step::an_imported_mark_raises_todays_landmarks_and_no_history` |
| `S10226-MARK-BYTES` | `crates/notifications/src/landmarks.rs` | the mark is the predecessor's bytes | `landmarks::the_mark_is_the_predecessors_json_for_every_golden_run` |
| `S10227-EVERY-OFFER-IN-TURN` | `crates/coordination/src/recompute/mod.rs` | every offer runs on every offer call, the landmarks after the awards | `landmarks_step::the_sync_cycle_offers_the_landmarks_after_the_awards` |
| `S10228-THE-CYCLE-OFFERS-LANDMARKS` | `crates/coordination/src/sync_cycle.rs` | the sync cycle hands the fold the landmarks' offers | `landmarks_step::the_sync_cycle_offers_the_landmarks_after_the_awards` |

**11.8, section 6.** Old: `- **A landmark read a day late and never due.** The step runs at each day's settle, so a study day first read by the next morning's sync is due at its own settle (A25).` New: `- **A landmark read a day late and never due.** The landmarks are offered before each day's settle write, so a study day first read by the next morning's sync is owed at its own settle, and the cursor stops before one the router did not answer (A25, A35).` Inserted after it (insert-only): `- **A decision row per recompute on a landmark's day.** The landmarks due today are offered again by each recompute that day and at the day's settle, and the router answers each repeat `already_recorded` with a decision row: the rows are bounded by the landmarks times the recomputes on their days. Persisting the answered keys dated after the cursor would remove them, at the cost of a second key and a second write site (ADR-322, ruling 86 Q-b).`

**11.9, the schematic.** `docs/schematics/landmarks-milestones-and-the-pinned-widget.md` drew the landmarks raised inside the awards phase, with the mark stored there. This part redraws that flow as the offers between the fold's writes: the seed's write, the cursor's owed set, the router's answer and the cursor's advance, naming the commit it was read at.
