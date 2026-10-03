# SPEC-326: celebrations and flushes read the stored streak, so the streak-break cap applies

- **Issue:** #572. **Context(s):** `deck-streak-coordination` (the one reader of the
  streak's facts, the three celebration routes and two flushers); `deck-streak-notifications` (the
  router's own database, read through a new accessor); `deck-streak-daemon` (the bot's flush);
  `formal` (`formal/tla/HeldFlush`, `formal/tla/AwardOnce`, `formal/tla/RelightOrder`).
- **Decided by:** ADR-327 (coordination reads the stored language streak through the router's
  database before every celebration route and every flush, and a failed read routes and flushes
  nothing). Amends SPEC-084 (section 11, insert-only).
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-326.md`. **Mutation band:** `S32600-S32699`.
- Every line number below was read at `dev` `c6d29f73`.

## 1. The problem, measured

SPEC-084 R5 caps every celebration of a study day on which the language streak broke at the
policy's `streak_break.cap` (T1), and R11 re-caps each held celebration at the flush's study day.
Neither ever applies in production, because no production path hands the ladder a streak fact:

- `crates/coordination/src/ladder_facts.rs:12-13` is `pub const fn streak_facts() ->
  Option<StreakFacts> { None }`.
- `Occasion::with_streak` (`crates/notifications/src/occasion.rs:379`) has no production caller:
  `git grep -n 'with_streak(' -- crates | grep /src/` names only its definition. The three
  production celebration routes build their occasions without it: `impl Celebrate for Router`
  (`crates/coordination/src/recompute/mod.rs:371-393`, the one door every badge, record, band-up and
  landmark goes through), `level_up.rs::announce_level_up` (23-49) and `relight.rs::announce_relight`
  (22-43). `git grep -n '\.route(' -- crates | grep /src/` outside the router names exactly those
  three calls.
- Every production flush passes no facts: `held_flush.rs:25` and `sync_cycle.rs:387` pass
  `streak_facts()`, which is `None`, and the bot's `impl Flush for Arc<Router>`
  (`crates/daemon/src/sync_request.rs:103-109`) calls `Router::flush`, which is `flush_with(None)`
  (`crates/notifications/src/router.rs:909-911`).
- So `ladder::streak_broke_on` (`crates/notifications/src/ladder.rs:107-114`) reads no break at
  route (`router.rs:622`) or at flush (`router.rs:975`). After SPEC-319 every production celebration
  is held at route time (the cycle's router has no transport), so the senders' flush is where the
  cap must bite.

The stored streak exists: `deck_streak_streaks::store::state` (`crates/streaks/src/store.rs:178-196`)
reads `streak_state` for a track, and `crates/coordination/src/progression/badges_view.rs:282-284`
already reads the `language` track in that form.

## 2. Requirements

R1. `Router::db(&self) -> &Db` answers the router's own database. It is a read of a field, added
    after `holding`, and no covered item of the router moves.
R2. `ladder_facts::streak_facts(db: &Db) -> Result<Option<StreakFacts>, KernelError>` acquires a
    reader connection and reads `deck_streak_streaks::store::state` for the track `language`,
    mapping `last_study_day`, `current` and `longest`; `freezes` and `comeback_armed` are dropped.
    No stored row answers `None`, which reads no break (SPEC-084 R5).
R3. `ladder_facts::with_streak_facts(db, occasion)` answers the occasion carrying the stored facts
    (`Occasion::with_streak`), or the occasion unchanged when none is stored.
R4. `ladder_facts::flush_re_capped(router: &Router)` reads the facts through `router.db()` and then
    runs `router.flush_with(facts)`.
R5. Each production celebration route reads the facts first, by ONE inserted statement before its
    route: `impl Celebrate for Router` in `recompute/mod.rs`, `announce_level_up` and
    `announce_relight`. `Router::route_photo` is unchanged (it has no production caller).
R6. Each production flusher calls `flush_re_capped`: `HeldFlushWork::perform` (only its `match`
    head changes; its four arms are byte-equal), `sync_cycle.rs::flush` (only its `match` head
    changes) and the bot's `impl Flush for Arc<Router>`. `Router::flush` stays `flush_with(None)`.
R7. A failed read FAILS CLOSED. The occasion is not routed: `celebrate` answers the error, so an
    award stays owed (ADR-303); a relight stays due; a level-up is logged and lost, as on any router
    error. The flush does not run, so the queue keeps its holds: `perform` answers the existing
    `Err(Reason::new("flush_failed"))`, the sync cycle logs it and never fails the sync, and the
    bot's flush answers the error to `answer`'s existing log arm.
R8. R5 applies to the relight as SPEC-084 states it, to every celebration: on a relight's own day
    the stored streak is back to 1 and was once longer, so the comeback's line is capped at T1.
    A relight first routed on a later day, at its settle or in the cycle after a restart, is raised
    on no break day and renders its line (ruling 131's pin in `relight_order.rs` measures both).
R9. The doc lines that said "until SPEC-076" say what is now true (ADR-327, D5):
    `ladder_facts.rs`'s module doc, `occasion.rs:317-318`, and `sync_cycle.rs:81`, `129` and
    `382-385`.
R10. SPEC-084 gains an insert-only section 11 that supersedes its three stale sentences (sections
    4, 6 and 10); nothing above its last line changes.
R11. `formal/tla/HeldFlush`, `formal/tla/AwardOnce` and `formal/tla/RelightOrder` are re-read
    against their moved covers and re-stamped, each with a dated re-read note: the read is a read
    of `streak_state` on a reader before the route or the flush's take, moves none of the model's
    variables, and fails as an arm each model has or names. `BandUpOnce` and `LandmarkOnce` are
    re-read and not re-stamped: no span they cover moves.

## 3. Acceptance criteria

Fixtures: a migrated database; a stored language streak `last_study_day = D`, `current = 1`,
`longest = 5` is "a break on D"; every clock is a `ManualClock` at noon of D, outside the quiet
window. A holding router records a deferred decision at T0 (`router.rs`'s `Subject::deferred`), so
the tier a holding router decided for a celebration is its held row's `tier_pending`. Where a
flush renders at T1, an owner's message is recorded first, so the T1 render is a reaction.

| id | criterion | decided by |
|---|---|---|
| A1 | an award offered through `Celebrate::celebrate` on a holding `Router` on D with a break on D is held with `tier_pending` `T1`; with no stored streak the same award on a fresh database is held `T2` (the control) | `cargo test -p deck-streak-coordination --test break_day_cap -- --exact an_award_raised_on_a_streak_break_day_is_decided_at_most_t1` |
| A2 | `announce_level_up` on a holding router on D with a break on D: its held row's `tier_pending` is `T1`; the control without a streak, `T2` | `cargo test -p deck-streak-coordination --test break_day_cap -- --exact the_level_up_on_a_streak_break_day_is_decided_at_most_t1` |
| A3 | `announce_relight` raised on D on a holding router with a break on D: `T1`; the control, `T2` (R8) | `cargo test -p deck-streak-coordination --test break_day_cap -- --exact the_relight_on_its_streak_break_day_is_decided_at_most_t1` |
| A4 | `HeldFlushWork::perform` over a T2 celebration held `quiet` on D, with a break on D: the bot port's calls EQUAL those a twin database gets from `router.flush_with(Some(the break's facts))` and DIFFER from `flush_with(None)`'s | `cargo test -p deck-streak-coordination --test break_day_cap -- --exact the_held_flush_job_re_caps_with_the_stored_streak` |
| A5 | the bot's `Flush::flush` for `Arc<Router>`, over the same held row and break: the same twin equality and difference | `cargo test -p deck-streak-daemon --test break_day_flush -- --exact the_bots_flush_re_caps_with_the_stored_streak` |
| A6 | a sync cycle that ran and succeeded flushes its bot-joined router with the stored facts (the `level_up_cycle.rs` harness, with no fold attached, so nothing rewrites the seeded streak): the same twin equality and difference | `cargo test -p deck-streak-coordination --test level_up_cycle -- --exact a_sync_cycles_flush_re_caps_with_the_stored_streak` |
| A7 | with the stored streak unreadable (the streaks table renamed in the test database): `celebrate` answers an error and writes no decision, and `perform` answers `Err(Reason::new("flush_failed"))` with the row still `held` | `cargo test -p deck-streak-coordination --test break_day_cap -- --exact an_unreadable_streak_leaves_the_award_owed_and_the_hold_held` |

```acceptance
A1: cargo test -p deck-streak-coordination --test break_day_cap -- --exact an_award_raised_on_a_streak_break_day_is_decided_at_most_t1
A2: cargo test -p deck-streak-coordination --test break_day_cap -- --exact the_level_up_on_a_streak_break_day_is_decided_at_most_t1
A3: cargo test -p deck-streak-coordination --test break_day_cap -- --exact the_relight_on_its_streak_break_day_is_decided_at_most_t1
A4: cargo test -p deck-streak-coordination --test break_day_cap -- --exact the_held_flush_job_re_caps_with_the_stored_streak
A5: cargo test -p deck-streak-daemon --test break_day_flush -- --exact the_bots_flush_re_caps_with_the_stored_streak
A6: cargo test -p deck-streak-coordination --test level_up_cycle -- --exact a_sync_cycles_flush_re_caps_with_the_stored_streak
A7: cargo test -p deck-streak-coordination --test break_day_cap -- --exact an_unreadable_streak_leaves_the_award_owed_and_the_hold_held
```

## 3b. Decided by the formal check

The formal criterion is judged by the repository's formal checker, not by a test, so it is stated
here and kept out of the fence above, which names tests only. No criterion here needs a box-run
pack.

| id | criterion | decided by |
|---|---|---|
| A8 | `formal/tla/HeldFlush`, `formal/tla/AwardOnce` and `formal/tla/RelightOrder` are each re-stamped with a dated re-read note; every property `clean` as at the base, every witness caught, no `STALE`; the unnarrowed ratchet has no `FAIL` | the formal check, `formal check --entry tla/<E>` for each of the three and `--ratchet-only` unnarrowed, against the base |

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-326-celebrations-and-flushes-read-the-stored-streak-so-the-streak-break-cap-applies.md` | docs | added |
| `docs/decisions/ADR-327-coordination-reads-the-stored-streak-through-the-routers-database-and-fails-closed.md` | docs | added |
| `docs/schematics/the-celebrations-read-the-stored-streak.md` | docs | added: the sequence of each read before its route or flush |
| `docs/red-first/SPEC-326.md` | docs | added |
| `docs/specs/SPEC-084-the-celebration-ladder-renders-tiers-on-the-one-router-within-weekly-budgets.md` | docs | changed: section 11 appended, insert-only |
| `changelog.d/fix-celebration-streak-facts-572.md` | repo | added |
| `scripts/mutation-rows.d/S32600-S32699.json` | scripts | added: rows S32600 to S32613 |
| `crates/coordination/src/ladder_facts.rs` | `deck-streak-coordination` | changed: the one reader of the stored streak's facts, and the re-capped flush |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: the `Celebrate` impl reads the facts before it routes |
| `crates/coordination/src/level_up.rs` | `deck-streak-coordination` | changed: the level-up reads the facts before it routes |
| `crates/coordination/src/relight.rs` | `deck-streak-coordination` | changed: the relight reads the facts before it routes; its module doc names the one routed celebration and its tier (ruling 131) |
| `crates/coordination/src/held_flush.rs` | `deck-streak-coordination` | changed: the job's flush is re-capped |
| `crates/coordination/src/sync_cycle.rs` | `deck-streak-coordination` | changed: the cycle's flush is re-capped; three doc lines |
| `crates/notifications/src/router.rs` | `deck-streak-notifications` | changed: `Router::db` |
| `crates/notifications/src/occasion.rs` | `deck-streak-notifications` | changed: lines 317-318 of `StreakFacts`'s doc |
| `crates/daemon/src/sync_request.rs` | `deck-streak-daemon` | changed: the bot's flush is re-capped |
| `crates/coordination/tests/break_day_cap.rs` | `deck-streak-coordination` | added: A1 to A4 and A7 |
| `crates/coordination/tests/level_up_cycle.rs` | `deck-streak-coordination` | changed: A6 |
| `crates/daemon/tests/break_day_flush.rs` | `deck-streak-daemon` | added: A5 |
| `formal/tla/HeldFlush/HeldFlush.tla` | formal | changed: a dated re-read note; covers re-stamped |
| `formal/tla/AwardOnce/AwardOnce.tla` | formal | changed: a dated re-read note; the level-up's cover re-stamped |
| `formal/tla/RelightOrder/RelightOrder.tla` | formal | changed: a dated re-read note; the relight's cover re-stamped; the note's reading of L1 (ruling 131) |
| `crates/coordination/tests/relight_order.rs` | `deck-streak-coordination` | changed: the cycle records an owner message and the transport records reactions (ruling 131); added: the pin with no owner message |
| `crates/coordination/tests/relight_settle.rs` | `deck-streak-coordination` | changed: an owner message before the route and the transport records reactions (ruling 131) |
| `docs/specs/SPEC-076-the-language-and-law-streaks-freezes-the-governor-and-the-relight-hold-across-every-study-day.md` | docs | changed: section 37 appended, insert-only (ruling 131) |

## 5. What this does NOT do

- `streak_break.defer_fanfare` stays unread (`crates/notifications/src/policy.rs:129-130`); #590
  tracks it.
- It does not close the read-then-render race (a fold that commits a break between a flusher's read
  and its render renders one held celebration uncapped once) or model HeldFlush's unguarded return
  on a failed read; both are named abstractions here and #589 tracks them.
- It adds no new tier rule and does not change R5's predicate, `ladder::streak_broke_on` (#572).
- It backfills nothing: a celebration already rendered before this delivery is not re-capped
  (#572).
- `crates/daemon/src/role_job.rs:9-11` stays as it is; that file is outside this delivery's write
  list (#572).

## 6. Risks

- **The read-then-render race.** A fold that commits a break between a flusher's read and its
  render renders one held celebration uncapped once. Detected by nothing until #589.
- **A stale reader snapshot.** A flush that read the facts from an older snapshot than the one its
  take sees would re-cap with the wrong day. Detected by A4 to A6, which seed the break before the
  flush and require the twin with the break's facts.
- **The relight is capped on its return day** (R8): the comeback's celebration there is a
  reaction, or a hold for the flush when the owner has not written, never a line. Disclosed here,
  in SPEC-076 section 37 and in the pull request; detected by A3 and by `relight_order.rs`'s pin
  with no owner message.
- **A failed read loses a level-up.** The level-up has no mark, so a failed read loses it as any
  router error does today. Detected by the cycle's error log; A7 holds the fail-closed arms.

## 7. The mutation rows

Band `S32600-S32699`. Each row is proved after the green commit by
`python3 scripts/mutation_rows.py prove`, and its verdict is recorded in
`docs/red-first/SPEC-326.md`. `Router::db` has no row: it is a pass-through whose only mutant
cannot be written (no other `&Db` is in reach of the router), and every killer below reads it.

| row | target | what it guards | killer |
|---|---|---|---|
| `S32600` | `crates/coordination/src/ladder_facts.rs` | the facts are the language track's | `break_day_cap::an_award_raised_on_a_streak_break_day_is_decided_at_most_t1` |
| `S32601` | `crates/coordination/src/ladder_facts.rs` | `current` is the stored current length | `break_day_cap::an_award_raised_on_a_streak_break_day_is_decided_at_most_t1` |
| `S32602` | `crates/coordination/src/ladder_facts.rs` | `longest` is the stored longest length | `break_day_cap::an_award_raised_on_a_streak_break_day_is_decided_at_most_t1` |
| `S32603` | `crates/coordination/src/ladder_facts.rs` | `last_study_day` is the stored day | `break_day_cap::an_award_raised_on_a_streak_break_day_is_decided_at_most_t1` |
| `S32604` | `crates/coordination/src/ladder_facts.rs` | the occasion carries the facts | `break_day_cap::an_award_raised_on_a_streak_break_day_is_decided_at_most_t1` |
| `S32605` | `crates/coordination/src/ladder_facts.rs` | a failed read fails the route | `break_day_cap::an_unreadable_streak_leaves_the_award_owed_and_the_hold_held` |
| `S32606` | `crates/coordination/src/ladder_facts.rs` | a failed read fails the flush | `break_day_cap::an_unreadable_streak_leaves_the_award_owed_and_the_hold_held` |
| `S32607` | `crates/coordination/src/ladder_facts.rs` | the flush is handed the facts it read | `break_day_cap::the_held_flush_job_re_caps_with_the_stored_streak` |
| `S32608` | `crates/coordination/src/recompute/mod.rs` | the `Celebrate` impl reads the facts | `break_day_cap::an_award_raised_on_a_streak_break_day_is_decided_at_most_t1` |
| `S32609` | `crates/coordination/src/level_up.rs` | the level-up reads the facts | `break_day_cap::the_level_up_on_a_streak_break_day_is_decided_at_most_t1` |
| `S32610` | `crates/coordination/src/relight.rs` | the relight reads the facts | `break_day_cap::the_relight_on_its_streak_break_day_is_decided_at_most_t1` |
| `S32611` | `crates/coordination/src/held_flush.rs` | the job's flush is re-capped | `break_day_cap::the_held_flush_job_re_caps_with_the_stored_streak` |
| `S32612` | `crates/coordination/src/sync_cycle.rs` | the cycle's flush is re-capped | `level_up_cycle::a_sync_cycles_flush_re_caps_with_the_stored_streak` |
| `S32613` | `crates/daemon/src/sync_request.rs` | the bot's flush is re-capped | `break_day_flush::the_bots_flush_re_caps_with_the_stored_streak` |

## 8. References

- #572 (this delivery), #589 (the read-then-render race and HeldFlush's unguarded return), #590
  (`defer_fanfare`), #571 and SPEC-319 (every production celebration is held for the senders).
- SPEC-084 (R5, R11; section 11 is this SPEC's amendment), SPEC-076 (the stored streak), SPEC-041
  (R7, R14), SPEC-073 (R4, R11).
- ADR-303 and ADR-319; ADR-327.
- `docs/schematics/the-celebrations-read-the-stored-streak.md`.
