# SPEC-083: the skip day is recorded by DeckStreak and bridges the game without writing to Anki

- **Wave:** W3. **Issue:** #108 (epic #4). **Context(s):** `deck-streak-ingest` (the skip record, its
  once-per-study-day key, its undo and its summary; no write to the collection);
  `deck-streak-economy` (the tariff's price, read from `economy.json`); `deck-streak-coordination`
  (the preview, the take and the undo, the tariff's purchase and refund, and the skip set every
  recompute step reads); `deck-streak-api`, `deck-streak-bot` and the Mini App (`web/app`).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-037 (one daily sync plus the
  owner's triggers, never an upload), ADR-071 (the recompute settles each study day once, in order),
  and ADR-083 (the skip day's write to the collection waits for the owner, and DeckStreak records
  the skip without one).
- **Prerequisites:** SPEC-071 (the rollup whose `due_today` the preview shows, and the recompute's
  step order), SPEC-072 (the consistency run and Ascendant's arming, which read the skip set),
  SPEC-076 (both streaks and the governor, which read it), SPEC-082 (the wallet's purchase and
  credit ports). SPEC-080 and SPEC-081 read the skip set when they land. **Mutation band:**
  `S08300-S08399`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-083.md` (ADR-016).

## 1. The problem, measured

- **What exists at `dev` c3d769b.** `crates/ingest/src/` syncs the private copy and reads it, and
  holds no skip; no table records a declared day off. SPEC-049's lapse episode takes its skip days
  from its caller, which passes an empty set "until the skip day exists (#108)" (SPEC-049 R15), and
  SPEC-072's consistency run and SPEC-076's streaks take theirs the same way.
- **What the predecessor does** (`27ee2bc`). `pipeline_layers/skip.py:SkipDaysLayer.skip_preview`
  shows today's due review count (the day's rollup `due_today`), whether a skip is already active,
  the day spec and this month's tariff. `SkipDaysLayer.take_skip_day` records a row and, through
  `sync.py:AnkiSyncer.skip_day`, converges the collection with the sync server, snapshots every
  affected card, reschedules today's due review cards with `set_due_date` and uploads, all or
  nothing. `SkipDaysLayer.undo_skip_day` restores the snapshot, uploads again and refunds the tariff.
  `pipeline_layers/economy.py:EconomyLayer._charge_skip_tariff` prices a skip by the month's prior
  skips, clipped to the wallet and outside the daily loss cap, and a skip it cannot fund still
  applies. `skip.py:summarize_skips` shows counts only. Every other context reads the set of
  applied, not-undone skip days (`database.py:GamifyStore.skip_days_set`).
- **What is not ported, and why.** DeckStreak never uploads (ADR-037), and the owner reused the Anki
  login only on the condition that no upload path exists, proven by a recording fake server
  (SPEC-022 R14). ADR-083 lays out the options for the write and recommends (b): DeckStreak records
  the skip and applies its effects, and the owner reschedules in Anki with the search and day spec
  DeckStreak shows. The owner decides at #266. Under (b) the converge, the card snapshot, the
  reschedule, the upload, the revert on a failed upload, the write's timeout and its background
  reconciliation have no work to do and are not built, and the 5,000-card guard
  (`constants.SKIP_MAX_CARDS`, recorded in `goldens/skip.constants.json`) guards nothing, because
  DeckStreak moves no card. Every criterion below holds under (b).
- **Corrections to the issue.** #108's first criterion (the reschedule under the 5,000-card guard)
  and its second (the card snapshot and its exact restore) describe the write that ADR-083 leaves to
  the owner: here the wrapped search and the day spec are shown to the owner, never run (R3), and
  undo reverses DeckStreak's record (R5). The preview's due count is absent when the study day has no
  rollup yet, where the predecessor showed 0 (R7). Its third criterion's golden of
  `EconomyLayer._charge_skip_tariff` is kept (R8), and its fourth holds as R11 and R18.
- **An inert switch and cap.** The predecessor reads a skip-day switch it never checks, and defines a
  monthly cap of skip-day bridges (`constants.SKIP_BRIDGE_MONTHLY_CAP`) it never enforces; both are
  excluded, inert in v9 (#269). `economy.json` keeps the cap declared
  (`streak.skip_bridge_monthly_cap`), unenforced, as the game-economy pack's reference requires.
- **What the parity oracle proves.** The day spec (`skip.py:skip_spec`); the search as the write path
  wraps it (`sync.py:AnkiSyncer._skip_day_blocking`, driven through a stand-in collection that
  records the search and holds no card, so nothing is written); the preview
  (`SkipDaysLayer.skip_preview`); the charge and the refund (`EconomyLayer._charge_skip_tariff`,
  `EconomyLayer._refund_skip_tariff`); the summary (`skip.py:summarize_skips`); and the constants.
- **Prerequisites.** SPEC-071, SPEC-072, SPEC-076 and SPEC-082, as the header lists. SPEC-080 and
  SPEC-081 read this SPEC's skip set and prove their own reactions to it.

## 2. Requirements

The record (#108)

R1. `deck-streak-ingest` owns `skip_days` (`migrations/008301_ingest_skip_days.sql`, `STRICT`,
    `created_at`): one row per skip taken, with its study day (an epoch day), the due review count
    the preview showed (absent when the day had no rollup), whether its tariff went unfunded, and
    whether and when it was undone. A partial unique index on the study day over the rows not undone
    holds the once-per-study-day rule in the migration (the predecessor's guard,
    `database.py:GamifyStore.get_active_skip_day`).
R2. Taking a skip on a study day that already holds one not undone is refused with
    `already_skipped` and writes nothing; after an undo the day can be skipped again. Take and undo
    run inside the kernel's `BEGIN IMMEDIATE` write (SPEC-020 R16), so two concurrent takes write one
    row and a take never interleaves with an undo.
R3. The skip record takes the service's database only: it opens no collection, calls no engine and
    sends no request to the sync server. What it gives the owner to use in Anki is the configured
    search (`DECKSTREAK_SKIP_SEARCH`, defaulting to the predecessor's `constants.SKIP_DEFAULT_SEARCH`)
    wrapped exactly as the golden of `sync.py:AnkiSyncer._skip_day_blocking` wraps it, and the day
    spec of the golden of `skip.py:skip_spec`, which carries no `!`, so Anki's Set Due Date keeps
    each card's interval.
R4. The skip set is exactly the study days that hold a skip not undone
    (`database.py:GamifyStore.skip_days_set`), read through one port in coordination that every
    consumer calls; no other module queries `skip_days`.
R5. Undo reverses the most recent skip not undone (`database.py:GamifyStore.latest_undoable_skip`),
    marking it undone at the undo's instant; with none it answers `nothing_to_undo`.
R6. The summary shows counts only, equal to the golden of `skip.py:summarize_skips`: the skips in the
    current study day's calendar month, all time, and the last skip's study day. The predecessor's
    `cards_moved_all_time` becomes the due review counts at the skips, labelled as due at the skip,
    never as moved.

The preview, the tariff and undo (#108)

R7. The preview, a coordination use case, shows the current study day's due review count (SPEC-071's
    rollup `due_today`, or absent, never 0, when the day has no rollup yet), whether a skip is active
    today, the search and the day spec (R3), the tariff, and whether the balance covers it; it equals
    the golden of `SkipDaysLayer.skip_preview` for every case, the cases with no rollup (class
    `no-rollup`) compared as absent.
R8. The tariff is `economy.json`'s `streak.skip_tariff_coins`, which equals the golden constant
    `constants.SKIP_TARIFF_LADDER` (0, 50 and 100 coins), indexed by the skips already applied and not
    undone in the study day's calendar month, the last price repeating. The charge is a purchase
    through economy's port, clipped to the wallet and outside the daily loss cap; a skip whose tariff
    the wallet cannot cover still applies and its row records the shortfall. The price and the
    amount paid equal the golden of `EconomyLayer._charge_skip_tariff`.
R9. Undo refunds exactly the coins the skip paid, as a credit on the undo's study day; a free skip
    refunds nothing (the golden of `EconomyLayer._refund_skip_tariff`).
R10. The tariff's price is economy's function, reading the ladder from `economy.json` once at start;
    no tariff number is typed in code.

What a skip changes, at the next recompute (#108)

R11. From the recompute that follows a take, each context that reads the skip set applies its own
    rule to the day: the language streak bridges it and consumes no freeze (`skip.py:real_misses`,
    SPEC-076); the law streak bridges it (`analytics.py:bridged_streak`, SPEC-076); the consistency
    run leaves it unchanged (`gamification/governor.py:tier_down_run`, SPEC-072); Ascendant never
    arms on it (`pipeline_layers/governor.py:GovernorLayer._maybe_grant_ascendant`, SPEC-072); and the
    governor's silent run neither counts it nor ends at it (SPEC-049, SPEC-076).
R12. The day's quests are voided, never failed (`pipeline_layers/loot.py:LootLayer._update_quests`
    returns before minting or evaluating them), no session chest is rolled on it
    (`LootLayer._grant_session_chests`), and a race week with two or more skip days is a rest week
    that is neither raced nor settled
    (`pipeline_layers/ghost_race.py:GhostRaceLayer._update_ghost_race` and
    `GhostRaceLayer._settle_due_races`). SPEC-080 and SPEC-081 read this SPEC's port and prove
    each when they land.
R13. An undo changes only what the next recompute reads: from then on the day is a missed day
    wherever a rule counts one. Transitions already settled stay as they were, as the predecessor's
    persisted transitions do (`pipeline.py:GamifyPipeline._advance_streak`), and no settled XP of a
    closed day is lowered (ADR-071).

The surfaces (#108)

R14. The bot keeps `/skip`, with `/cheat` dispatched as its alias and not listed in the menu, and
    `/skipundo` and `/skipstats`: `/skip` answers the preview with Confirm and Cancel buttons
    (callback data `sk:go` and `sk:no`), `/skipundo` asks before undoing (`sk:undo`), and
    `/skipstats` shows the summary. Each answers the owner only (SPEC-026).
R15. The API serves `GET /api/skip/preview`, `POST /api/skip`, `POST /api/skip/undo` and
    `GET /api/skip/stats` to the owner's session only (SPEC-024).
R16. The Mini App's `/skip` route shows the preview, the tariff and whether the balance covers it,
    with one confirm; after a take it shows the search and the day spec with the steps to reschedule
    in Anki, and says that an undo in DeckStreak does not undo a reschedule made in Anki. The route
    joins `ROUTES`. Its copy names the skip as a day off the tariff prices, and threatens no loss.

Privacy and the collection (#108)

R17. `skip_days` is declared once in ingest's data-rights port as exported and erased, in
    `privacy.json` as the category `skip-days`, and with one line in `PRIVACY.md`; the ownership
    register gains its row.
R18. A reschedule the owner makes in Anki writes review-log rows of type 4 with ease 0, and the read
    never counts one as a study event (SPEC-023's rule): a skip day on which the owner rescheduled
    stays a day with no study.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a skip is recorded once per study day, a second take is refused with `already_skipped` writing nothing, and after an undo the day can be skipped again | `a_skip_is_recorded_once_per_study_day_and_again_after_an_undo` |
| A2 | a raw insert of a second skip not undone for one study day is refused by the migration's key | `the_migration_refuses_a_second_skip_not_undone_for_one_study_day` |
| A3 | undo reverses the most recent skip not undone, and with none answers `nothing_to_undo` | `undo_reverses_the_most_recent_skip_not_undone` |
| A4 | the skip set holds exactly the study days with a skip not undone | `the_skip_set_holds_exactly_the_days_with_a_skip_not_undone` |
| A5 | taking and undoing a skip sends no request through a recording layer in front of the sync endpoint, and leaves the collection copy's bytes unchanged | `taking_and_undoing_a_skip_sends_no_request_and_leaves_the_copy_unchanged` |
| A6 | no skip module names an engine write or a sync call, and a planted fixture that does is refused (examined count reported, zero refused) | `no_skip_module_names_an_engine_write_or_a_sync_call` |
| A7 | the search and the day spec shown to the owner equal the goldens of the wrap and of `skip.py:skip_spec` | `the_search_and_day_spec_shown_equal_the_parity_goldens` |
| A8 | the summary shows counts only and equals the golden of `skip.py:summarize_skips` | `the_summary_shows_counts_equal_to_the_parity_golden` |
| A9 | a reschedule made with the engine's own Set Due Date on a skip day writes rows the read does not count as study events | `a_reschedule_in_anki_on_a_skip_day_is_not_a_study_event` |
| A10 | the preview equals the golden of `SkipDaysLayer.skip_preview`, and its due count is absent when the study day has no rollup | `the_preview_matches_the_parity_golden_and_is_absent_without_a_rollup` |
| A11 | the tariff charged equals the golden of `EconomyLayer._charge_skip_tariff` for every count of prior skips in the month, and never counts against the daily loss cap | `the_tariff_charged_matches_the_parity_golden_outside_the_loss_cap` |
| A12 | a skip the wallet cannot fund still applies, the wallet stops at zero, and the row records the shortfall | `an_unfunded_skip_still_applies_and_records_the_shortfall` |
| A13 | undo refunds what the skip paid as a credit on the undo's study day, and a free skip refunds nothing | `undo_refunds_what_the_skip_paid_on_the_undo_day` |
| A14 | the tariff ladder is read from `economy.json` and equals the golden constant | `the_tariff_ladder_is_read_from_economy_json_and_equals_the_golden` |
| A15 | at the next recompute a recorded skip day bridges the language streak without consuming a freeze, and bridges the law streak | `a_skip_day_bridges_both_streaks_without_a_freeze` |
| A16 | a skip day leaves the consistency run unchanged and arms no Ascendant | `a_skip_day_leaves_the_consistency_run_unchanged_and_arms_no_ascendant` |
| A17 | a skip day neither counts toward nor ends the governor's silent run | `a_skip_day_neither_counts_nor_ends_the_governors_silent_run` |
| A18 | after an undo, the next recompute treats the day as a missed day, and transitions already settled stay as they were | `after_an_undo_the_day_is_a_missed_day_at_the_next_recompute` |
| A19 | `/skip` and `/cheat` answer the preview with Confirm and Cancel, and only for the owner | `skip_and_cheat_answer_the_preview_with_confirm_and_cancel` |
| A20 | `/skipundo` asks before undoing, and `/skipstats` shows counts only | `skipundo_asks_before_undoing_and_skipstats_shows_counts_only` |
| A21 | the four skip routes answer the owner's session and refuse any other caller with no data | `the_skip_routes_answer_only_the_owner` |
| A22 | the Mini App's skip sheet shows the due count, the tariff and whether the balance covers it, then the search and the steps to reschedule in Anki | `shows the due count, the tariff and the steps to reschedule in Anki` |
| A23 | ingest's data-rights port lists `skip_days` as exported and erased, and an erase empties it | `the_skip_days_table_is_exported_and_erased` |

```acceptance
A1: cargo test -p deck-streak-ingest --test skip_record -- --exact a_skip_is_recorded_once_per_study_day_and_again_after_an_undo
A2: cargo test -p deck-streak-ingest --test skip_record -- --exact the_migration_refuses_a_second_skip_not_undone_for_one_study_day
A3: cargo test -p deck-streak-ingest --test skip_record -- --exact undo_reverses_the_most_recent_skip_not_undone
A4: cargo test -p deck-streak-ingest --test skip_record -- --exact the_skip_set_holds_exactly_the_days_with_a_skip_not_undone
A5: cargo test -p deck-streak-ingest --test skip_no_write -- --exact taking_and_undoing_a_skip_sends_no_request_and_leaves_the_copy_unchanged
A6: cargo test -p deck-streak-ingest --test skip_census -- --exact no_skip_module_names_an_engine_write_or_a_sync_call
A7: cargo test -p deck-streak-ingest --test skip_record -- --exact the_search_and_day_spec_shown_equal_the_parity_goldens
A8: cargo test -p deck-streak-ingest --test skip_record -- --exact the_summary_shows_counts_equal_to_the_parity_golden
A9: cargo test -p deck-streak-ingest --test skip_no_write -- --exact a_reschedule_in_anki_on_a_skip_day_is_not_a_study_event
A10: cargo test -p deck-streak-coordination --test skip_flow -- --exact the_preview_matches_the_parity_golden_and_is_absent_without_a_rollup
A11: cargo test -p deck-streak-coordination --test skip_flow -- --exact the_tariff_charged_matches_the_parity_golden_outside_the_loss_cap
A12: cargo test -p deck-streak-coordination --test skip_flow -- --exact an_unfunded_skip_still_applies_and_records_the_shortfall
A13: cargo test -p deck-streak-coordination --test skip_flow -- --exact undo_refunds_what_the_skip_paid_on_the_undo_day
A14: cargo test -p deck-streak-economy --test skip_tariff -- --exact the_tariff_ladder_is_read_from_economy_json_and_equals_the_golden
A15: cargo test -p deck-streak-coordination --test skip_effects -- --exact a_skip_day_bridges_both_streaks_without_a_freeze
A16: cargo test -p deck-streak-coordination --test skip_effects -- --exact a_skip_day_leaves_the_consistency_run_unchanged_and_arms_no_ascendant
A17: cargo test -p deck-streak-coordination --test skip_effects -- --exact a_skip_day_neither_counts_nor_ends_the_governors_silent_run
A18: cargo test -p deck-streak-coordination --test skip_effects -- --exact after_an_undo_the_day_is_a_missed_day_at_the_next_recompute
A19: cargo test -p deck-streak-bot --test skip_commands -- --exact skip_and_cheat_answer_the_preview_with_confirm_and_cancel
A20: cargo test -p deck-streak-bot --test skip_commands -- --exact skipundo_asks_before_undoing_and_skipstats_shows_counts_only
A21: cargo test -p deck-streak-api --test skip_routes -- --exact the_skip_routes_answer_only_the_owner
A22: pnpm exec vitest run web/app/src/lib/skip/skip-sheet.test.ts -t "shows the due count, the tariff and the steps to reschedule in Anki"
A23: cargo test -p deck-streak-ingest --test skip_rights -- --exact the_skip_days_table_is_exported_and_erased
```

A5 starts the recording layer SPEC-022 built (`crates/ingest/tests/support/recording.rs`) in front of
an upstream nobody listens on, points the fixture's sync endpoint at it, takes and undoes a skip, and
asserts that the layer kept no request and that the copy's SHA-256 is unchanged. A9 builds a small
collection with the engine, as the reader's tests do, and reschedules one due review card with the
engine's own Set Due Date, which writes the same kind of row the owner's Anki writes.

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. Every pack named here is enforced already, and this
delivery changes no pack's state.

| id | criterion | decided by |
|---|---|---|
| B1 | the privacy inventory declares the new category, over `privacy.json`, `PRIVACY.md` and `crates/ingest/src/data_rights.rs`, examining the `skip-days` category and the `skip_days` table | the privacy-gdpr pack |
| B2 | the economy stays equal to its reference, over `economy.json`, examining the skip tariff as a rising list of coin prices and the unenforced bridge cap as declared | the game-economy pack |
| B3 | the skip sheet's copy shames no choice and threatens no streak loss, over `web/app/src/routes/skip/` and `web/app/src/lib/skip/` | the ux-laws pack |
| B4 | the `/skip` route passes the audit in both Telegram colour schemes, over `web/app/src/routes/skip/+page.svelte` | the accessibility pack |
| B5 | the skip commands keep their callback data within the Bot API's limit and answer the owner only, over `crates/bot/src/skip_commands.rs` and `crates/bot/src/commands.rs` | the telegram-platform pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/ingest/src/skip.rs` | `deck-streak-ingest` | added: the record, its once-per-study-day take, the undo, the skip set, the summary, the search and day spec shown |
| `crates/ingest/src/settings.rs` | `deck-streak-ingest` | changed: `DECKSTREAK_SKIP_SEARCH`, defaulting to the golden constant |
| `crates/ingest/src/data_rights.rs` | `deck-streak-ingest` | changed: `skip_days`, exported and erased |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the skip module |
| `crates/ingest/tests/skip_record.rs` | `deck-streak-ingest` | added: A1 to A4, A7, A8 |
| `crates/ingest/tests/skip_no_write.rs` | `deck-streak-ingest` | added: A5, A9 |
| `crates/ingest/tests/skip_census.rs` | `deck-streak-ingest` | added: A6, with its planted fixture |
| `crates/ingest/tests/skip_rights.rs` | `deck-streak-ingest` | added: A23 |
| `crates/economy/src/tariff.rs` | `deck-streak-economy` | added: the skip tariff's price, from `economy.json`'s ladder |
| `crates/economy/src/lib.rs` | `deck-streak-economy` | changed: the tariff module |
| `crates/economy/Cargo.toml` | `deck-streak-economy` | changed: `serde` and `serde_json` as dev-dependencies for the golden reader, when SPEC-082 has not added them (SPEC-029 R8) |
| `crates/economy/tests/skip_tariff.rs` | `deck-streak-economy` | added: A14 |
| `crates/coordination/src/skip/mod.rs` | `deck-streak-coordination` | added: the preview, the take and the undo, the tariff's purchase and refund |
| `crates/coordination/src/skip/days.rs` | `deck-streak-coordination` | added: the skip-set port every recompute step reads, in place of the empty set SPEC-071's recompute passes |
| `crates/coordination/src/recompute/` | `deck-streak-coordination` | changed: the steps that take skip days read them from `skip/days.rs` |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the skip module |
| `crates/coordination/tests/skip_flow.rs` | `deck-streak-coordination` | added: A10 to A13 |
| `crates/coordination/tests/skip_effects.rs` | `deck-streak-coordination` | added: A15 to A18 |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded `skip_days` row |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | unchanged: ingest's port is registered already (SPEC-021); listed under SPEC-021's six-file rule |
| `crates/api/src/skip_routes.rs` | `deck-streak-api` | added: the four skip routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: mounts the skip routes |
| `crates/api/tests/skip_routes.rs` | `deck-streak-api` | added: A21 |
| `crates/bot/src/skip_commands.rs` | `deck-streak-bot` | added: the skip, cheat, skipundo and skipstats commands, and the `sk:` callbacks |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the command table, and the owner's menu gains the skip, skipundo and skipstats commands |
| `crates/bot/tests/skip_commands.rs` | `deck-streak-bot` | added: A19, A20 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the skip use cases joined to the record, the rollup and the wallet |
| `web/app/src/routes/skip/+page.svelte` | miniapp | added: the skip sheet's screen |
| `web/app/src/lib/skip/SkipSheet.svelte` | miniapp | added: the preview, the confirm and the steps for Anki |
| `web/app/src/lib/skip/api.ts` | miniapp | added: the skip routes' client |
| `web/app/src/lib/skip/skip-sheet.test.ts` | miniapp | added: A22 |
| `web/app/src/lib/routes.ts` | miniapp | changed: the skip route joins `ROUTES` |
| `migrations/008301_ingest_skip_days.sql` | `deck-streak-ingest` | added: `skip_days`, `STRICT`, with the partial unique index on the study day for rows not undone |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `.env.example` | repo | changed: `DECKSTREAK_SKIP_SEARCH`, empty, with the default named in its comment |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `skip_days` |
| `privacy.json` | repo | changed: the `skip-days` category |
| `PRIVACY.md` | repo | changed: the `skip-days` line |
| `tools/parity-oracle/registry/spec_083.py` | repo | added: this SPEC's registrations (SPEC-029's registry) |
| `tools/parity-oracle/goldens/skip_spec.json` | repo | added: the golden of `skip.py:skip_spec` (function) |
| `tools/parity-oracle/goldens/skip_search.json` | repo | added: the golden of `sync.py:AnkiSyncer._skip_day_blocking` (adapter; a stand-in collection records the search and holds no card) |
| `tools/parity-oracle/goldens/skip_preview.json` | repo | added: the golden of `pipeline_layers/skip.py:SkipDaysLayer.skip_preview` (adapter; a stub store) |
| `tools/parity-oracle/goldens/skip_tariff.json` | repo | added: the golden of `pipeline_layers/economy.py:EconomyLayer._charge_skip_tariff` (adapter; a stub store records the charge) |
| `tools/parity-oracle/goldens/skip_tariff_refund.json` | repo | added: the golden of `pipeline_layers/economy.py:EconomyLayer._refund_skip_tariff` (adapter; a stub store records the credit) |
| `tools/parity-oracle/goldens/skip_summary.json` | repo | added: the golden of `skip.py:summarize_skips` (adapter; days as epoch days) |
| `tools/parity-oracle/goldens/skip.constants.json` | repo | added: the skip constants (constants) |
| `scripts/mutation-rows.d/S08300-S08399.json` | repo | added: the hand-proved rows of §9 |
| `docs/specs/SPEC-083-the-skip-day-is-recorded-by-deckstreak-and-bridges-the-game-without-writing-to-anki.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-083-the-skip-days-write-to-the-collection-waits-for-the-owner.md` | docs | changed: accepted with the owner's decision at #266 |
| `docs/red-first/SPEC-083.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It reschedules no card in the collection and uploads nothing: the write waits for the owner's
  decision (#266).
- It enforces neither the predecessor's unchecked skip switch nor its unenforced monthly cap of
  skip-day bridges, both inert in v9 (#269).
- It proves no quest void, chest pause or race-week exemption itself: the quests, the chests and the
  race read this SPEC's port and prove each when they land (#100, #102, #124).
- It pauses no committed-window verdict (#109), no contract breach (#113) and no fine (#110), and
  extends no wager (#112): the discipline wave reads the same port.
- It withholds no evening nudge on a skip day and adds no skip button to the evening stakes card
  (#117).
- It publishes no skip badge or public skip count (#156).
- It serves no agent tool that takes, undoes or lists skips (#157).
- It imports none of the predecessor's skip rows (#61).
- It amends no charter constraint: CHARTER constraint 4's sentence waits for the owner's decision
  (#266).

## 6. Risks

- **The owner expects a skip to empty today's Anki queue**, as the predecessor's did. Detected by the
  sheet's steps (A22) and the bot's preview (A19); the owner's decision at #266 settles it.
- **A consumer reads the skip days its own way** and drifts from the port. Detected by A4 and the
  census's rule that no other module queries `skip_days`, and by each consumer's own criteria over
  the port (SPEC-072, SPEC-076, SPEC-080, SPEC-081).
- **A retried take charges the tariff twice.** Prevented by the once-per-study-day key (A2) and the
  coin ledger's unique (day, source, ref) key (SPEC-082); detected by A11.
- **An undo of an old skip surprises the owner** by turning a bridged day into a missed one at the
  next recompute. Detected by A18; the preview and the undo's confirmation say so first.
- **Someone adds an upload path for the skip without the owner's decision.** Detected by A5 and A6,
  and by SPEC-022's no-upload census.
- **The preview's due count is as old as the study day's last recompute.** Visible: the count is the
  rollup's, and the owner's `/sync` refreshes it (ADR-037).

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_083.py`; every day is an epoch day number and every
instant epoch milliseconds (SPEC-029 R3).

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `skip_spec` | `skip.py:skip_spec` | function | nothing: equal, reversed and sub-one bounds included |
| `skip_search` | `sync.py:AnkiSyncer._skip_day_blocking` | adapter | a stand-in syncer whose guarded open yields a stand-in collection: its login answers a token, the converge is patched to succeed, and its card search records the query and answers no card, so the function returns before any write; it returns the recorded search, for the default search and synthetic custom ones |
| `skip_preview` | `pipeline_layers/skip.py:SkipDaysLayer.skip_preview` | adapter | a stand-in layer on the case's study day with a stub store answering the day's rollup `due_today` or none (class `no-rollup`), an active skip or none, the month's skip rows and the balance; the day returned as an epoch day |
| `skip_tariff` | `pipeline_layers/economy.py:EconomyLayer._charge_skip_tariff` | adapter | a stub store answering the case's skip rows (study days, applied, undone) and balance, recording the coin delta and the detail it writes; classes `first-free`, `ladder-top`, `unfunded`, `month-boundary`, `undone-not-counted` |
| `skip_tariff_refund` | `pipeline_layers/economy.py:EconomyLayer._refund_skip_tariff` | adapter | a stub store answering what the skip paid and recording the credit and the day it lands on |
| `skip_summary` | `skip.py:summarize_skips` | adapter | the rows and `today` built from epoch days; the last day returned as an epoch day or null |
| `skip.constants` | `constants.SKIP_TARIFF_LADDER`, `SKIP_DEFAULT_SEARCH`, `SKIP_SPREAD_MIN_DAYS`, `SKIP_SPREAD_MAX_DAYS`, `SKIP_MAX_CARDS`, `SKIP_BRIDGE_MONTHLY_CAP` | constants | nothing |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `skip_days` | `ingest` | `migrations/008301_ingest_skip_days.sql` (SPEC-083) | `skip_days`: each applied row maps to one row with its day, its undone flag and instant, and its moved count as the due count; rows never applied are not imported; `skip_card_snapshot` has no DeckStreak table, because DeckStreak moves no card (ADR-083) | exported and erased |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S08301-ONE-SKIP-PER-STUDY-DAY` | `migrations/008301_ingest_skip_days.sql` | the partial unique index on the study day over rows not undone (a script-mutation row) | `skip_record::the_migration_refuses_a_second_skip_not_undone_for_one_study_day` |
| `S08302-THE-SET-EXCLUDES-UNDONE` | `crates/ingest/src/skip.rs` | the skip set's filter on rows not undone | `skip_record::the_skip_set_holds_exactly_the_days_with_a_skip_not_undone` |
| `S08303-UNDO-TAKES-THE-LATEST` | `crates/ingest/src/skip.rs` | the undo's order, the most recent skip first | `skip_record::undo_reverses_the_most_recent_skip_not_undone` |
| `S08304-THE-SEARCH-IS-WRAPPED` | `crates/ingest/src/skip.rs` | the wrap that keeps new and learning cards out of the search | `skip_record::the_search_and_day_spec_shown_equal_the_parity_goldens` |
| `S08305-THE-LAST-PRICE-REPEATS` | `crates/economy/src/tariff.rs` | the ladder's index clamped to its last price | `skip_tariff::the_tariff_ladder_is_read_from_economy_json_and_equals_the_golden` |
| `S08306-THE-TARIFF-IS-A-PURCHASE` | `crates/coordination/src/skip/mod.rs` | the charge taken through the purchase port, outside the daily loss cap | `skip_flow::the_tariff_charged_matches_the_parity_golden_outside_the_loss_cap` |
| `S08307-AN-UNFUNDED-SKIP-APPLIES` | `crates/coordination/src/skip/mod.rs` | the amount paid clipped to the balance while the skip stands | `skip_flow::an_unfunded_skip_still_applies_and_records_the_shortfall` |
| `S08308-THE-REFUND-LANDS-ON-THE-UNDO-DAY` | `crates/coordination/src/skip/mod.rs` | the refund credited on the undo's study day | `skip_flow::undo_refunds_what_the_skip_paid_on_the_undo_day` |
