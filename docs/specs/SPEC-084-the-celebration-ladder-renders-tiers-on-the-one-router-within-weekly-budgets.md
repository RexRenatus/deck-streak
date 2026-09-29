# SPEC-084: the celebration ladder renders each tier on the one router within weekly budgets

- **Wave:** W3. **Issue:** #120 (epic #4). **Context(s):** `deck-streak-notifications` (the ladder on
  SPEC-041's router: the requested tier, the weekly budgets, the rare floor, the streak-break cap,
  the near-miss gate, each tier's render, the reaction's freshness and its own breaker, the flush's
  ranking and cap, and `owner_last_message`); `deck-streak-bot` (the owner's latest message recorded,
  the dice, the reaction, the pin and the reveal's edit); `deck-streak-coordination` (the streak facts
  each occasion and each flush carries); `deck-streak-api` (the feed item's tier); the Mini App
  (`web/app`, the tier animation).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-041 (one router, its surface rule
  and its in-app feed), ADR-071 (an occasion is raised at the recompute that settles the study day
  it was earned on), and ADR-084 (the reveal is its own delivery call, `push_reveal`, policed by the
  one-router check beside the other five).
- **Prerequisites:** SPEC-041 (the router, its deferral, its failed-send hold, its decision ledger and
  its five tables), SPEC-026 (the bot's transport and owner gate). SPEC-076 supplies the streak
  facts when it lands; until then the caller passes none. **Mutation band:** `S08400-S08499`.
- **Status:** delivered with its tests, its hand-proved rows and `docs/red-first/SPEC-084.md`
  (ADR-016). The delivery settled what the SPEC left open (§10).

## 1. The problem, measured

- **What exists at `dev` c3d769b.** `crates/notifications/src/` holds only `lib.rs`. SPEC-041
  (planned, W1) builds the one router: a kind's setting, dedupe, the lapse rule, quiet hours, the
  deferral and its flush, the failed-send hold, the in-app feed and the decision ledger. It renders
  every celebration as a line, and its exclusions leave "the ladder's reveal, dice and pin renders,
  the weekly T4 and T5 budgets, the streak-break cap and the near-miss rule" to #120.
  `notifications-policy.json` already declares the ladder: the tiers, the rarity and event tiers, the
  rare floor, the budgets by intensity, the streak-break cap, the near-miss bounds and the reaction's
  24-hour age.
- **What is ported** (`27ee2bc`). `gamification/ladder.py`'s `requested_tier`, `weekly_budget`,
  `apply_budget`, `outcome_cap` and `near_miss_ok`, in the order
  `pipeline_layers/celebrations.py:CelebrationsLayer.celebrate` applies them; the streak-break rule
  `CelebrationsLayer._streak_broke_today`; the reaction `CelebrationsLayer._react_to_owner` and its
  own breaker; the flush's ranking and re-capping in `CelebrationsLayer.flush_deferred_celebrations`;
  and the week's count `database.py:GamifyStore.celebrations_at_or_above`, which counts a held
  celebration toward its week.
- **Corrections to the issue.** #120's second criterion says Epic and Legendary never drop below T2
  when a streak broke today. The predecessor applies the rare floor inside the budget
  (`apply_budget`) and the streak-break cap after it (`celebrate` takes the smaller of the two), so on
  such a day every celebration renders at most T1, Epic and Legendary included, as the ladder's own
  module says ("on a streak-death day everything caps at T1"). R5 states the code's order, proved by
  the golden `celebration_tier`. Its fourth criterion's retries are SPEC-041's failed-send hold for
  T2 to T5; a T1 hold carries no retry count and has a breaker of its own (R9).
- **Under the daily sync.** Occasions are raised at the recompute that settles the study day they
  were earned on (ADR-071), often after that day closed, while the predecessor celebrated during the
  day and read the current study day. The ladder therefore reads the budget week and the
  streak-break cap of the occasion's own study day, and a flush re-caps for the flush's study day, as
  the predecessor's flush does.
- **What the parity oracle proves.** Each of the five ladder functions; the streak-break rule; the
  composition and the transport calls of each tier (`celebrate`, through a recording notifier); the
  week's count; the reaction's freshness; the flush's ranking and caps; and the constants.
- **Prerequisites.** SPEC-041 and SPEC-026, as the header lists.

## 2. Requirements

The tier (#120)

R1. A celebration occasion carries its event type and, for a chest, its rarity. The router derives
    the requested tier: the rarity's tier when there is one, else the event's tier, else the tier of
    an unknown event, equal to the golden of `gamification/ladder.py:requested_tier` and read from
    `notifications-policy.json`'s `ladder.rarity`, `ladder.events` and `ladder.unknown_event`.
R2. The weekly budget is the owner's intensity's T4 and T5 counts, an unknown intensity reading as
    the default, from `celebration_budgets`, equal to the golden of `weekly_budget` and to the
    golden constant `constants.CELEBRATION_BUDGETS`. The intensity is a setting in SPEC-041's
    `notification_settings`, defaulting to the policy's `default_intensity`.
R3. Over budget, a T5 renders as T4 and a T4 as T3; a band-up is exempt; and an Epic or a Legendary
    never falls below the rare floor through the budget (`apply_budget` and the golden constant
    `gamification.ladder._RARE_FLOOR_TIER`).
R4. The budget's use is counted over the Monday-start week of the occasion's study day, over the
    celebrations delivered and the celebrations still held on the queue, each at its rendered or held
    tier; a held celebration the flush abandons releases its slot. The count equals the golden of
    `database.py:GamifyStore.celebrations_at_or_above`.
R5. After the budget comes the streak-break cap: on a study day the streak broke on, every
    celebration renders at most T1, Epic and Legendary included, the order
    `CelebrationsLayer.celebrate` applies (the golden `celebration_tier`). The streak broke on a day
    exactly when the language streak's last study day is that day, its current length is 1 and its
    longest is above 1 (the golden of `CelebrationsLayer._streak_broke_today`). Coordination supplies
    those three facts from the streaks context (SPEC-076); an occasion without them reads no break.
R6. The router applies the ladder after SPEC-041's first three rules (the setting, dedupe, the lapse)
    and before quiet hours, so a celebration held in quiet hours is held at its tier and counts
    toward its week, as the predecessor's `celebrate` orders them.
R7. Every ladder value is read from `notifications-policy.json` (`ladder`, `celebration_budgets`,
    `streak_break`, `near_miss`), never typed in code, and a test holds each to its golden.

The render (#120)

R8. On the bot, each tier makes the transport calls of the golden `celebration_tier`, in its order:
    T0 records the decision and sends nothing; T1 reacts to the owner's latest message with the
    golden's emoji; T2 sends the message; T3 sends a placeholder, waits the golden's pause
    (`pipeline_layers.celebrations._REVEAL_SUSPENSE_SECS`) and edits the placeholder into the message,
    sending the message anew when the edit fails; T4 sends a dice with the golden's emoji, then the
    message; T5 sends a dice, the message, and pins it. The calls stay inside the router module
    (SPEC-041 R1). The reveal is its own delivery call, `push_reveal`, which the one-router check
    polices beside the other five (ADR-084).
R9. T1 reacts only to an owner message at most `ladder.reaction_max_age_hours` old (the golden of
    `CelebrationsLayer._react_to_owner`); with no such message it makes no attempt and holds the
    celebration. A T1 hold carries no retry count: each flush retries it once, until it lands or the
    deferral's age cap (SPEC-041) abandons it by name. A refused reaction opens a reaction breaker of
    its own for `constants.CELEBRATION_OUTAGE_COOLDOWN_MS`, during which a T1 holds without an attempt
    while T2 to T5 still send; neither a refused reaction nor a failed dice or pin ever opens the
    payload breaker SPEC-041 keeps.
R10. In the Mini App (ADR-041's origin rule), the feed item carries its rendered tier, and the Mini
    App animates it: T3 as a two-beat reveal, T4 as a dice then the message, T5 as a dice, the message
    and a card that stays until the owner dismisses it. When reduced motion is requested, each tier
    shows the same content without motion. The feed route serves the tier to the owner only.

The flush (#120)

R11. A flush re-derives each held celebration's tier for the flush's study day, as the smaller of its
    held tier and that day's streak-break cap. It renders in full the two held celebrations of
    highest held tier, the older first on a tie, in the order they were held, and settles every other
    one in a single rollup line at most at T2, naming each it abandons. This equals the golden of
    `CelebrationsLayer.flush_deferred_celebrations`; SPEC-041's bounds (two in full, a queue of
    twenty, the age cap) stand.

The near-miss gate (#120)

R12. Every near-miss line passes one gate: a gap is quotable only when it is positive and at most
    `near_miss.max_units` units or at most `near_miss.max_fraction` of its target, equal to the golden
    of `gamification/ladder.py:near_miss_ok`. The notifications context exports the gate, and the
    callers that quote a gap call it through coordination.

The owner's latest message (#120)

R13. `deck-streak-notifications` owns `owner_last_message`
    (`migrations/008401_notifications_owner_last_message.sql`, `STRICT`, `created_at`): one row, the
    id of the owner's latest message to the bot and the instant it arrived. The bot records it for
    every inbound message its owner gate admits (SPEC-026), and never for another chat's. It is
    state, never a setting: writing it bumps no settings generation, as the predecessor kept its
    latest-message key out of its change gate (`constants.SYNC_GATE_CONFIG_EXEMPT_KEYS`).
R14. `owner_last_message` is declared once in notifications' data-rights port as reset in place, in
    `privacy.json` as the category `owner-last-message`, and with one line in `PRIVACY.md`; the
    ownership register gains its row.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the requested tier of every event and rarity, an unknown event included, equals the golden of `requested_tier` | `the_requested_tier_matches_the_parity_golden` |
| A2 | the weekly budget of every intensity, an unknown one included, equals the golden of `weekly_budget` | `the_weekly_budget_matches_the_parity_golden` |
| A3 | over budget a tier steps down as the golden of `apply_budget` says, a band-up is exempt, and an Epic or a Legendary keeps the rare floor | `an_over_budget_tier_steps_down_as_the_parity_golden_says` |
| A4 | the streak broke on a day exactly when the golden of `_streak_broke_today` says it did | `the_streak_broke_on_a_day_as_the_parity_golden_says` |
| A5 | on a day the streak broke, every celebration renders at most T1, Epic and Legendary included | `on_a_streak_break_day_every_celebration_renders_at_most_t1` |
| A6 | the week's count covers celebrations delivered and held at or above a tier, and an abandoned hold releases its slot, as the golden of `celebrations_at_or_above` counts | `the_week_counts_delivered_and_held_celebrations_as_the_parity_golden_does` |
| A7 | each tier makes the transport calls of the golden `celebration_tier` in its order, with its emoji and the reveal's pause | `each_tier_makes_the_transport_calls_of_the_parity_golden_in_order` |
| A8 | a reaction is attempted only on an owner message within the golden's age, and otherwise the celebration is held with no attempt | `a_reaction_is_attempted_only_on_a_message_within_its_age` |
| A9 | a refused reaction holds the next reactions without an attempt for the cooldown while a T2 still sends | `a_refused_reaction_holds_later_reactions_but_not_other_tiers` |
| A10 | a flush ranks, re-caps and rolls up held celebrations as the golden of `flush_deferred_celebrations` does | `a_flush_ranks_re_caps_and_rolls_up_as_the_parity_golden_does` |
| A11 | the near-miss gate equals the golden of `near_miss_ok` at every boundary | `the_near_miss_gate_matches_the_parity_golden` |
| A12 | the policy file's ladder, budgets, streak-break cap and near-miss bounds equal the goldens, and the router reads them from the file | `the_policy_ladder_values_equal_the_parity_goldens` |
| A13 | the bot records the owner's latest message id and instant, and records nothing for another chat's message | `the_owners_latest_message_is_recorded_and_no_other_chats` |
| A14 | a bot reveal edits its placeholder after the pause, and sends the message anew when the edit fails | `a_reveal_edits_its_placeholder_and_falls_back_to_a_new_message` |
| A15 | the in-app feed item carries its rendered tier, and the feed answers the owner only | `the_feed_item_carries_its_tier_and_answers_only_the_owner` |
| A16 | the Mini App animates each tier, and shows the same content without motion when reduced motion is requested | `animates each tier and shows the same content with reduced motion` |
| A17 | notifications' data-rights port exports `owner_last_message` and an erase resets it in place | `the_owners_latest_message_is_exported_and_reset_by_an_erase` |

```acceptance
A1: cargo test -p deck-streak-notifications --test ladder_tiers -- --exact the_requested_tier_matches_the_parity_golden
A2: cargo test -p deck-streak-notifications --test ladder_tiers -- --exact the_weekly_budget_matches_the_parity_golden
A3: cargo test -p deck-streak-notifications --test ladder_tiers -- --exact an_over_budget_tier_steps_down_as_the_parity_golden_says
A4: cargo test -p deck-streak-notifications --test ladder_tiers -- --exact the_streak_broke_on_a_day_as_the_parity_golden_says
A5: cargo test -p deck-streak-notifications --test ladder_render -- --exact on_a_streak_break_day_every_celebration_renders_at_most_t1
A6: cargo test -p deck-streak-notifications --test ladder_budget -- --exact the_week_counts_delivered_and_held_celebrations_as_the_parity_golden_does
A7: cargo test -p deck-streak-notifications --test ladder_render -- --exact each_tier_makes_the_transport_calls_of_the_parity_golden_in_order
A8: cargo test -p deck-streak-notifications --test ladder_render -- --exact a_reaction_is_attempted_only_on_a_message_within_its_age
A9: cargo test -p deck-streak-notifications --test ladder_render -- --exact a_refused_reaction_holds_later_reactions_but_not_other_tiers
A10: cargo test -p deck-streak-notifications --test ladder_flush -- --exact a_flush_ranks_re_caps_and_rolls_up_as_the_parity_golden_does
A11: cargo test -p deck-streak-notifications --test ladder_tiers -- --exact the_near_miss_gate_matches_the_parity_golden
A12: cargo test -p deck-streak-notifications --test ladder_policy -- --exact the_policy_ladder_values_equal_the_parity_goldens
A13: cargo test -p deck-streak-bot --test ladder_owner_message -- --exact the_owners_latest_message_is_recorded_and_no_other_chats
A14: cargo test -p deck-streak-bot --test ladder_transport -- --exact a_reveal_edits_its_placeholder_and_falls_back_to_a_new_message
A15: cargo test -p deck-streak-api --test ladder_feed -- --exact the_feed_item_carries_its_tier_and_answers_only_the_owner
A16: pnpm exec vitest run web/app/src/lib/ladder/tier-animation.test.ts -t "animates each tier and shows the same content with reduced motion"
A17: cargo test -p deck-streak-notifications --test ladder_rights -- --exact the_owners_latest_message_is_exported_and_reset_by_an_erase
```

A5, A7 and A9 run the router over a recording transport on a manual clock; A14 runs the bot
transport against a recording Bot API stand-in, the pause on paused time.

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. Every pack named here is enforced already, and this
delivery changes no pack's state; the notifications-policy pack's one-router check, which SPEC-041
enforces, keeps judging the router module.

| id | criterion | decided by |
|---|---|---|
| B1 | the ladder, the budgets, the streak-break cap, the near-miss bounds and the one router hold, over `notifications-policy.json` and `crates/notifications/src/`, examining every declared kind and event | the notifications-policy pack |
| B2 | the tier animation honours reduced motion and passes the audit in both Telegram colour schemes, over `web/app/src/lib/ladder/` and the screens that mount it | the accessibility pack |
| B3 | the dice, reaction, pin and edit calls stay within the Bot API's rules, over `crates/bot/src/transport.rs` and `crates/bot/src/gate.rs` | the telegram-platform pack |
| B4 | the privacy inventory declares the new category, over `privacy.json`, `PRIVACY.md` and `crates/notifications/src/data_rights.rs`, examining `owner-last-message` | the privacy-gdpr pack |
| B5 | no animation dresses a loss as a win and no line quotes a fabricated near miss, over `web/app/src/lib/ladder/` and `crates/notifications/src/ladder.rs` | the ux-laws pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/notifications/src/ladder.rs` | `deck-streak-notifications` | added: the requested tier, the budget, the rare floor, the streak-break rule and cap, the near-miss gate |
| `crates/notifications/src/router.rs` | `deck-streak-notifications` | changed: the ladder before quiet hours, each tier's render, the reaction's freshness and breaker, the flush's ranking and caps |
| `crates/notifications/src/policy.rs` | `deck-streak-notifications` | changed: the typed `ladder`, `celebration_budgets`, `streak_break` and `near_miss` sections |
| `crates/notifications/src/occasion.rs` | `deck-streak-notifications` | changed: a celebration carries its event type, its rarity and the streak facts |
| `crates/notifications/src/owner_message.rs` | `deck-streak-notifications` | added: the owner's latest message, recorded and read |
| `crates/notifications/src/data_rights.rs` | `deck-streak-notifications` | changed: `owner_last_message`, reset in place (the port SPEC-041 delivers, named by SPEC-021's rule) |
| `crates/notifications/src/lib.rs` | `deck-streak-notifications` | changed: the modules above |
| `crates/notifications/tests/ladder_tiers.rs` | `deck-streak-notifications` | added: A1 to A4, A11 |
| `crates/notifications/tests/ladder_render.rs` | `deck-streak-notifications` | added: A5, A7 to A9 |
| `crates/notifications/tests/ladder_budget.rs` | `deck-streak-notifications` | added: A6 |
| `crates/notifications/tests/ladder_flush.rs` | `deck-streak-notifications` | added: A10 |
| `crates/notifications/tests/ladder_policy.rs` | `deck-streak-notifications` | added: A12 |
| `crates/notifications/tests/ladder_rights.rs` | `deck-streak-notifications` | added: A17 |
| `crates/bot/src/transport.rs` | `deck-streak-bot` | changed: the dice, reaction and pin calls, and the reveal's placeholder, pause and edit |
| `crates/bot/src/gate.rs` | `deck-streak-bot` | changed: records the owner's latest message after the gate admits it |
| `crates/bot/tests/ladder_owner_message.rs` | `deck-streak-bot` | added: A13 |
| `crates/bot/tests/ladder_transport.rs` | `deck-streak-bot` | added: A14 |
| `crates/api/src/notifications_routes.rs` | `deck-streak-api` | changed: each feed item carries its rendered tier |
| `crates/api/tests/ladder_feed.rs` | `deck-streak-api` | added: A15 |
| `crates/coordination/src/ladder_facts.rs` | `deck-streak-coordination` | added: the streak facts an occasion and a flush carry, none until SPEC-076 supplies them |
| `crates/coordination/src/sync_cycle.rs` | `deck-streak-coordination` | changed: the flush after a successful sync carries the flush day's streak facts |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module above |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded `owner_last_message` row |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | unchanged: notifications' port is registered by SPEC-041; listed under SPEC-021's six-file rule |
| `web/app/src/lib/ladder/TierAnimation.svelte` | miniapp | added: each tier's animation, and its reduced-motion form |
| `web/app/src/lib/ladder/feed.ts` | miniapp | added: reads the owner's in-app feed and hands each item to the animation |
| `web/app/src/lib/ladder/tier-animation.test.ts` | miniapp | added: A16 |
| `web/app/src/routes/+layout.svelte` | miniapp | changed: mounts the animation layer on every screen |
| `migrations/008401_notifications_owner_last_message.sql` | `deck-streak-notifications` | added: `owner_last_message`, `STRICT`, one row |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `owner_last_message` |
| `privacy.json` | repo | changed: the `owner-last-message` category |
| `PRIVACY.md` | repo | changed: the `owner-last-message` line |
| `tools/parity-oracle/registry/spec_084.py` | repo | added: this SPEC's registrations (SPEC-029's registry) |
| `tools/parity-oracle/goldens/requested_tier.json` | repo | added: the golden of `gamification/ladder.py:requested_tier` (function) |
| `tools/parity-oracle/goldens/weekly_budget.json` | repo | added: the golden of `gamification/ladder.py:weekly_budget` (function) |
| `tools/parity-oracle/goldens/apply_budget.json` | repo | added: the golden of `gamification/ladder.py:apply_budget` (function) |
| `tools/parity-oracle/goldens/outcome_cap.json` | repo | added: the golden of `gamification/ladder.py:outcome_cap` (function) |
| `tools/parity-oracle/goldens/near_miss_ok.json` | repo | added: the golden of `gamification/ladder.py:near_miss_ok` (function) |
| `tools/parity-oracle/goldens/streak_broke_today.json` | repo | added: the golden of `pipeline_layers/celebrations.py:CelebrationsLayer._streak_broke_today` (adapter; a stub store's streak state) |
| `tools/parity-oracle/goldens/celebration_tier.json` | repo | added: the golden of `pipeline_layers/celebrations.py:CelebrationsLayer.celebrate` (adapter; a stub store and a recording notifier) |
| `tools/parity-oracle/goldens/celebration_week_count.json` | repo | added: the golden of `database.py:GamifyStore.celebrations_at_or_above` (adapter; a temporary store database) |
| `tools/parity-oracle/goldens/reaction_freshness.json` | repo | added: the golden of `pipeline_layers/celebrations.py:CelebrationsLayer._react_to_owner` (adapter; a stub store and a patched clock) |
| `tools/parity-oracle/goldens/celebration_flush.json` | repo | added: the golden of `pipeline_layers/celebrations.py:CelebrationsLayer.flush_deferred_celebrations` (adapter; held rows and a recording notifier) |
| `tools/parity-oracle/goldens/ladder.constants.json` | repo | added: the ladder's tier maps, the rare floor, the budgets, the retry cap, the cooldown and the reveal's pause (constants) |
| `scripts/mutation-rows.d/S08400-S08499.json` | repo | added: the hand-proved rows of §9 |
| `docs/specs/SPEC-084-the-celebration-ladder-renders-tiers-on-the-one-router-within-weekly-budgets.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-084.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It builds no settings screen for the intensity, the quiet window or a kind's switch (#57).
- It re-pins no widget after a T5 (#121).
- It draws no holdout arm and reports no ablation readout (#132).
- It raises no share card (#125), landmark (#127) or milestone ping (#128): each raises its occasion
  through this ladder when it lands.
- It writes no near-miss line: the digests and the evening nudges that quote a gap call the gate
  (#129, #117).
- It decides no streak break itself: the streaks context supplies the facts (#81).

## 6. Risks

- **A restart during a reveal's pause leaves only the placeholder.** The decision is recorded before
  the placeholder is sent (SPEC-041's once-ever delivery), so nothing repeats; the loss is the
  reveal's second beat, as in the predecessor. Visible in the bot; A14 covers the failed edit.
- **Celebrations raised by the settle land after quiet hours**, because the one daily sync runs
  inside the quiet window (ADR-037), so the T4 and T5 of a day with many celebrations meet in one
  flush. Detected by A10's ranking, which renders the highest tiers first and names every other in
  the rollup; A6 counts held ones, so the week's budget holds.
- **The streak facts are absent until SPEC-076 lands**, so no day reads as a break. Detected by
  SPEC-076's criteria, which supply them; until then there is no streak to break.
- **A tier value is typed in code and drifts from the policy file.** Detected by A12, and by the
  notifications-policy pack in the box run (B1).
- **An animation ignores reduced motion.** Detected by A16 and by the accessibility pack (B2).

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_084.py`; every day is an epoch day number and every
instant epoch milliseconds (SPEC-029 R3). No golden holds a message's text: an adapter returns tiers,
calls and their non-text arguments only.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `requested_tier` | `gamification/ladder.py:requested_tier` | function | nothing: every event, every rarity, an unknown event and an empty rarity |
| `weekly_budget` | `gamification/ladder.py:weekly_budget` | function | nothing: every intensity and an unknown one |
| `apply_budget` | `gamification/ladder.py:apply_budget` | function | nothing: each tier at, under and over its slots, exempt and rare cases |
| `outcome_cap` | `gamification/ladder.py:outcome_cap` | function | nothing |
| `near_miss_ok` | `gamification/ladder.py:near_miss_ok` | function | nothing: gaps at and beyond 5 units and 10 percent, zero and negative gaps and targets |
| `streak_broke_today` | `pipeline_layers/celebrations.py:CelebrationsLayer._streak_broke_today` | adapter | a stand-in layer whose stub store answers the case's streak state (last study day, current, longest) |
| `celebration_tier` | `pipeline_layers/celebrations.py:CelebrationsLayer.celebrate` | adapter | a stand-in layer with a stub store answering the case's intensity and week counts, the streak-break check patched to the case's answer, quiet hours and both breakers closed, the reveal's sleep patched to record its seconds, and a recording notifier that accepts every call; it returns the rendered tier and the ordered calls with their emoji and pause |
| `celebration_week_count` | `database.py:GamifyStore.celebrations_at_or_above` | adapter | a temporary store database the predecessor's own `GamifyStore` creates, seeded through its `record_celebration` and `record_celebration_pending` from the case's rows (study day, rendered or held tier) |
| `reaction_freshness` | `pipeline_layers/celebrations.py:CelebrationsLayer._react_to_owner` | adapter | a stand-in layer whose stub store holds the case's latest owner message, the module's clock patched to the case's instant, and a recording notifier; it returns whether a reaction was attempted, and its emoji |
| `celebration_flush` | `pipeline_layers/celebrations.py:CelebrationsLayer.flush_deferred_celebrations` | adapter | a stand-in layer whose stub store holds the case's held rows (key, held tier, held-at instant, order), the clock at the case's instant outside quiet hours, the streak-break check patched, and a recording notifier; it returns the keys rendered in full with their final tiers in render order, and the keys rolled up and abandoned |
| `ladder.constants` | `gamification.ladder._RARITY_TIER`, `_EVENT_TIER`, `_RARE_FLOOR_TIER`, `constants.CELEBRATION_BUDGETS`, `CELEBRATION_SEND_RETRY_MAX`, `CELEBRATION_OUTAGE_COOLDOWN_MS`, `pipeline_layers.celebrations._REVEAL_SUSPENSE_SECS` | constants | nothing |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `owner_last_message` | `notifications` | `migrations/008401_notifications_owner_last_message.sql` (SPEC-084) | `settings_kv`'s latest-message key: its message id and instant become the one row | reset in place: no message |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S08401-THE-RARITY-WINS` | `crates/notifications/src/ladder.rs` | the rarity's tier chosen before the event's | `ladder_tiers::the_requested_tier_matches_the_parity_golden` |
| `S08402-T5-STEPS-TO-T4` | `crates/notifications/src/ladder.rs` | an over-budget T5 renders as T4 | `ladder_tiers::an_over_budget_tier_steps_down_as_the_parity_golden_says` |
| `S08403-THE-RARE-FLOOR` | `crates/notifications/src/ladder.rs` | an Epic or a Legendary keeps the floor through the budget | `ladder_tiers::an_over_budget_tier_steps_down_as_the_parity_golden_says` |
| `S08404-THE-CAP-FOLLOWS-THE-BUDGET` | `crates/notifications/src/router.rs` | the streak-break cap applied after the budget, to every rarity | `ladder_render::on_a_streak_break_day_every_celebration_renders_at_most_t1` |
| `S08405-A-BREAK-NEEDS-A-LONGER-PAST` | `crates/notifications/src/ladder.rs` | the break rule's longest above one | `ladder_tiers::the_streak_broke_on_a_day_as_the_parity_golden_says` |
| `S08406-A-HELD-ONE-COUNTS` | `crates/notifications/src/router.rs` | the week's count includes celebrations still held | `ladder_budget::the_week_counts_delivered_and_held_celebrations_as_the_parity_golden_does` |
| `S08407-THE-FLUSH-RANKS-BY-TIER` | `crates/notifications/src/router.rs` | held celebrations ranked by held tier before age | `ladder_flush::a_flush_ranks_re_caps_and_rolls_up_as_the_parity_golden_does` |
| `S08408-THE-ROLLUP-SETTLES-AT-T2` | `crates/notifications/src/router.rs` | a rolled-up celebration settles at most at T2 | `ladder_flush::a_flush_ranks_re_caps_and_rolls_up_as_the_parity_golden_does` |
| `S08409-THE-REACTION-AGE` | `notifications-policy.json` | the reaction's 24-hour age read from the policy (a script-mutation row) | `ladder_policy::the_policy_ladder_values_equal_the_parity_goldens` |
| `S08410-THE-NEAR-MISS-UNITS` | `notifications-policy.json` | the near-miss gate's five units (a script-mutation row) | `ladder_policy::the_policy_ladder_values_equal_the_parity_goldens` |
| `S08411-THE-REVEAL-IS-A-POLICED-DELIVERY-CALL` | `notifications-policy.json` | `push_reveal` among the bot's delivery calls the one-router check polices (ADR-084; a script-mutation row) | `ladder_policy::the_policy_polices_the_reveal_beside_the_other_delivery_calls` |
| `S08412-THE-DICE-IS-THE-SLOT-MACHINE` | `crates/notifications/src/ladder.rs` | the dice, whole (a constant) | `ladder_render::each_tier_makes_the_transport_calls_of_the_parity_golden_in_order` |
| `S08413-THE-REACTION-IS-A-PARTY-POPPER` | `crates/notifications/src/ladder.rs` | the reaction's emoji, whole (a constant) | `ladder_render::a_reaction_is_attempted_only_on_a_message_within_its_age` |
| `S08414-THE-REVEAL-PAUSES-2500-MS` | `crates/notifications/src/ladder.rs` | the reveal's pause, whole (a constant) | `ladder_render::each_tier_makes_the_transport_calls_of_the_parity_golden_in_order` |
| `S08415-THE-REVEAL-OPENS-WITH-ITS-PLACEHOLDER` | `crates/notifications/src/ladder.rs` | the reveal's placeholder, whole (a constant the bot's test pins) | `ladder_transport::a_reveal_edits_its_placeholder_and_falls_back_to_a_new_message` |
| `S08416-A-REACTION-AGE-IS-IN-HOURS` | `crates/notifications/src/ladder.rs` | the reaction's age counted in hours (a constant) | `ladder_render::a_reaction_is_attempted_only_on_a_message_within_its_age` |
| `S08417-THE-OWNERS-LATEST-MESSAGE-TABLE` | `crates/notifications/src/ledger.rs` | the port declares the table the migration creates (a constant) | `ladder_rights::the_owners_latest_message_is_exported_and_reset_by_an_erase` |
| `S08418-THE-INTENSITY-IS-THE-PREDECESSORS-KEY` | `crates/notifications/src/router.rs` | the owner's intensity read under the predecessor's key (a constant) | `ladder_render::each_tier_makes_the_transport_calls_of_the_parity_golden_in_order` |

## 10. Amendments at delivery

- **R8: the reveal's own call (ADR-084).** The bot's port gains `push_reveal`, `push_dice`,
  `push_reaction` and `push_pin` beside `push_message`, each taking the router's pass. Each has a
  default that answers `Unsupported`, never a silent delivery, and the bot's transport implements all
  four; `push_message` and its implementers are unchanged. `notifications-policy.json` names
  `push_reveal` in `router.transport.bot`, so the policy names six delivery calls. The router records
  an unsupported call by its name as the ladder's degraded render: a reveal or a pin that cannot be
  made renders as the line at T2, a dice that cannot be rolled is recorded and the render goes on,
  and a reaction that cannot be made renders at T0.
- **R8 and R10: the tier rendered.** The decision records the tier a celebration actually rendered
  at: a failed reveal or pin that fell back to the line records T2. The in-app feed item carries that
  tier, read from the ledger's feed row, so the feed route itself is unchanged.
- **R9 and R13: the owner's latest message.** The bot records it when the owner's message is handled
  (`commands.rs`), after the gate admitted it; the gate hands on the message's id. A message id beyond
  the Bot API's 32-bit range is never reacted to: the celebration is held for the next flush.
- **R11: a failed recap keeps its rows' holds (ADR-084).** The flush's golden keeps the hold of a row
  that a failed recap only named, so SPEC-041's test of a failed recap now expects the quiet hold the
  row had (`deferral.rs`, `a_failed_recap_holds_its_rolled_celebration_again`). A failed full render
  still holds its row with the failed-send hold.
- **A11 and A12: each near-miss bound where it alone decides.** The near-miss golden gains a gap of 5
  toward 40 and of 5.5 toward 40, where the units bound decides and the fraction does not, and A12
  parses the policy with other bounds and checks the reaction's age and the near-miss units follow
  the file.
- **The census.** SPEC-041 A15's census names each new call at its one call site in the bot's
  transport, and leaves out the parity oracle's tooling (`tools/parity-oracle/`), which ships
  nothing and whose recording stand-ins name the Bot API's calls they record; a directory of the same
  name anywhere else is still read.
- **§9: the rows added at delivery.** S08411 holds `push_reveal` in the policy (ADR-084), and
  S08412 to S08418 plant each module-level constant the delivery adds and pin it by its whole value,
  since cargo-mutants never mutates a constant. A7 and A14 read the intensity's key and the reveal's
  placeholder as literals, so each pins its constant rather than reading it back. S08415's constant
  lives in notifications but only the bot's test pins it, so its row runs the bot's test.
- **The flush's streak facts.** The sync cycle's flush carries `ladder_facts::streak_facts()`, which
  is none until SPEC-076 lands.
- **The manifest.** The delivery also changes:
  - `notifications-policy.json` (`push_reveal`) and `docs/decisions/ADR-084-*.md`;
  - `crates/notifications/src/transport.rs` (the port's four calls and their defaults) and
    `crates/notifications/src/ledger.rs` (the feed item's tier, the week's counts, a relatch that
    keeps a hold);
  - `crates/bot/src/commands.rs` (the record), `crates/bot/tests/gate.rs` (the message id) and
    `crates/bot/tests/support/fake_bot_api.rs` (the dice answered as a message);
  - the notifications tests `one_router.rs`, `rights.rs`, `router.rs` and `deferral.rs`, and the
    shared `tests/support/ladder.rs` and `tests/support/mod.rs`;
  - `web/app/src/lib/api.ts` (the feed's read), `web/app/messages/*.json` (the animation's four
    strings) and `web/app/src/routes/layout.test.ts` (the layer on every screen);
  - `scripts/mutation-equivalent.d/miniapp.json`: the record excusing `api.ts`'s JSON-error mutant
    names `parseFeed` beside the other two parsers it holds for;
  - `scripts/mutation-equivalent.d/deck-streak-notifications.json` (added): the guard of
    `Router::send_bot` recorded equivalent, since a non-celebration reaches it only at T0 or T2;
  - `crates/api/src/notifications_routes.rs` is unchanged: the feed item it serves carries the tier.
