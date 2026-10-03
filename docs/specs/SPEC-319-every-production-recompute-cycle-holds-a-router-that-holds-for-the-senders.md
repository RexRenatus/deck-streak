# SPEC-319: every production recompute cycle holds a router that holds for the senders

- **Wave:** W4. **Issue:** #571 (found by the design pass for #127). **Context(s):**
  `deck-streak-notifications` (the holding router and the celebrations' switch seed);
  `deck-streak-daemon` (the composition root hands the router to every recompute cycle); `formal`
  (`formal/tla/HeldFlush`).
- **Decided by:** ADR-319 (a router with no bot transport that holds each celebration for the
  bot's senders). Amends SPEC-041 (section 10), ADR-066 and ADR-109, each insert-only.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-319.md`. **Mutation band:** `S31900-S31999`.
- Every line number below was read at `dev` `6773f6e`.

## 1. The problem, measured

No production recompute cycle holds a notification router, so no celebration the awards phase
offers is ever raised: a badge, a personal record, the level-up line and a due relight are never
handed to the router, and a later landmark celebration (#127) would be inert the same way.

- `crates/coordination/src/sync_cycle.rs:121` builds `CycleParts` with `router: None`; only
  `CycleParts::with_flush` (`:130`) sets one. The awards' offers (`:297-303`), the level-up line
  and the due relights (`:321-333`) run only when a router is set.
- Both production cycles are built by one seam, `RecomputeSetup::cycle`
  (`crates/daemon/src/wiring.rs:327-341`): the scheduled sync (`crates/daemon/src/role_job.rs:270`)
  and the owner's sync (`crates/daemon/src/wiring.rs:437`). Neither attaches a router.
  `git grep -n '\.with_flush(' -- crates | grep /src/` finds one production call, the bot's sync
  requester (`crates/daemon/src/role_bot.rs:114`), which is a different type.
- A router with no bot transport withholds a bot occasion `no_notifier`
  (`crates/notifications/src/router.rs:836`), and the award's port treats any answer as the mark
  (`crates/coordination/src/recompute/mod.rs:281-283`, ADR-303). So a plain transport-less router
  attached to the cycle would mark every owed award withheld and lose it by name.
- The job process loads no bot credential (ADR-066); the senders that hold the bot's credentials
  are the bot's flush after it observes the owner's request answered
  (`crates/daemon/src/sync_request.rs:219-223`) and the `held_flush` job at 07:36
  (SPEC-041 R14).
- No migration or crate at `6773f6e` writes `celebrations_enabled`
  (`git grep -c celebrations_enabled -- migrations crates` counts one test file only), so this
  delivery is the first to let a celebration leave the box, and the owner's open decision #402
  (items 8 and 11) asks it to seed its own switch off with an insert that ignores an existing row.

## 2. Requirements

R1. `RecomputeSetup::cycle` attaches a router to every cycle it hands out, through the existing
    `CycleParts::with_flush`, so the scheduled sync's cycle and the owner's sync's cycle each offer
    every owed award, the level-up line and the due relights to it (SPEC-073 R4, R11; ADR-303).
    Neither caller is edited.
R2. That router is a holding router: `Router::new(..).holding()` with no bot transport. Outside the
    quiet window a bot celebration it routes is decided `deferred` with the hold `send` and held on
    `notification_queue` with its claim kept, as the breaker-open arm holds one; a bot occasion of
    any other class is still withheld `no_notifier` and its key released. Inside the window a
    celebration is deferred `quiet`, as before. A router built without `holding()` is unchanged.
R3. The job process still loads no bot credential and sends nothing: the holding router has no
    transport, so the flush after the cycle's own sync answers `Flushed::NoNotifier`. The senders
    of what it holds are the bot's flush after it observes the owner's request answered by a sync
    that ran and succeeded (SPEC-041 R7, R13), and the `held_flush` job (SPEC-041 R14), which is
    the backstop when no answer is observed.
R4. `RecomputeSetup::load` compiles the notification policy, refusing start with
    `RecomputeError::Policy` when it does not parse, and seeds the celebrations' switch, the
    setting the policy's `celebration` kind names (`celebrations_enabled`), to the policy's
    disable value `"0"` with an insert that ignores an existing row (#402 items 8 and 11). A value
    already stored, the owner's or the cutover checklist's, is never overwritten. A seed that
    cannot be written refuses the load with `RecomputeError::Switch`. With the switch off, the
    router withholds every celebration `nudges_disabled` before the claim, and the award is
    marked (ADR-303).
R5. `formal/tla/HeldFlush` models a celebration held outside the quiet window by the owner's sync
    before the bot's flush that follows its answer, and that flush skipped when the answer bound
    expires; with a scheduled flush after the last sync trigger, every held item still reaches the
    owner at most once and ends delivered or abandoned by name.
R6. No new `Cargo.toml` edge, no migration, no change to `deploy/**` or to
    `notifications-policy.json`.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | through `RecomputeSetup::load`, `RecomputeSetup::cycle` and `sync_cycle(.., Trigger::Scheduled)` over an empty collection copy, a catalog badge awarded unmarked before the cycle gets one decision row under `badge:<key>:<tier>`, and its `celebrated_at` is set | `cargo test -p deck-streak-daemon --test recompute_router -- --exact the_production_recompute_offers_an_owed_award_to_the_router` |
| A2 | with `celebrations_enabled` stored `"1"` before the load, A1's badge is decided `deferred` and one `notification_queue` row is `held` for its key, at any wall-clock time | `cargo test -p deck-streak-daemon --test recompute_router -- --exact a_switched_on_celebration_is_held_for_the_senders` |
| A3 | a bot-joined router (a recording transport; a manual clock at the first instant outside the window at or after the row's hold time, so the row is within the 720-minute age) over the same database flushes A2's row: exactly one push names it, and a second flush pushes nothing | `cargo test -p deck-streak-daemon --test recompute_router -- --exact the_bots_flush_delivers_what_the_recompute_held` |
| A4 | after `RecomputeSetup::load` on a fresh database `celebrations_enabled` reads `"0"`; a stored `"1"` stays `"1"`; with no stored value A1's badge is decided `withheld` `nudges_disabled` and marked | `cargo test -p deck-streak-daemon --test recompute_router -- --exact a_fresh_start_seeds_the_celebrations_switch_off` |
| A5 | outside the window `Router::new(..).holding()` defers a celebration `send` and holds it; it withholds a nudge `no_notifier` and releases its key; a plain `Router::new` still withholds the celebration `no_notifier` (the test `a_bot_occasion_with_no_transport_is_withheld_and_its_key_released` is unchanged) | `cargo test -p deck-streak-notifications --test router -- --exact a_holding_router_holds_a_celebration_it_cannot_send` |

```acceptance
A1: cargo test -p deck-streak-daemon --test recompute_router -- --exact the_production_recompute_offers_an_owed_award_to_the_router
A2: cargo test -p deck-streak-daemon --test recompute_router -- --exact a_switched_on_celebration_is_held_for_the_senders
A3: cargo test -p deck-streak-daemon --test recompute_router -- --exact the_bots_flush_delivers_what_the_recompute_held
A4: cargo test -p deck-streak-daemon --test recompute_router -- --exact a_fresh_start_seeds_the_celebrations_switch_off
A5: cargo test -p deck-streak-notifications --test router -- --exact a_holding_router_holds_a_celebration_it_cannot_send
```

## 3b. Decided by the formal check

The formal criterion is judged by the repository's formal checker, not by a test, so it is stated
here and kept out of the fence above, which names tests only.

| id | criterion | decided by |
|---|---|---|
| A6 | `formal/tla/HeldFlush`: every property clean at `MCHeldFlush.cfg` and at `MCHeldSendHold.cfg`, every witness caught (`witness/a-hold-outside-the-window-with-no-later-flush.cfg` too), no `STALE`; the unnarrowed ratchet has no `FAIL` | the formal check, `--entry tla/HeldFlush`, and the unnarrowed `--ratchet-only` |

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-319-every-production-recompute-cycle-holds-a-router-that-holds-for-the-senders.md` | docs | added |
| `docs/decisions/ADR-319-the-recompute-holds-each-celebration-for-the-bots-senders-through-a-router-with-no-transport.md` | docs | added |
| `docs/schematics/the-recompute-holds-a-router-for-the-senders.md` | docs | added: the sequence of the hold and its senders |
| `docs/specs/SPEC-041-notification-router-core.md` | docs | changed: a new last section 10, insert-only |
| `docs/decisions/ADR-066-the-owners-sync-is-requested-with-a-stored-flag-and-a-path-unit-doorbell.md` | docs | changed: an appended amendment note |
| `docs/decisions/ADR-109-the-widget-is-one-silent-pinned-message-a-study-day-edited-in-place-through-the-router.md` | docs | changed: an appended amendment note |
| `docs/red-first/SPEC-319.md` | docs | added |
| `crates/notifications/src/router.rs` | `deck-streak-notifications` | changed: the `holding` flag and `Router::holding`, one arm of `after_claim`, and `seed_celebrations_off` |
| `crates/notifications/tests/router.rs` | `deck-streak-notifications` | changed: A5 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the policy and the seed in `RecomputeSetup::load`, the router in `RecomputeSetup::cycle`, `holding_router`, two `RecomputeError` variants, the module note |
| `crates/daemon/tests/recompute_router.rs` | `deck-streak-daemon` | added: A1 to A4 |
| `formal/tla/HeldFlush/HeldFlush.tla` | formal | changed: `SendHold`, `HoldSend`, `Unanswered`, two covers |
| `formal/tla/HeldFlush/MCHeldFlush.cfg` | formal | changed: `SendHold = FALSE` |
| `formal/tla/HeldFlush/witness/LeaseLapses.cfg`, `witness/NoSerialisation.cfg`, `witness/OnlyTheSyncFlusher.cfg`, `witness/ReleaseOnFail.cfg` | formal | changed: `SendHold = FALSE` |
| `formal/tla/HeldFlush/MCHeldSendHold.cfg` | formal | added: report mode, `SendHold = TRUE` |
| `formal/tla/HeldFlush/witness/a-hold-outside-the-window-with-no-later-flush.cfg` | formal | added |
| `scripts/mutation-rows.d/S31900-S31999.json` | repo | added: rows S31900 to S31905 |
| `changelog.d/fix-recompute-router-571.md` | repo | added |

## 5. What this does NOT do

- It moves no bot credential into the job process and changes no deploy file: the job's router has
  no transport, and the bot and the `held_flush` job send what it holds (ADR-066, #571).
- It edits nothing in `crates/coordination`. The streak facts a flush carries, the stale notes of
  `CycleParts` and `CycleParts::with_flush`, and `ladder_facts.rs` and `occasion.rs` wait for #572.
- It does not amend the planned SPECs that say a scheduled sync carries no router: SPEC-102 R20 and
  its section 5 move with #127 part b and #574, and SPEC-104 A30, SPEC-106 R17 and SPEC-107 A40
  move with their own deliveries (#127).
- It does not turn the celebrations on. The switch is seeded off, and the cutover checklist turns
  it on (#402).
- Known deviation: a row the holding router holds is stored with the hold `send`, so a recap that
  rolls it up reads "(send failures)" and one abandoned at the age limit reads "(gave up retrying,
  unseen)" (`router.rs::recap`, lines 1346-1360 at `6773f6e`), though no send failed. No new hold
  value and no migration is added for it; the wording is #575's.
- It adds no send budget: the queue's bound (20, `deferral.queue_max`) abandons the rest by name at
  hold time, and a flush renders at most two in full and one recap line (#571).

## 6. Risks

- **A celebration held at a daytime owner's sync whose answer is not observed waits for the next
  07:36 flush**, which may find it past the 720-minute age and abandon it by name. Detected by the
  recap line and the decision ledger's `no_notifier` withhold; modelled by `MCHeldSendHold.cfg`.
- **The seed overwrites a value the owner stored.** Detected by A4 and row S31904.
- **A later delivery builds a recompute cycle outside `RecomputeSetup::cycle`.** Detected by A1,
  which drives the production seam, and row S31903.
- **The holding arm leaks to a nudge.** Detected by A5 and row S31901.

## 7. The mutation rows

Band `S31900-S31999`. Each row is proved after the green commit by
`python3 scripts/mutation_rows.py prove --band S31900-S31999`, and its verdict is recorded in
`docs/red-first/SPEC-319.md`.

| row | target | what it guards | killer |
|---|---|---|---|
| `S31900-HOLDING-DEFERS-SEND` | `crates/notifications/src/router.rs` | the holding arm defers a celebration `send` rather than withholding it | `router::a_holding_router_holds_a_celebration_it_cannot_send` |
| `S31901-HOLDING-CELEBRATIONS-ONLY` | `crates/notifications/src/router.rs` | only a celebration is held; a nudge is still withheld | `router::a_holding_router_holds_a_celebration_it_cannot_send` |
| `S31902-HOLDING-SETS-FLAG` | `crates/notifications/src/router.rs` | `holding()` sets the flag | `router::a_holding_router_holds_a_celebration_it_cannot_send` |
| `S31903-CYCLE-ATTACHES-ROUTER` | `crates/daemon/src/wiring.rs` | `RecomputeSetup::cycle` attaches the router | `recompute_router::the_production_recompute_offers_an_owed_award_to_the_router` |
| `S31904-SEED-KEEPS-STORED` | `crates/notifications/src/router.rs` | the seed ignores an existing row | `recompute_router::a_fresh_start_seeds_the_celebrations_switch_off` |
| `S31905-LOAD-SEEDS-SWITCH` | `crates/daemon/src/wiring.rs` | `RecomputeSetup::load` seeds the switch | `recompute_router::a_fresh_start_seeds_the_celebrations_switch_off` |

## 8. References

- #571 (this delivery), #402 items 8 and 11 (the switch seeded off), #572 and #127 (what follows).
- SPEC-041 (R7, R13, R14, R15; section 10 is this SPEC's amendment), SPEC-073 (R4, R11), SPEC-059.
- ADR-066, ADR-109 and ADR-303; ADR-319.
- `docs/schematics/the-recompute-holds-a-router-for-the-senders.md`.
