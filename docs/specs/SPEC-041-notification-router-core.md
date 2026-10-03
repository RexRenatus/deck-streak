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
| A15 | the census reads every shipped source of these kinds: the Rust, Python and web source files, the Mini App's HTML among them, the shell scripts by extension or by a `#!` first line, and the systemd units of every type and their drop-ins; it leaves out symlinks, test files, test directories outside a `src/`, and in a Rust file its comments and `#[cfg(test)]` modules. In what it reads, no name it holds appears outside its place: outside the bot's sources nothing names the Bot API's host, a send or delivery method of the pinned client's table, or the bot's `DEFAULT_API_URL`, SPEC-031's alert path aside; inside them such a method is named only by its own named send; the bot's `send_html`, `edit_html` and command handler are used only at named sites, and the handler's own replies and dispatch are called only by their named callers; only the router's modules name the in-app feed or the held queue, and in the notifications crate only they name its ledger, the root's declaration of it aside; no source of that crate carries `#[path]`, `#[macro_export]` or `#[macro_use]`, or re-exports the ledger, its feed's and queue's tables or its writes to the feed and the queue by a `pub use`; every method of the pinned client's table is a send, a delivery or not a delivery, in one class only; and each of the eight named sends is found once | `no_delivery_goes_around_the_port` |
| A16 | the census refuses a hand-built Bot API send URL that names the bot's `api_url`, wherever the bot crate's transport builds it: `api_url` is read only by `Transport::send_document` and `Transport::send_photo`, the two multipart sends the pinned client cannot make, and by the constructor `Transport::with_waits`, which composes it once with the token; every other read of it, in a `format!`, a `concat!`, a `String` push, a helper function, a `const`'s path, or a string literal's inline argument, is refused, over every Bot API method the transport names, every form, and every function of the transport outside the named sites | `a_hand_built_send_url_is_refused_wherever_the_transport_builds_it` |
| A17 | in the bot's sources, a request whose Bot API method the census cannot read is refused outside its named request site: every name through which one is made, the pinned client's generic requests `request`, `request_with_form_data` and `request_with_possible_form_data`, its HTTP client `client`, and the HTTP crate `reqwest`, is found only at its named site, as often as the shipped site names it; any other mention is refused at its line, and a second one at a site by the count, over eight request forms in every function of the bot's sources and at every place outside a function | `a_request_the_census_cannot_read_is_refused_wherever_the_bot_makes_it` |
| A18 | a use of one of four reqwest paths, the client type `reqwest::Client` and the calls `reqwest::get`, `reqwest::Client::new` and `reqwest::Client::builder`, is named by the compiler's resolved path, not by a text token: `clippy.toml` names these four, so a use of one under any name a source binds it to, by a `use`, a re-export or a `type` alias, is flagged by the workspace's clippy stage under `-D warnings`; the only suppression of either lint in the workspace is an `#[expect]` with a reason at the bot transport's client construction, and any other suppression of them or of a lint group that holds them (`clippy::style`, `clippy::all`, `warnings`), whether an item's or a crate's, through `cfg_attr`, in a manifest's `[lints]` table or as a compiler flag, and a `clippy.toml` that no longer names the four paths, is refused, over every spelling, form and place of a generated population | `clippy_names_the_four_paths_that_make_reqwests_client`, `the_only_suppression_of_the_rule_is_an_expect_at_a_named_transport_site`, `a_suppression_of_the_rule_or_of_its_group_is_refused_wherever_it_is_planted` |

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
A16: cargo test -p deck-streak-notifications --test one_router -- --exact a_hand_built_send_url_is_refused_wherever_the_transport_builds_it
A17: cargo test -p deck-streak-notifications --test one_router -- --exact a_request_the_census_cannot_read_is_refused_wherever_the_bot_makes_it
A18: cargo test -p deck-streak-notifications --test request_allow_list
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
| `crates/notifications/tests/one_router.rs` | `deck-streak-notifications` | added: A2, and A15's census, which reads the shipped sources of the kinds A15 names and refuses in them a delivery around the port by a name it holds; code written to evade it goes unread (§5, #297) |
| `crates/notifications/tests/request_allow_list.rs` | `deck-streak-notifications` | added: A18, which audits the allow-list of the compiler's request rule: `clippy.toml`'s four paths, the one `#[expect]` at the transport, and every other suppression of the two lints or of a group that holds them |
| `clippy.toml` | repo | changed: names reqwest's client type and its three constructors as disallowed, each with its reason (A18) |
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
- It holds no delivery the census does not read. The census guards ordinary code; it is not a
  sandbox against code written to evade it, which review catches (#297). A text census reads names,
  not requests, statements or what the compiler resolves, so these go unread (#297):
  - a request whose method's name is assembled from parts: outside the bot's sources on a base URL
    read through `ApiUrl::as_str`, and inside them beside the Bot API's host or on
    `DEFAULT_API_URL` (#297);
  - a statement whose table's name is assembled from parts, by `format!` or `concat!`, and a source
    that `include!` pulls in from a file of a kind the census does not read (#297);
  - a source of a kind the census does not read, such as SQL (#297);
  - a symlink, which the walker neither reads nor follows (#297);
  - a test file, or a file in a test directory outside a `src/`, that a shipped crate pulls in by
    `#[path]` (outside the notifications crate) or a unit runs (#297);
  - a re-export under another name from one of the router's modules other than by a `pub use`, and
    a `pub` wrapper: a function, a macro or a constant that hands out a write to the feed or the
    held queue, or a table's name, under a name the census does not hold, or a reply of the bot's
    command handler made `pub` and called from outside the handler's module (#297).
  - the bot's base URL read without the field's name: through an accessor or a `Debug` print of the
    pinned client's bot, and a name for it that a macro assembles, which no text census reads. Every
    form that names the field `api_url`, as an identifier or inside a string literal's inline
    argument, is read, and one it cannot read is refused rather than passed (#429).
  - a request written over a raw socket (tokio's `TcpStream`) to a plain-HTTP local Bot API server,
    which names none of the request names (#429);
  - two compensating edits that move a `reqwest` mention between places the census names alike, in
    no function, which the count reads as unchanged (#429);
  - a new HTTP dependency in another crate: `Cargo.lock` gives an HTTP client to the bot crate
    alone, so it needs a change of the lock, which review sees (#429).
- A use of one of the four reqwest paths in `clippy.toml` is named by the compiler's resolved path
  (A18, #429), so the forms above that bind one of them to a name of the source's own are closed,
  and what the rule still cannot read is each of these:
  - a request over a raw socket (tokio's `TcpStream`) or through an HTTP crate other than reqwest:
    the four paths in `clippy.toml` are reqwest's, and such a request names none of them (#429);
  - a request through the pinned client's generic requests, which name no reqwest path at the call:
    the text census's request names read them, as A17 states, and the compiler's rule does not (#429);
  - two compensating edits that move a `reqwest` mention between places the census names alike, in
    no function: the count still reads them as unchanged, but a client made at the new place names
    one of the four paths, which the compiler's rule flags (#429);
  - a new HTTP dependency in another crate: it needs a change of the lock, which review sees, and
    it names no reqwest path (#429).
  - a reqwest client value obtained without naming the type or one of its listed constructors: the
    four paths in `clippy.toml` flag a use that names one of them, and this form was not measured
    (#429).

## 6. Risks

- **A delivery call appears outside the router**, for example a bot command reply written with
  `push_message`. Detected by A2 (the compiler) for the port's call and by the box run's `one-router`
  row (§3a B1) for a call the policy names, and by A15's census for a delivery around the port by a
  name it holds, in the sources of the kinds it reads: the bot's own send, edit or command handler,
  or a call of the handler's replies beside its dispatch; a raw request to the Bot API, or a call of
  one of the send or delivery methods of the pinned client's table; or a write to the Mini App's feed or to the held queue.
  Code written to evade the census goes unread, and review catches it (§5, #297).
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
  Amendment (2026-09-29): the owner's `/sync` runs as the sync job (SPEC-059); the bot flushes this router when it observes that sync succeed, and the job holds no bot credential (ADR-066).
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
    or by a raw request to the Bot API from the daemon or the API. A15's census read the shipped
    sources of these kinds, by extension: the Rust, Python and web source files, the shell scripts
    and the systemd services and timers, with test directories and test files left out, and in a
    Rust file its comments and `#[cfg(test)]` items too. Outside `crates/bot/` nothing may name `api.telegram.org` or one of the Bot API's
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
  reads more kinds and more names, and each claim that only the router delivers (the pull request,
  ADR-041, the schematic and the module docs) says what its guard reads.
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
- **The third fix round.** The third review planted more deliveries around the port, and the
  architect ruled that the census guards ordinary code and is not a sandbox against code written to
  evade it. The ordinary kinds and names it missed are now read, each red first with a row; the ways
  to evade it are named in §5, not planted further; and every claim says only what the census
  reads.
  - Its kinds: a systemd unit of every type (`systemd.unit(5)`), whatever its name, so a socket, a
    path, a mount and a unit named as a test file are read; the Mini App's HTML, whose inline
    scripts run in the owner's browser; a CommonJS TypeScript module (`.cts`); and a script by its
    `.bash` or `.zsh` extension.
  - A send or delivery method of the pinned client's table is held. Outside the bot's sources
    nothing names one; inside them one is named only by its own named send, and the transport's
    `edit_html` is `editMessageText`'s, one of ten named send, which the census finds once although
    no shipped source calls it.
  - A method is held when it can make content the bot chose visible to a user: text, media, a title,
    a button, a status or a badge's description. The transport's answer to a callback and its menu of
    commands are named sends. Reads, removals, permissions and membership, tokens, webhooks, bare file uploads
    and the account's own gift state (`upgradeGift` and its kin) are classified, not held. The
    census holds 29 send methods, 65 delivery methods and 91 that are classified, not held, 185 in
    all.
  - The pinned client's whole table is listed in the census with the client's version, which a
    bump of the lock turns red until the list is derived again, and every method in it is a send,
    a delivery or not a delivery, in one class only. The 91 that are not held are the reads (32),
    deletions and unpins (11), bot and session configuration (9), a sticker's emoji, keywords, mask,
    position and bare upload and a set's removal (7), chat administration without user-visible
    text (22), and business, star and gift account state (10). None carries content the bot chose
    that a user sees, so each is classified, not held (#297).
  - The notifications crate: no source of it carries `#[path]`, `#[macro_export]` or
    `#[macro_use]`, since each hands the ledger's writes to code the census reads under another
    name; and no `pub` or `pub(...)` `use` in it re-exports the ledger, the constants of its feed's and queue's
    tables or its writes to the feed and the queue, under any name.
  - The command handler: its replies (`send`, and `export`, `ask_erase`, `sync` and `score`, which send one)
    and its dispatch (`on_message`, `on_callback`) are called only by their named callers, the
    handler and the dispatch, each of which the census finds in the tree.
- **A16: the transport's base URL (#429).** `api_url` joins the census's guarded names, beside
  `send_html`, `edit_html` and `handle`, defined in `crates/bot/src/transport.rs`: every use of it is
  a named site or a refusal, so a request URL built from it anywhere else is refused whatever builds
  the URL. The named sites are the two sends that make a multipart request by hand and the
  constructor that composes the base URL with the token once. The constructor is a third site than
  the issue named: the census cannot read the builder's argument without it, and it makes no
  request. A read of the name inside a string literal, which the census blanks, is refused at any
  other place, so the inline form `{api_url}` is not a way round. The three planted requests of
  A15 that hold `api_url` (`celebrate.rs`, `copy.rs` and `rich.rs`) gain the refusals this rule
  adds, so their expected lines grew by two each; no line was removed. The population is read from
  the transport: the methods it names, six forms and every function outside the named sites.
- **A17: a request the census cannot read (#429).** The census reads a typed call of the pinned client
  by its method's name. Every other request carries its method as a string or a URL, which no identifier
  search reads, and the Bot API takes any case of a method's name. So the bot's sources are held to one
  rule: every name through which such a request is made (the client's generic requests, its HTTP client
  and the HTTP crate) is found only at its named request site, as often as the shipped site names it
  (`REQUEST_SITES`, 21 mentions), and any other mention is refused at its line, a second one at a site by
  the count. The named sites are the update poll's generic request, the constructor that builds the
  client, the two multipart sends and their forms, and the transport's use of the crate in no function.
  The three planted requests of A15 (`celebrate.rs`, `copy.rs` and `rich.rs`) gain one refusal each; no
  line was removed. Each named site's read of `api_url` is counted once by A15's test. The population is
  eight request forms in every function of the eight bot sources and at three places outside a function.
- **A18: request capability by the compiler's resolved path (#429).** The text census reads a name, so
  reqwest's client bound to a name of the source's own, by an alias, by its own type name or by a
  `type` or `pub use` binding, added no mention it counts and passed. Clippy resolves the path of every
  use through such a binding, so `clippy.toml` now names `reqwest::Client` as a disallowed type and
  `reqwest::get`, `reqwest::Client::new` and `reqwest::Client::builder` as disallowed methods, each with
  its reason, and the workspace's clippy stage, which runs every target under `-D warnings`, refuses a
  use of one. The bot transport's construction of its client is the one legitimate site, and it carries
  an `#[expect]` for each lint with a reason, so a site that stops needing it is itself a warning. Those
  annotations are the audited allow-list: `request_allow_list.rs` refuses any other suppression of the
  two lints, any suppression of a lint group that holds them in every spelling a crate accepts, and a
  `clippy.toml` that no longer names the four paths. It reads files as text, which is right for an
  audit of the allow-list, and clippy does the resolving. The text census of A15 to A17 stays: it reads
  the Bot API's method names, and its claim narrows to the forms that name a token. The population is
  1,920 planted suppressions: eight spellings of a lint or of a group that holds it, twelve attribute forms
  (an `allow` and an `expect`, with and without a reason, beside another lint, over lines, in a
  `cfg_attr`, and as a crate's inner attribute) at up to three places (before an item, before a
  statement, and at the top of a file), in each of the bot's eight sources. The test file
  changed once between the red and the green commit, in the style of the workspace's lints (a range,
  a `let` chain, a file-extension helper and an `expect` allowance for a test crate), with no
  assertion touched.

## 8. Amendment: the held queue is flushed once the quiet window ends (#291)

R7 flushes the held queue after every successful sync. The scheduled sync runs inside the default
quiet window, so on that path the flush finds the window closed and delivers nothing, and a
celebration held overnight waits for a sync that happens to land after the window. This amendment
adds a flush of its own, outside the window, and states the class it closes: **every held
notification reaches the owner at most once, and ends delivered or abandoned by name, on every path that can flush.**

- **R14: a scheduled flush step, outside the window.** The job table gains `held_flush`, a daily job
  at 07:36 UTC with catch-up, its own timer and its own job run, never a step of a readings job
  (#39). The window the router reads ends at 07:30, and the step fires after it. A missed fire is
  replayed by the timer up to 360 minutes late (13:36), which is still outside the window. A hold
  raised at the window's start (23:00) is 516 minutes old at the 07:36 fire and 876 minutes old at the
  latest replay, against the 720-minute limit: a fire replayed after 11:00 abandons that hold by name
  in the recap line instead of delivering it, so a missed fire can cost a hold its delivery and
  never its name. ADR-300 records the choice, and why the catch-up bound is not shortened.
- **R15: the window is read when the flush sends.** A flush inside the window delivers nothing and
  answers `QuietHours`; a flush after it delivers each hold it finds and abandons by name a hold
  past its age limit, in the recap line R7 already prints. The three flushers are the flush after a
  scheduled sync, the scheduled step, and the flush after an owner-triggered sync.
- **R16: one flush at a time.** A flush takes a lease before its first send, held as the setting
  `flush_lease` whose value is the instant it lapses, ten minutes after it was taken. A flush that
  finds an unlapsed lease answers `Busy` and sends nothing; a flush releases its own lease when it
  ends, by its token, so a flush that outlived its lease never releases the lease of the flush that
  took over, and a lease whose flush died lapses on its own. The lease is the queue's coarse
  exclusion; it is not what keeps one item from being sent twice, because it lapses after ten
  minutes while its holder may still be sending.
- **R16a: a held item is claimed, sent and settled, at most once.** In its first transaction the
  flush moves each held row it will send from `held` to `sending`, writing its lease token in the
  row's `claim`, by one update that matches only `held` rows; a row another flush claimed is never
  read for sending. A push is made only for a row the flush claimed, and the settle that removes the
  row, the relatch after a failed send and the abandonment each match that token. A flush that ends
  gives back the rows it claimed and did not reach. A flush that finds a `sending` row whose claim has
  lapsed (a claim is the instant its lease lapses) abandons it by name, "may have been sent", and
  never pushes it, whether its claimant died after the push reached the owner or is still sending
  past its lease. An abandonment reaches a log line naming the item, its claimant and why (expired,
  over the queue bound, send failed, or may have been sent), and the recap line names it. Every held
  item therefore ends delivered once, or abandoned by name; one whose fate is unknown is named, not
  resent.
- **R16b: a row the flush pushed is never given back.** A row joins the flush's pushed set when its
  push answers delivered, before the write that settles it: a full render's row when its render
  answers delivered, a held reaction's row when its reaction answers delivered, and every row the
  recap line rolls up when the recap's push answers delivered. When the flush ends, on success or on
  error, its release first abandons each row of the pushed set that it still claims, with its claim
  kept, and only then gives back to `held` every other row it still claims. So when the write after
  a delivered push fails (the settle, the decision record or its commit), the flush ends with that
  error, and the row it pushed is never given back to `held` and never pushed again: a log line
  names the item, its claimant and "may have been sent", and the next recap names it "may have been
  sent". A row the failed flush never pushed is given back untouched and reaches the owner at a
  later flush. The release is one write with the lease's release; when that write fails too, the
  rows stay claimed, and R16a abandons them by name once their claim lapses.
- **R17: the job process has a router.** Only the bot's process held a router, so the scheduled job's
  process built none. The `held_flush` job builds its own from the policy, the owner's chat and the
  bot's credentials, through a service drop-in that loads the same two credentials the bot loads. The
  job answers done for a flush that ran, one inside the window and one that found the lease taken;
  it answers not delivered while the outage breaker is open, and fails by name with `no_notifier`
  when no bot is joined or `flush_failed` when the flush errs.
- **The surface.** The flush sends through the bot's transport and records its decisions in the
  decision ledger the Mini App feed reads, so both surfaces see the same decisions.
- **What this does NOT do.**
  - It is not a readings step: the readings jobs are #39's, and the flush is its own job.
  - It does not re-check the window at each send of one flush: a flush that starts just before the
    window opens delivers its held items under the check made at its start (#291).
  - It adds no table. It adds one column, `claim`, and the state `sending`, to `notification_queue`
    (migration 004102, a rebuild of the table that re-creates every index and trigger the old table
    had, of which there were none), and the lease stays a settings row that the data-rights export
    lists while it is held (#291).
  - It does not resend a row whose claimant is gone: that row is abandoned by name, "may have been
    sent", so a send that reached the owner and was never settled is never doubled (#291).

File manifest of the amendment:

| path | change |
|---|---|
| `crates/notifications/src/router.rs` | the lease, the `Busy` answer, the flush's split into the lease and the delivery, and the claim, token-matched settle and named abandonment |
| `crates/notifications/src/ledger.rs` | the claim of held rows, the token-matched settle, relatch and abandon, and the lapsed-claim abandonment |
| `migrations/004102_notifications_queue_claim.sql` | the `sending` state and the `claim` column (a STRICT rebuild) |
| `crates/notifications/src/data_rights.rs` | the export and erasure of the `claim` column |
| `crates/coordination/src/held_flush.rs` | the job's work and its answers |
| `crates/coordination/src/jobs.rs` | the `held_flush` entry of the job table |
| `crates/daemon/src/role_job.rs` | the job process builds its router and runs `held_flush` |
| `deploy/systemd/deck-streak-job@held_flush.timer` | the timer, at 07:36 UTC with catch-up |
| `deploy/systemd/deck-streak-job@held_flush.service.d/20-bot-credentials.conf` | the bot's two credentials for the job |
| `deploy/README.md`, `deploy/rail-contract.json` | the schedule and credential rows, and the rail's calendar |
| `crates/coordination/tests/held_flush.rs`, `held_flush_calendar.rs`, `held_flush_answers.rs` | A19, A20 and A21 |
| `crates/notifications/tests/flush_lease.rs`, `flush_outlives_lease.rs` | A20's lease cases, and the cases of a flush that outlives its lease or dies after its push |
| `crates/notifications/tests/flush_fails_after_push.rs` | A23: a flush whose write after a delivered push fails |
| `formal/tla/HeldFlush/` | the model of two flushers over one queue |

## 9. Acceptance criteria of the held-flush amendment

| # | criterion | test |
|---|---|---|
| A19 | over every flusher (the flush after a scheduled sync, the scheduled step, the flush after an owner-triggered sync), every clock position of the window read from the policy (before it, at its start, inside, at its end, after it, across midnight) and every hold state (fresh, at the age limit, past it, already delivered), the delivered set and the abandoned set are exactly the expected ones, and a flush inside the window delivers nothing | `every_flusher_delivers_once_or_abandons_by_name_and_only_while_the_window_is_open` |
| A20 | a flush that finds an unlapsed lease answers `Busy` and sends nothing, a lease that lapsed this instant is taken over, the lease holds ten minutes and is released when the flush ends | `two_flushers_over_one_queue_never_send_one_item_twice`, `a_flush_finds_an_unlapsed_lease_and_sends_nothing`, `a_lease_that_lapsed_this_instant_is_taken_over`, `the_lease_holds_for_ten_minutes_and_is_released_when_the_flush_ends` |
| A22 | a held item reaches the owner at most once: a flush that outlives its lease is never doubled by a second flush, a flush that dies after its push reached is never resent and is named "may have been sent", a row another flush claimed is never sent, a late settle by a flush that lost its claim removes nothing, every abandonment reaches a log line naming the item and its claimant, and the claim migration keeps every index and trigger | `a_flush_that_outlives_its_lease_is_never_doubled_by_a_second_flush`, `a_flush_that_dies_after_its_send_reached_is_never_resent`, `a_row_another_flush_claimed_inside_its_claim_is_never_sent`, `a_late_settle_by_a_flush_that_lost_its_claim_removes_nothing`, `a_lapsed_claim_is_abandoned_by_name_in_the_log`, `a_failed_send_that_spent_its_retries_is_abandoned_by_name_in_the_log`, `the_claim_migration_keeps_every_index_and_trigger_the_queue_had` |
| A21 | every flush step of the job table and of the deploy templates fires outside the quiet window read from the policy, a slot inside it is told from one outside it, and the job answers done, not delivered or a named refusal for each answer of the flush | `every_flush_step_of_the_job_table_fires_outside_the_quiet_window`, `every_flush_step_of_the_deploy_templates_fires_outside_the_quiet_window`, `the_check_tells_a_slot_inside_the_window_from_one_outside_it`, `a_flush_that_ran_and_one_inside_the_window_are_done`, `a_flush_whose_lease_is_held_elsewhere_is_done`, `an_open_breaker_is_a_send_attempted_and_nothing_delivered`, `a_router_with_no_bot_is_a_named_refusal` |
| A23 | a flush whose write after a delivered push fails never pushes that row again and names it "may have been sent", for a full render, a row the recap line rolls up and a held reaction alike, and a row the failed flush never pushed is given back and reaches the owner at a later flush; a log line names the pushed item, its claimant and "may have been sent" | `a_settle_that_fails_after_a_full_render_is_not_followed_by_a_second_push`, `a_settle_that_fails_after_a_recap_is_not_followed_by_a_second_delivery`, `a_settle_that_fails_after_a_reaction_is_not_followed_by_a_second_reaction`, `a_row_the_failed_flush_never_pushed_reaches_the_owner_at_a_later_flush`, `a_settle_that_fails_after_a_push_names_the_item_and_its_claimant_in_the_log` |

```acceptance
A19: cargo test -p deck-streak-coordination --test held_flush -- --exact every_flusher_delivers_once_or_abandons_by_name_and_only_while_the_window_is_open
A20: cargo test -p deck-streak-coordination --test held_flush -- --exact two_flushers_over_one_queue_never_send_one_item_twice
A20: cargo test -p deck-streak-notifications --test flush_lease
A22: cargo test -p deck-streak-notifications --test flush_outlives_lease
A21: cargo test -p deck-streak-coordination --test held_flush_calendar
A21: cargo test -p deck-streak-coordination --test held_flush_answers
A23: cargo test -p deck-streak-notifications --test flush_fails_after_push
```

## 10. Amendment: every recompute cycle holds a router that holds for the senders (#571)

SPEC-319 and ADR-319 attach a router to every recompute cycle of the job role. Two statements
above are false after it, and each now reads as follows:

- **R13's roles note** ("The job role's scheduled cycle carries no router yet: a flush without a
  bot transport would do nothing, and the first job that sends joins one (#39).") now reads: Every
  recompute cycle of the job role, scheduled or the owner's, holds a router with no bot transport
  that holds each celebration it routes for the senders (SPEC-319, ADR-319); the job role still
  loads no bot credential, and the flush after its sync does nothing.
- R15's "the flush after a scheduled sync" is that no-op; the senders are the bot's flush after the
  owner's request is answered and the scheduled step (R14).

## 11. Amendment: the census reads what the compiler pulls in and refuses what it cannot follow (#297)

§5 lists, at lines 229-241, what the A15 census leaves unread. Six of its units name a file the
compiler, the migrator or a unit pulls in by a literal the census can read, or a reply whose
visibility the compiler already settles, and this amendment closes each of them in the census
(`crates/notifications/tests/one_router.rs`), each by its criterion in §12. ADR-324 records the
decisions and what each was chosen against. No production code changes.

- **Closed: a source that `include!` pulls in (§5 :232-233), A24.** In every Rust source the census
  reads, each `include!`, `include_str!` or `include_bytes!` outside comments and `#[cfg(test)]`
  modules is followed: its argument must be one string literal, resolved against the including
  file's directory, and the file is read under its own path with every rule of that path, as Rust
  from `include!` and as text from the other two, transitively and each file once. An argument that
  is not one literal, and a file not in the tree, are refused (#297).
- **Closed: SQL among the kinds the census does not read (§5 :234), A25.** `sql` joins the shipped
  kinds, so a migration, which runs at every start, is read with every rule; the router's two
  migrations, `migrations/004101_notifications_router.sql` and
  `migrations/004102_notifications_queue_claim.sql`, join the places that may name the feed and the
  held queue (#297).
- **Closed: a symlink (§5 :235), A26.** The walker refuses each symlink it meets in a place it walks,
  by its path (#297).
- **Closed: a test file a shipped crate pulls in by `#[path]` outside the notifications crate (§5
  :236-237), A27.** A module's `#[path = "…"]` outside a `#[cfg(test)]` module is followed, resolved
  against the declaring file's directory, and its target is read as Rust under its own path,
  whatever its name or directory; a target not in the tree, and a `#[path]` inside an inline module,
  are refused. Inside the notifications crate a `#[path]` stays refused, as A15 holds (#297).
- **Closed: a test file a unit runs (§5 :236-237), A28.** In every unit and drop-in the census reads,
  a path through a test directory with no `src` before it, or to a file named as a test file, is
  refused (#297).
- **Closed: a reply of the bot's command handler made `pub` (§5 :240-241), A29.** In the handler's
  module, a reply of `COMMAND_REPLIES` defined `pub` or `pub(...)`, or inside a trait impl for the
  handler, is refused. With every reply private, a call from outside the module does not compile,
  and a call inside it is held by A15's named callers (#297).
- **Kept, and named.** These stay unread, with the reason a text census cannot read them:
  - a request's method or a statement's table named from parts: the name exists only after the
    compiler expands a `concat!` or the program runs a `format!` or a push, so reading it needs
    macro expansion or execution, and A16 to A18 hold the bot's ordinary request forms by their
    names and by the compiler's resolved paths instead (#297);
  - a source of a kind the census does not read other than SQL: such a file runs nothing, and what
    a source loads from it at run time becomes a name only when the program runs, which is the unit
    above; reading every kind would red on the prose under `docs/`, which names `sendMessage`
    throughout (#297);
  - a re-export under another name other than by a `pub use`: a `const`, a `static`, a function
    pointer, a closure or a type alias that binds a router module's item to a new name is resolved
    by the compiler, and a text match on its initializer is defeated by one `use … as` inside a
    block (#297);
  - a `pub` wrapper, a function, a macro or a constant, that hands out a write to the feed or the
    held queue, or a table's name: telling one from the router's own public API needs a call graph;
    the queue's writes are `pub(crate)`, so only a wrapper inside the notifications crate can hand
    one out, and there the census holds that only the router's modules name the ledger (#297);
  - a test file that a shipped script runs, and a path that a unit assembles from a specifier or an
    environment variable: the script and the unit are read, and the file they reach is not (#297).
- **The census's own account.** Its module doc says what it reads and what stays unread, and A15
  prints, beside the shipped sources it examined, the files it brought in, the unit path tokens it
  read and the reply definitions it found (#297).
- **§3a's box run.** §3a's statement that the `message-metadata` row stays deferred on its own issue
  (#257) is superseded by SPEC-323, #257's delivery, which decides that row.

File manifest of the amendment:

| path | change |
|---|---|
| `crates/notifications/tests/one_router.rs` | the walk's symlink refusal, the included and `#[path]` files brought in, SQL and the router's migrations, the unit test-path rule, the reply visibility rule, A24 to A29, A15's new examined lines and the module doc |
| `docs/decisions/ADR-324-the-one-router-census-follows-what-the-compiler-pulls-in.md` | the decisions and what each was chosen against |
| `docs/decisions/ADR-041-notification-router-core.md` | an amendment to its last Bad bullet |
| `docs/schematics/notification-router.md` | the census's reach |
| `docs/red-first/SPEC-041.md` | A24 to A29's red and green lines |
| `scripts/mutation-rows.d/S04100-S04199.json` | rows S04182 to S04194 |
| `changelog.d/census-reach-297.md` | the changelog fragment |

## 12. Acceptance criteria of the census's reach (#297)

Each planted case asserts the exact refusal line, never only that something was refused, and A15
(`no_delivery_goes_around_the_port`) stays the committed tree's arm.

| id | criterion | decided by |
|---|---|---|
| A24 | an included file is read under its own path with every rule of that path, as Rust from `include!` and as text from `include_str!` and `include_bytes!`, and an include the census cannot name or find is refused; planted: a send in a file `include!` brings, the Bot API host in a file `include_str!` brings, a held table's name in a file `include_bytes!` brings, `include!(concat!(env!("OUT_DIR"), "/x.rs"))`, a missing file; control: an include inside a `#[cfg(test)]` module is not followed | `an_included_file_is_read_and_one_the_census_cannot_follow_is_refused` |
| A25 | SQL is read, and outside the router's two migrations no `.sql` names the feed or the held queue; planted: a migration inserting into `notification_queue`, one inserting into `in_app_feed`; control: the router's migrations | `a_migration_outside_the_routers_names_neither_the_feed_nor_the_queue` |
| A26 | a symlink in a place the walker walks is refused by its path; control: a symlink in a place the walker skips is not met | `a_symlink_in_a_walked_place_is_refused` |
| A27 | a module `#[path]` brings in is read as Rust under its own path, and a `#[path]` to a file not in the tree or inside an inline module is refused; planted: a send in a test file brought in by `#[path]`, a missing target, one inside `mod outer { … }`; control: one on a `#[cfg(test)]` module | `a_module_brought_in_by_path_is_read_and_one_the_census_cannot_follow_is_refused` |
| A28 | a unit or drop-in naming a path through a test directory, or a test file by name, is refused; planted: `ExecStart=/usr/local/lib/deck-streak/current/agent/tests/run.sh`, `ExecStartPost=/usr/bin/python3 /usr/local/lib/deck-streak/current/deploy/scripts/test_page.py` in a drop-in; control: `deploy/scripts/backup.py` | `a_unit_that_runs_a_test_file_is_refused` |
| A29 | a reply of the command handler defined `pub` or `pub(crate)`, or inside a trait impl for the handler, is refused; control: the handler's private inherent definitions | `a_command_reply_made_visible_outside_its_module_is_refused` |

```acceptance
A24: cargo test -p deck-streak-notifications --test one_router -- --exact an_included_file_is_read_and_one_the_census_cannot_follow_is_refused
A25: cargo test -p deck-streak-notifications --test one_router -- --exact a_migration_outside_the_routers_names_neither_the_feed_nor_the_queue
A26: cargo test -p deck-streak-notifications --test one_router -- --exact a_symlink_in_a_walked_place_is_refused
A27: cargo test -p deck-streak-notifications --test one_router -- --exact a_module_brought_in_by_path_is_read_and_one_the_census_cannot_follow_is_refused
A28: cargo test -p deck-streak-notifications --test one_router -- --exact a_unit_that_runs_a_test_file_is_refused
A29: cargo test -p deck-streak-notifications --test one_router -- --exact a_command_reply_made_visible_outside_its_module_is_refused
```
