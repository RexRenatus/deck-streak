# SPEC-041: one router decides every celebration and nudge for both surfaces, with quiet hours and a withhold ledger

- **Wave:** W1. **Issue:** #27 (epic #2). **Context(s):** `deck-streak-notifications`; its transport in `deck-streak-bot`; the feed route in `deck-streak-api`; the flush step in `deck-streak-coordination`.
- **Decided by:** ADR-002 (the crate graph, transports joined in the composition root), ADR-011
  (side by side: DeckStreak sends only kinds the predecessor does not), ADR-012 (the parity oracle),
  and ADR-041 (the surface rule, the pulled in-app feed, the lapse input and the `reading_ready`
  kind).
- **Status:** delivered with its tests, its hand-proved rows and `docs/red-first/SPEC-041.md`
  (ADR-016). The delivery re-planned its criteria over pack rows and settled what the SPEC left open
  (§7).

## 1. The problem, measured

- **The charter's second constraint has no code yet.** Every celebration and nudge must pass
  through one router, or an event raised from the bot and from the Mini App celebrates twice
  (constraint 2; ARCHITECTURE.md's invariant `notifications::router::route`). `crates/notifications/src/`
  holds only `lib.rs`.
- **The policy is declared and waiting.** `notifications-policy.json` names
  `crates/notifications/src/router.rs` and its symbol `route`, with the bot's `push_message`,
  `push_dice`, `push_reaction` and `push_pin` and the Mini App's `push_in_app` as the only delivery
  calls. The notifications-policy pack is enforced, with its `one-router` row deferred until this
  delivery (the box-run packs' wiring, `deferred_rows`; ADR-069).
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
  SPEC-021 (export and erase), SPEC-022 and SPEC-023 (the sync cycle, `coordination::sync_cycle`,
  whose successful syncs the flush step follows), SPEC-024 (the owner session the feed route
  requires), SPEC-025 (the API shell and its router), SPEC-026 (the bot's notifier, which gains the
  transport calls) and SPEC-027 (the scheduler that runs the sync).

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
    Coordination calls `flush` as a step of the sync cycle (`coordination::sync_cycle`) after every
    successful sync.
R8. A failed send is held on the queue with its first deferral time kept, retried by later flushes
    at most 2 times, and every send waits 60 seconds after a failure (the outage breaker). After its
    retries it is abandoned and named.
R9. The quiet window is evaluated on the kernel's clock at the configured local offset, from the
    policy's 23:00 and 07:30 unless the owner's settings override them, and equals the golden of
    `quiet_hours.py:in_quiet_hours`.
R10. The policy gains the kind `reading_ready`: class `nudge`, tiers `["T2"]`, budget `null`, dedupe
    `per-study-day`, setting `reading_ready_enabled`. Its `deviations` records
    `{"key": "kinds.reading_ready", "adr": "docs/decisions/ADR-041-notification-router-core.md"}`.
R11. Notifications owns five tables, each `STRICT` and with `created_at`, all created by
    `migrations/004101_notifications_router.sql`: `notification_decisions`,
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
| A1 | the typed policy reads every key of `notifications-policy.json`: written back, it equals the file | `the_typed_policy_reads_every_key_of_the_file` |
| A2 | a call of the port's delivery call outside the router module does not compile: it takes the router's pass, which no other module can make — not by its field, `Default`, or cloning a borrowed pass (one compile-fail case each) | `a_delivery_call_outside_the_router_does_not_compile` |
| A3 | the typed policy reads `reading_ready` as a nudge of tier T2 with no budget, deduplicated per study day behind `reading_ready_enabled`, and its one deviation cites ADR-041, which names `kinds.reading_ready` | `the_reading_ready_kind_is_a_recorded_deviation` |
| A4 | a celebration raised at 23:30 is deferred, is not sent by a flush at 07:29, and is delivered by the first flush at 07:30 (injected clock) | `a_celebration_raised_in_quiet_hours_is_delivered_when_the_window_ends` |
| A5 | the same celebration, first flushed more than 720 minutes after it was raised, is abandoned and named in the recap line (injected clock) | `a_deferred_celebration_older_than_720_minutes_is_abandoned_by_name` |
| A6 | a nudge raised in quiet hours is withheld with `quiet_hours` and recorded under its kind with `:withheld` | `a_nudge_in_quiet_hours_is_withheld_and_recorded` |
| A7 | the same event raised from the bot and from the Mini App is delivered once, and the second decision is `already_recorded` | `the_same_event_from_both_surfaces_is_delivered_once` |
| A8 | the quiet window equals the golden of `quiet_hours.py:in_quiet_hours`, wrapping and disabled windows included | `the_quiet_window_matches_the_parity_golden` |
| A9 | a fourth comeback for one lapse id, and a second one inside 3 study days — counted in the owner's study days, so one raised before the rollover hour on the third calendar day is inside the gap — are withheld with `budget_spent`, and a comeback sent before the rollover hour is counted from its study day, so one 3 study days later is sent | `a_comeback_past_the_cap_or_inside_the_gap_is_withheld` |
| A10 | a nudge during an open lapse is withheld with `lapse`, and a comeback in the same lapse is sent | `a_nudge_in_a_lapse_is_withheld_and_a_comeback_is_not` |
| A11 | a failed send is held with its first deferral time, retried at most twice, then abandoned by name | `a_failed_send_is_held_and_retried_twice` |
| A12 | a flush renders at most 2 deferred celebrations in full and one rollup line naming the rest; the queue never exceeds 20 | `a_flush_renders_two_and_rolls_up_the_rest` |
| A13 | the in-app feed serves its items to the owner's session and refuses any other caller with no item | `the_in_app_feed_answers_only_the_owner` |
| A14 | the notifications data-rights port lists its five tables as exported and erased, and an erase empties them | `the_notification_tables_are_exported_and_erased` |
| A15 | no delivery goes around the port in a shipped source of the kinds the census reads (the Rust, Python and web source files, the shell scripts by extension or by a `#!` first line, and the systemd units and their drop-ins): outside the bot's sources nothing names the Bot API's host, a send method or the bot's `DEFAULT_API_URL`, SPEC-031's alert path aside; inside them a send method is named only by its own named send; the bot's `send_html`, `edit_html` and command handler are used only at named sites; only the router's modules name the in-app feed or the held queue, and in the notifications crate only they name its ledger, the root's declaration of it aside; and each of the seven named sends is found once | `no_delivery_goes_around_the_port` |

```acceptance
A1: cargo test -p deck-streak-notifications --test policy -- --exact the_typed_policy_reads_every_key_of_the_file
A2: cargo test -p deck-streak-notifications --test one_router -- --exact a_delivery_call_outside_the_router_does_not_compile
A3: cargo test -p deck-streak-notifications --test policy -- --exact the_reading_ready_kind_is_a_recorded_deviation
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
A15: cargo test -p deck-streak-notifications --test one_router -- --exact no_delivery_goes_around_the_port
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree. They have no
line in the acceptance fence, because no public test can run a pack's row. When this delivery
merges, the maintainer's private wiring stops deferring notifications-policy's `one-router` row, so
that row, red, VOID or in error, fails the run like every other blocking row of the enforced pack;
the verdict of a run with that wiring is posted as the `box/packs` status at the merge. The
`message-metadata` row stays deferred on its own issue (#257).

| id | criterion | decided by |
|---|---|---|
| B1 | every policy row of notifications-policy passes over `notifications-policy.json`, examining its 9 declared kinds, `reading_ready` among them, and every section; and `one-router` passes over every shipped source file of the tree, test directories left out, with `crates/notifications/src/router.rs` defining `route` | notifications-policy, its 13 policy rows and `one-router` |
| B2 | every blocking row of privacy-gdpr passes over `privacy.json`, `PRIVACY.md`, the migrations and the data-rights ports, examining every table the migrations create, the five notifications tables among them | privacy-gdpr |
| B3 | every blocking row of telegram-platform passes over the bot's sources, examining `crates/bot/src/transport.rs`, which gains the bot transport's call | telegram-platform |

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
| `crates/notifications/src/data_rights.rs` | `deck-streak-notifications` | added: the data-rights port |
| `migrations/004101_notifications_router.sql` | `deck-streak-notifications` | added |
| `crates/notifications/tests/router.rs` | `deck-streak-notifications` | added |
| `crates/notifications/tests/deferral.rs` | `deck-streak-notifications` | added |
| `crates/notifications/tests/quiet_hours.rs` | `deck-streak-notifications` | added |
| `crates/notifications/tests/comeback_budget.rs` | `deck-streak-notifications` | added |
| `crates/notifications/tests/rights.rs` | `deck-streak-notifications` | added |
| `crates/notifications/tests/policy.rs` | `deck-streak-notifications` | added: A1, A3, and the policy's refusals |
| `crates/notifications/tests/one_router.rs` | `deck-streak-notifications` | added: A2, and A15's census of the shipped sources of the kinds it reads, which refuses a write to the Mini App's feed or to the held queue outside the router's modules |
| `crates/notifications/tests/ui/push_outside_the_router.rs`, `.stderr` | `deck-streak-notifications` | added: A2's compile-fail fixture and the refusal it records |
| `crates/notifications/tests/ui/pass_by_default.rs`, `.stderr`, `crates/notifications/tests/ui/pass_kept_by_a_clone.rs`, `.stderr` | `deck-streak-notifications` | added: A2's fixtures for a pass made by `Default` and one kept by cloning a borrowed pass, each with the refusal it records |
| `crates/notifications/tests/support/mod.rs` | `deck-streak-notifications` | added: the tests' database, clock and recording transport |
| `crates/bot/src/transport.rs` | `deck-streak-bot` | changed: implements the bot transport calls, and keeps Telegram's own base URL (`DEFAULT_API_URL`) private to the bot's crate (A15) |
| `crates/bot/src/lib.rs` | `deck-streak-bot` | changed: exports the owner's chat, the port's implementation |
| `crates/bot/tests/transport.rs` | `deck-streak-bot` | changed: the router's pushes reach the owner's chat through the bot's transport, and the unset base URL is read as Telegram's own, whose constant no other crate can read |
| `crates/api/src/notifications_routes.rs` | `deck-streak-api` | added: the feed route |
| `crates/api/src/router.rs`, `crates/api/src/lib.rs` | `deck-streak-api` | changed: mounts the feed route |
| `crates/api/src/session_routes.rs` | `deck-streak-api` | changed: the owner's access lends its clock to the feed route |
| `crates/api/tests/notifications_feed.rs` | `deck-streak-api` | added |
| `crates/coordination/src/sync_cycle.rs` | `deck-streak-coordination` | changed: the flush step after a successful sync |
| `crates/coordination/tests/flush_step.rs` | `deck-streak-coordination` | added: a successful sync flushes, a failed one does not |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: registers the notifications port (SPEC-021's rule) |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row in each notifications table (SPEC-021's rule) |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the bot transport joined to the router |
| `crates/daemon/src/role_bot.rs` | `deck-streak-daemon` | changed: reads the compiled policy at start, and the owner's sync flushes the router joined to the role's transport |
| `notifications-policy.json` | repo | changed: the `reading_ready` kind and its deviation |
| the box-run packs' private wiring (ADR-069) | the maintainer's | changed: `one-router` is no longer deferred |
| `tools/parity-oracle/registry/spec_041.py` | repo | added: registers `quiet_hours.py:in_quiet_hours` (SPEC-029's registry) |
| `tools/parity-oracle/goldens/in_quiet_hours.json` | repo | added |
| `docs/CONTEXT-MAP.md` | docs | changed: the ownership register's five notifications tables |
| `privacy.json` | repo | changed: the notifications categories |
| `PRIVACY.md` | repo | changed: one line per notifications category (SPEC-021's rule) |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `scripts/mutation-rows.d/S04100-S04199.json` | repo | added: the hand-proved rows of quiet hours, the caps and dedupe, and of A15's census |
| `changelog.d/feat-router-041.md` | repo | added |
| `docs/schematics/notification-router.md` | docs | changed: the design delivered, with a held celebration's states and the joins |
| `docs/specs/SPEC-041-notification-router-core.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-041-notification-router-core.md` | docs | changed: accepted, with the decisions made at delivery |
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
- It holds no delivery the census cannot read, because a text census reads names, not requests or
  statements: a request whose URL or method is assembled from parts, so that neither the Bot API's
  host nor a send method's name appears (outside the bot, on a base URL read through
  `ApiUrl::as_str`; inside the bot's sources, with a method's name built from pieces); a statement
  whose table's name is assembled from parts, so that neither the feed's nor the held queue's name
  appears; and a source of a kind the census does not read (#297).

## 6. Risks

- **A delivery call appears outside the router**, for example a bot command reply written with
  `push_message`. Detected by A2 (the compiler) for the port's call and by the box run's `one-router`
  row (§3a B1) for a call the policy names, and by A15's census for a delivery around the port in
  the sources it reads: the bot's own send, edit or command handler, a raw request to the Bot API,
  or a write to the Mini App's feed or to the held queue.
- **The lapse context is empty until the governor exists**, so a nudge could reach an owner in a
  real lapse. Detected by SPEC-049's lapse tests, which run over the minimal lapse-episode slice
  SPEC-049 builds in `streaks` ahead of W3's governor; the W1 kinds that nudge (`reading_ready`,
  `comeback`) declare that prerequisite.
- **The quiet window is read at the wrong offset.** Detected by
  `the_quiet_window_is_read_at_the_configured_offset`; the golden (A8) judges the window itself.
- **A background celebration reaches the bot while the Mini App is open** (the origin rule).
  Visible in the decision ledger's surface column; accepted by ADR-041.
- **The flush never runs** because no sync succeeds. The dead-man watch (SPEC-027) pages on stopped
  syncs, and the queue's age cap names every abandoned celebration.
- **The scheduled sync runs inside the default quiet window** (the rollover hour, minute 7, before
  07:30), so the scheduled cycle's flush does nothing, and a held celebration reaches the owner only
  through a later flush outside the window, such as the owner's `/sync`, or is abandoned by name at
  the age cap. Visible in the decision ledger's withholds; a flush that runs after the window ends is
  needed before a production path raises a celebration.

## 7. Amendments at delivery

- **The criteria over pack rows.** No public test can run a pack's row (ADR-069, SPEC-056), so A1 to
  A3 are tests of DeckStreak's own behaviour: the typed policy reads every key (A1), a call of the
  port's delivery call outside the router module does not compile (A2), and the typed policy reads
  `reading_ready` and its deviation (A3). The box run judges the rows themselves (§3a), and
  `scripts/tests/test_notifications_router_rows.py` is not added.
- **R1 and R13: the router's pass.** Each bot transport call takes a `Pass`, which only the router
  module can make, so a call of the port anywhere else is a compile error, not a finding (A2). The port carries
  `push_message` alone: the router renders nothing beyond a line, and the dice, reaction and pin calls
  arrive with the ladder's renders (§5). The bot implements it on its transport, sending to the
  owner's chat as HTML, chunked as every bot message is.
- **R2: the policy is compiled in.** The router's binary carries `notifications-policy.json` and parses
  it at start, so the policy it runs is the policy the box run judged. A refusal names the section,
  or `kinds.<kind>` for a kind, with serde's words for the field inside it. Start is also refused
  when the holdout names an undeclared kind, when a kind names an undeclared budget, and when the
  policy's withhold reasons lack one the router records.
- **R3: the payload, the key and a comeback's lapse.** The payload is the message's text, as the bot's
  HTML. The dedupe key is at most 128 bytes of the token grammar and holds no calendar date by the
  pack's pattern. A kind on the `comeback` budget is refused without an open lapse, since its cap is
  counted per lapse id.
- **R4: one transaction per decision.** Rule 2 claims the key by inserting the delivery, and a later
  withhold releases the claim, inside the one write that records the decision, so a key is claimed
  exactly when it is sent or held. A Mini App send appends its feed item in that write. A bot send
  commits its claim before the call and records the outcome after it.
- **R4, rule 4: a digest.** Inside quiet hours only the policy's exempt classes pass; a celebration is
  deferred, and a nudge or a digest is withheld with `quiet_hours`.
- **R4, rule 6: no transport answers.** It holds when no bot transport is joined, while the outage
  breaker is open (60 seconds after a failed bot send), and when a send of anything but a
  celebration fails; that send's claim is released, so a caller's retry can still deliver it. A
  celebration is deferred instead of withheld while the breaker is open, as the failed-send hold
  requires (R8).
- **The tier rendered.** A celebration that asks for T0 sends nothing; any other tier renders as a
  line (T2) until the ladder's renders (§5). A deferral or a withhold renders T0, and the flush's
  decision records the tier it rendered.
- **R6 and R7: an abandonment is a decision.** A celebration the flush or the queue's bound abandons
  is recorded as withheld: with `quiet_hours` when quiet hours held it, and with `no_notifier` when a
  failed send did.
- **R7: the bound is kept at deferral.** Past 20 held celebrations, the lowest-ranked (the lowest tier,
  then the newest) is abandoned when the next is deferred, so the queue never holds more than 20.
  Every abandoned celebration waits in the queue until a recap line names it: one that a failed recap
  could not name is named by the next.
- **R7: when the flush runs.** The sync cycle flushes after a sync that ran and succeeded. With no bot
  transport a flush does nothing, as the predecessor's did; it also does nothing inside quiet hours
  and while the breaker is open.
- **R9 and R4: the owner's settings.** `notification_settings` holds the owner's overrides by key:
  `quiet_start_min` and `quiet_end_min`, minutes of the day, the predecessor's names, override the
  policy's window, and a kind's switch is off at `"0"`, the policy's `comeback.disable_value`, which
  every switch shares. An absent key is the policy's default.
- **R12: before the database opens.** The feed route answers 503 until the API's database is open,
  as readiness does.
- **R13: the roles.** The bot role reads the compiled policy at start and joins its transport to the
  router its owner's `/sync` flushes. The job role's scheduled cycle carries no router yet: a flush
  without a bot transport would do nothing, and the first job that sends joins one (#39).
- **The port's name.** Notifications' data-rights port is `crates/notifications/src/data_rights.rs`,
  not `rights.rs`: `privacy.json`'s export and erase globs read that name (SPEC-021's rule).
- **The manifest.** SPEC-021's rule adds the ports' registry, the symmetry probe's seeds and
  `PRIVACY.md`. The feed route reads the time through the owner's access (`session_routes.rs`). The
  bot exports its side of the port (`lib.rs`), and its transport's tests prove the join, since only
  the router can make a push. The flush step has its own coordination test. A2 adds its test, its
  compile-fail fixture and the refusal it records; the router's tests share one support module; and
  the delivery adds its rows' band and its changelog fragment.
- **The fix round.** The first review planted three defects the tests let through; each now reads
  red.
  - A pass made by `Default`, or kept by cloning a borrowed one, would have compiled outside the
    router. A2 gains a compile-fail case for each, beside the one for the private field.
  - A delivery could go around the port and never take a pass: through the bot's own `send_html`,
    or by a raw request to the Bot API from the daemon or the API. A15's census reads every shipped
    source: the Rust, Python and web source files, the shell scripts and the systemd units, with
    test directories and test files left out, and in a Rust file its comments and `#[cfg(test)]`
    items too. Outside `crates/bot/` nothing may name `api.telegram.org` or one of the Bot API's
    send methods, in its own spelling (`sendMessage`) or a client's (`send_message`). SPEC-031's
    alert path, `deploy/scripts/alert-telegram.sh`, is the one exception, because it pages the
    owner that a unit failed, the daemon among them. A call of `Transport::send_html`, or of a Bot
    API send method in a client's spelling, was allowed only at the named call sites, each found
    exactly once: `OwnerChat`'s push, the bot's command replies (the erase prompt, every other reply
    and the export, #257), and the transport's own requests. That count saw calls alone, so it
    missed the transport's raw `sendDocument` request, which the second review found (below). The
    test also refuses a planted source of each kind.
  - The comeback's gap was tested only on a UTC rule at noon, where a calendar day and a study day
    agree. A9 gains a case on a rule five hours west of UTC, across the 04:00 rollover. The owner's
    quiet window is off in it (its start equal to its end), because 03:00 and 05:00 are inside the
    default window, where rule 4 withholds a nudge with `quiet_hours` before rule 5 counts the gap.
- **The second fix round.** The second review planted ten deliveries around the port that A15's
  census let through, and found a comeback A9 never exercised; each now reads red. A15's census
  reads every shipped source of the kinds it names, and each claim that only the router delivers
  (the pull request, ADR-041, the schematic and the module docs) says what its guard reads.
  - Inside a source: only a `#[cfg(test)]` module is left out, any attributes stacked on it passed
    over, so a field compiled for tests alone hides nothing after it; a directory under a `src/` is
    always read, whatever its name; every use of the bot's `send_html` is read, not only a call;
    inside the bot's sources a send method is named, in either spelling, only in the named send
    that makes its request; and the transport's raw `sendDocument` request is the seventh named
    send, which the census finds once.
  - Its reach: a script with no extension is read by its `#!` first line, and a systemd drop-in
    (`*.d/*.conf`) is read. The bot's `edit_html`, which no shipped source calls, has no named call
    site, and its command handler is used only by the bot's entry, the long poll. Outside the bot's
    sources nothing may name `DEFAULT_API_URL`, which is `pub(crate)`, so no other crate can build a
    request on it; the bot's own test of the unset base URL reads Telegram's URL as written.
  - The Mini App's feed: only the router's ledger, router and data-rights modules name its table,
    `in_app_feed` in any case, the ledger's `FEED_TABLE` or its `append_feed`.
  - The held queue, by the architect's ruling on the round: a write to `notification_queue` from
    outside the router's modules is a delivery around the router, because a flush delivers what the
    queue holds. The modules that own its writes are the same three: the ledger holds the SQL of
    each write (`hold`, `abandon`, `relatch`, `settle`), the router is their one caller, and the
    data-rights port exports and erases it. Only they name its table, `notification_queue` in any
    case, or the ledger's `QUEUE_TABLE`; and because the ledger's writes to the queue are private to
    the notifications crate, only they name the ledger in that crate's sources, the root's
    declaration of it aside.
  - `Transport::answer_callback` is not a delivery: it answers a callback query with no text, so the
    owner's client stops its progress indicator.
  - `Transport::set_chat_menu` is not a delivery: it registers the owner's command menu, which the
    owner opens.
  - A9 gains a case on the rule five hours west of UTC, with the owner's quiet window off: a
    comeback sent at 03:00 on the calendar day after day 0 is recorded on study day 0, before the
    rollover, so one at noon two calendar days later, on study day 3, is sent.
  - What the census still cannot read is named in §5 (#297).
