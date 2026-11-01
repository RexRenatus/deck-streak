# Red-first record: SPEC-585

SPEC-585 (R1 to R14, A1 to A6). Its acceptance fence names thirteen tests: A1 is decided by
`test_slo_declaration.py`'s `test_every_alert_routes_to_the_one_alert_path` and
`test_the_mcp_slo_pages_and_tickets_on_the_one_alert_path_with_exact_burn_rates`; A2 by
`test_pager_declaration.py`'s `test_the_pager_carries_the_route_every_slo_alert_names`,
`test_the_pager_matches_the_alert_templates_line_at_error_priority` and
`test_a_failed_alert_instance_pages_without_its_own_line`; A3 by its
`test_the_evaluators_silence_pages_within_four_of_its_periods` and
`test_a_daily_backup_two_hours_late_pages`; A4 by its
`test_only_deck_streak_lines_ship_and_none_outlives_the_journal` and
`test_a_page_reaches_a_person_by_two_channels_of_two_kinds`; A5 by its
`test_the_request_counters_carry_no_request_id_and_no_identity`; and A6 by
`test_privacy_policy.py`'s `test_the_policy_bounds_the_monitoring_copy_of_the_journal`,
`test_the_policy_states_how_use_is_measured` and
`test_the_policy_and_the_privacy_manifest_agree_no_usage_is_collected`. Each is a RED-FIRST test.

The tests, the renamed A1 test with SPEC-582's A17 lines and S33301's killer re-pointed to it, and
a stub `docs/monitoring/pager.json` were committed alone
(ce779f7e40ed0984e012a6b7921e2fb27b36a6d6). The stub carries every key of the declaration with the
values the journal-only install implies: the route `journal`, no retention period, every unit
shipped, no page, no channel and no counter label. `deploy/slo.json` and `PRIVACY.md` were the
base's. The tests read files only and execute no deploy script, so each module ran on the build
host over a copy of that commit's files: `test_pager_declaration` whole read
`FAILED (failures=8)` over 8 tests, `test_slo_declaration` whole read `FAILED (failures=2)` over 8
tests, and the three new tests of `test_privacy_policy`, run by id, read `FAILED (failures=3)` over
3 tests. Each of the thirteen failed by its own assertion, never by an error. The declaration, the
routes of `deploy/slo.json` and the privacy page's two changes were committed next
(4b59b2864c376a54a4b8519aa19ff4321133ef89), and over a copy of its files the same three runs read
`OK` over 8, 8 and 3 tests.

```red-first
A1: red at ce779f7e40ed0984e012a6b7921e2fb27b36a6d6: AssertionError: {'unit': 'deck-streak-alert@.service', 'channel': 'journal'} != {'unit': 'deck-streak-alert@.service', 'channel': 'pager'}
A2: red at ce779f7e40ed0984e012a6b7921e2fb27b36a6d6: AssertionError: 'journal' != 'pager'
A3: red at ce779f7e40ed0984e012a6b7921e2fb27b36a6d6: AssertionError: examined 0 page(s) declared: the population is empty, so nothing was judged
A4: red at ce779f7e40ed0984e012a6b7921e2fb27b36a6d6: AssertionError: '*' != 'deck-streak-*'
A5: red at ce779f7e40ed0984e012a6b7921e2fb27b36a6d6: AssertionError: {} != {'unit': 'the unit', 'route': 'span.route'[47 chars]tus'}
A6: red at ce779f7e40ed0984e012a6b7921e2fb27b36a6d6: AssertionError: 0 != 1 : one bullet bounds the monitoring service's copy of the journal
A1: green at 4b59b2864c376a54a4b8519aa19ff4321133ef89
A2: green at 4b59b2864c376a54a4b8519aa19ff4321133ef89
A3: green at 4b59b2864c376a54a4b8519aa19ff4321133ef89
A4: green at 4b59b2864c376a54a4b8519aa19ff4321133ef89
A5: green at 4b59b2864c376a54a4b8519aa19ff4321133ef89
A6: green at 4b59b2864c376a54a4b8519aa19ff4321133ef89
```

Each criterion's line above quotes the first of its tests. Every test's own red at the red commit,
read from the same runs, is quoted here, below the criteria, so that no criterion is recorded twice:

```text
test_slo_declaration.TheApiSloBurnsAsDeclared.test_every_alert_routes_to_the_one_alert_path: AssertionError: {'unit': 'deck-streak-alert@.service', 'channel': 'journal'} != {'unit': 'deck-streak-alert@.service', 'channel': 'pager'}
test_slo_declaration.TheMcpSloBurnsAsDeclared.test_the_mcp_slo_pages_and_tickets_on_the_one_alert_path_with_exact_burn_rates: AssertionError: {'pag[25 chars].6, 'journal'), 'ticket': ('3d', '6h', 0.1, 0.9333, 'journal')} != {'pag[25 chars].6, 'pager'), 'ticket': ('3d', '6h', 0.1, 0.9333, 'pager')}
test_pager_declaration.ThePagerIsDeclared.test_the_pager_carries_the_route_every_slo_alert_names: AssertionError: 'journal' != 'pager'
test_pager_declaration.ThePagerIsDeclared.test_the_pager_matches_the_alert_templates_line_at_error_priority: AssertionError: examined 0 page(s) declared: the population is empty, so nothing was judged
test_pager_declaration.ThePagerIsDeclared.test_a_failed_alert_instance_pages_without_its_own_line: AssertionError: examined 0 page(s) declared: the population is empty, so nothing was judged
test_pager_declaration.ThePagerHearsSilence.test_the_evaluators_silence_pages_within_four_of_its_periods: AssertionError: examined 0 page(s) declared: the population is empty, so nothing was judged
test_pager_declaration.ThePagerHearsSilence.test_a_daily_backup_two_hours_late_pages: AssertionError: examined 0 page(s) declared: the population is empty, so nothing was judged
test_pager_declaration.TheShippedLinesAreBounded.test_only_deck_streak_lines_ship_and_none_outlives_the_journal: AssertionError: '*' != 'deck-streak-*'
test_pager_declaration.TheShippedLinesAreBounded.test_a_page_reaches_a_person_by_two_channels_of_two_kinds: AssertionError: 0 not greater than or equal to 2
test_pager_declaration.TheRequestCountersCarryNoIdentity.test_the_request_counters_carry_no_request_id_and_no_identity: AssertionError: {} != {'unit': 'the unit', 'route': 'span.route'[47 chars]tus'}
test_privacy_policy.ThePolicyStatesHowUseIsMeasured.test_the_policy_bounds_the_monitoring_copy_of_the_journal: AssertionError: 0 != 1 : one bullet bounds the monitoring service's copy of the journal
test_privacy_policy.ThePolicyStatesHowUseIsMeasured.test_the_policy_states_how_use_is_measured: AssertionError: Lists differ: ['route', 'method', 'status', 'no account'[124 chars]ult'] != []
test_privacy_policy.ThePolicyStatesHowUseIsMeasured.test_the_policy_and_the_privacy_manifest_agree_no_usage_is_collected: AssertionError: 'the apps measure nothing about how you use them' not found in ''
```

The five tests of `test_pager_declaration` that name a page fail first on the stub's empty list of
pages, through the module's `page` helper, which refuses a population of zero: that is the stub's
missing page, the behaviour each criterion asks for, and not a missing file.

## The rows

The band `S58500` to `S58515` holds one row per criterion's property. Each row was proved by hand
in a copy of the green commit's files: the target's sha256 recorded, the killer run by id without
the mutant (`Ran 1 test`, `OK`), the replacement installed once, the killer run by id again
(`Ran 1 test`, `FAILED (failures=1)`, no error), and the saved bytes written back with the sha256
compared equal. The three targets read, after the last restore, the bytes of the green commit:
`deploy/slo.json` `95863a1f4e8aeb668b196172441254bbc1b725c75032c2ebfdbd1422ab3a137d`,
`docs/monitoring/pager.json` `1dae8a86a2a2de02915cc1af03b3a0e55a0aada86b7b933e63b974afd35a0f73`
and `PRIVACY.md` `12b3a534329abca4c862a834308ccfa903e12385084972eb290dea7632e606ee`. Each killer's
failing assertion under its mutant:

- S58500 (`"channel": "journal"`): `AssertionError: {'unit': 'deck-streak-alert@.service', 'channel': 'journal'} != {'unit': 'deck-streak-alert@.service', 'channel': 'pager'}`.
- S58501 (the API's ticket routed to `journal`): `AssertionError: Items in the first set but not the second: 'journal'`, the route set.
- S58502 (the MCP server's ticket routed to `journal`): the declared alerts differ in the ticket's
  route, `('3d', '6h', 0.1, 0.9333, 'journal')` against `'pager'`.
- S58503 (the declaration's route `journal`): `AssertionError: Items in the first set but not the second: 'pager' Items in the second set but not the first: 'journal'`.
- S58504 (`priority_at_most` 2): `AssertionError: 2 != 3`.
- S58505 (the prefix `deck-streak-slo`): `AssertionError: 'deck-streak-slo' != 'deck-streak-alert@'`.
- S58506 (the failed instance matched as a `line`): `AssertionError: 'line' != 'unit-failed'`.
- S58507 (silence at 60 minutes): `AssertionError: 60 not less than or equal to 20`.
- S58508 (a late backup at 2880 minutes): `AssertionError: 2880 not less than or equal to 1560`.
- S58509 (every unit shipped): `AssertionError: '*' != 'deck-streak-*'`.
- S58510 (a window of `P30D`): `AssertionError: 'P30D' != 'P14D'`.
- S58511 (one channel): `AssertionError: 1 not greater than or equal to 2`.
- S58512 (the request id as a label): the labels differ by `'request_id': 'span.request_id'`.
- S58513 (the copy kept 30 days): `AssertionError: 0 != 1 : one bullet bounds the monitoring service's copy of the journal`.
- S58514 (on until turned off): `AssertionError: Lists differ: ['off until you turn it on'] != []`.
- S58515 (the apps measure how you use them): `AssertionError: 'the apps measure nothing about how you use them' not found in` the section's text.

S33301 keeps its target and its mutant; its killer is the renamed A1 test, and CI's mutation verdict
reads it by id.
