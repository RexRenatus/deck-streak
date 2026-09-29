# Schematic: the nudge coordinator, the joined route and the holdout

Kind: flowchart, state machine and sequence. Read at DeckStreak `dev` 5216bcf and at the
predecessor's `27ee2bc` (`pipeline_layers/nudges.py:NudgesLayer`'s `run_morning_brief`,
`_send_quest_offer`, `_streak_risk_payload`, `_evening_ping_budget_left` and
`run_last_chance_nudge`, `pipeline_layers/focus.py:FocusLayer.run_evening_nudges`,
`pipeline_layers/habits.py:HabitsLayer._habit_nudge_decayed`, and `database.py:GamifyStore`'s
`decide_nudge_arm`, `settle_nudge_ablation` and `nudge_ablation_readout`). Added by the W5 architect
turn, under ADR-100 (the evening check-in is one routed message whose parts keep their own kinds),
ADR-101 (the comeback is the owner's one-reading sequence under the predecessor's cadence) and
ADR-102 (a hold withholds the whole nudge, and what the owner holds travels as its own message).
SPEC-100 builds it.

It extends seven accepted schematics without changing them: `docs/schematics/notification-router.md`
(the decision this adds rules to), `docs/schematics/cron-fire-ledger-and-catch-up.md` (the jobs'
fires), `docs/schematics/bot-update-loop.md` (the loop whose turn fires a snooze),
`docs/schematics/streaks-and-governor-state-machine.md` (the lapse and the streak the nudges read),
`docs/schematics/skip-day-record-and-effects.md` (the skip day a nudge declines on),
`docs/schematics/quests-and-chests-lifecycle.md` (the offer and the vaulted chests the morning
carries) and `docs/schematics/celebration-ladder-on-the-router.md` (the celebrations the same router
decides).

## The decision, with the nudge rules

The router's order after this SPEC. The rules drawn with a double border are new.

```mermaid
flowchart TD
  O[an occasion, or one part of a joined route] --> SW{the kind's switch is 0}
  SW -- yes --> W1[withhold, nudges_disabled]
  SW -- no --> CL{the key is already claimed}
  CL -- yes --> W2[withhold, already_recorded]
  CL -- no --> LP{a lapse is open and the class is suppressed}
  LP -- yes --> HB{{the kind is habit and the back-off allows it}}
  HB -- no --> W3[withhold, lapse]
  HB -- yes --> SK
  LP -- no --> SK{{the caller declines for a skip day}}
  SK -- yes --> W4[withhold, skip_day]
  SK -- no --> QH{quiet hours and the class is not exempt}
  QH -- yes --> W5[defer a celebration, else withhold quiet_hours]
  QH -- no --> DC{{the caller declines below the streak floor, already studied or with no payload}}
  DC -- yes --> W6[withhold with that reason]
  DC -- no --> CB{a comeback over its cap or inside its gap}
  CB -- yes --> W7[withhold, budget_spent]
  CB -- no --> RQ{{requested by the owner}}
  RQ -- yes --> SF
  RQ -- no --> BG{{an evening kind with 2 standing deliveries today}}
  BG -- yes --> W8[withhold, budget_spent]
  BG -- no --> HO{{a holdout kind whose arm is hold}}
  HO -- yes --> W9[withhold, ablation_hold]
  HO -- no --> SF{the surface}
  SF -- no bot or breaker open --> W10[withhold, no_notifier]
  SF -- bot --> SD[send with its button rows]
```

- Every withhold releases the claim and records `<kind>:withheld` with its reason, in the decision's
  write.
- A requested occasion (the snooze) skips the budget and the holdout, so it spends nothing and draws
  no arm.
- The habit back-off allows the check-in in a lapse while fewer than 5 days have passed since the
  language track's last study day, and after that only when the kind's most recent delivery is at
  least 3 study days old, read from the ledger alone.

## The morning, the evening and the last chance

```mermaid
sequenceDiagram
  participant J as coordination job
  participant X as context reads
  participant R as router
  participant T as bot transport
  J->>X: offers, vaulted chests, rollup, streak, habit courses, buff, skip set, wager, multiplier
  Note over J: morning-nudge at 08:06
  J->>R: quest_offer with one pick row per offer
  J->>R: morning brief with Open today, holdout decided
  J->>R: chests_vaulted with the first chest's rows
  Note over J: evening-nudge at 20:06
  J->>R: route_together of streak_risk, habit and focus
  R->>T: one push of the passing parts with their rows
  Note over J: last-chance at 22:06
  J->>R: last_chance with the stakes rows
  R->>T: one push, or a withhold recorded
```

- The offer and the chests are digests: neither a lapse nor a hold withholds them, so a held or
  lapsed morning still carries what the owner must act on.
- A joined route records each passing part once under its own kind; a failed push releases every
  claim and records each `no_notifier`.
- The comeback stays the morning readings job's (SPEC-049), under the same router.

## A holdout arm's life

```mermaid
stateDiagram-v2
  [*] --> Drawn: the first decision of a holdout kind on a study day
  Drawn --> Send: the draw is at or above pct times 100
  Drawn --> Hold: the draw is below pct times 100
  Send --> [*]: the push fails, and the arm is deleted with the claim
  Send --> Settled: the weekly settle finds the day's rollup
  Hold --> Settled: the weekly settle finds the day's rollup
  Settled --> [*]
```

- An arm is drawn once for its kind and study day and reused unchanged, even when the percent
  changes.
- A day with no rollup row is never settled, so no outcome is fabricated.
- The settle takes at most 400 arms of the trailing 90 days, and the readout shows only kinds with
  both arms.

## A snooze

```mermaid
sequenceDiagram
  participant O as owner
  participant B as bot update loop
  participant S as nudge_snoozes
  participant C as coordination stakes
  participant R as router
  O->>B: Snooze 1h
  B->>S: record the study day's snooze, due one hour on, once
  loop each turn of the loop
    B->>S: unfired snoozes due at or before now
    S-->>B: the due snooze
    B->>C: the stakes preview again, key snooze, requested
    C->>R: route, exempt from the budget and the holdout
    B->>S: mark it fired
  end
```

- The snooze is a row, so a restart keeps it, and the router's claim on the key keeps a fire from
  sending twice.
- A snooze whose study day has passed is marked fired and raises nothing.
