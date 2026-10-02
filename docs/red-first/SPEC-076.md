# Red-first record: SPEC-076

The stubs and red tests were committed before the rules they test: the streaks crate's tests at
adc879a6, and the coordination, api, bot and web tests, with the migration and the compiling stubs,
at 46b3a57d. Every criterion below was replayed against 46b3a57d's tree and fails by assertion,
and against the merged tree 5ac898b6 it passes. Each red line is the line the replay printed
for that criterion's own test.

```red-first
A1: red at 46b3a57d: thread 'the_language_streak_transitions_match_the_parity_goldens' (3010657) panicked at crates/streaks/tests/streak_goldens.rs:80:9:
A1: green at 5ac898b6
A2: red at 46b3a57d: thread 'a_day_without_study_breaks_only_an_unsavable_streak' (3010732) panicked at crates/streaks/tests/streak_goldens.rs:114:9:
A2: green at 5ac898b6
A3: red at 46b3a57d: thread 'the_heat_and_the_comeback_view_match_the_parity_goldens' (3011311) panicked at crates/streaks/tests/streak_goldens.rs:147:9:
A3: green at 5ac898b6
A4: red at 46b3a57d: thread 'the_streak_constants_and_economy_json_match_the_predecessors' (3011970) panicked at crates/streaks/tests/streak_goldens.rs:185:5:
A4: green at 5ac898b6
A5: red at 46b3a57d: thread 'a_settled_day_writes_its_freeze_events_once' (3054166) panicked at crates/coordination/tests/streak_fold.rs:145:5:
A5: green at 5ac898b6
A6: red at 46b3a57d: thread 'a_streak_studied_daily_survives_one_sync_a_day' (3055078) panicked at crates/coordination/tests/streak_fold.rs:163:5:
A6: green at 5ac898b6
A7: red at 46b3a57d: thread 'the_freeze_port_holds_the_hold_cap_and_the_monthly_drop_cap' (3055986) panicked at crates/streaks/tests/freeze_port.rs:40:9:
A7: green at 5ac898b6
A8: red at 46b3a57d: thread 'the_drop_gate_matches_the_parity_golden' (3056108) panicked at crates/streaks/tests/freeze_port.rs:86:9:
A8: green at 5ac898b6
A9: red at 46b3a57d: thread 'the_law_streak_matches_the_parity_golden' (3056567) panicked at crates/streaks/tests/streak_law.rs:37:9:
A9: green at 5ac898b6
A10: red at 46b3a57d: thread 'the_law_streak_decays_without_law_study' (3056769) panicked at crates/coordination/tests/streak_fold.rs:188:5:
A10: green at 5ac898b6
A11: red at 46b3a57d: thread 'the_law_row_never_spends_or_receives_a_freeze' (3056901) panicked at crates/streaks/tests/streak_law.rs:60:5:
A11: green at 5ac898b6
A12: red at 46b3a57d: thread 'strength_and_the_verdict_match_the_parity_goldens' (3057145) panicked at crates/streaks/tests/governor_goldens.rs:47:9:
A12: green at 5ac898b6
A13: red at 46b3a57d: thread 'the_anchor_beyond_the_walk_matches_the_parity_golden' (3057273) panicked at crates/streaks/tests/governor_goldens.rs:110:9:
A13: green at 5ac898b6
A14: red at 46b3a57d: thread 'one_episode_keeps_its_anchor_across_recomputes' (3057468) panicked at crates/coordination/tests/streak_fold.rs:226:5:
A14: green at 5ac898b6
A15: red at 46b3a57d: thread 'the_standby_notice_rule_matches_the_parity_golden' (3057648) panicked at crates/streaks/tests/governor_goldens.rs:173:9:
A15: green at 5ac898b6
A16: red at 46b3a57d: thread 'the_relight_rule_matches_the_parity_golden' (3058144) panicked at crates/streaks/tests/relight_rule.rs:22:9:
A16: green at 5ac898b6
A17: red at 46b3a57d: thread 'a_lapse_relights_once_per_episode' (3059127) panicked at crates/coordination/tests/relight_settle.rs:174:5:
A17: green at 5ac898b6
A18: red at 46b3a57d: thread 'a_return_day_relights_at_its_settle_with_its_whole_count' (3059337) panicked at crates/coordination/tests/relight_settle.rs:190:9:
A18: green at 5ac898b6
A19: red at 46b3a57d: thread 'an_erase_returns_both_tracks_to_their_start_state' (3059692) panicked at crates/streaks/tests/streak_rights.rs:24:5:
A19: green at 5ac898b6
A20: red at 46b3a57d: thread 'the_streak_routes_answer_only_the_owner' (3082296) panicked at crates/api/tests/streak_routes.rs:256:5:
A20: green at 5ac898b6
A21: red at 46b3a57d: thread 'streak_shows_both_tracks_law_first_when_law_is_active' (3128120) panicked at crates/bot/tests/streak_commands.rs:61:35:
A21: green at 5ac898b6
A22: red at 46b3a57d: AssertionError: the two tracks are shown: expected +0 to be 2 // Object.is equality (measured in an export with its own install; the replay's own line for A22 was a module-resolution error, not an assertion)
A22: green at 5ac898b6
A23: red at 46b3a57d: assertion `left == right` failed: class Some("empty-history-no-stored-anchor"), input {"skip_days":[],"stored_anchor":null,"study_days":[],"today":20000} ; left: Some(20000) ; right: Some(19880) (measured by hand: the criterion's fence carried no acceptance tag until this delivery)
A23: green at 5ac898b6
A24: red at 46b3a57d: thread 'a_relight_is_granted_on_the_folds_connection' (3180787) panicked at crates/coordination/tests/relight_settle.rs:214:5:
A24: green at 5ac898b6
A25: red at 46b3a57d: thread 'a_second_recompute_routes_the_relight_and_one_send_is_recorded' (3198922) panicked at crates/coordination/tests/relight_settle.rs:263:9:
A25: green at 5ac898b6
```

## Disclosures

- **A14's constant.** The criterion's test asserts the walk's horizon anchor as `D0 - 121`; the
  constant is corrected to that value, and the red line above is the test failing at the stub.
- **Three symmetry tests, red as collateral.** The migration 007601 arrived with the red commit
  46b3a57d, and the six data-rights files for its four tables only at 7884d53d. Between the two,
  the schema held tables no port declared. Measured at 46b3a57d, `data_rights_symmetry` runs four
  tests and three fail:
  - `every_table_of_the_schema_is_declared_by_exactly_one_port` (data_rights_symmetry.rs:370):
    left `["freeze_events is declared by no port", "governor_state is declared by no port",
    "habit_strength is declared by no port", "streak_state is declared by no port"]`, right `[]`;
  - `the_exported_tables_equal_the_erased_tables_over_every_port` (data_rights_symmetry.rs:272);
  - `erase_leaves_the_cron_fire_ledger_and_the_schema_table_untouched` (data_rights_symmetry.rs:272).
  The fourth, `privacy_json_names_every_table_the_ports_export_or_erase`, passes. They are not
  the criteria's own tests; they turned green with the six files at 7884d53d.
- **External freezes are an approximation.** The fold replays the freezes it can derive from study
  days. What a chest, the weekly quest, a season node or a shop purchase paid is added back from
  the ledger as a net sum (`external_freezes`) and the language row is clamped to the hold cap of
  three, so a freeze the outside sources paid and the replay spent is approximated by the net.
- **The `delta <> 0` CHECK was removed.** A break marker is a freeze event with a delta of zero, so
  the ledger's signed delta may be zero; the migration says so in its header.
- **A22 and A23 in the replay.** The replay ran A22 through a symlinked install and got a module
  resolution error at both revisions, so A22's red line was measured in an export with its own
  install, and its green line is the test passing at the head. A23's fence carried no acceptance
  tag when the replay ran, so the replay skipped it; the tag is added in this delivery and A23's
  red line was run by hand at 46b3a57d.
- **Test edits after the red commit.** Commit 14081b2 changes the two anchor assertions of
  `one_episode_keeps_its_anchor_across_recomputes` in `crates/coordination/tests/streak_fold.rs`
  from `D0 - 120` to `D0 - 121`, the same corrected constant as A14's. Commit 93f4ee6 changes the
  menu assertion in `crates/bot/tests/commands.rs` from six commands to seven and the expected
  help and start messages under `crates/bot/tests/messages/`, because the streak command joins the
  menu; those files are the streak command's own surface.

## Addendum, 2026-09-30: the mutation-hardening tests (A26 to A40)

These tests pin rules that already held, so each is green at the head and is recorded `not red`,
with the mutant that turns it red, applied by hand under a 16 GiB address-space cap.

- **A26 not red.** `civil_month`'s `era * 400` replaced by `era + 400` (freeze.rs:19:26) fails at
  `population_rules.rs:107`: `day -134712 against day 11017`.
- **A27 to A32 not red.** `persists`' `age <= 1` replaced by `age < 1` fails
  `the_fold_stores_today_yesterday_and_the_first_run_window` at `population_rules.rs:432`
  (`left: false, right: true`). The other mutants of the crate's rules named in the run of the
  changed files are caught, and eight `civil_month` mutants are recorded as equivalent.
- **A33 not red.** The store mutants of the run are caught by the raw read-back oracles.
- **A34 not red.** `AtStake::as_str`'s `"none"` replaced by `""` fails
  `what_is_at_stake_is_named_in_the_views_words` at `streak_views.rs:136`
  (`left: ["freeze", "break", ""]`).
- **A36 not red.** The bot's `view.law.current > 0` replaced by `>= 0` fails
  `the_law_leads_only_above_zero_and_the_noun_follows_the_freezes` at `streak_commands.rs:104`.
- **A37 not red.** The route's `streak_unreadable` replaced by `x` fails
  `the_streak_routes_name_why_they_cannot_answer` at `streak_routes.rs:325`.
- **A38 and A39 not red.** `0..=SILENCE_WALK_CAP_DAYS` replaced by `0..SILENCE_WALK_CAP_DAYS` fails
  both, A38 with `silent_days: 120` against `121`, A39 at distance 121; `number -= 1` replaced by
  `number += 1` fails both with `first_silent: StudyDay(120)` against `StudyDay(-120)`. The bound
  itself is a production change, made after the test-only commits.
- **A40 not red.** The web tests are new files with no prior head: `$` dropped from the date
  regex, `law.current >= 0`, the freezes `&&` to `||`, the stake test to `true`, `?? 1`, the
  governor URL typo, `{void 0}`, and `typeof ... !== 'string'` to `false` each turn one red.

```red-first
A26: not red: the test pins a rule the code already held, so it is green at the head; the mutant that turns it red is named in this addendum
A27: not red: the test pins a rule the code already held, so it is green at the head; the mutant that turns it red is named in this addendum
A28: not red: the test pins a rule the code already held, so it is green at the head; the mutant that turns it red is named in this addendum
A29: not red: the test pins a rule the code already held, so it is green at the head; the mutant that turns it red is named in this addendum
A30: not red: the test pins a rule the code already held, so it is green at the head; the mutant that turns it red is named in this addendum
A31: not red: the test pins a rule the code already held, so it is green at the head; the mutant that turns it red is named in this addendum
A32: not red: the test pins a rule the code already held, so it is green at the head; the mutant that turns it red is named in this addendum
A33: not red: the test pins a rule the code already held, so it is green at the head; the mutant that turns it red is named in this addendum
A34: not red: the test pins a rule the code already held, so it is green at the head; the mutant that turns it red is named in this addendum
A35: not red: the test pins a rule the code already held, so it is green at the head; the mutant that turns it red is named in this addendum
A36: not red: the test pins a rule the code already held, so it is green at the head; the mutant that turns it red is named in this addendum
A37: not red: the test pins a rule the code already held, so it is green at the head; the mutant that turns it red is named in this addendum
A38: not red: the test pins a rule the code already held, so it is green at the head; the mutant that turns it red is named in this addendum
A39: not red: the test pins a rule the code already held, so it is green at the head; the mutant that turns it red is named in this addendum
A40: not red: the test pins a rule the code already held, so it is green at the head; the mutant that turns it red is named in this addendum
```

- **A40's two refusal tests, paired with a valid body.** Not red: they pin that the governor reader
  answers a well-formed body with the exact view, beside the refusals that assert only a null; their
  red is `parseGovernor` returning null for every body (a `return null;` at the head of the
  function), which fails both with `expected null to deeply equal { verdict: 'armed', ... }`.

## Addendum, 2026-09-30: the named mutants of A27 to A35, and A41 to A43

The addendum above names no mutant of its own for A27, A28, A29, A30, A32, A33 and A35. Each is
named here, applied by hand one at a time under a 16 GiB address-space cap:

- **A27.** `freeze_events_for`'s `unwrap_or(i32::MAX)` replaced by `unwrap_or(i32::MAX - 1)` fails
  at `population_rules.rs:166` (`freezes 0 to 4294967295`).
- **A28.** The law replay's `else if !skips.contains(&day)` replaced by `else` fails at
  `population_rules.rs:239` (`days {20000, 20002}, skips {20001}, through 20004`).
- **A29.** `real_misses(last, today, skips) < 2` replaced by `< 3` fails at `population_rules.rs:353`
  (`study days {20000}, through 20003`).
- **A30.** The heat's `days >= *threshold` replaced by `days > *threshold` fails at
  `population_rules.rs:412` (`1 days`).
- **A32.** The walk's `exhausted: !study_days.contains(...)` replaced by `exhausted: true` fails at
  `population_rules.rs:475` (`study day Some(0) back, skips {}`).
- **A33.** The store's `i64::from(event.delta)` replaced by `i64::from(event.delta).abs()` fails
  `the_freeze_events_are_stored_once_and_read_in_order` at `store_effects.rs:232`.
- **A35.** `STREAKS_STEP` renamed to `streaks.streaks_and_the_governor` fails at
  `streak_fold.rs:242`. The outside freezes' `.saturating_add(outside)` replaced by
  `.saturating_add(outside.max(0))` leaves A35's fence green and fails
  `the_outside_freezes_join_the_language_row_within_zero_and_three` at `streak_fold.rs:269`
  (`outside net -5`, `left: 1`, `right: 0`), which A43 now fences.
- **A41 not red.** The test pins that the bounded lapse walk answers what the earlier loop
  answered. Its red is the conversion without the smallest-day guard, which fails at
  `open_lapse_bound.rs:94` with `left: Some(StudyDay(-9223372036854775808))`, `right: None`.
- **A42 not red.** The reply was right. Its red is each of three mutants that pass A36 and the whole
  bot crate: `view.law.current > 0` replaced by `view.law.longest > 0` (`streak_commands.rs:171`,
  `law 0 best 1`), the language line's run and best swapped (`streak_commands.rs:176`, `language
  best 10`), and the law line's run and best swapped (`streak_commands.rs:171`, `left: "Law: 1 (best
  0)"`).
- **A43 not red.** The rule held. Its red is the A35 mutant above.

```red-first
A41: not red: the test pins a rule the conversion keeps, so it is green at the head; the mutant that turns it red is named in this addendum
A42: not red: the test pins a rule the code already held, so it is green at the head; the mutant that turns it red is named in this addendum
A43: not red: the test pins a rule the code already held, so it is green at the head; the mutant that turns it red is named in this addendum
```

## Addendum, 2026-09-30: the relight's order, the served pairs and the studied day (A44 to A49), A30's two sites and A33's due list

- **A44 to A46.** The tests were committed at e146cbca with the head's order: the due days held in
  memory and answered inside the per-day write. The cycle's take was reshaped into a read of the
  days still due and a mark once the router decides a day, both still in memory, so the tests
  compile and fail by assertion. The stored due list is the fix at fb0244ae.
- **A33, the due list.** The store's three due-list functions are new at fb0244ae, so
  `the_relight_due_list_holds_each_day_once_until_it_is_cleared` is green at its commit d6b1c572. Its
  red is each function replaced by its constant answer, `put_relight_due` and `clear_relight_due` by
  `Ok(())` and `relight_due` by `Ok(vec![])`: each left the streaks crate green before this test and
  fails it now. A33's `not red` line above stands for it.
- **A47 and A49.** Each is red by assertion at its test commit, 5ace2cc3 and 991bdb2c: a day that
  already has a study review was served, and put in the view, as a break or a freeze (R30). The fix
  is at 66bc4479.
- **A48 not red.** The reader was right. Its red is `parseStreak` reading the language track's
  freeze cap as its heat and its heat as its freeze cap, which passed every earlier web test and
  fails `reads each served value of the streak into its own place, for every member`.
- **A30, both sites.** The heat's `days >= *threshold` replaced by `days > *threshold` matches two
  sites. The failure the addendum above quotes, at `population_rules.rs:412`, is `heat_tier`'s.
  `heat_for`'s mutant passes A30 and fails A1, A2 and A3, at `streak_goldens.rs:147` for A3.

```red-first
A44: red at e146cbca: thread 'a_celebration_is_sent_only_for_a_grant_that_committed' (1163965) panicked at crates/coordination/tests/relight_order.rs:362:5:
A44: green at fb0244ae
A45: red at e146cbca: thread 'a_crash_between_the_commit_and_the_route_is_recovered_at_the_next_cycle' (1163966) panicked at crates/coordination/tests/relight_order.rs:398:5:
A45: green at fb0244ae
A46: red at e146cbca: thread 'every_failure_point_later_sync_and_crash_keeps_one_celebration_per_committed_grant' (1163967) panicked at crates/coordination/tests/relight_order.rs:414:5:
A46: green at fb0244ae
A47: red at 5ace2cc3: thread 'every_served_value_is_read_where_each_pair_of_served_values_differs' (1746089) panicked at crates/api/tests/streak_routes.rs:583:13:
A47: green at 66bc4479
A48: not red: the test pins a rule the reader already held, so it is green at the head; the mutant that turns it red is named in this addendum
A49: red at 991bdb2c: thread 'a_studied_day_puts_nothing_at_stake_on_either_track' (1811440) panicked at crates/coordination/tests/streak_views.rs:252:21:
A49: green at 66bc4479
```


## Addendum, 2026-09-30: the failed route and the reply's heat (A50 to A52)

- **A50 not red.** The reply already carried each run's heat in its place. Its red is each of five
  mutants of the language line's heat, which passed every earlier bot test: the heat's place given
  the language track's at-stake word, the freezes' noun, the language best or the law run, and a
  run above zero shown with no heat (the rows S07625 to S07629). Each fails A50 at
  `streak_commands.rs:251`.
- **A51 and A52 not red.** The cycle already left a day whose route failed on the due list. Their
  red is a failed route that clears the day, spelled in the error arm (S07622), before the route
  (S07623), or by reading the route's error as a decided route that sent nothing (S07624). The
  first passed every earlier test of the coordination, streaks, daemon and notifications crates.
  Each fails A51 at `relight_order.rs:749` (`left: []`, `right: [StudyDay(20000)]`) and A52 at
  `relight_order.rs:798`, in each of its 24 members whose claim fails. A member whose decision
  record fails stays green under each: its claim and its line were written before the failure, so
  no celebration is lost.
- **A44 to A46, their helper.** `relight_order.rs`'s `cycle` now runs its fold through a helper
  that A51 and A52 share. What A44 to A46 run and assert is unchanged, and their red lines above
  stand at their red commit.

```red-first
A50: not red: the test pins a rule the reply already held, so it is green at the head; the mutant that turns it red is named in this addendum
A51: not red: the test pins a rule the cycle already held, so it is green at the head; the mutant that turns it red is named in this addendum
A52: not red: the test pins a rule the cycle already held, so it is green at the head; the mutant that turns it red is named in this addendum
```

## Addendum, 2026-10-01: a failing day holds back none, the give-up guard and every heat band (A53 to A56); A50 and A52 derived

- **A50 and A52, derived.** Their `not red` lines above stand. A50's replies are now derived from
  the streak crate's band table, run 0 and each band's first and last run (the open top band's first
  and the run after it) at both leads and three freeze counts, beside round 4's eight replies, and
  its expected heats are written out by hand: 74 replies over 6 heats. A52's cases are derived from
  the count of consecutive failed routes: 72 cases. The mutants named above still turn each red, at
  lines that moved: S07625 to S07629 fail A50 at `streak_commands.rs:390`, with A56 beside it, and a
  failed route that clears the day, in each of the three spellings of S07622 to S07624, fails A51 at
  `relight_order.rs:768` (`left: []`, `right: [StudyDay(20000)]`) and A52 at `relight_order.rs:833`,
  in each of its 36 members whose claim fails, and A53 to A55 beside them.
- **A53 not red.** The cycle already went on to the next due day after a failed route. Its red is the
  route's error returned from the cycle (S07630), or the route ended at its first failure (S07631).
  Each passes A51 and A52 and fails A53 at `relight_order.rs:860` (`left: (0, 0)`,
  `right: (1, 1)`).
- **A54 not red.** No route gave a day up. Its red is a day dropped from the due list after two
  failed routes, or after three, which fails A54 at `relight_order.rs:917` (`left: []`,
  `right: [StudyDay(20000)]`).
- **A55 not red.** The route already kept no count of a day's failed routes. Its red is a count kept
  in memory (S07632) or in the database (S07633), and a count kept in four other places: the due
  list's own type, the router, a static in another module and a thread-local. A give-up after a
  thousand failed routes passes A51 to A54, because no case fails a route that often, and fails A55
  at `relight_order.rs:1359`, as each of the others does.
- **A56 not red.** The reply already carried every band's heat in the heat's own place, whichever
  line leads. Its red is the heat table changed: a band's first run moved, a band's heat changed,
  two bands out of the longest-first order, or a band removed. Each fails A56 at
  `streak_commands.rs:478` and A50 beside it. A56's runs are written out by hand, so it judges the
  bands it names: a band added between two of its runs, with its heat written out in A50's oracle,
  passes A50 and fails A56 alone, and a band changed on purpose is changed in A56 by hand. A band
  added above its runs is A50's to judge: with no heat written out in A50's oracle, it fails A50.

```red-first
A53: not red: the test pins a rule the cycle already held, so it is green at the head; the mutant that turns it red is named in this addendum
A54: not red: the test pins a rule the cycle already held, so it is green at the head; the mutant that turns it red is named in this addendum
A55: not red: the test pins a rule the route already held, so it is green at the head; the mutant that turns it red is named in this addendum
A56: not red: the test pins a rule the reply already held, so it is green at the head; the mutant that turns it red is named in this addendum
```

## Addendum, 2026-10-01: the streak calendar and its markers (A57 to A64)

- **A57 and A60, red then green (first round).** Both ran at `83cd74b` over a calendar that returned
  the window with no markers. A60 failed at `calendar_population.rs:239` (`left: []`,
  `right: [(20001, [Freeze])]`) and A57 at `calendar_population.rs:187` (`language, served 20002`,
  the freeze missing on 20001). Both were green at `51b4241`. After the red, the population gained
  the expectations for the real misses that trail a history's last study day (the language run
  breaks at the third day after it, the law run resets on the day after the first); the first
  oracle had left them out, and the production code was right.
- **A58 and A61 did not exist at `83cd74b`.** That commit adds only the streaks crate's calendar
  and its population test, so the first round's record called both "not red". Each was then shown
  red by assertion over stubs, which this round re-ran. A58, the first round's route test over
  `83cd74b`'s unmarked calendar (`cargo test --locked -p deck-streak-daemon --test
  streak_calendar_route -- --test-threads 1`), exits 101:
  `the_route_serves_each_days_markers_after_the_fold` panicked at `streak_calendar_route.rs:317:9`,
  `the served freezes are the spent freezes' covered days`, `left: {}`, `right: {"2024-12-23"}`.
  A61, the first round's screen test over the screen before the calendar (`pnpm exec vitest run
  src/lib/streak/streak-calendar.test.ts -t "the screen reads each marker from its own day's
  cell"`), exits 1: `AssertionError: member 0 language: expected [] to have a length of 35 but got
  +0`.
- **The second round, one red commit for every test it adds or amends.** `1024ecc` commits the
  tests for the law track's break day (A62), the predecessor's window (A57 and A58 amended, A63)
  and the week grid (A61 amended, A64) over the first round's code, which serves 35 days, puts the
  law break on the miss itself and draws a marker as a whole word. Each fails by assertion there.
  The fixes follow one rule each: `2eb8959` moves the law break, `a038df4` serves the
  predecessor's window and `3511bf3` lays the screen out as whole weeks.
- **A63's body was split after its red commit.** `14ab257` moves A63's serving loop and its two
  controls into helper functions, under clippy's bound on a function's length; no assertion
  changed. The new body, run over `1024ecc`'s tree, fails the same way: it panicked at
  `streak_calendar_route.rs:666:5`, `the route differs from the predecessor: Judged { cases: 11,
  days: 770, window_mismatches: 22, unnamed: 802, named: 0 }`. The fence line below quotes the
  earlier body, which failed at line 659 with the same message.
- **A59 not red.** The settlement already held its iff when the test was written. Its red is the
  threshold flipped to `>= 0` (S07644) or the track filter dropped (S07645), each killed by A59
  alone; the fold writes neither a zero row nor another track's row, so A59 plants them.
- **The phone width is not measured red here.** `web/app/tests/streak-calendar.spec.ts` (360 px, no
  sideways scroll, each cell in its weekday's column and week's row) runs in a browser in the web
  stage's end-to-end run. Its local red was not measured: the browser's page crashed on launch
  under the local run's memory limit (`browserContext.newPage: Target crashed`, from `pnpm exec
  playwright test tests/streak-calendar.spec.ts`). It holds no acceptance line.

```red-first
A57: red at 1024ecc: assertion failed: language, served 20000, the window starts at 19966 where the predecessor's starts at 19814
A57: green at a038df4
A58: red at 1024ecc: assertion failed: language: the window's length, left: 35, right: 184
A58: green at a038df4
A59: not red: the settlement already held its iff, so it is green at the head; the mutants that turn it red are named in this addendum
A60: red at 83cd74b: assertion failed: left: [], right: [(20001, [Freeze])] on the open day's window
A60: green at 51b4241
A61: red at 1024ecc: AssertionError: expected [] to deeply equal [ 'S' ], the marker's letter missing from its own day's cell
A61: green at 3511bf3
A62: red at 1024ecc: assertion failed: served 20003, left: [20002], right: [20003]
A62: green at 2eb8959
A63: red at 1024ecc: the route differs from the predecessor: Judged { cases: 11, days: 770, window_mismatches: 22, unnamed: 802, named: 0 }
A63: green at a038df4
A64: red at 1024ecc: AssertionError: member 0 language 2024-07-15: expected [] to deeply equal [ 'col-start-1' ]
A64: green at 3511bf3
```

## Addendum, 2026-10-01: the open lapse walk against its Lean port (A65 to A67)

- **The red commit.** 4e688bc3 committed the vectors test with the Lean package: a port of
  `open_lapse` without its guard at the smallest day, its three theorems not yet proved, their three
  witnesses, and the vectors that port wrote. The Rust function answers none where that port
  answers its smallest day, so the test is red by assertion, not by a compile error. The green
  commit bd283264 gives the port the guard, proves the theorems and writes the vectors again. The
  test's text is the same at both commits, and no Rust source changes.
- **A65 and A67, red.** At 4e688bc3 the test examined 1704 vectors, and 73 of them differ: the
  first is today at the smallest day with a count of zero on it, where the port answers
  `Some(-9223372036854775808)` and the Rust function `None`. A67 fails on the same case:
  `left: Some(-9223372036854775808)`, `right: None`.
- **A66 not red.** The axes already held every case the property names at the red commit. Its red
  is a population without the smallest day: with the window's first days moved one day up, it
  examines 1800 vectors and fails at `formal_vectors_open_lapse.rs:342` with "no vector holds the
  window starts at the smallest day".

```red-first
A65: red at 4e688bc3: thread 'the_open_lapse_walk_answers_every_vector_its_lean_port_wrote' (3020364) panicked at crates/streaks/tests/formal_vectors_open_lapse.rs:267:5:
A65: green at bd283264
A66: not red: the test pins a population the axes already held, so it is green at the head; the change that turns it red is named in this addendum
A67: red at 4e688bc3: thread 'a_walk_that_reaches_the_smallest_day_without_a_review_answers_none_in_the_vectors' (3020363) panicked at crates/streaks/tests/formal_vectors_open_lapse.rs:364:5:
A67: green at bd283264
```

## Addendum, 2026-10-02: the silent-day count at its bound (A68)

- **The red commit.** d6389452 commits the test beside a stub `next_silent_count` that compiles and
  answers `silent.wrapping_add(1)`, and `open_lapse` calls it. The test is red by assertion, not by a
  compile error: at the bound the stub answers 0 where the rule answers `u32::MAX`.
- **The green commit.** 7bd0d710 gives the step `saturating_add(1)`; the test's text is the same at
  both commits.
- **A68, red.** `assert_eq!(next_silent_count(u32::MAX), u32::MAX)` fails at
  `open_lapse_bound.rs:123:5` with `left: 0` and `right: 4294967295`.

```red-first
A68: red at d6389452: thread 'the_silent_day_count_saturates_at_its_bound' panicked at crates/streaks/tests/open_lapse_bound.rs:123:5: assertion `left == right` failed: left: 0, right: 4294967295
A68: green at 7bd0d710
```
