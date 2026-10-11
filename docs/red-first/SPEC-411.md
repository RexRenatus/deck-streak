# Red-first record: SPEC-411

The SPEC, ADR-425 and the schematic's section were committed first (ce03cbf2), before any test or
code. The criteria's tests were then committed with the stubs that keep every input and lack the
behaviour, each before the code that turns it green. Each red below names ce03cbf2, the commit the
tests and stubs were written over, and quotes the run of the tree the red commit records.

## The fence, line by line

Each of the 20 lines of SPEC-411 section 3's fence resolves to a test this delivery adds or names.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `crates/progression/tests/settle_token_source.rs` `a_double_xp_source_is_derived_only_with_a_positive_token_id` | added (step 2) |
| 2 | A2 | `crates/coordination/tests/chests_steps.rs` `the_chest_step_settles_a_closed_day_once` | added (step 2) |
| 3 | A3 | `crates/coordination/tests/chests_steps.rs` `the_chest_step_rolls_nothing_on_a_backfill_or_a_revisit` | added (step 2) |
| 4 | A4 | `crates/coordination/tests/chests_steps.rs` `a_session_still_open_earns_no_chest_until_its_gap_has_passed` | added (step 2) |
| 5 | A5 | `crates/coordination/tests/chests_steps.rs` `the_sweep_pays_a_stale_chest_once_on_its_own_day` | added (step 2) |
| 6 | A6 | `crates/coordination/tests/chests_steps.rs` `the_token_bonus_step_settles_the_window_and_a_revisit_never_lowers_it` | added (step 2) |
| 7 | A7 | `crates/coordination/tests/chests_steps.rs` `a_failed_draw_keeps_the_day_and_the_next_fold_grants_the_chest` | added (step 2) |
| 8 | A8 | `crates/coordination/tests/chests_use_cases.rs` `opening_a_chest_pays_once_in_the_write_that_opens_it` | added (step 2) |
| 9 | A9 | `crates/coordination/tests/chests_use_cases.rs` `an_epic_freeze_pick_grants_one_freeze_in_the_write_that_settles_it` | added (step 2) |
| 10 | A10 | `crates/coordination/tests/chests_use_cases.rs` `a_freeze_pick_at_the_cap_becomes_a_token_and_grants_no_freeze` | added (step 2) |
| 11 | A11 | `crates/coordination/tests/chests_use_cases.rs` `the_inventory_withholds_a_sealed_or_vaulted_chests_rarity` | added (step 2) |
| 12 | A12 | `crates/coordination/tests/chests_use_cases.rs` `a_second_activation_opens_no_second_window` | added (step 2) |
| 13 | A13 | `crates/api/tests/chests_routes.rs` `the_chest_routes_answer_only_the_owner` | added (step 2) |
| 14 | A14 | `crates/api/tests/chests_routes.rs` `a_cross_site_chest_post_is_refused_and_writes_nothing` | added (step 2) |
| 15 | A15 | `crates/api/tests/chests_routes.rs` `a_replayed_open_pays_once` | added (step 2) |
| 16 | A16 | `crates/api/tests/chests_routes.rs` `the_chest_routes_answer_each_outcome_by_its_status` | added (step 2) |
| 17 | A17 | `crates/daemon/src/wiring.rs` `wiring::tests::the_recompute_fold_registers_the_chest_steps_in_their_phases` | added (step 2) |
| 18 | A18 | `crates/notifications/tests/quiet_window.rs` `the_quiet_window_reads_the_owners_hours_before_the_policys` | added (step 2) |
| 19 | A19 | `crates/quests/tests/tokens_window.rs` `the_holdings_count_unspent_tokens_and_name_the_open_window` | added (step 2) |
| 20 | A20 | `crates/quests/tests/chests_roll.rs` `the_chest_constants_equal_the_golden_and_economy_json` | named |

## The reds and greens

Each line's command is the criterion's line in SPEC-411 section 3's fence. Every red ran one test
(`running 1 test`) and failed on its own assertion.

```red-first
A1: red at ce03cbf2: panicked at crates/progression/tests/settle_token_source.rs:33:5: assertion `left == right` failed: a 2x source is derived only with a positive token id, and no other source moved; left: [("2x:1", false), ("2x:42", false), ("2x:9223372036854775807", false), ...], right: [("2x:1", true), ("2x:42", true), ("2x:9223372036854775807", true), ...]
A2: red at ce03cbf2: panicked at crates/coordination/tests/chests_steps.rs:274:5: assertion `left == right` failed: the closed day's session is rolled once, by two draws, and its XP is granted once; left: ([], [], 0), right: ([(19999, 1727956800000, "rare", 60, "resolved")], [(19999, "chest:1", "language", 60, "once")], 2)
A3: red at ce03cbf2: panicked at crates/coordination/tests/chests_steps.rs:299:5: assertion `left == right` failed: only today's session is rolled; the backfilled and revisited day rolls and grants nothing; left: ([], [], 0), right: ([(20000, 1728032400000, "rare", 60, "sealed")], [], 2)
A4: red at ce03cbf2: panicked at crates/coordination/tests/chests_steps.rs:318:5: assertion `left == right` failed: a gap less one millisecond after its end the session earns nothing; at the gap it earns its chest; left: (([], 0), ([], 0)), right: (([], 0), ([(20000, 1728043200000, "rare", 60, "sealed")], 2))
A5: red at ce03cbf2: panicked at crates/coordination/tests/chests_steps.rs:366:5: assertion `left == right` failed: only the current day sweeps, once, paying the stale chest on its own day and track; left: ([0, 0, 0, 0, 0], [], [(19997, 1727784000000, "rare", 40, "sealed"), ...]), right: ([0, 0, 0, 1, 1], [(19997, "chest:1", "language", 40, "once")], [(19997, 1727784000000, "rare", 40, "resolved"), ...])
A6: red at ce03cbf2: panicked at crates/coordination/tests/chests_steps.rs:413:5: assertion `left == right` failed: the open day's bonus is replaced as it moves, and its settle and revisit never lower it; left: [None, None, None, None], right: [Some(48), Some(24), Some(48), Some(48)]
A7: red at ce03cbf2: panicked at crates/coordination/tests/chests_steps.rs:434:5: assertion `left == right` failed: the refused draw writes no chest and keeps the day's XP; the next fold grants the chest; left: (([], Some(360)), [], 0), right: (([], Some(360)), [(20000, 1728043200000, "rare", 60, "sealed")], 3)
A8: red at ce03cbf2: panicked at crates/coordination/tests/chests_use_cases.rs:173:5: assertion `left == right` failed: the open reveals the stored Rare and pays its 45 once, on the chest's own day; left: (NoSuchChest, NoSuchChest, [], [(1, "sealed", "")]), right: (Opened(OpenedChest { chest_id: 1, rarity: "rare", xp: Some(45), choice_pending: false }), AlreadyOpened, [(19999, "chest:1", "language", 45, "once")], [(1, "resolved", "")])
A9: red at ce03cbf2: panicked at crates/coordination/tests/chests_use_cases.rs:210:5: assertion `left == right` failed: the pick writes one chest freeze beside the start's one, and the replay writes nothing; left: (([], None), NoSuchChest, NoSuchChest, ([], None), [(1, "opened", "")], []), right: (([], None), Chosen { prize: Freeze, capped: false, token_id: None }, AlreadySettled, ([FreezeEvent { day: StudyDay(20000), delta: 1, reason: Chest }], Some(2)), [(1, "resolved", "freeze")], [])
A10: red at ce03cbf2: panicked at crates/coordination/tests/chests_use_cases.rs:255:5: assertion `left == right` failed: the capped freeze becomes the chest's one token, and the freezes stay as seeded; left: (Granted, NoSuchChest, ..., [(1, "opened", "")], []), right: (Granted, Chosen { prize: Token, capped: true, token_id: Some(1) }, ..., [(1, "resolved", "token")], [(1, 1, 0, 0, 0)])
A11: red at ce03cbf2: panicked at crates/coordination/tests/chests_use_cases.rs:302:5: assertion `left == right` failed: the sealed and vaulted chests show no rarity or payout, the opened one shows both; left: Inventory { chests: [], tokens_held: 0, window_ends_at: None }, right: Inventory { chests: [InventoryChest { chest_id: 1, state: "sealed", study_day: StudyDay(19999), rarity: None, xp: None }, ...], tokens_held: 1, ... }
A12: red at ce03cbf2: panicked at crates/coordination/tests/chests_use_cases.rs:347:5: assertion `left == right` failed: the oldest token opens a two-hour window, and the second activation leaves the next held; left: (NoneHeld, NoneHeld, [(1, 0, 0, 0, 0), (2, 0, 0, 0, 0)]), right: (Activated { token_id: 1, window_ends_at: UtcMillis(1728050400000) }, AlreadyActive, [(1, 0, 1728043200000, 1728050400000, 0), (2, 0, 0, 0, 0)])
A13: red at ce03cbf2: panicked at crates/api/tests/chests_routes.rs:274:5: assertion `left == right` failed: every route answers 401 without the owner's session and writes nothing; the owner reads; left: ([200, 200, 200, 200], ..., (200, Null)), right: ([401, 401, 401, 401], ..., (200, Object {"chests": ...}))
A14: red at ce03cbf2: panicked at crates/api/tests/chests_routes.rs:321:5: assertion `left == right` failed: a cross-site open is refused; left: (200, Null), right: (403, Object {"reason": String("cross_site_request")})
A15: red at ce03cbf2: panicked at crates/api/tests/chests_routes.rs:368:5: assertion `left == right` failed: the replayed open answers already opened, and the ledger holds one grant of 45; left: ((200, Null), (200, Null), []), right: ((200, Object {"chest_id": Number(1), ...}), (409, Object {"reason": String("already_opened")}), [(20102, "chest:1", 45)])
A16: red at ce03cbf2: panicked at crates/api/tests/chests_routes.rs:507:5: assertion `left == right` failed: each outcome answers its status and code; left: [("open a bad id", (200, Null)), ...], right: [("open a bad id", (400, Null)), ...]
A17: red at ce03cbf2: panicked at crates/daemon/src/wiring.rs:1090:9: assertion `left == right` failed: the chest and sweep steps follow the writing step, the token bonus the day bonuses; left: [None, None, None], right: [Some(6), Some(7), Some(7)]
A18: red at ce03cbf2: panicked at crates/notifications/tests/quiet_window.rs:67:5: assertion `left == right` failed: the policy's 23:00 to 07:30 answers until the owner sets hours, and the owner's after; left: ([false, true, false], [false, true, false]), right: ([false, true, false], [true, false, true])
A19: red at ce03cbf2: panicked at crates/quests/tests/tokens_window.rs:265:5: assertion `left == right` failed: two tokens are held, and the window opened an hour ago ends an hour from now; left: Holdings { held: 5, window_ends_at: None }, right: Holdings { held: 2, window_ends_at: Some(UtcMillis(1700103600000)) }
A20: not red: pins SPEC-081 A15's table, which the base already holds; it guards that table against this delivery
```
