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
