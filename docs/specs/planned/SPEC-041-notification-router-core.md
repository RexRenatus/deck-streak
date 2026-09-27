# SPEC-041: one router decides every celebration and nudge for both surfaces, with quiet hours and a withhold ledger

- **Wave:** W1. **Issue:** #27 (epic #2). **Context(s):** `deck-streak-notifications`; its transport in `deck-streak-bot`; the feed route in `deck-streak-api`; the flush step in `deck-streak-coordination`.
- **Decided by:** ADR-002 (the crate graph, transports joined in the composition root), ADR-011
  (side by side: DeckStreak sends only kinds the predecessor does not), ADR-012 (the parity oracle),
  and ADR-041 (the surface rule, the pulled in-app feed, the lapse input and the `reading_ready`
  kind).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-041.md` (ADR-016).

## 1. The problem, measured

- **The charter's second constraint has no code yet.** Every celebration and nudge must pass
  through one router, or an event raised from the bot and from the Mini App celebrates twice
  (constraint 2; ARCHITECTURE.md's invariant `notifications::router::route`). `crates/notifications/src/`
  holds only `lib.rs`.
- **The policy is declared and waiting.** `notifications-policy.json` names
  `crates/notifications/src/router.rs` and its symbol `route`, with the bot's `push_message`,
  `push_dice`, `push_reaction` and `push_pin` and the Mini App's `push_in_app` as the only delivery
  calls. The notifications-policy pack is enforced, with its `one-router` row deferred until this
  delivery (`.packs/wiring.json`, `deferred_rows`).
- **What is ported.** The predecessor's `quiet-hours` feature (`quiet_hours.py:in_quiet_hours`:
  23:00 to 07:30, wrapping midnight, start equal to end disables), the deferral and flush of
  `pipeline_layers/celebrations.py:CelebrationsLayer.flush_deferred_celebrations` (720 minutes, 2
  per flush, a queue of 20, a rollup line), its failed-send hold, the comeback cap keyed by the
  lapse episode, and the recorded reason for every declined send.
- **Who needs it in W1.** The morning readings line (SPEC-052), the comeback reading (SPEC-049)
  and the readings health pages (SPEC-050).
- **What the parity oracle proves.** `quiet_hours.py:in_quiet_hours` over minutes on both sides of
  each boundary, a wrapping window, a same-day window and a disabled one.
- **Prerequisites.** SPEC-020 (the clock, the study day, typed configuration, the SQLite base),
  SPEC-021 (export and erase), SPEC-025 (the API shell), SPEC-026 (the bot's notifier, which gains
  the transport calls) and SPEC-027 (the scheduler whose post-sync step list calls the flush).

## 2. Requirements

R1. `crates/notifications/src/router.rs` defines `route`, the one entry point for every
    celebration, nudge, digest and alert of either surface. The five delivery calls the policy's
    `router.transport` names are called only inside that module.
R2. The router reads `notifications-policy.json` once at start into a typed policy. An unknown
    key, an undeclared kind or a malformed value refuses start with an error naming the key. An
    occasion's kind is a typed value that only a declared kind can produce.
R3. An occasion carries its kind, an opaque dedupe key (`^[a-z0-9][a-z0-9:._-]*$`, never a calendar
    date), its origin surface, its requested tier (one of the kind's tiers), its payload, the study
    day, and a lapse context (no lapse, or the open lapse's id) that the caller supplies.
R4. `route` decides in this order, and the first rule that matches decides:
    1. the kind's setting is off: withhold with `nudges_disabled` (the comeback setting is off at
       the value `"0"`);
    2. a delivery already exists for the kind and the key's dedupe scope (`once-ever`,
       `per-study-day`, `per-episode-day` or `per-incident`, as the kind declares): withhold with
       `already_recorded`;
    3. a nudge while a lapse is open: withhold with `lapse`, unless the kind's budget is `comeback`,
       which exists only inside a lapse;
    4. inside quiet hours: a celebration is deferred, a nudge is withheld with `quiet_hours`, and an
       alert is exempt;
    5. a comeback whose lapse id already has 3 sends, or whose previous send was fewer than 3 study
       days earlier: withhold with `budget_spent`; after the cap the episode stays silent;
    6. no transport answers for the chosen surface: withhold with `no_notifier`;
    7. otherwise: send through the surface's transport and record the delivery.
R5. The surface: a celebration raised by a Mini App request goes to the Mini App's feed through
    `push_in_app`; every other occasion goes to the bot. One dedupe key is delivered at most once
    across both surfaces.
R6. Every decision is recorded in the decision ledger in the pack's `phx.notifications.decision.v1`
    shape: dedupe key, kind, surface, arm (`send`, `defer` or `withhold`), reason, tier requested and
    tier rendered. A withheld occasion is recorded under its kind with `:withheld` appended, so it
    never stands for a delivery of its key.
R7. Deferral holds at most 20 celebrations. A flush outside quiet hours renders at most 2 of them in
    full and collapses the rest into one rollup line; a celebration older than 720 minutes at the
    flush is abandoned and named in the recap line; nothing is dropped without being named.
    Coordination calls `flush` as a step after every successful sync.
R8. A failed send is held on the queue with its first deferral time kept, retried by later flushes
    at most 2 times, and every send waits 60 seconds after a failure (the outage breaker). After its
    retries it is abandoned and named.
R9. The quiet window is evaluated on the kernel's clock at the configured local offset, from the
    policy's 23:00 and 07:30 unless the owner's settings override them, and equals the golden of
    `quiet_hours.py:in_quiet_hours`.
R10. The policy gains the kind `reading_ready`: class `nudge`, tiers `["T2"]`, budget `null`, dedupe
    `per-study-day`, setting `reading_ready_enabled`. Its `deviations` records
    `{"key": "kinds.reading_ready", "adr": "docs/decisions/ADR-041-notification-router-core.md"}`.
R11. Notifications owns five tables, each with `created_at`: `notification_decisions`,
    `notification_deliveries` (unique on kind and scoped dedupe key), `notification_queue`,
    `in_app_feed` and `notification_settings`. Each is registered in the context map's ownership
    register (the predecessor's `notifications` and `celebration_log` map onto them), declared in
    `privacy.json`, and listed in the notifications data-rights port.
R12. `GET /api/notifications/feed` serves the unseen in-app items to the authenticated owner only
    and marks them seen; any other caller gets 401 or 403 and no item.
R13. The bot implements the bot transport calls in `crates/bot/src/transport.rs` on its SPEC-026
    notifier; the composition root joins it to the router in `crates/daemon/src/wiring.rs`.
    `push_in_app` appends to `in_app_feed` inside the router module.
R14. The typed policy reads every key of the file, and names the key `defer_fanfare` through a
    serde rename, so no identifier in the notifications context says `fanfare` or `confetti` (the
    lexicon's lock for `celebration`), nor `absence` or `dropout` (its lock for `lapse`).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every row of the notifications-policy pack is green over the tree, `one-router` included, with the row's deferral removed from the wiring | notifications-policy, every row; `test_every_notifications_policy_row_is_green_with_one_router_run` |
| A2 | a delivery call planted outside the router module is refused by `one-router` | notifications-policy `one-router`; `test_a_transport_call_outside_the_router_is_refused` |
| A3 | `reading_ready` differs from the baseline only through a deviation whose ADR names `kinds.reading_ready` | notifications-policy `policy-deviation-has-adr`; `test_the_reading_ready_kind_is_a_recorded_deviation` |
| A4 | a celebration raised at 23:30 is deferred, is not sent by a flush at 07:29, and is delivered by the first flush at 07:30 (injected clock) | `a_celebration_raised_in_quiet_hours_is_delivered_when_the_window_ends` |
| A5 | the same celebration, first flushed more than 720 minutes after it was raised, is abandoned and named in the recap line (injected clock) | `a_deferred_celebration_older_than_720_minutes_is_abandoned_by_name` |
| A6 | a nudge raised in quiet hours is withheld with `quiet_hours` and recorded under its kind with `:withheld` | `a_nudge_in_quiet_hours_is_withheld_and_recorded` |
| A7 | the same event raised from the bot and from the Mini App is delivered once, and the second decision is `already_recorded` | `the_same_event_from_both_surfaces_is_delivered_once` |
| A8 | the quiet window equals the golden of `quiet_hours.py:in_quiet_hours`, wrapping and disabled windows included | `the_quiet_window_matches_the_parity_golden` |
| A9 | a fourth comeback for one lapse id, and a second one inside 3 study days, are withheld with `budget_spent` | `a_comeback_past_the_cap_or_inside_the_gap_is_withheld` |
| A10 | a nudge during an open lapse is withheld with `lapse`, and a comeback in the same lapse is sent | `a_nudge_in_a_lapse_is_withheld_and_a_comeback_is_not` |
| A11 | a failed send is held with its first deferral time, retried at most twice, then abandoned by name | `a_failed_send_is_held_and_retried_twice` |
| A12 | a flush renders at most 2 deferred celebrations in full and one rollup line naming the rest; the queue never exceeds 20 | `a_flush_renders_two_and_rolls_up_the_rest` |
| A13 | the in-app feed serves its items to the owner's session and refuses any other caller with no item | `the_in_app_feed_answers_only_the_owner` |
| A14 | the notifications data-rights port lists its five tables as exported and erased, and an erase empties them | `the_notification_tables_are_exported_and_erased` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_notifications_router_rows.py -k test_every_notifications_policy_row_is_green_with_one_router_run
A2: python3 -m unittest discover -s scripts/tests -p test_notifications_router_rows.py -k test_a_transport_call_outside_the_router_is_refused
A3: python3 -m unittest discover -s scripts/tests -p test_notifications_router_rows.py -k test_the_reading_ready_kind_is_a_recorded_deviation
A4: cargo test -p deck-streak-notifications --test deferral -- --exact a_celebration_raised_in_quiet_hours_is_delivered_when_the_window_ends
A5: cargo test -p deck-streak-notifications --test deferral -- --exact a_deferred_celebration_older_than_720_minutes_is_abandoned_by_name
A6: cargo test -p deck-streak-notifications --test router -- --exact a_nudge_in_quiet_hours_is_withheld_and_recorded
A7: cargo test -p deck-streak-notifications --test router -- --exact the_same_event_from_both_surfaces_is_delivered_once
A8: cargo test -p deck-streak-notifications --test quiet_hours -- --exact the_quiet_window_matches_the_parity_golden
A9: cargo test -p deck-streak-notifications --test comeback_budget -- --exact a_comeback_past_the_cap_or_inside_the_gap_is_withheld
A10: cargo test -p deck-streak-notifications --test comeback_budget -- --exact a_nudge_in_a_lapse_is_withheld_and_a_comeback_is_not
A11: cargo test -p deck-streak-notifications --test deferral -- --exact a_failed_send_is_held_and_retried_twice
A12: cargo test -p deck-streak-notifications --test deferral -- --exact a_flush_renders_two_and_rolls_up_the_rest
A13: cargo test -p deck-streak-api --test notifications_feed -- --exact the_in_app_feed_answers_only_the_owner
A14: cargo test -p deck-streak-notifications --test rights -- --exact the_notification_tables_are_exported_and_erased
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/notifications/Cargo.toml` | `deck-streak-notifications` | changed: the workspace dependencies it uses |
| `crates/notifications/src/lib.rs` | `deck-streak-notifications` | changed: the modules below |
| `crates/notifications/src/policy.rs` | `deck-streak-notifications` | added: the typed policy |
| `crates/notifications/src/occasion.rs` | `deck-streak-notifications` | added: the occasion, the kind, the dedupe key |
| `crates/notifications/src/router.rs` | `deck-streak-notifications` | added: `route`, `flush`, `push_in_app` |
| `crates/notifications/src/quiet.rs` | `deck-streak-notifications` | added: the quiet window |
| `crates/notifications/src/ledger.rs` | `deck-streak-notifications` | added: decisions, deliveries, queue, feed, settings |
| `crates/notifications/src/transport.rs` | `deck-streak-notifications` | added: the bot transport port |
| `crates/notifications/src/rights.rs` | `deck-streak-notifications` | added: the data-rights port |
| `crates/notifications/migrations/0001_router.sql` | `deck-streak-notifications` | added |
| `crates/notifications/tests/router.rs` | `deck-streak-notifications` | added |
| `crates/notifications/tests/deferral.rs` | `deck-streak-notifications` | added |
| `crates/notifications/tests/quiet_hours.rs` | `deck-streak-notifications` | added |
| `crates/notifications/tests/comeback_budget.rs` | `deck-streak-notifications` | added |
| `crates/notifications/tests/rights.rs` | `deck-streak-notifications` | added |
| `crates/bot/src/transport.rs` | `deck-streak-bot` | changed: implements the bot transport calls |
| `crates/api/src/routes/notifications.rs` | `deck-streak-api` | added: the feed route |
| `crates/api/tests/notifications_feed.rs` | `deck-streak-api` | added |
| `crates/coordination/src/post_sync.rs` | `deck-streak-coordination` | changed: the flush step after a successful sync |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the bot transport joined to the router |
| `notifications-policy.json` | repo | changed: the `reading_ready` kind and its deviation |
| `.packs/wiring.json` | repo | changed: `one-router` is no longer deferred |
| `scripts/tests/test_notifications_router_rows.py` | repo | added |
| `tools/parity-oracle/generate.py` | repo | changed: registers `quiet_hours.py:in_quiet_hours` |
| `tools/parity-oracle/goldens/in_quiet_hours.json` | repo | added |
| `docs/CONTEXT-MAP.md` | docs | changed: the ownership register's five notifications tables |
| `privacy.json` | repo | changed: the notifications categories |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `docs/schematics/notification-router.md` | docs | added |
| `docs/specs/SPEC-041-notification-router-core.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-041-notification-router-core.md` | docs | added |
| `docs/red-first/SPEC-041.md` | docs | added |

## 5. What this does NOT do

- It renders no tier beyond a line: the ladder's reveal, dice and pin renders, the weekly T4 and T5
  budgets, the streak-break cap and the near-miss rule are the celebration ladder's
  (#120).
- It draws no holdout arm and reports no ablation readout (#132).
- It declares no evening budget consumer: streak risk and last chance come with their features
  (#117, #118).
- It writes no morning brief, digest or comeback text; each duty owns its words
  (#122, #129, #123).
- It decides no lapse: the governor owns the lapse and its id, and the caller passes it in
  (#83).
- It builds no settings screen for quiet hours or a kind's switch (#57).
- It pins and re-pins no widget (#121).

## 6. Risks

- **A delivery call appears outside the router**, for example a bot command reply written with
  `push_message`. Detected by the `one-router` row in the gate (A1, A2).
- **The lapse context is empty until the governor exists**, so a nudge could reach an owner in a
  real lapse. Detected by SPEC-049's and SPEC-052's lapse tests, which cannot pass without the
  governor's lapse episode; the W1 kinds that nudge (`reading_ready`, `comeback`) declare that
  prerequisite.
- **The quiet window is read at the wrong offset.** Detected by the golden (A8) and by A4's
  injected clock.
- **A background celebration reaches the bot while the Mini App is open** (the origin rule).
  Visible in the decision ledger's surface column; accepted by ADR-041.
- **The flush never runs** because no sync succeeds. The dead-man watch (SPEC-027) pages on stopped
  syncs, and the queue's age cap names every abandoned celebration.
