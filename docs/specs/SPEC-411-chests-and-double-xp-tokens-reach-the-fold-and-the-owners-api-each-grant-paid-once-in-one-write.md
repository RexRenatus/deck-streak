# SPEC-411: chests and double-XP tokens reach the fold and the owner's API, each grant paid once in one write

- **Issue:** #102 (session chests), #103 (double-XP tokens); neither is delivered whole here.
  **Context(s):** `deck-streak-progression` (the `2x:` settle source); `deck-streak-notifications`
  (one public quiet-window function); `deck-streak-quests` (the token holdings read);
  `deck-streak-coordination` (`grant_freeze_on`, the fold's chest, sweep and token-bonus steps, and
  the open, pick, activate and inventory use cases); `deck-streak-daemon` (the steps' registration);
  `deck-streak-api` (four owner routes).
- **Decided by:** ADR-425 (E2b delivers the fold and the owner's chest routes first, each grant in
  the write that resolves it); ADR-041 (the one router), ADR-071 (each study day is settled once, in
  order), ADR-072 (the token bonus is settled XP), ADR-081 (one draw per chest, stored with its pity
  in one write).
- **Prerequisites:** SPEC-081 (E2, the quests core this SPEC wires), SPEC-071 (the recompute's
  fold), SPEC-072 (the grant and settle ports), SPEC-076 (the streaks' freeze port), SPEC-041 (the
  router). **Mutation band:** `S41100-S41199`.
- **Schematic:** `docs/schematics/quests-and-chests-lifecycle.md`, its section "A chest's grants,
  from the fold and from the owner's routes (E2b)".
- **Status:** delivered in part by the pull request that installs it (E2b-1, the server half);
  E2b-2 and E2b-3 follow (#102, #103).

## 1. The problem, measured

SPEC-081's section 3c splits E2 into parts: E2 delivered the quests core, and E2b wires it into the
fold, the use cases, the API, the bot, the Mini App and the odds page. At dev `ef4d9077` the core is
in the tree and nothing outside `crates/quests` calls it. Each reading below names the line it was
read at; a criterion with no test is NOT MET.

| SPEC-081 criterion | reading at `ef4d9077` | the test that decides it |
|---|---|---|
| SPEC-081 A1 | MET | `sessions_and_their_effort_match_the_predecessors_goldens`, `crates/quests/tests/chests_sessions.rs:59` |
| SPEC-081 A2 | MET | `the_rarity_roll_matches_the_predecessors_golden`, `crates/quests/tests/chests_roll.rs:29` |
| SPEC-081 A3 | MET | `the_payout_matches_the_predecessors_golden`, `crates/quests/tests/chests_roll.rs:58` |
| SPEC-081 A4 | MET | `the_session_chest_grant_matches_the_predecessors_golden`, `crates/quests/tests/chests_grant.rs:303` |
| SPEC-081 A5 | MET | `a_second_recompute_never_rolls_a_session_again`, `crates/quests/tests/chests_grant.rs:336` |
| SPEC-081 A6 | MET | `a_failed_draw_writes_no_chest_and_no_pity`, `crates/quests/tests/chests_grant.rs:402` |
| SPEC-081 A7 | MET | `the_challenge_and_weekly_chests_match_the_predecessors_goldens`, `crates/quests/tests/chests_grant.rs:453` |
| SPEC-081 A8 | MET | `opening_pays_once_and_reveals_the_stored_rarity`, `crates/quests/tests/chests_open.rs:200` |
| SPEC-081 A9 | MET | `a_capped_freeze_choice_becomes_a_token`, `crates/quests/tests/chests_open.rs:272` |
| SPEC-081 A10 | MET | `the_sweep_matches_the_predecessors_golden`, `crates/quests/tests/chests_open.rs:368` |
| SPEC-081 A11 | MET | `token_activation_matches_the_predecessors_golden`, `crates/quests/tests/tokens_window.rs:99` |
| SPEC-081 A12 | MET | `the_token_bonus_matches_the_predecessors_golden`, `crates/quests/tests/tokens_window.rs:184` |
| SPEC-081 A13 | NOT MET (E2c, #104) | `the_perfect_week_matches_the_predecessors_golden`: 0 occurrences outside `docs/` |
| SPEC-081 A14 | NOT MET (E2c, #104) | `test_no_copy_promises_a_smoke_bomb_spend`: 0 occurrences outside `docs/` |
| SPEC-081 A15 | MET | `the_chest_constants_equal_the_golden_and_economy_json`, `crates/quests/tests/chests_roll.rs:88` |
| SPEC-081 A16 | NOT MET (E2b) | `odds.test.ts`: 0 occurrences outside `docs/` |
| SPEC-081 A17 | NOT MET (E2b) | `open.test.ts`: 0 occurrences outside `docs/` |
| SPEC-081 A18 | NOT MET (E2b) | `the_chest_step_settles_a_closed_day_once` (`chests_steps`): 0 occurrences outside `docs/` |
| SPEC-081 A19 | NOT MET (E2b) | `the_chest_routes_answer_only_the_owner` (`chests_routes`): 0 occurrences outside `docs/` |
| SPEC-081 A20 | NOT MET (E2b) | `a_chest_callback_opens_once_for_the_owner_only` (`chests_callbacks`): 0 occurrences outside `docs/` |
| SPEC-081 A21 | MET | `the_chest_tables_are_exported_and_erased`, `crates/quests/tests/chests_rights.rs:51` |
| SPEC-081 A22 | NOT MET (E3, #107) | `the_shop_answer_lists_tokens_and_smoke_bombs_and_activates_the_oldest`: 0 occurrences outside `docs/` |
| SPEC-081 A23 | NOT MET (E3, #107) | `shows the token card and the smoke bombs held on the shop`: 0 occurrences outside `docs/` |

The count is 14 MET, 0 PARTIAL and 9 NOT MET. Three of SPEC-081's amendments are undelivered too:
T1 (the settle registry admits no `2x:` source: `crates/progression/src/settle.rs:18` lists the
derived sources and `:32` the derived prefixes, with no `2x:`, and `is_derived` is `:41`); T3 (no
store function sets a chest's announced mark; the field is `crates/quests/src/chest_store.rs:178`);
and T4 (there is no `grant_freeze_on`: `crates/coordination/src/freeze.rs:27`'s `grant_freeze`
opens its own write).

What each wiring surface holds at `ef4d9077`:

- **The quests ports.** `grant_session_chests_on`, `open_chest_on`, `sweep_stale_chests_on`,
  `activate_token_on`, `settle_token_bonuses_on` and `settle_epic_choice_on` are named in six files
  of `crates/quests`, one decision record and three formal entries, and in no file of
  coordination, the API, the bot, the daemon or the web app: 0 callers.
- **Quests.** `crates/quests/src/chests.rs`: `SessionChestGrant` `:97-113` (the study day, the
  sessions as `EarnedSession { session, base_xp }`, the skip day, the Ascendant flag, the local
  hour and the quiet flag), `grant_session_chests_on` `:115`, `Payout` `:352` (its source is
  `chest:<id>`), `Opened` `:373`, `Settled` `:391`, `Swept` `:409`, `open_chest_on` `:427`,
  `settle_epic_choice_on` `:465`, `sweep_stale_chests_on` `:505`, `ChestError` `:57`;
  `EarnedSession`'s `base_xp` caps a Common or Rare payout (SPEC-081 R4).
  `crates/quests/src/chest_store.rs`: `ChestState` `:50`, `StoredChest` `:160`, `unresolved_chests`
  `:266`. `crates/quests/src/tokens.rs`: `activate_token_on` `:122`, `settle_token_bonuses_on`
  `:203`, and no holdings read. `crates/quests/src/sessions.rs`: `SESSION_GAP_MS` `:13`,
  `sessions_from_reviews` `:111`. `crates/quests/src/draw.rs`: `Draw` `:20`, `OsDraw` `:31`.
- **Progression.** `crates/progression/src/review_xp.rs:15`; `crates/progression/src/grant.rs`:
  `GrantSource::new` `:36` (its grammar admits `chest:12` and `2x:12`), `GrantScope` `:72`,
  `GrantRequest` `:92-103`; `crates/progression/src/ledger.rs`: `grant_on` `:109` (an
  `ON CONFLICT DO NOTHING` insert); `crates/progression/src/consistency.rs:11-19` already leaves
  `chest:` and `2x:` sources out. `crates/analytics/src/constants.rs:9`:
  `ANSWER_TIME_CAP_SECONDS` is `60.0`.
- **Coordination.** `crates/coordination/src/lib.rs:18-48` lists the modules, with no chests
  module. `crates/coordination/src/recompute/mod.rs`: its modules `:27-39`, `Evaluation` `:103`,
  `DayStep` `:258`. `crates/coordination/src/recompute/day_bonuses.rs` is the model step: closed is
  `!Current` (`:45`), `skip_epoch_days` `:53`, `is_ascendant_day` `:56`, `settle_source` `:64`.
  Coordination already depends on quests, streaks, progression, notifications, analytics and
  economy.
- **The daemon.** `crates/daemon/src/wiring.rs`: `recompute_fold` `:273`,
  `recompute_fold_with_relights` `:285-312` (the day-bonuses step at `:304`), `Policy::compiled()`
  `:420`, and the step-list tests in `wiring::tests` `:1016-1095`, built by
  `recompute_fold(AnalyticsSettings::default())`.
- **Notifications.** `crates/notifications/src/router.rs`: `in_quiet_window` `:1203-1216` is
  private, and so is `minutes_setting` `:1311`; the policy's `quiet_hours` is `pub(crate)`
  (`crates/notifications/src/policy.rs:64`). `crates/notifications/src/quiet.rs`: `in_quiet_hours`
  `:23`, `local_minute` `:34`. `crates/notifications/src/transport.rs:86`: `push_message` sends text
  only, with no keyboard.
- **The API.** It depends on kernel, identity, notifications and coordination only, never on
  quests. `crates/api/src/router.rs:223-284` merges each module's `routes(access, readiness)`. The
  model route is `crates/api/src/sensitive_decks_routes.rs`: its path `"/api/decks/{id}/sensitive"`
  `:31`; its handler `:77-91` takes the owner session, the state-change guard, the state, the path
  id and the JSON body last; a digits-only positive `i64` id parse `:95-100`; 503 until the
  database opens.
- **The bot and the web app.** `crates/bot/src/commands.rs`: `on_callback` `:667-678` has no chest
  prefix; `crates/bot/src/gate.rs:27`: `MAX_CALLBACK_DATA_BYTES` is 64. The Mini App's route list
  has no `/chests` route. `economy.json:84-117` holds the chests, with `"disclosure": []` and
  `"purchasable": false`.
- **Formal.** The entries `tla/ChestOpenedOnce` (an MC configuration and four witnesses),
  `tla/ChestRolledOnce` (three witnesses) and `tla/TokenWindow` (three witnesses) model the core;
  `config/formal.json` carries no per-entry budget for them.

## 2. Requirements

R1. A settle source `2x:<id>` is derived exactly when `<id>` is a positive decimal integer with no
    sign and no leading zero that fits `i64`. `2x:0`, `2x:-1`, `2x:01`, `2x:` and an id past
    `i64::MAX` are not derived, and no other source changes its answer. (A1)

R2. The chest step runs in phase 4's `DaySteps` on a `Settle` or a `Current` evaluation only. It
    grants the day's session chests through `grant_session_chests_on`, with the inputs section 1
    lists: the study day, its earned sessions with their base XP, the skip day, the Ascendant flag,
    the owner's local hour and the quiet flag. A backfilled or revisited day rolls nothing. (A2, A3)

R3. On a `Current` evaluation the chest step passes only the sessions that ended at least
    `SESSION_GAP_MS` before now; a session ended exactly `SESSION_GAP_MS` before now passes. (A4)

R4. A draw error logs one warning naming the study day and the session's start, and keeps the day's
    other writes, so the next fold grants that session's chest. A database error fails the day's
    write. (A7)

R5. The sweep step runs in phase 4's `DaySteps` on a `Current` evaluation only. It resolves each
    stale chest through `sweep_stale_chests_on` and grants each payout through `grant_on` on the
    chest's own study day and track, with the source `chest:<id>` and the once scope, in the same
    write. (A5)

R6. The token-bonus step runs in phase 5's `DerivedBonuses` on every evaluation. It settles each
    window's bonus from `settle_token_bonuses_on` under the source `2x:<id>`, with closed equal to
    `!Current`: an open day's row is replaced, and a closed day's row is only ever raised. (A6)

R7. `open_chest` opens a chest in one write: `open_chest_on`, then the payout through `grant_on`
    with the source `chest:<id>` and the once scope. A second open answers already opened and writes
    nothing. (A8)

R8. `pick_epic_choice` settles an Epic's choice in one write. Whether the freeze is capped comes
    from the streaks' own admission of a freeze. `settle_epic_choice_on` runs with that answer, and
    a freeze it grants is written by `grant_freeze_on` in the same write. A capped freeze choice
    becomes a token and grants no freeze. (A9, A10)

R9. `grant_freeze` delegates to the new `grant_freeze_on`, which takes the caller's write; the
    existing tests of `grant_freeze` stay unchanged and green.

R10. `activate_token` activates the oldest held token in one write through `activate_token_on`. A
    second activation while a window is open answers already active and opens no second window.
    (A12)

R11. `chest_inventory` answers the chests by state with their study day, the tokens held and the
    open window's end. A sealed or vaulted chest's rarity and payout are never sent. The tokens held
    are counted by a new quests read of the unspent tokens, which also names the open window's end.
    (A11, A19)

R12. `deck-streak-notifications` exports `in_quiet_window_on(write, policy, offset, now)`, which
    reads the owner's quiet hours before the policy's; the router's own quiet check delegates to it,
    so the router and the chest step read one function. (A18)

R13. Four routes, each merged through `crates/api/src/router.rs` like every other module so it keeps
    the router's layers, each answering 401 without the owner's session before anything is read,
    400 for an id that is not a positive decimal `i64`, and 503 until the database opens; the three
    POSTs answer 403 `cross_site_request` to a cross-site request and write nothing:
    - `GET /api/chests`: 200 with the chests (each with `chest_id`, `state`, `study_day`, and
      `rarity` and `xp`, both null for a sealed or vaulted chest), `tokens_held` and
      `window_ends_at` (null when no window is open).
    - `POST /api/chests/{id}/open`: 200 `{chest_id, rarity, xp, choice_pending}` (`xp` null while an
      Epic's choice is pending); 409 `already_opened`; 404 `no_such_chest`.
    - `POST /api/chests/{id}/pick` with the body `{"choice": "token"}` or `{"choice": "freeze"}`:
      200 `{choice, capped, token_id}` (`token_id` null for a freeze); 409 `not_an_epic`; 409
      `already_settled`; 404 `no_such_chest`; 422 for a body whose choice is neither.
    - `POST /api/tokens/activate`: 200 `{token_id, window_ends_at}`; 409 `already_active`; 409
      `none_held`.
    (A13, A14, A15, A16)

R14. The daemon registers the chest and sweep steps in phase 4's `DaySteps` and the token-bonus step
    in phase 5's `DerivedBonuses`; `recompute_fold(analytics)` keeps its signature. (A17)

R15. The odds have one source, `economy.json`'s `chests`: the draw's constants equal it (SPEC-081
    A15's pin, unchanged), and no route or second file restates them. (A20)

R16. `tla/ChestOpenedOnce` gains the constant `CreditInCall`, a credits variable, the property
    `AResolvedChestIsPaid` and its witness, and covers for the new callers; `tla/TokenWindow` and
    `tla/ChestRolledOnce` gain covers for theirs (section 8).

R17. No migration and no new dependency edge between the workspace's crates. The one manifest
    change is test-only: `crates/coordination/Cargo.toml` names `getrandom` under
    `[dev-dependencies]` as a workspace entry, so A7 builds a refused draw as quests' own tests do,
    and `Cargo.lock` changes by that one line in coordination's dependency list and no other.
    `.sqlx/` gains added query files only.

## 3. Acceptance criteria of E2b-1

| id | criterion | decided by |
|---|---|---|
| A1 | a `2x:` source is derived exactly when its id is a positive decimal that fits `i64`, and no other source changes its answer | `a_double_xp_source_is_derived_only_with_a_positive_token_id` |
| A2 | the chest step settles a closed day's chests once: a second fold of the day rolls no session again and grants their XP once (SPEC-081 A18's test name) | `the_chest_step_settles_a_closed_day_once` |
| A3 | a backfilled or revisited day rolls nothing and grants nothing | `the_chest_step_rolls_nothing_on_a_backfill_or_a_revisit` |
| A4 | on the current study day a session ended less than `SESSION_GAP_MS` before now earns no chest, and one ended exactly `SESSION_GAP_MS` before now earns it | `a_session_still_open_earns_no_chest_until_its_gap_has_passed` |
| A5 | the sweep pays a stale chest once, on the chest's own study day and track | `the_sweep_pays_a_stale_chest_once_on_its_own_day` |
| A6 | the token-bonus step settles the window's bonus under `2x:<id>`, replaces it while the day is open, and a revisit of a closed day never lowers it | `the_token_bonus_step_settles_the_window_and_a_revisit_never_lowers_it` |
| A7 | a failed draw keeps the day's other writes, and the next fold grants that session's chest | `a_failed_draw_keeps_the_day_and_the_next_fold_grants_the_chest` |
| A8 | opening a chest pays its payout once, in the write that opens it | `opening_a_chest_pays_once_in_the_write_that_opens_it` |
| A9 | an Epic's freeze pick grants one freeze in the write that settles the choice | `an_epic_freeze_pick_grants_one_freeze_in_the_write_that_settles_it` |
| A10 | a freeze pick at the cap becomes a token and grants no freeze | `a_freeze_pick_at_the_cap_becomes_a_token_and_grants_no_freeze` |
| A11 | the inventory withholds a sealed or vaulted chest's rarity and payout | `the_inventory_withholds_a_sealed_or_vaulted_chests_rarity` |
| A12 | a second activation while a window is open opens no second window | `a_second_activation_opens_no_second_window` |
| A13 | the chest routes answer the owner's session only (SPEC-081 A19's test name) | `the_chest_routes_answer_only_the_owner` |
| A14 | a cross-site POST to each of the three POST routes answers 403 and writes nothing | `a_cross_site_chest_post_is_refused_and_writes_nothing` |
| A15 | a replayed open pays once | `a_replayed_open_pays_once` |
| A16 | each route answers each outcome R13 names by its status and code | `the_chest_routes_answer_each_outcome_by_its_status` |
| A17 | the fold registers the chest and sweep steps in phase 4 and the token-bonus step in phase 5 | `wiring::tests::the_recompute_fold_registers_the_chest_steps_in_their_phases` |
| A18 | the quiet window reads the owner's hours before the policy's | `the_quiet_window_reads_the_owners_hours_before_the_policys` |
| A19 | the holdings count unspent tokens only and name the open window's end | `the_holdings_count_unspent_tokens_and_name_the_open_window` |
| A20 | the chest constants equal the golden and `economy.json` (not red: SPEC-081 A15's pin, green at the base) | `the_chest_constants_equal_the_golden_and_economy_json` |

```acceptance
A1: cargo test -p deck-streak-progression --test settle_token_source -- --exact a_double_xp_source_is_derived_only_with_a_positive_token_id
A2: cargo test -p deck-streak-coordination --test chests_steps -- --exact the_chest_step_settles_a_closed_day_once
A3: cargo test -p deck-streak-coordination --test chests_steps -- --exact the_chest_step_rolls_nothing_on_a_backfill_or_a_revisit
A4: cargo test -p deck-streak-coordination --test chests_steps -- --exact a_session_still_open_earns_no_chest_until_its_gap_has_passed
A5: cargo test -p deck-streak-coordination --test chests_steps -- --exact the_sweep_pays_a_stale_chest_once_on_its_own_day
A6: cargo test -p deck-streak-coordination --test chests_steps -- --exact the_token_bonus_step_settles_the_window_and_a_revisit_never_lowers_it
A7: cargo test -p deck-streak-coordination --test chests_steps -- --exact a_failed_draw_keeps_the_day_and_the_next_fold_grants_the_chest
A8: cargo test -p deck-streak-coordination --test chests_use_cases -- --exact opening_a_chest_pays_once_in_the_write_that_opens_it
A9: cargo test -p deck-streak-coordination --test chests_use_cases -- --exact an_epic_freeze_pick_grants_one_freeze_in_the_write_that_settles_it
A10: cargo test -p deck-streak-coordination --test chests_use_cases -- --exact a_freeze_pick_at_the_cap_becomes_a_token_and_grants_no_freeze
A11: cargo test -p deck-streak-coordination --test chests_use_cases -- --exact the_inventory_withholds_a_sealed_or_vaulted_chests_rarity
A12: cargo test -p deck-streak-coordination --test chests_use_cases -- --exact a_second_activation_opens_no_second_window
A13: cargo test -p deck-streak-api --test chests_routes -- --exact the_chest_routes_answer_only_the_owner
A14: cargo test -p deck-streak-api --test chests_routes -- --exact a_cross_site_chest_post_is_refused_and_writes_nothing
A15: cargo test -p deck-streak-api --test chests_routes -- --exact a_replayed_open_pays_once
A16: cargo test -p deck-streak-api --test chests_routes -- --exact the_chest_routes_answer_each_outcome_by_its_status
A17: cargo test -p deck-streak-daemon --lib -- --exact wiring::tests::the_recompute_fold_registers_the_chest_steps_in_their_phases
A18: cargo test -p deck-streak-notifications --test quiet_window -- --exact the_quiet_window_reads_the_owners_hours_before_the_policys
A19: cargo test -p deck-streak-quests --test tokens_window -- --exact the_holdings_count_unspent_tokens_and_name_the_open_window
A20: cargo test -p deck-streak-quests --test chests_roll -- --exact the_chest_constants_equal_the_golden_and_economy_json
```

**Oracles.** Every expected value is a fixture literal, never built by the code under test. A
scripted draw counts its calls, so a test can say a session was rolled once or not at all. Each
criterion asserts a positive artifact (the `xp_ledger` row with its study day, track and amount;
the freeze row; the stored rarity; the token row and its window's end; the status and the code),
never an absence alone: a test that reads "nothing was granted" also reads one row that was. A14
posts to each of the three POST routes, one assertion per route, so each route's guard is observed
on its own. A17 builds the fold by `recompute_fold(AnalyticsSettings::default())`, as the tests at
`crates/daemon/src/wiring.rs:1016-1095` do. A1 to A19 are red at the base for their own reason; A20
is `not red`, because it pins a table the base already holds and guards it against this delivery.

## 4. File manifest

| path | part | change |
|---|---|---|
| `crates/progression/src/settle.rs` | `deck-streak-progression` | changed: a `2x:` prefix constant of its own and its own arm in `is_derived`, for a positive id; `DERIVED_PREFIXES` keeps its three entries (R1) |
| `crates/progression/tests/settle_token_source.rs` | `deck-streak-progression` | added: A1 |
| `crates/notifications/src/router.rs` | `deck-streak-notifications` | changed: the public free function `in_quiet_window_on`, and the router's quiet check delegates to it (R12) |
| `crates/notifications/tests/quiet_window.rs` | `deck-streak-notifications` | added: A18 |
| `crates/quests/src/tokens.rs` | `deck-streak-quests` | changed: the holdings read, unspent tokens and the open window's end (R11) |
| `crates/quests/tests/tokens_window.rs` | `deck-streak-quests` | changed: A19 appended |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: one module line, `pub mod chests;` |
| `crates/coordination/src/freeze.rs` | `deck-streak-coordination` | changed: `grant_freeze_on`, and `grant_freeze` delegates to it (R9) |
| `crates/coordination/src/chests.rs` | `deck-streak-coordination` | added: `open_chest`, `pick_epic_choice`, `activate_token`, `chest_inventory` (R7, R8, R10, R11) |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: one module line |
| `crates/coordination/src/recompute/chests.rs` | `deck-streak-coordination` | added: the chest, sweep and token-bonus steps and their free functions `grant_day_chests`, `sweep_and_pay`, `settle_day_token_bonuses` (R2 to R6) |
| `crates/coordination/tests/chests_steps.rs` | `deck-streak-coordination` | added: A2 to A7 |
| `crates/coordination/tests/chests_use_cases.rs` | `deck-streak-coordination` | added: A8 to A12 |
| `crates/coordination/Cargo.toml` | `deck-streak-coordination` | changed: `getrandom` joins `[dev-dependencies]` as a workspace entry, test-only, for A7's refused draw (R17) |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the three steps registered, A17 in `wiring::tests`, and the expected step lists of the two step-list tests at `:1016` and `:1059` extended to name the new steps, with no expected step removed (R14) |
| `crates/api/src/lib.rs` | `deck-streak-api` | changed: one module line |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: merges `chests_routes::routes(access, readiness)` |
| `crates/api/src/chests_routes.rs` | `deck-streak-api` | added: the four routes (R13) |
| `crates/api/tests/chests_routes.rs` | `deck-streak-api` | added: A13 to A16 |
| `formal/tla/ChestOpenedOnce/ChestOpenedOnce.tla` | formal | changed: `CreditInCall`, the credits variable, `AResolvedChestIsPaid`, the covers (R16) |
| `formal/tla/ChestOpenedOnce/MCChestOpenedOnce.cfg` | formal | changed: `CreditInCall = TRUE`, the property, at least two actors |
| `formal/tla/ChestOpenedOnce/witness/a-resolved-chest-left-unpaid.cfg` | formal | added: the witness, `CreditInCall = FALSE` |
| `formal/tla/TokenWindow/TokenWindow.tla` | formal | changed: covers for `activate_token` and `settle_day_token_bonuses` |
| `formal/tla/ChestRolledOnce/ChestRolledOnce.tla` | formal | changed: a cover for `grant_day_chests` |
| `Cargo.lock` | workspace | changed: one line, `getrandom` in `deck-streak-coordination`'s dependency list, and no other (R17) |
| `.sqlx/` | offline query cache | added: one file per new compile-checked query; no existing file changes (R17) |
| `scripts/mutation-rows.d/S41100-S41199.json` | rows | added: section 9's rows |
| `docs/red-first/SPEC-411.md` | docs | added: the red-first record |
| `docs/specs/SPEC-411-chests-and-double-xp-tokens-reach-the-fold-and-the-owners-api-each-grant-paid-once-in-one-write.md` | docs | added: this SPEC |
| `docs/decisions/ADR-425-e2b-delivers-the-fold-and-the-owners-chest-routes-first-each-grant-in-the-write-that-resolves-it.md` | docs | added: its decision |
| `docs/schematics/quests-and-chests-lifecycle.md` | docs | changed: one section appended |
| `changelog.d/quests-e2b-411.md` | docs | added: the changelog fragment |

## 5. What this does NOT cover

- The Mini App's chest inventory and its open animation, SPEC-081 A17: E2b-2 (#102).
- The odds page and `economy.json`'s disclosure, SPEC-081 A16: E2b-2 (#102).
- The token inventory screen and its countdown: E2b-2 (#103); the widget footer for an active
  token stays SPEC-081's own exclusion (#121).
- The bot's chest callbacks, SPEC-081 A20, the announcement's Open and Open-in-app buttons, and the
  keyboard they need: E2b-3 (#102).
- The announced mark and the offers of SPEC-081's amendment T3: E2b-3 (#102).
- Smoke bombs, their earning and the inventory's smoke bombs held: E2c (#104).
- The shop's arms, SPEC-081 A22 and A23: E3 (#107).
- SPEC-081's Status line and the move of its criteria A16 to A20 back into its section 3: the
  delivery that makes E2 whole (#102, #103).
- No smoke-bomb spend (#268).
- No chest lock and no ransom (#110).
- No morning brief, and no vaulted-chests line in one (#122).
- No widget footer for the active token (#121).
- No settings screen for the day's chest cap or the vault hour (#57).
- No share art for a Legendary chest (#125).
- No import of the predecessor's chests, tokens, pity counters or inventory (#61).
- No sale of a chest, a token or any randomized reward, now or later (#172).

## 6. Risks

- **The fold's write grows**: it now draws and grants inside each study day's one write. Detected
  by A2 and A7, which run the fold over a fixture day, and by the formal model, whose actors include
  the fold's writes.
- **A draw fails mid-day.** Detected by A7: the scripted draw fails once, the day's other writes
  stay, and the next fold grants the chest.
- **The router and the chest step drift apart on quiet hours.** Detected by A18: both read one
  function, and the owner's hours win.
- **An open through the API raises no celebration.** By design (ADR-041 and ADR-425 D2): the Mini
  App's animation of the route's answer is the reveal, and E2b-2 delivers it.
- **The inventory leaks a sealed or vaulted chest's rarity.** Detected by A11 and by row
  `S41109-INVENTORY-WITHHOLDS-SEALED`.
- **A chest, a pick or a window is paid twice.** Detected by A8, A9, A12 and A15, which replay each
  write, and by `AResolvedChestIsPaid` with its witness.

## 7. Delivered by the next pull requests

SPEC-411 delivers SPEC-081 A18 and A19 under their own test names (SPEC-411 A2 and A13). These
SPEC-081 criteria stay with later deliveries:

| id | criterion | delivered by |
|---|---|---|
| SPEC-081 A16 | the odds page states every rarity's odds and both guarantees as `economy.json` declares them | E2b-2 (#102) |
| SPEC-081 A17 | the open animation plays the rarity the server stored and never draws | E2b-2 (#102) |
| SPEC-081 A20 | a chest callback opens a chest once, for the owner only | E2b-3 (#102) |

The delivery that makes E2 whole moves SPEC-081 A16 to A20 back into SPEC-081's section 3 verbatim,
as its section 3c asks, and changes its Status.

E2b-2 also owes what its screens show a person (#102, #103). The open animation honours a
reduced-motion preference and never flashes more than three times a second. The reveal is announced
to assistive technology (`role="status"`). Each new screen joins the rendered accessibility audit.
The odds are shown wherever a chest is offered, not only on the odds page. A token's window is
worded by the time it ends, never as a countdown that presses a choice.

## 8. Formal

FORMAL: REQUIRED (an interleaving surface: the owner's open, pick and activate writes against the
fold's chest, sweep and token-bonus writes, over the chests, `xp_tokens`, `xp_ledger` and the freeze
rows; a TLA+ model, ADR-425 D5).

- `tla/ChestOpenedOnce` gains a constant `CreditInCall` (TRUE in `MCChestOpenedOnce.cfg`) and a new
  variable for the credits. Its `TypeOK` gains one conjunct for that variable; no base conjunct
  changes and no existing variable's domain widens.
- It gains the property `AResolvedChestIsPaid` (a resolved chest has its credit), entering at
  report, checked with at least two actors.
- Its witness `witness/a-resolved-chest-left-unpaid.cfg` sets `CreditInCall = FALSE` and cites
  #102. It models the design ADR-425 rejected, a credit in a later write, because no caller exists
  at the base.
- It gains covers for `crates/coordination/src/chests.rs`'s `open_chest` and `pick_epic_choice`,
  `crates/coordination/src/freeze.rs`'s `grant_freeze_on`, and
  `crates/coordination/src/recompute/chests.rs`'s `sweep_and_pay`.
- `tla/TokenWindow` gains covers for `activate_token` and `settle_day_token_bonuses`;
  `tla/ChestRolledOnce` gains a cover for `grant_day_chests`.
- Each step's work lives in a uniquely named free function, because a cover anchors one item.
- Floors are only raised. An entry's budget is overridden only when its measured need times 1.5
  exceeds the default, and then a test and a row pin it.
- Lean: NOT APPLICABLE. No new arithmetic (`TokenBonus.lean` and `Chest.lean` unchanged), and no
  balance exists: holdings are a count of rows and every amount is typed.

## 9. Mutation rows

Every row is in the `MUTATIONS` table of `scripts/mutation-rows.d/S41100-S41199.json`, its killer in
its own crate.

| row | target | what it guards | killer |
|---|---|---|---|
| `S41100-2X-IS-DERIVED` | `crates/progression/src/settle.rs` | `2x:` is a derived prefix | `settle_token_source::a_double_xp_source_is_derived_only_with_a_positive_token_id` |
| `S41101-2X-ID-POSITIVE` | `crates/progression/src/settle.rs` | `2x:0` is not derived | `settle_token_source::a_double_xp_source_is_derived_only_with_a_positive_token_id` |
| `S41102-CHEST-STEP-TODAY-ONLY` | `crates/coordination/src/recompute/chests.rs` | the chest step rolls on `Settle` and `Current` only | `chests_steps::the_chest_step_rolls_nothing_on_a_backfill_or_a_revisit` |
| `S41103-SESSION-GAP-CLOSED` | `crates/coordination/src/recompute/chests.rs` | a session ended exactly `SESSION_GAP_MS` before now passes | `chests_steps::a_session_still_open_earns_no_chest_until_its_gap_has_passed` |
| `S41104-SWEEP-CURRENT-ONLY` | `crates/coordination/src/recompute/chests.rs` | the sweep runs on `Current` only | `chests_steps::the_sweep_pays_a_stale_chest_once_on_its_own_day` |
| `S41105-SWEEP-PAYS-ON-THE-CHESTS-DAY` | `crates/coordination/src/recompute/chests.rs` | a swept payout is granted on the chest's own study day | `chests_steps::the_sweep_pays_a_stale_chest_once_on_its_own_day` |
| `S41106-OPEN-GRANTS-PAYOUT` | `crates/coordination/src/chests.rs` | the open's payout is granted in the open's write | `chests_use_cases::opening_a_chest_pays_once_in_the_write_that_opens_it` |
| `S41107-PICK-FREEZE-IN-WRITE` | `crates/coordination/src/chests.rs` | a freeze pick writes its freeze in the pick's write | `chests_use_cases::an_epic_freeze_pick_grants_one_freeze_in_the_write_that_settles_it` |
| `S41108-PICK-CAP-FROM-ADMIT` | `crates/coordination/src/chests.rs` | the cap comes from the streaks' admission | `chests_use_cases::a_freeze_pick_at_the_cap_becomes_a_token_and_grants_no_freeze` |
| `S41109-INVENTORY-WITHHOLDS-SEALED` | `crates/coordination/src/chests.rs` | a sealed or vaulted chest's rarity is not sent | `chests_use_cases::the_inventory_withholds_a_sealed_or_vaulted_chests_rarity` |
| `S41110-TOKEN-BONUS-RAISE-ONLY-CLOSED` | `crates/coordination/src/recompute/chests.rs` | closed is `!Current` for the token bonus | `chests_steps::the_token_bonus_step_settles_the_window_and_a_revisit_never_lowers_it` |
| `S41111-QUIET-OWNER-FIRST` | `crates/notifications/src/router.rs` | the owner's quiet hours are read before the policy's | `quiet_window::the_quiet_window_reads_the_owners_hours_before_the_policys` |
| `S41112-HOLDINGS-UNSPENT-ONLY` | `crates/quests/src/tokens.rs` | the holdings count only unspent tokens | `tokens_window::the_holdings_count_unspent_tokens_and_name_the_open_window` |
| `S41113-STEP-REGISTERED` | `crates/daemon/src/wiring.rs` | the chest step is registered | `wiring::tests::the_recompute_fold_registers_the_chest_steps_in_their_phases` |
| `S41114-INVENTORY-OWNER-ONLY` | `crates/api/src/chests_routes.rs` | `GET /api/chests` takes the owner session | `chests_routes::the_chest_routes_answer_only_the_owner` |
| `S41115-OPEN-OWNER-ONLY` | `crates/api/src/chests_routes.rs` | the open route takes the owner session | `chests_routes::the_chest_routes_answer_only_the_owner` |
| `S41116-PICK-OWNER-ONLY` | `crates/api/src/chests_routes.rs` | the pick route takes the owner session | `chests_routes::the_chest_routes_answer_only_the_owner` |
| `S41117-ACTIVATE-OWNER-ONLY` | `crates/api/src/chests_routes.rs` | the activate route takes the owner session | `chests_routes::the_chest_routes_answer_only_the_owner` |
| `S41118-OPEN-SAME-SITE-ONLY` | `crates/api/src/chests_routes.rs` | the open route takes the state-change guard | `chests_routes::a_cross_site_chest_post_is_refused_and_writes_nothing` |
| `S41119-PICK-SAME-SITE-ONLY` | `crates/api/src/chests_routes.rs` | the pick route takes the state-change guard | `chests_routes::a_cross_site_chest_post_is_refused_and_writes_nothing` |
| `S41120-ACTIVATE-SAME-SITE-ONLY` | `crates/api/src/chests_routes.rs` | the activate route takes the state-change guard | `chests_routes::a_cross_site_chest_post_is_refused_and_writes_nothing` |
| `S41121-ACTIVATE-ALREADY-ACTIVE-409` | `crates/api/src/chests_routes.rs` | an open window answers 409 `already_active` | `chests_routes::the_chest_routes_answer_each_outcome_by_its_status` |

`S41112`'s find is the guard in the holdings query, re-spelled at runtime by its replacement,
never a mutant inside a compile-checked query. `S41113`'s replacement keeps the step used
(`let _ = step;`), so the mutant builds under `-D warnings`. Each route row's find spans the
handler's `fn` line and its guard's line, so it occurs once.
