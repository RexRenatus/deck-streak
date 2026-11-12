# SPEC-100: the nudges speak once through the one router, and a holdout measures them

- **Wave:** W5. **Issues:** #122 (the morning brief), #117 (the evening nudge coordinator and its
  stakes preview), #118 (the last-chance nudge), #97 (the habit check-in), #123 (the comeback
  protocol) and #132 (the nudge ablation), in epic #6. **Context(s):** `deck-streak-notifications`
  (the withhold reasons, the caller's declines, the evening budget, the habit back-off, the
  holdout's arm, settle and readout, the joined route, the button rows, the estimate, the nudge
  texts and constants, `nudge_holdout_arms` and `nudge_snoozes`); `deck-streak-coordination` (the
  three nudge jobs, the inputs they gather, the snooze's fire, the holdout's settle use case, and
  the comeback's skip-day decline); `deck-streak-analytics` (the recent rollups' read);
  `deck-streak-habits` (two reads for the habit check-in); `deck-streak-bot` (the button rows on the
  transport, the `nu:` callbacks, the snooze's turn); `deck-streak-api` (the holdout route);
  `deck-streak-daemon` (the wiring, and the job role's transport); the Mini App (`web/app`, the startapp token `habits`).
- **Decided by:** ADR-100 (this SPEC's: the evening check-in is one routed message whose parts keep
  their own kinds), ADR-101 (the comeback is the owner's one-reading sequence under the
  predecessor's cadence), ADR-102 (a hold withholds the whole nudge, and what the owner holds
  travels as its own message), ADR-041 (the router core), ADR-080 (the draw), ADR-049 (the comeback
  reading), ADR-052 (deep links are URL buttons), ADR-011 and ADR-027 (the job minutes), ADR-053 (the private rail's reserved slots), ADR-012 (the parity oracle) and ADR-124 (a job that sends runs under its own template, which loads the bot's two credentials).
- **Prerequisites:** SPEC-041 (the router, its ledger and its queue), SPEC-049 (the comeback reading
  and its lapse slice), SPEC-052 (deep links), SPEC-080 (the kernel's `draw_bp`, the challenge
  offers and their pick), SPEC-081 (the vaulted chests and their buttons), SPEC-071 (the rollups),
  SPEC-072 (the multiplier's preview and the Ascendant buff), SPEC-076 (the streak state and
  `real_misses`), SPEC-078 (the habit summary, its freshness and the `/habits` screen), SPEC-079
  (the focus nudge's eligibility), SPEC-083 (the skip set and `/skip`), SPEC-027 (the job table),
  SPEC-021 (the six files of a table), SPEC-029 (the goldens) and SPEC-062 (the unit guards the sending template must pass).
  **Mutation band:** `S10000-S10099`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-100.md` (ADR-016).

## 1. The problem, measured

- **What exists.** SPEC-041's router (`crates/notifications/src/router.rs`) decides every occasion
  inside one write, in this order: the switch (`nudges_disabled`), the claim (`already_recorded`), a
  lapse (`lapse`), quiet hours (`quiet_hours`), the comeback's cap and gap (`budget_spent`), then
  the surface (`no_notifier`). `Reason::ALL` names those six; the policy's `withhold.reasons` names
  eleven. The policy declares the evening budget (`nudge_budgets.evening`), the holdout (`holdout`)
  and the habit back-off (`lapse.habit_backoff`), and the router applies none of them.
  `BotTransport::push_message` carries text and no button, so SPEC-081 R7's chest announcement,
  which names an Open button and an Open in app button, has no path to carry them. The job table
  holds the sync, the upkeep and the liveness watch. No morning brief, evening nudge, last-chance
  nudge, habit check-in or holdout exists.
- **What is ported** (at `27ee2bc`):
  - the morning brief, `pipeline_layers/nudges.py:NudgesLayer.run_morning_brief`,
    `_send_quest_offer`, `writing_adoption_codes` and `telegram.py:render_morning_brief`;
  - the evening coordinator, `pipeline_layers/focus.py:FocusLayer.run_evening_nudges` and
    `_focus_nudge_body`, `telegram.py:render_focus_nudge`;
  - the stakes preview, `NudgesLayer._streak_risk_payload`, `_stake_lines`, `_median_sec_per_card`,
    `_estimate_minutes`, `_evening_ping_budget_left` and `_stakes_keyboard`, and
    `run_streak_risk_nudge` for the snooze's resend;
  - the last chance, `NudgesLayer.run_last_chance_nudge` and `constants.py:LAST_CHANCE_TEMPLATES`;
  - the habit check-in, `pipeline_layers/habits.py:HabitsLayer._habit_nudge_body`,
    `_habit_nudge_decayed` and `_reading_days_since_by_lang`, and `telegram.py:render_habit_nudge`;
  - the comeback's cadence, `telegram.py:comeback_gate`;
  - the holdout, `database.py:nudge_draw_bp`, `GamifyStore.decide_nudge_arm`, `nudge_holdout_pct`,
    `nudge_ablation_seed`, `settle_nudge_ablation` and `nudge_ablation_readout`,
    `pipeline_layers/digests.py:DigestsLayer._nudge_ablation_block`, and
    `NudgesLayer.nudge_silence_age_days`;
  - the buttons, `bot.py:CommandBot`'s `nu:sn` and `nu:skip` callbacks.
- **Deviations from the predecessor, each with its reason.**
  - A hold withholds the whole brief, and the quest offer and the vaulted chests travel as their own
    messages (ADR-102). The predecessor draws a hold and still sends the brief when it carries an
    offer or a chest, so its held arm holds occasions that were delivered, and the readout compares
    arms that are not what they say.
  - The evening check-in is one routed message whose parts keep their own kinds, and nothing sends
    around the router (ADR-100). The predecessor's coordinator sends directly, and records a second
    withhold row under a coordinator kind beside the part's own.
  - The comeback keeps SPEC-049's one-reading sequence and the predecessor's cadence (ADR-101). Its
    texts, which name the silent days and a landmark weekday, are not ported.
  - A skip day withholds the comeback with `skip_day`. SPEC-049's comeback does not read the skip
    set; the predecessor's comeback morning does.
  - The evening budget is decided after the suppressions and the caller's declines, so a withheld
    row names the first reason a person would give; it is still decided after the exempt branch, as
    #118 asks. The predecessor checks the budget first.
  - The per-occasion reasons are the policy's `no_payload` and `already_recorded`, not the
    predecessor's `no_payload_from_any_kind` and `already_recorded_today`.
  - A lapse withholds the brief even when the comeback is off. The predecessor sends the brief in a
    lapse when the comeback is disabled, against #123's rule that the invoice is never sent in a
    lapse.
  - The focus nudge is withheld in a lapse, as every nudge is but the habit check-in, whose back-off
    the policy declares. The predecessor never reads the lapse for it.
  - The snooze is a stored row the bot's loop fires, not a wait inside the bot's process, so a
    restart keeps it. A snooze whose study day has passed raises nothing.
  - The jobs run at minute 6: 08:06, 20:06 and 22:06 local, off the predecessor's minutes and the
    reserved ones (SPEC-027 R2, SPEC-053 R2).
  - The new kinds' settings are seeded `"0"` while the predecessor runs side by side (ADR-011); the
    cutover sets them (#62).
  - `last_chance_enabled` turns the nudge off only at `"0"`, as every kind's setting does
    (SPEC-041). The predecessor turns it off at any value but `"1"`.
  - The brief omits the drills line until the law drills are live (#136), as #122 notes.
  - The brief's Open today button, the stakes' Open app button and the habit check-in's Habits
    button are added, as #122, #117 and #97 ask.
  - The silence age is served on the owner's holdout route. The predecessor exports it as a service
    gauge, and DeckStreak exports none (SPEC-025).
- **Corrections to the issues.**
  - #117's reason names are the policy's (`no_payload`, `already_recorded`), and a third automatic
    evening send cannot occur, because only two kinds spend the budget, each once a study day; the
    budget's test withholds one with two standing deliveries.
  - #122's 08:00 is 08:06, and "never held when it carries a quest offer or vaulted chests" becomes:
    a hold withholds the brief alone, and the offer and the chests arrive as their own messages
    (ADR-102).
  - #122's "web-app button" is a URL button with a `startapp` deep link (ADR-052); so are #117's
    Open app and #97's Habits buttons.
  - #123's "keep the message text" and its landmark windows yield to SPEC-049's variants (ADR-101):
    the policy's `comeback.landmarks` stays declared and read by no rule. #123's button opens the
    comeback reading (SPEC-049 R5), not the dealt hand (#149).
  - #118's 22:00 is 22:06.
  - #132's weekly settle is kept, and it runs in the weekly report (#130), which SPEC-101 builds.

## 2. Requirements

The router (#117, #118, #132)

R1. `Reason` gains `SkipDay`, `BelowStreakMin`, `AlreadyStudied`, `AblationHold` and `NoPayload`,
    and `Reason::ALL` holds the policy's eleven `withhold.reasons` in the policy's order.
R2. An occasion may carry the caller's decline: `skip_day`, `below_streak_min`, `already_studied` or
    `no_payload`. The router decides in this order, and the first rule that matches decides: the
    switch; the claim; a lapse (R4); a `skip_day` decline; quiet hours; a `below_streak_min`,
    `already_studied` or `no_payload` decline; the comeback's cap and gap; the kind's daily budget
    (R3); the holdout (R5); the surface. A withhold releases the claim and records `<kind>:withheld`
    with its reason (SPEC-041 R6). A caller that declines for more than one reason passes the first
    of `skip_day`, `below_streak_min` and `already_studied`, the predecessor's order in
    `NudgesLayer._streak_risk_payload`.
R3. The evening budget. An occasion may be marked requested by the owner (the snooze, R19). A
    requested occasion is decided exempt first and spends none of the budget. Otherwise an occasion
    of a kind whose policy budget is `evening` (`streak_risk`, `last_chance`) is withheld with
    `budget_spent` when the study day already holds `nudge_budgets.evening.per_day` (2) standing
    deliveries of those kinds that were not requested. `notification_deliveries` gains `requested`
    (0 or 1). A released claim leaves no delivery, so a failed push spends nothing. The verdict at
    each count equals the golden `evening_budget` (`NudgesLayer._evening_ping_budget_left`).
R4. The habit back-off. A lapse withholds a nudge with `lapse` (SPEC-041), except the kind `habit`,
    which the policy's `lapse.habit_backoff` decides: while fewer than `after_days` (5) days have
    passed since the language track's last study day, or it has none, the check-in is decided as
    outside a lapse; after that, only when the kind's most recent standing delivery is at least
    `interval_days` (3) study days old, or it has none; otherwise it is withheld with `lapse`. The
    occasion's lapse context carries the language track's last study day (SPEC-076 R1). The verdicts
    equal the golden `habit_backoff` (`HabitsLayer._habit_nudge_decayed`), from the ledger alone, so
    a restart changes none.
R5. The holdout. An occasion of a kind the policy's `holdout.kinds` names (`morning`, `streak_risk`,
    `last_chance`) and not requested has an arm. An arm already recorded for the kind and the study
    day is reused unchanged. Otherwise the arm is `hold` when `draw_bp(seed, kind, reference) < pct
    × 100`, and `send` otherwise, where `draw_bp` is SPEC-080 R1's, the reference is the study day's
    ISO date, and pct is the setting `holdout_pct` clamped to 0..=50, or the policy's `holdout.pct`
    (10) when it is absent or not an integer. The seed is the setting `holdout_seed`, minted once,
    when absent, with `INSERT OR IGNORE` of 32 lowercase hex digits from SQLite's `randomblob(16)`,
    and never replaced. The arm is written in the decision's transaction. A `hold` withholds with
    `ablation_hold`, spends no budget and leaves no delivery; a failed push that releases the claim
    deletes that decision's `send` arm with it. The arms, the draws and the reuse equal the golden
    `holdout_arm` (`GamifyStore.decide_nudge_arm`, `nudge_holdout_pct`, `nudge_draw_bp`).
R6. Button rows. An occasion may carry rows of buttons, each a URL, a Mini App token, the configured
    study link or a callback of 1 to 64 bytes. The bot transport's push carries them, the bot
    renders a token as SPEC-052 R2's deep link and the study link from its configuration, and omits
    the study link's button when none is configured. A deferred occasion keeps its rows in
    `notification_queue.buttons` (a JSON array, `'[]'` by default) and the flush pushes them. This
    is the path SPEC-081 R7's chest buttons take.
R7. `route_together(parts)` routes the evening check-in (ADR-100). Each part is an occasion decided
    as if routed alone, in the given order, inside one write. When no part passes, nothing is
    pushed. When one passes, its text is pushed as it is; when two or three pass, the text is the
    header `⏰ <b>Evening check-in</b>`, a blank line, and the parts joined by blank lines. The push
    carries every passing part's rows, in part order. Each passing part is recorded under its own
    kind and key, so no part records a second row. A failed push releases every passing part's claim
    and records each `no_notifier`. The message equals the golden `evening_check_in`
    (`FocusLayer.run_evening_nudges`).
R8. The policy gains three kinds, each with its deviation (ADR-100, ADR-102): `focus` (class
    `nudge`, tiers `["T2"]`, budget null, `per-study-day`, setting `focus_enabled`), and
    `quest_offer` and `chests_vaulted` (class `digest`, tiers `["T2"]`, budget null,
    `per-study-day`, settings `quest_offer_enabled` and `chests_vaulted_enabled`).

The estimate (#117)

R9. `crates/notifications/src/estimate.rs` holds `sec_per_card(rows)`, over the 31 most recent
    rollup rows: the per-day seconds per card of each row with reviews and seconds above 0, and 6.0
    when their reviews total under 20 or none qualify, else the sorted list's middle element (index
    `len / 2`) clamped to 4.0..=20.0; and `estimate_minutes(due, sec_per_card)`: 0 when due is 0 or
    less, else the ceiling of due times the seconds over 60. They equal the goldens `sec_per_card`
    (`NudgesLayer._median_sec_per_card`) and `estimate_minutes` (`NudgesLayer._estimate_minutes`).
    Coordination reads the rows through analytics' `RollupStore::recent_before(day, count)`, the
    most recent stored rollups before a study day, newest first, at the day after the current study
    day and a count of 31; SPEC-105's reminder takes its estimate from this pair.

The morning job (#122)

R10. The job `morning_nudge` runs at 08:06 local, without catch-up. It raises, in this order: the
    quest offer, when the study day's offers are not yet picked (SPEC-080 R8); the brief; the
    vaulted chests, when any are vaulted (SPEC-081 R7).
R11. The quest offer: kind `quest_offer`, key `quest_offer`, text `🏆 <b>Choose today's challenge
    quest:</b>`, and one row per offer, labelled with the first 60 code points of the offer's emoji,
    a space and its description, carrying SPEC-080 R23's pick callback. It equals the golden
    `quest_offer_message` (`NudgesLayer._send_quest_offer`).
R12. The brief: kind `morning`, key `morning`. Its text is `render_morning_brief` over the study
    day's due count and backlog (SPEC-071) and the language streak (SPEC-076), then `✍️ Write
    today:` with each writing course not yet confirmed today that has a writing confirmation in the
    trailing 30 days, then the Ascendant line when the study day holds the Ascendant buff
    (SPEC-072). It carries one row, Open today (the token `today`). A lapse withholds it (R2), and
    the holdout decides it (R5). It equals the golden `morning_brief`
    (`NudgesLayer.run_morning_brief`'s brief, with no offer and no chest).
R13. The vaulted chests: kind `chests_vaulted`, key `chests_vaulted`, the line naming the count of
    chests vaulted last night, and SPEC-081's chest rows for the first vaulted chest. It equals the
    golden `vaulted_chests_message` (the vaulted branch of `NudgesLayer.run_morning_brief`). Neither
    a lapse nor a hold withholds it or the offer (ADR-102).

The evening job (#117, #97)

R14. The job `evening_nudge` runs at 20:06 local, without catch-up, and routes together (R7), in
    this order, the stakes preview, the habit check-in and the focus nudge.
R15. The stakes preview: kind `streak_risk`, key `evening`. It declines with `skip_day` on an active
    skip day (SPEC-083), with `below_streak_min` when the language streak's current run is 0, and
    with `already_studied` when its last study day is the current study day. Its lines, from
    `NudgesLayer._stake_lines`: the streak line only while the streak is savable (no real miss, or
    one real miss and a freeze left; SPEC-076's `real_misses`), in the predecessor's three forms;
    the multiplier line when the multiplier is above 1.0 (SPEC-072 R18's preview); the wager line
    when the job passes an active wager's stake, which it passes as none until SPEC-106 R26 passes
    it; the due count; and the estimate (R9). Its rows: Study now
    (the configured study link) and Open app (the token `today`), then Cheat day (`nu:skip`) and
    Snooze 1h (`nu:sn`). It equals the golden `stakes_nudge` (`NudgesLayer._streak_risk_payload`).
    SPEC-109 R4 leaves out the Cheat day row while the skip day is switched off.
R16. The habit check-in: kind `habit`, key `evening`. It declines with `no_payload` when no writing
    course adopted in the trailing 30 days is unconfirmed today and no reading course is behind its
    pace (not met, pace above 0). Its text is `render_habit_nudge` over the adopted writing courses
    and every reading course with its days since its last reading, a reading course 21 days or more
    gone cold (SPEC-078 R11). Its row opens the Habits screen (the token `habits`). It equals the
    golden `habit_nudge` (`HabitsLayer._habit_nudge_body`). The back-off is R4's.
R17. The focus nudge: kind `focus`, key `evening`. It declines with `no_payload` when SPEC-079 R15's
    eligibility is false. Its text is `render_focus_nudge`, with no row. It equals the golden
    `focus_nudge` (`FocusLayer._focus_nudge_body`).

The last chance (#118)

R18. The job `last_chance_nudge` runs at 22:06 local, without catch-up. Kind `last_chance`, key
    `last_chance`. It declines with `skip_day`, with `below_streak_min` when the language streak's
    current run is under 4, and with `already_studied`. Its text is `LAST_CHANCE_TEMPLATES[ordinal %
    8]`, the ordinal being the study day's proleptic ordinal (the epoch day plus 719163), over the
    streak, the hours to the next rollover instant floored at 0 and written to one decimal, the due
    count and the estimate. Its rows are R15's. It equals the golden `last_chance_nudge`
    (`NudgesLayer.run_last_chance_nudge`), in which two consecutive days pick different templates.

The snooze and the cheat day (#117)

R19. `nu:sn`, from the owner only, records the study day's snooze in `nudge_snoozes`, due one hour
    after the tap; a second tap on the same study day changes nothing. Each turn of the bot's update
    loop fires every unfired snooze whose due instant is at or before the turn's instant: when its
    study day is the current study day, it raises the stakes preview (R15) with key `snooze`,
    requested, its payload computed again with the same declines; it then marks the snooze fired. A
    snooze whose study day has passed is marked fired and raises nothing. The router's claim on
    `snooze` keeps a fire from sending twice.
R20. `nu:skip`, from the owner only, answers with SPEC-083 R14's `/skip` preview, with its Confirm
    and Cancel; while the skip day is switched off it answers as `/skip` does (SPEC-109 R13).

The comeback (#123)

R21. The comeback stays SPEC-049's: one reading per lapse, its three variants chosen by the sends of
    the episode, one URL button to the reading, and no date, count or streak. The router's cap and
    gap (`comeback.max_per_episode` 3, `comeback.min_gap_days` 3) equal the golden `comeback_gate`
    (`telegram.py:comeback_gate`), and the count restarts with a new lapse id. SPEC-049's comeback
    declines with `skip_day` on an active skip day. No text of the predecessor's comeback is ported
    (ADR-101).

The holdout's readout (#132)

R22. `crates/notifications/src/holdout.rs` settles arms from rollups: the arms whose study day is
    before the current study day and no older than 90 days before it, not yet settled, and whose
    study day has a rollup row, ordered by study day, kind and key, at most 400, each settled with
    its reviews and whether they were above 0; an arm with no rollup row stays unsettled. It equals
    the golden `holdout_settle` (`GamifyStore.settle_nudge_ablation` as
    `DigestsLayer._settle_nudge_ablation` calls it). Coordination's `nudges::holdout::settle`
    gathers the arms and the rollups (SPEC-071's `RollupStore::days`) and writes the outcomes in one
    transaction; the weekly report runs it before the readout (#130).
R23. The readout groups the settled arms of the trailing 90 days by kind and arm. The block renders
    only kinds with both arms, sorted, each as `<kind>: sent <n> → <rate>% studied · held <n> →
    <rate>% studied`, with ` (low n)` when the smaller arm is under 10, under the header `🧪 <b>Nudge
    ablation</b>`, and is empty when no kind has both arms. It equals the golden `holdout_readout`
    (`GamifyStore.nudge_ablation_readout`, `DigestsLayer._nudge_ablation_block`).
R24. The silence age of `streak_risk` and `last_chance` is the current study day minus the study day
    of the kind's most recent standing delivery, floored at 0, and -1 when it has none. It equals
    the golden `nudge_silence_age` (`NudgesLayer.nudge_silence_age_days`).
R25. `GET /api/nudges/holdout` serves, to the owner's session only, the readout's rows (kind, arm,
    n, studied, reviews) and the two silence ages. It settles nothing.

Tables, settings and surfaces

R26. Notifications owns `nudge_holdout_arms` (kind, dedupe key, study day, arm `send` or `hold`,
    draw 0..=9999, pct 0..=50, seed, reviews, studied 0 or 1, settled at, `created_at`; primary key
    the kind and the study day), created by `migrations/010001_notifications_holdout_arms.sql`, and
    `nudge_snoozes` (study day as primary key, due at, fired at, `created_at`), created by
    `migrations/010002_notifications_snoozes.sql`, each `STRICT`.
    `migrations/010003_notifications_requested_and_buttons.sql` adds
    `notification_deliveries.requested` and `notification_queue.buttons`. Each table owes SPEC-021's
    six files; notifications' data-rights port exports and erases both.
R27. `migrations/010004_notifications_nudges_side_by_side_defaults.sql` seeds `morning_enabled`,
    `evening_enabled`, `last_chance_enabled`, `habit_enabled`, `focus_enabled`,
    `quest_offer_enabled` and `chests_vaulted_enabled` as `"0"` with `INSERT OR IGNORE` (ADR-011),
    so nothing speaks twice while the predecessor runs.
R28. The three jobs join the job table, each with its timer and its calendar key in the rail
    contract; none shares a minute with the predecessor's schedule, a reserved minute (0, 25, 39),
    the private rail's reserved slots or the sync's slot (SPEC-027 R2, SPEC-053 R2). Further,
    the job role joins the bot's transport to the router before a job that sends runs: it loads
    `owner-user-id` and `telegram-bot-token` through its credential loader, builds the transport
    and passes wiring's `TransportMarker` as the notifier, as SPEC-026 R10 and SPEC-041 R13
    leave to the first job that sends. The job table marks each job as sending or not. Each
    sending job runs as an instance of a second job template, `deck-streak-job-send@.service`
    (ADR-124), with its `@<id>.timer`: it is identical to `deck-streak-job@.service` but for the
    two `LoadCredential=` lines of `deck-streak-alert@.service`'s form. A job that sends nothing
    stays on `deck-streak-job@` and requests neither credential. A sending job started without
    either credential refuses start, and a job that sends nothing builds no transport. The
    rail's map answers the two ids to the sending template's instances alone, as it answers the
    sync login to `sync` (SPEC-061 R4). The sending jobs are `morning_nudge`, `evening_nudge`,
    `last_chance_nudge`, `daily_digest`, `weekly_report`, `widget_refresh` and
    `discipline_tick`; each job's own SPEC adds its timer on the sending template.
R29. The Mini App's startapp map gains `habits`, opening `/habits` (SPEC-078).
R30. The constants equal the golden `nudges.constants`, and the policy's numbers each equal a
    constant there.
R31. The reads this SPEC adds to other contexts, each that context's own and a read only: analytics'
    `RollupStore::recent_before(day, count)` (R9); and habits' most recent confirmation day of each
    writing course and most recent day with minutes of each reading course (R12, R16), from which
    notifications' pure texts apply the 30-day adoption window and the reading gaps. The due count
    and backlog, the streak state and `real_misses`, the multiplier's preview, the Ascendant buff,
    the skip set, the offers, the vaulted chests, the habit summary and the focus eligibility are
    read through the reads SPEC-071, SPEC-076, SPEC-072, SPEC-083, SPEC-080, SPEC-081, SPEC-078 and
    SPEC-079 already give. The active wager's stake is SPEC-106 R26's to pass.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | `Reason::ALL` equals the policy's eleven withhold reasons in order, and each reason's name is the policy's | `the_reasons_are_the_policys_in_its_order` |
| A2 | each decline is decided at its place: a skip day after a lapse and before quiet hours, the others after quiet hours and before the comeback's cap, the budget and the holdout, each recorded with its reason | `each_decline_is_decided_at_its_place` |
| A3 | the evening budget equals the golden `evening_budget`, at 1 and 2 standing deliveries, a requested one and a released claim not counted | `the_evening_budget_matches_the_parity_golden` |
| A4 | a requested occasion is decided exempt first: with 2 standing deliveries it is sent, spends nothing and draws no arm | `a_requested_occasion_spends_no_budget_and_draws_no_arm` |
| A5 | the habit back-off equals the golden `habit_backoff`, at 4 and 5 days since the last study day, no last study day, and a last check-in never, 2 and 3 study days before | `the_habit_backoff_matches_the_parity_golden` |
| A6 | the habit back-off reads only the ledger: a fresh connection over the same database decides the same | `the_habit_backoff_survives_a_restart` |
| A7 | the arms equal the golden `holdout_arm`: a draw of exactly pct × 100 sends and one less holds, pct settings of 0, 50, 51 and none, a kind outside the holdout, and a recorded arm reused after the setting changes | `the_holdout_arm_matches_the_parity_golden` |
| A8 | a hold is withheld with `ablation_hold`, leaves no delivery and spends no budget | `a_hold_is_withheld_and_spends_nothing` |
| A9 | a failed push deletes its `send` arm with the claim, and a hold's arm stays | `a_failed_push_deletes_its_send_arm` |
| A10 | the production percent is the policy's 10, and a test pins 0 through the fixture's setting only | `the_production_holdout_percent_stays_ten` |
| A11 | the seed is minted once as 32 lowercase hex digits and a second decision reads the same seed | `the_seed_is_minted_once` |
| A12 | the rows travel with the push, a deferred occasion's rows wait in the queue and the flush pushes them, and a callback over 64 bytes is refused | `button_rows_travel_with_the_push_and_the_queue` |
| A13 | the joined route equals the golden `evening_check_in`: one, two and three parts, and a part withheld | `the_evening_check_in_matches_the_parity_golden` |
| A14 | a joined route records each part once under its own kind and key, and a failed push releases every claim and records each `no_notifier` | `a_joined_route_records_each_part_once` |
| A15 | the kinds `focus`, `quest_offer` and `chests_vaulted` are declared with their class, tier, dedupe and setting, each a recorded deviation | `the_nudge_kinds_are_recorded_deviations` |
| A16 | the seconds per card equal the golden `sec_per_card`, at reviews totalling 19 and 20, a row with 0 seconds, and medians of 3.9, 4.0, 20.0 and 20.1 | `the_seconds_per_card_match_the_parity_golden` |
| A17 | the estimate equals the golden `estimate_minutes`, at due counts of -1, 0 and 1 and a product that divides 60 exactly | `the_estimate_matches_the_parity_golden` |
| A18 | the brief equals the golden `morning_brief`: a course confirmed today, one adopted 30 and 31 days back, the Ascendant buff held and not | `the_morning_brief_matches_the_parity_golden` |
| A19 | the quest offer equals the golden `quest_offer_message`, with a description of 59, 60 and 61 characters and a multibyte emoji | `the_quest_offer_matches_the_parity_golden` |
| A20 | the vaulted chests equal the golden `vaulted_chests_message`, with 1 and 3 chests | `the_vaulted_chests_match_the_parity_golden` |
| A21 | the stakes preview equals the golden `stakes_nudge`: 0, 1 and 2 real misses with and without a freeze, a multiplier of 1.0 and above, a wager and none, and each decline | `the_stakes_preview_matches_the_parity_golden` |
| A22 | the habit check-in equals the golden `habit_nudge`: nothing pending, an unadopted course, a pace of 0, and reading gaps of 20 and 21 days | `the_habit_nudge_matches_the_parity_golden` |
| A23 | the focus nudge equals the golden `focus_nudge` | `the_focus_nudge_matches_the_parity_golden` |
| A24 | the last chance equals the golden `last_chance_nudge`: streaks of 3 and 4, two consecutive days, the rollover 0.04 hours away and passed, and each decline | `the_last_chance_matches_the_parity_golden` |
| A25 | the router's comeback cap and gap equal the golden `comeback_gate`, at 2 and 3 sends and gaps of 2 and 3 days | `the_comeback_cap_matches_the_parity_golden` |
| A26 | a new lapse id restarts the comeback count at its first variant | `a_new_lapse_restarts_the_comeback_count` |
| A27 | the settle equals the golden `holdout_settle`: an arm on the current study day, 90 and 91 days back, one with no rollup row, a settled one, and 401 candidates | `the_holdout_settle_matches_the_parity_golden` |
| A28 | the readout equals the golden `holdout_readout`: one arm only, smaller arms of 9 and 10, and two kinds out of order | `the_holdout_readout_matches_the_parity_golden` |
| A29 | the silence age equals the golden `nudge_silence_age`: never, today, 3 days back, and a delivery a day ahead | `the_silence_age_matches_the_parity_golden` |
| A30 | the constants equal the golden `nudges.constants` and the policy's numbers equal them | `the_nudge_constants_match_the_predecessors` |
| A31 | `nudge_holdout_arms` and `nudge_snoozes` are exported and erased with the other notifications tables | `the_holdout_arms_and_snoozes_are_exported_and_erased` |
| A32 | the morning job raises the offer, the brief and the chests in order with every input it names, and in a lapse or on a hold the offer and the chests still arrive while the brief is withheld | `the_morning_job_gathers_every_input_and_keeps_the_offer` |
| A33 | the morning job raises one brief per study day: a second run pushes nothing | `the_morning_job_raises_one_brief_a_day` |
| A34 | the evening job gathers every input it names and routes its three parts together in order | `the_evening_job_gathers_every_input` |
| A35 | the last-chance job gathers every input it names, and a 3-day streak pushes nothing | `the_last_chance_job_gathers_every_input` |
| A36 | a due snooze fires once at the first turn at or after its due instant on a fake clock, never before it, and a passed study day raises nothing | `a_snooze_fires_once_at_its_due_instant` |
| A37 | the settle use case reads the arms and the rollups and writes the outcomes in one transaction | `the_settle_writes_its_outcomes_once` |
| A38 | on an active skip day the comeback is declined and nothing is pushed | `a_skip_day_declines_the_comeback` |
| A39 | no comeback envelope names a due count, a streak, a penalty, a fine, a coin, a date, a clock time or a count of missed days | `no_comeback_envelope_names_a_count_or_a_stake` |
| A40 | the three jobs' minutes keep off every predecessor, reserved and private-rail minute and the sync's slot, and their timers and the rail contract hold the calendars | `the_nudge_jobs_keep_off_every_reserved_minute` |
| A41 | the recent rollups before a day are read newest first, at most the count, none on or after the day | `the_recent_rollups_before_a_day_are_read_newest_first` |
| A42 | each writing course's most recent confirmation day and each reading course's most recent day with minutes are read back | `the_habit_nudge_reads_are_read_back` |
| A43 | `nu:sn` records one snooze a study day, `nu:skip` answers the skip preview, and both refuse anyone but the owner | `the_nudge_callbacks_are_the_owners_only` |
| A44 | the bot renders each row kind, a token as the deep link and the study link from its configuration, omitting it when unset | `the_bot_renders_every_button_row` |
| A45 | the holdout route answers the owner's session only, and serves the readout and the silence ages | `the_holdout_route_answers_only_the_owner` |
| A46 | the startapp token `habits` opens `/habits` | `opens the habits screen from its token` |
| A47 | a nudge job run by the job role with the two credentials routes through a joined transport and records `sent`, not `no_notifier`, and a job that sends nothing builds no transport; the test's environment carries the two credentials as files in a credentials directory, never as values in the environment, and a decoy environment variable `TELEGRAM_BOT_TOKEN` carrying a different value never reaches the transport, which carries the loaded file's value | `a_sending_job_routes_through_the_joined_transport` |
| A48 | a sending job started without either credential refuses start, through the credential loader and never the environment: a decoy environment variable `TELEGRAM_BOT_TOKEN` carrying a value never stands in for the missing file | `a_sending_job_without_its_credentials_refuses_start` |
| A49 | the two job templates differ only by the two `LoadCredential=` lines, and the sending template's lines are the alert template's form; the test compares the files' parsed assignments (key and value per line, comments and blank lines ignored), never their raw text | `test_the_two_job_templates_differ_only_by_the_credential_lines` |

```acceptance
A1: cargo test -p deck-streak-notifications --test nudge_router -- --exact the_reasons_are_the_policys_in_its_order
A2: cargo test -p deck-streak-notifications --test nudge_router -- --exact each_decline_is_decided_at_its_place
A3: cargo test -p deck-streak-notifications --test nudge_router -- --exact the_evening_budget_matches_the_parity_golden
A4: cargo test -p deck-streak-notifications --test nudge_router -- --exact a_requested_occasion_spends_no_budget_and_draws_no_arm
A5: cargo test -p deck-streak-notifications --test nudge_router -- --exact the_habit_backoff_matches_the_parity_golden
A6: cargo test -p deck-streak-notifications --test nudge_router -- --exact the_habit_backoff_survives_a_restart
A7: cargo test -p deck-streak-notifications --test holdout -- --exact the_holdout_arm_matches_the_parity_golden
A8: cargo test -p deck-streak-notifications --test holdout -- --exact a_hold_is_withheld_and_spends_nothing
A9: cargo test -p deck-streak-notifications --test holdout -- --exact a_failed_push_deletes_its_send_arm
A10: cargo test -p deck-streak-notifications --test holdout -- --exact the_production_holdout_percent_stays_ten
A11: cargo test -p deck-streak-notifications --test holdout -- --exact the_seed_is_minted_once
A12: cargo test -p deck-streak-notifications --test nudge_router -- --exact button_rows_travel_with_the_push_and_the_queue
A13: cargo test -p deck-streak-notifications --test nudge_router -- --exact the_evening_check_in_matches_the_parity_golden
A14: cargo test -p deck-streak-notifications --test nudge_router -- --exact a_joined_route_records_each_part_once
A15: cargo test -p deck-streak-notifications --test nudge_kinds -- --exact the_nudge_kinds_are_recorded_deviations
A16: cargo test -p deck-streak-notifications --test nudge_goldens -- --exact the_seconds_per_card_match_the_parity_golden
A17: cargo test -p deck-streak-notifications --test nudge_goldens -- --exact the_estimate_matches_the_parity_golden
A18: cargo test -p deck-streak-notifications --test nudge_goldens -- --exact the_morning_brief_matches_the_parity_golden
A19: cargo test -p deck-streak-notifications --test nudge_goldens -- --exact the_quest_offer_matches_the_parity_golden
A20: cargo test -p deck-streak-notifications --test nudge_goldens -- --exact the_vaulted_chests_match_the_parity_golden
A21: cargo test -p deck-streak-notifications --test nudge_goldens -- --exact the_stakes_preview_matches_the_parity_golden
A22: cargo test -p deck-streak-notifications --test nudge_goldens -- --exact the_habit_nudge_matches_the_parity_golden
A23: cargo test -p deck-streak-notifications --test nudge_goldens -- --exact the_focus_nudge_matches_the_parity_golden
A24: cargo test -p deck-streak-notifications --test nudge_goldens -- --exact the_last_chance_matches_the_parity_golden
A25: cargo test -p deck-streak-notifications --test nudge_goldens -- --exact the_comeback_cap_matches_the_parity_golden
A26: cargo test -p deck-streak-notifications --test nudge_router -- --exact a_new_lapse_restarts_the_comeback_count
A27: cargo test -p deck-streak-notifications --test holdout -- --exact the_holdout_settle_matches_the_parity_golden
A28: cargo test -p deck-streak-notifications --test holdout -- --exact the_holdout_readout_matches_the_parity_golden
A29: cargo test -p deck-streak-notifications --test holdout -- --exact the_silence_age_matches_the_parity_golden
A30: cargo test -p deck-streak-notifications --test nudge_goldens -- --exact the_nudge_constants_match_the_predecessors
A31: cargo test -p deck-streak-notifications --test nudge_rights -- --exact the_holdout_arms_and_snoozes_are_exported_and_erased
A32: cargo test -p deck-streak-coordination --test nudges_morning -- --exact the_morning_job_gathers_every_input_and_keeps_the_offer
A33: cargo test -p deck-streak-coordination --test nudges_morning -- --exact the_morning_job_raises_one_brief_a_day
A34: cargo test -p deck-streak-coordination --test nudges_evening -- --exact the_evening_job_gathers_every_input
A35: cargo test -p deck-streak-coordination --test nudges_last_chance -- --exact the_last_chance_job_gathers_every_input
A36: cargo test -p deck-streak-coordination --test nudges_snooze -- --exact a_snooze_fires_once_at_its_due_instant
A37: cargo test -p deck-streak-coordination --test nudges_holdout -- --exact the_settle_writes_its_outcomes_once
A38: cargo test -p deck-streak-coordination --test nudges_comeback -- --exact a_skip_day_declines_the_comeback
A39: cargo test -p deck-streak-coordination --test nudges_comeback -- --exact no_comeback_envelope_names_a_count_or_a_stake
A40: cargo test -p deck-streak-coordination --test nudge_jobs -- --exact the_nudge_jobs_keep_off_every_reserved_minute
A41: cargo test -p deck-streak-analytics --test rollup_recent -- --exact the_recent_rollups_before_a_day_are_read_newest_first
A42: cargo test -p deck-streak-habits --test nudge_reads -- --exact the_habit_nudge_reads_are_read_back
A43: cargo test -p deck-streak-bot --test nudge_callbacks -- --exact the_nudge_callbacks_are_the_owners_only
A44: cargo test -p deck-streak-bot --test button_rows -- --exact the_bot_renders_every_button_row
A45: cargo test -p deck-streak-api --test nudges_routes -- --exact the_holdout_route_answers_only_the_owner
A46: pnpm exec vitest run web/app/src/lib/startapp-habits.test.ts -t "opens the habits screen from its token"
A47: cargo test -p deck-streak-daemon --test roles -- --exact a_sending_job_routes_through_the_joined_transport
A48: cargo test -p deck-streak-daemon --test roles -- --exact a_sending_job_without_its_credentials_refuses_start
A49: python3 -m unittest discover -s scripts/tests -p test_deploy_templates.py -k test_the_two_job_templates_differ_only_by_the_credential_lines
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
| B1 | `nudge_holdout_arms` and `nudge_snoozes` are declared under notifications' categories with data, purpose, basis, retention and erase mode, over `privacy.json`, the two migrations `migrations/010001_notifications_holdout_arms.sql` and `migrations/010002_notifications_snoozes.sql`, and `crates/notifications/src/data_rights.rs` | the privacy-gdpr pack |
| B2 | the three new kinds each carry their deviation, the evening budget's exempt branch is checked first, the holdout is declared with its kinds and its record of held occasions, and every withhold reason is recorded, over `notifications-policy.json` | the notifications-policy pack |
| B3 | every nudge goes through the one router, over every file under `crates/coordination/src/nudges/` | the notifications-policy pack |
| B4 | every `nu:` button's data fits the platform's limit, every callback is answered, and every deep link is a URL button, over `crates/bot/src/nudge_commands.rs` and `crates/bot/src/transport.rs` | the telegram-platform pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/notifications/src/router.rs` | `deck-streak-notifications` | changed: the reasons, the declines at their places, the evening budget, the habit back-off, the holdout, the rows and `route_together` |
| `crates/notifications/src/occasion.rs` | `deck-streak-notifications` | changed: the decline, the requested mark, the rows, and the lapse context's last study day |
| `crates/notifications/src/transport.rs` | `deck-streak-notifications` | changed: `push_message` carries the rows |
| `crates/notifications/src/ledger.rs` | `deck-streak-notifications` | changed: the budget's count, the habit's last delivery, the silence age's read, `requested`, and the queue's rows |
| `crates/notifications/src/policy.rs` | `deck-streak-notifications` | changed: the typed policy reads the budget's kinds, the back-off and the holdout for the router |
| `crates/notifications/src/holdout.rs` | `deck-streak-notifications` | added: the arm, the seed, the percent, the settle, the readout and the silence age, over `nudge_holdout_arms` |
| `crates/notifications/src/estimate.rs` | `deck-streak-notifications` | added: `sec_per_card` and `estimate_minutes`, pure |
| `crates/notifications/src/nudges.rs` | `deck-streak-notifications` | added: the nudge texts, rows and declines, pure, the adoption window and the constants |
| `crates/notifications/src/snooze.rs` | `deck-streak-notifications` | added: the repository over `nudge_snoozes` |
| `crates/notifications/src/data_rights.rs` | `deck-streak-notifications` | changed: the two tables, exported and erased |
| `crates/notifications/src/lib.rs` | `deck-streak-notifications` | changed: the modules above |
| `crates/notifications/Cargo.toml` | `deck-streak-notifications` | changed: `serde_json` with `float_roundtrip` as a dev-dependency for the golden reader |
| `crates/notifications/tests/nudge_router.rs` | `deck-streak-notifications` | added: A1 to A6, A12 to A14, A26 |
| `crates/notifications/tests/holdout.rs` | `deck-streak-notifications` | added: A7 to A11, A27 to A29 |
| `crates/notifications/tests/nudge_kinds.rs` | `deck-streak-notifications` | added: A15 |
| `crates/notifications/tests/nudge_goldens.rs` | `deck-streak-notifications` | added: A16 to A25, A30 |
| `crates/notifications/tests/nudge_rights.rs` | `deck-streak-notifications` | added: A31 |
| `crates/notifications/tests/support/mod.rs` | `deck-streak-notifications` | changed: the recording transport keeps the rows |
| `crates/notifications/tests/ui/pass_kept_by_a_clone.rs` | `deck-streak-notifications` | changed: the transport's new signature |
| `crates/coordination/src/nudges/mod.rs` | `deck-streak-coordination` | added: the nudge use cases |
| `crates/coordination/src/nudges/morning.rs` | `deck-streak-coordination` | added: the morning job |
| `crates/coordination/src/nudges/evening.rs` | `deck-streak-coordination` | added: the evening job |
| `crates/coordination/src/nudges/last_chance.rs` | `deck-streak-coordination` | added: the last-chance job |
| `crates/coordination/src/nudges/stakes.rs` | `deck-streak-coordination` | added: the stakes' inputs and the seconds per card, which SPEC-105 reuses |
| `crates/coordination/src/nudges/snooze.rs` | `deck-streak-coordination` | added: the snooze's record and fire |
| `crates/coordination/src/nudges/holdout.rs` | `deck-streak-coordination` | added: the settle use case and the route's read model |
| `crates/coordination/src/readings/comeback.rs` | `deck-streak-coordination` | changed: the skip day's decline |
| `crates/coordination/src/jobs.rs` | `deck-streak-coordination` | changed: the jobs `morning_nudge`, `evening_nudge` and `last_chance_nudge`, and the field that marks each job as sending (R28), which `job_table.rs`'s test, as this SPEC changes it, holds equal to their timers |
| `crates/coordination/tests/job_table.rs` | `deck-streak-coordination` | changed: the timers of both job templates are read, and a job's timer is on `deck-streak-job-send@` exactly when the table marks it sending (R28) |
| `crates/coordination/src/runner.rs` | `deck-streak-coordination` | changed: the three jobs run their work by id |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module `nudges` |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the two tables |
| `crates/coordination/tests/nudges_morning.rs` | `deck-streak-coordination` | added: A32, A33 |
| `crates/coordination/tests/nudges_evening.rs` | `deck-streak-coordination` | added: A34 |
| `crates/coordination/tests/nudges_last_chance.rs` | `deck-streak-coordination` | added: A35 |
| `crates/coordination/tests/nudges_snooze.rs` | `deck-streak-coordination` | added: A36 |
| `crates/coordination/tests/nudges_holdout.rs` | `deck-streak-coordination` | added: A37 |
| `crates/coordination/tests/nudges_comeback.rs` | `deck-streak-coordination` | added: A38, A39 |
| `crates/coordination/tests/nudge_jobs.rs` | `deck-streak-coordination` | added: A40 |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row for each table |
| `crates/coordination/tests/flush_step.rs` | `deck-streak-coordination` | changed: the transport's new signature |
| `crates/analytics/src/rollup.rs` | `deck-streak-analytics` | changed: `RollupStore::recent_before` |
| `crates/analytics/tests/rollup_recent.rs` | `deck-streak-analytics` | added: A41 |
| `crates/habits/src/store.rs` | `deck-streak-habits` | changed: each writing course's most recent confirmation day and each reading course's most recent day with minutes |
| `crates/habits/tests/nudge_reads.rs` | `deck-streak-habits` | added: A42 |
| `crates/bot/src/transport.rs` | `deck-streak-bot` | changed: the rows rendered as an inline keyboard, the study link from the configuration |
| `crates/bot/src/nudge_commands.rs` | `deck-streak-bot` | added: the `nu:sn` and `nu:skip` callbacks |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the `nu:` prefix |
| `crates/bot/src/poll.rs` | `deck-streak-bot` | changed: each turn fires the due snoozes |
| `crates/bot/tests/nudge_callbacks.rs` | `deck-streak-bot` | added: A43 |
| `crates/bot/tests/button_rows.rs` | `deck-streak-bot` | added: A44 |
| `crates/api/src/nudges.rs` | `deck-streak-api` | added: `GET /api/nudges/holdout` |
| `crates/api/src/lib.rs` | `deck-streak-api` | changed: the route |
| `crates/api/tests/nudges_routes.rs` | `deck-streak-api` | added: A45 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the three jobs, the snooze's port and the route's read model |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: the new ports named, A47 and A48 |
| `crates/daemon/src/role_job.rs` | `deck-streak-daemon` | changed: a job that sends loads the two credentials, builds the transport and joins it to the router (R28) |
| `web/app/src/lib/startapp.ts` | miniapp | changed: the token `habits` |
| `web/app/src/lib/startapp-habits.test.ts` | miniapp | added: A46 |
| `migrations/010001_notifications_holdout_arms.sql` | `deck-streak-notifications` | added |
| `migrations/010002_notifications_snoozes.sql` | `deck-streak-notifications` | added |
| `migrations/010003_notifications_requested_and_buttons.sql` | `deck-streak-notifications` | added |
| `migrations/010004_notifications_nudges_side_by_side_defaults.sql` | `deck-streak-notifications` | added |
| `notifications-policy.json` | repo | changed: the three kinds and their deviations |
| `deploy/systemd/deck-streak-job-send@morning_nudge.timer` | deploy | added: on the sending template (SPEC-100 R28) |
| `deploy/systemd/deck-streak-job-send@evening_nudge.timer` | deploy | added: on the sending template (SPEC-100 R28) |
| `deploy/systemd/deck-streak-job-send@last_chance_nudge.timer` | deploy | added: on the sending template (SPEC-100 R28) |
| `deploy/systemd/deck-streak-job-send@.service` | deploy | added: the sending template, `deck-streak-job@.service` with the two `LoadCredential=` lines of the alert template's form (R28, ADR-124) |
| `deploy/README.md` | deploy | changed: the sending template's credential ids and why |
| `deploy/host-budget.json` | deploy | changed: the sending template's entry, equal to the job template's (ADR-032) |
| `deploy/rail-contract.json` | deploy | changed: the sending template's neutral values, equal to the job template's (SPEC-061 R7), and the three calendar keys |
| `scripts/tests/test_deploy_templates.py` | repo | changed: A49; the sending template joins ROLE_CREDENTIALS (`owner-user-id`, `telegram-bot-token`), ROLES (`job %i`), PER_SERVICE, the credential-loading units, the stack-share test's oneshot list and the job-timer prefix check; `adr_budget()` reads ADR-124's budget row; WAIVED gains the two waivers of each of the seven sending timers (`morning_nudge`, `evening_nudge`, `last_chance_nudge`, `daily_digest`, `weekly_report`, `widget_refresh`, `discipline_tick`) |
| `scripts/tests/test_rail_contract.py` | repo | changed: `test_only_the_sync_job_reads_the_sync_login` admits the bot's two credentials loaded for a sending job outside sync's cycle, and still holds the sync login to `run_scheduled` (R28) |
| `deploy/scripts/effective-check.py` | deploy | changed: an instance's name may carry `_`, as the job ids do (R28); `INSTANCE` becomes `^[a-z0-9][a-z0-9_-]*$` |
| `tools/parity-oracle/registry/spec_100.py` | tools | added: the adapters |
| `tools/parity-oracle/goldens/` | tools | added: the goldens of section 7 |
| `scripts/mutation-rows.d/S10000-S10099.json` | scripts | added: the rows of section 9 |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `nudge_holdout_arms` and `nudge_snoozes` |
| `privacy.json` | repo | changed: the two tables' categories |
| `PRIVACY.md` | repo | changed: one line for each category |
| `docs/schematics/nudge-coordinator-and-holdout.md` | docs | added by the W5 architect turn; this delivery corrects it only where the code proves it wrong |
| `.sqlx/` | workspace | changed: the offline query cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `docs/specs/SPEC-100-the-nudges-speak-once-through-the-one-router-and-a-holdout-measures-them.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-100.md` | docs | added |
| `changelog.d/feat-nudges-100.md` | repo | added: the changelog fragment |

## 5. What this does NOT do

- It places the holdout's readout in no report and draws no research card: the weekly report runs
  the settle and shows the block, and its Mini App card reads the route (#130).
- It sends no daily digest and no lapse line in it, and deals no hand (#129, #149).
- It shows no drills line in the brief until the law drills are live (#136).
- It writes no comeback text of the predecessor's, no reading-less comeback and no landmark (#123).
- It adds no settings screen for the new switches: the side-by-side seeds stand until the cutover
  (#57, #62).
- It flushes no held occasion itself: the router's flush does, and now carries the rows (#291).
- It imports none of the predecessor's arms, seed or snoozes (#61).
- It pins no daily widget (#121).

## 6. Risks

- **A held arm that was delivered.** A hold withholds the whole brief and the offer and the chests
  travel apart, so no held occasion reaches the owner (A8, A32).
- **A readout that counts a failed send.** A failed push deletes its `send` arm with the claim (A9).
- **A nudge sent twice.** Every part, the snooze included, claims its key in the router's one write,
  and a joined route records each part once (A14, A33, A36).
- **A snooze lost on a restart, or one that waits for ever.** The snooze is a stored row, and its
  fire is proved on a fake clock that bounds the wait (A36).
- **A holdout that silences the nudges.** The percent is clamped to 50, and the production value is
  the policy's 10 (A7, A10).
- **The estimate drifts from the predecessor's by a bit.** Its golden holds the median's clamp at
  both ends and the fallback's floor (A16, A17).
- **Shame in a lapse.** A lapse withholds every nudge but the backed-off habit check-in, and no
  comeback envelope names a count, a date or a stake (A5, A39).

## 7. Parity goldens

All are registered in `tools/parity-oracle/registry/spec_100.py` and generated on the owner's
checkout of the predecessor at `27ee2bc` (SPEC-029). Every day is an epoch day number and every
draw's reference an opaque token; no golden holds a calendar date or a personal value.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `goldens/evening_budget.json` | `pipeline_layers/nudges.py:NudgesLayer._evening_ping_budget_left` | adapter | a temporary store; 0, 1, 2 and 3 delivered automatic sends, a snooze resend and a failed send |
| `goldens/habit_backoff.json` | `pipeline_layers/habits.py:HabitsLayer._habit_nudge_decayed` | adapter | a stub governor and streak; no lapse, 4 and 5 days since the last study day, none, and a last check-in never, 2 and 3 days before |
| `goldens/holdout_arm.json` | `database.py:GamifyStore.decide_nudge_arm`, `nudge_holdout_pct`, `nudge_draw_bp` | adapter | a temporary store; pct settings of 0, 10, 50, 51 and none; references whose draw is exactly pct × 100 and one less; a kind outside the holdout; a recorded arm under a changed setting |
| `goldens/evening_check_in.json` | `pipeline_layers/focus.py:FocusLayer.run_evening_nudges` | adapter | a recording notifier and stub parts; one, two and three parts, and each part withheld |
| `goldens/sec_per_card.json` | `pipeline_layers/nudges.py:NudgesLayer._median_sec_per_card` | adapter | stub rollups; reviews totalling 19 and 20, a row with 0 seconds, medians of 3.9, 4.0, 20.0 and 20.1, and 30 and 32 rows |
| `goldens/estimate_minutes.json` | `NudgesLayer._estimate_minutes` | pure | due counts of -1, 0, 1 and 60 at seconds per card of 4.0, 6.0 and 20.0 |
| `goldens/morning_brief.json` | `NudgesLayer.run_morning_brief` | adapter | a recording notifier, stub rollup, streak, habit summary and buff, the send arm, no offer and no chest; a course confirmed today, adopted 30 and 31 days back, the buff held and not |
| `goldens/quest_offer_message.json` | `NudgesLayer._send_quest_offer` | adapter | a recording notifier; offers with descriptions of 59, 60 and 61 characters and a multibyte emoji |
| `goldens/vaulted_chests_message.json` | `NudgesLayer.run_morning_brief` | adapter | the vaulted branch alone, with 1 and 3 chests |
| `goldens/stakes_nudge.json` | `NudgesLayer._streak_risk_payload`, `_stake_lines`, `_stakes_keyboard` | adapter | stub streak, rollup, multiplier, wager and skip set; 0, 1 and 2 real misses with and without a freeze, a multiplier of 1.0 and 1.2, a wager and none, and each decline |
| `goldens/habit_nudge.json` | `pipeline_layers/habits.py:HabitsLayer._habit_nudge_body` | adapter | a stub summary and writing rows; nothing pending, an unadopted course, a pace of 0, reading gaps of 20 and 21 days; no lapse |
| `goldens/focus_nudge.json` | `pipeline_layers/focus.py:FocusLayer._focus_nudge_body` | adapter | stub summaries at the streak's floor and one under, and under and at the goal |
| `goldens/last_chance_nudge.json` | `NudgesLayer.run_last_chance_nudge` | adapter | a recording notifier and a stub clock; streaks of 3 and 4, two consecutive days, the rollover 0.04 hours away and passed, and each decline |
| `goldens/comeback_gate.json` | `telegram.py:comeback_gate` | pure | sends of 0 to 3 and gaps of none, 2 and 3 days |
| `goldens/holdout_settle.json` | `database.py:GamifyStore.settle_nudge_ablation` | adapter | a temporary store; arms on the current study day, 90 and 91 days back, one with no rollup row, a settled one, and 401 candidates |
| `goldens/holdout_readout.json` | `database.py:GamifyStore.nudge_ablation_readout`, `pipeline_layers/digests.py:DigestsLayer._nudge_ablation_block` | adapter | a temporary store; one arm only, smaller arms of 9 and 10, two kinds out of order |
| `goldens/nudge_silence_age.json` | `NudgesLayer.nudge_silence_age_days` | adapter | a temporary store; never, today, 3 days back and a delivery a day ahead |
| `goldens/nudges.constants.json` | `constants.py`, `database.py`, `pipeline_layers/digests.py` | constants | `LAST_CHANCE_TEMPLATES`, the last chance's floor, the evening cap, the holdout's percents and kinds, the silence kinds, the adoption window, and the readout's days, low n and settle limit |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `nudge_holdout_arms` | `notifications` | `migrations/010001_notifications_holdout_arms.sql` (SPEC-100) | `nudge_ablation`, its day as an epoch day, its reference dropped for the kind's key | exported and erased |
| `nudge_snoozes` | `notifications` | `migrations/010002_notifications_snoozes.sql` (SPEC-100) | none: the predecessor stores no snooze | exported and erased |

## 9. Mutation rows

The band is `S10000-S10099`, in `scripts/mutation-rows.d/S10000-S10099.json`. Each row's killer
selects exactly one test, and each is proved with its file restored byte for byte (SPEC-039). No
row's mutant makes a loop or a wait unbounded: `S10016`'s can only delay a fire, and its killer's
fake clock bounds that.

| row | target | what it guards | killer |
|---|---|---|---|
| `S10000-A-HOLD-IS-STRICTLY-BELOW` | `crates/notifications/src/holdout.rs` | a draw of exactly pct × 100 sends | `holdout::the_holdout_arm_matches_the_parity_golden` |
| `S10001-THE-PERCENT-CAP-IS-50` | `crates/notifications/src/holdout.rs` | the percent's clamp | `holdout::the_holdout_arm_matches_the_parity_golden` |
| `S10002-A-RECORDED-ARM-IS-REUSED` | `crates/notifications/src/holdout.rs` | no redraw under a changed setting | `holdout::the_holdout_arm_matches_the_parity_golden` |
| `S10003-A-REQUESTED-SEND-IS-EXEMPT` | `crates/notifications/src/router.rs` | the exempt branch first | `nudge_router::a_requested_occasion_spends_no_budget_and_draws_no_arm` |
| `S10004-THE-EVENING-BUDGET-IS-TWO` | `crates/notifications/src/router.rs` | the budget's strict count | `nudge_router::the_evening_budget_matches_the_parity_golden` |
| `S10005-THE-BACKOFF-STARTS-AT-FIVE` | `crates/notifications/src/router.rs` | the back-off's inclusive start | `nudge_router::the_habit_backoff_matches_the_parity_golden` |
| `S10006-THE-BACKOFF-WAITS-THREE` | `crates/notifications/src/router.rs` | the back-off's interval | `nudge_router::the_habit_backoff_matches_the_parity_golden` |
| `S10007-TWENTY-REVIEWS-TRUST-THE-MEDIAN` | `crates/notifications/src/estimate.rs` | the fallback under 20 | `nudge_goldens::the_seconds_per_card_match_the_parity_golden` |
| `S10008-THE-MEDIAN-IS-CLAMPED` | `crates/notifications/src/estimate.rs` | the clamp to 4..=20 | `nudge_goldens::the_seconds_per_card_match_the_parity_golden` |
| `S10009-THE-ESTIMATE-ROUNDS-UP` | `crates/notifications/src/estimate.rs` | the ceiling | `nudge_goldens::the_estimate_matches_the_parity_golden` |
| `S10010-THE-LAST-CHANCE-NEEDS-FOUR` | `crates/notifications/src/nudges.rs` | the streak's floor | `nudge_goldens::the_last_chance_matches_the_parity_golden` |
| `S10011-THE-TEMPLATE-TURNS-ON-THE-ORDINAL` | `crates/notifications/src/nudges.rs` | the proleptic ordinal | `nudge_goldens::the_last_chance_matches_the_parity_golden` |
| `S10012-THE-STREAK-LINE-NEEDS-A-SAVABLE-STREAK` | `crates/notifications/src/nudges.rs` | one miss with a freeze | `nudge_goldens::the_stakes_preview_matches_the_parity_golden` |
| `S10013-THE-READOUT-NEEDS-BOTH-ARMS` | `crates/notifications/src/holdout.rs` | no one-armed line | `holdout::the_holdout_readout_matches_the_parity_golden` |
| `S10014-LOW-N-IS-UNDER-TEN` | `crates/notifications/src/holdout.rs` | the marker's strict floor | `holdout::the_holdout_readout_matches_the_parity_golden` |
| `S10015-A-DAY-WITHOUT-A-ROLLUP-STAYS-OPEN` | `crates/notifications/src/holdout.rs` | no fabricated outcome | `holdout::the_holdout_settle_matches_the_parity_golden` |
| `S10016-A-SNOOZE-FIRES-AT-ITS-DUE-INSTANT` | `crates/coordination/src/nudges/snooze.rs` | the due comparison | `nudges_snooze::a_snooze_fires_once_at_its_due_instant` |
| `S10017-THE-SETTLE-STOPS-AT-400` | `crates/notifications/src/holdout.rs` | the settle's bound | `holdout::the_holdout_settle_matches_the_parity_golden` |
| `S10018-NEVER-SPOKEN-IS-MINUS-ONE` | `crates/notifications/src/holdout.rs` | the silence sentinel | `holdout::the_silence_age_matches_the_parity_golden` |
| `S10019-THE-OFFER-LABEL-CUTS-AT-60` | `crates/notifications/src/nudges.rs` | the cut in characters | `nudge_goldens::the_quest_offer_matches_the_parity_golden` |
| `S10020-THE-WRITING-LINE-NEEDS-30-DAYS` | `crates/notifications/src/nudges.rs` | the adoption window | `nudge_goldens::the_morning_brief_matches_the_parity_golden` |
| `S10021-A-FAILED-PUSH-DELETES-ITS-ARM` | `crates/notifications/src/router.rs` | the readout counts no failed send | `holdout::a_failed_push_deletes_its_send_arm` |
| `S10022-THE-JOB-ROLE-JOINS-THE-TRANSPORT` | `crates/daemon/src/role_job.rs` | a job that sends builds the transport and joins it to the router | `roles::a_sending_job_routes_through_the_joined_transport` |
| `S10023-THE-SENDING-TEMPLATE-LOADS-BOTH-CREDENTIALS` | `deploy/systemd/deck-streak-job-send@.service` | the two credential lines that make a job's send reach a transport (a script-mutation row) | `test_deploy_templates.TheSendingTemplate.test_the_two_job_templates_differ_only_by_the_credential_lines` |
