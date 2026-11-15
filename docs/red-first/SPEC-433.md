# Red-first record: SPEC-433

SPEC-433 (R1 to R12, A1 to A18). The SPEC, ADR-447, the schematic, the register
`config/fsm-lints.json`, the changelog fragment, the five test modules of A1 to A18 and the helper
`scripts/tests/_fsm_lints.py` were committed first and alone, in the red commit, over a helper that
keeps every input but the missing behaviour: the data types, the three walks, a register loader that
accepts any rule, level, key and entry, and four rule functions that each return a reading whose
file count is the walk's and whose hits, pairs and covers are empty, never VOID, printing nothing.
Each red below is therefore its criterion's assertion, never a missing name, fixture or register,
and never an empty selection. The test modules run only in CI, so every red is quoted from the pull
request's checks over that commit, in the job `hygiene`, the job that runs `scripts/tests`.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_fsm_check_list.py -k test_the_tree_reading_reads_every_migration_and_every_pair
A2: python3 -m unittest discover -s scripts/tests -p test_fsm_check_list.py -k test_a_planted_drift_names_its_column_and_each_missing_and_extra_spelling
A3: python3 -m unittest discover -s scripts/tests -p test_fsm_check_list.py -k test_a_rebuilt_and_renamed_table_is_read_as_the_last_migration_leaves_it
A4: python3 -m unittest discover -s scripts/tests -p test_fsm_check_list.py -k test_only_a_whole_body_check_is_a_column_domain
A5: python3 -m unittest discover -s scripts/tests -p test_fsm_check_list.py -k test_a_state_column_neither_paired_nor_listed_is_a_new_hit
A6: python3 -m unittest discover -s scripts/tests -p test_fsm_check_list.py -k test_a_pair_naming_what_is_absent_is_void
A7: python3 -m unittest discover -s scripts/tests -p test_fsm_wildcard.py -k test_the_tree_reading_reads_every_product_file_and_its_matches
A8: python3 -m unittest discover -s scripts/tests -p test_fsm_wildcard.py -k test_each_catch_all_arm_over_a_state_enum_is_a_hit
A9: python3 -m unittest discover -s scripts/tests -p test_fsm_wildcard.py -k test_only_the_planted_catch_all_is_a_hit
A10: python3 -m unittest discover -s scripts/tests -p test_fsm_unreachable.py -k test_the_tree_reading_reads_every_product_file
A11: python3 -m unittest discover -s scripts/tests -p test_fsm_unreachable.py -k test_each_panicking_arm_is_a_hit_and_nothing_else_is
A12: python3 -m unittest discover -s scripts/tests -p test_fsm_uncovered.py -k test_the_tree_reading_reads_every_formal_module_and_state_enum
A13: python3 -m unittest discover -s scripts/tests -p test_fsm_uncovered.py -k test_only_an_anchor_on_the_enum_covers_it
A14: python3 -m unittest discover -s scripts/tests -p test_fsm_lints.py -k test_every_rule_is_void_with_nothing_to_examine
A15: python3 -m unittest discover -s scripts/tests -p test_fsm_lints.py -k test_an_entry_outside_the_seed_needs_a_ruling_that_exists
A16: python3 -m unittest discover -s scripts/tests -p test_fsm_lints.py -k test_a_rule_at_fail_fails_on_new_and_stale_and_at_warn_prints_them
A17: python3 -m unittest discover -s scripts/tests -p test_fsm_lints.py -k test_a_hit_line_names_its_site_and_never_its_source
A18: python3 -m unittest discover -s scripts/tests -p test_fsm_lints.py -k test_the_register_names_four_rules_at_warn_and_refuses_the_unknown
```

The reds were read in run 38896439752, job 116957930195, `hygiene` over the red commit, a job of 1257 s.
Every failing line below is quoted from the `python.log` of that run's `check-stage-logs-hygiene`
artifact, which reads `Ran 1303 tests` and `FAILED (failures=34)`. The 34 failures fall in 18
tests, the tests the fence above names for A1 to A18, every one of them and no other. Five of them
failed in more than one subtest (A6's test in 6, A14's in 5, A18's in 4, A15's and A16's in 3
each), and for each test the first failure the log prints is the one quoted. Each failing line is
quoted as CI printed it, unittest's own `[N chars]` cuts included. A set difference prints its
items on the lines after its header: for A2, A5, A8, A9, A11 and A13 the first item is joined to
the header with one space, and where more items follow, they are cut and marked `[cut]`.

Test edits after the red commit, each named here:
- The green commit, 5fca3889ed113e576b3189e537d193a183491960, is the one commit between red and
  green that changes a file under `scripts/tests`: it fills R1 to R12 into the helper
  `scripts/tests/_fsm_lints.py` alone, behind the inputs the red commit kept. It changes no line of
  the five test modules or of the register.
- The rows commit, 1535584799c93037b592377a360b08cefade78b8, and the SPEC-fix commit,
  ef55f7d1bb586ec00a64b21b00885384bc60ca3c, touch no test or code file: the first adds the band
  `scripts/mutation-rows.d/S43300-S43399.json`, and the second renames one placeholder token in the
  SPEC's section 1 table. This record's commit adds this file alone.

The identity proof, run after the SPEC-fix commit, exits with rc 0:
`git diff --quiet 5fca3889 HEAD -- scripts/tests/_fsm_lints.py scripts/tests/test_fsm_check_list.py scripts/tests/test_fsm_lints.py scripts/tests/test_fsm_uncovered.py scripts/tests/test_fsm_unreachable.py scripts/tests/test_fsm_wildcard.py config/fsm-lints.json`.
The helper, the five test modules and the register therefore read at the second push's head as the
green commit left them.

No CI run reads the green commit, the rows commit or the SPEC-fix commit alone. The second push
carries the green commit, the rows, the SPEC fix and this record together, and its CI, read by name
at that head, confirms every green below.

```red-first
A1: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: Lists differ: [] != [('sync_runs', 'status'), ('chests', 'stat[144 chars]ct')]
A1: green at 5fca3889ed113e576b3189e537d193a183491960
A2: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: Items in the second set but not the first: ('new', 'migrations/001_lamps.sql', 7, ('column=taps.step', 'lacks=drip', 'unspelled=drop')) [cut]
A2: green at 5fca3889ed113e576b3189e537d193a183491960
A3: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: {} != {('jobs', 'state'): ('text', ('queued', 'r[116 chars]e'))}
A3: green at 5fca3889ed113e576b3189e537d193a183491960
A4: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: {} != {('gauges', 'status'): ('text', ('on', 'of[45 chars]1'))}
A4: green at 5fca3889ed113e576b3189e537d193a183491960
A5: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: Items in the second set but not the first: ('listed', 'migrations/001_machines.sql', 16, ('column=queues.state',)) [cut]
A5: green at 5fca3889ed113e576b3189e537d193a183491960
A6: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: Void not raised
A6: green at 5fca3889ed113e576b3189e537d193a183491960
A7: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: 0 not greater than 0
A7: green at 5fca3889ed113e576b3189e537d193a183491960
A8: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: Items in the second set but not the first: ('new', 'crates/plant/src/lib.rs', ('enum=BuildStage', 'fn=by_ref')) [cut]
A8: green at 5fca3889ed113e576b3189e537d193a183491960
A9: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: Items in the second set but not the first: ('new', 'crates/plant/src/lib.rs', ('enum=DoorState', 'fn=planted'))
A9: green at 5fca3889ed113e576b3189e537d193a183491960
A10: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: Regex didn't match: '(?m)^FSM-LINT FSM-UNREACHABLE level=(warn|fail) examined=[1-9][0-9]* arms hits=[0-9]+ new=[0-9]+ listed=[0-9]+ stale=[0-9]+$' not found in ''
A10: green at 5fca3889ed113e576b3189e537d193a183491960
A11: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: Items in the second set but not the first: ('new', 'crates/plant/src/lib.rs', ('fn=bare', 'macro=todo')) [cut]
A11: green at 5fca3889ed113e576b3189e537d193a183491960
A12: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: 0 not greater than 0
A12: green at 5fca3889ed113e576b3189e537d193a183491960
A13: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: Items in the second set but not the first: ('stale', 'crates/plant/src/lib.rs', 0, ('enum=GoneState',)) [cut]
A13: green at 5fca3889ed113e576b3189e537d193a183491960
A14: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: Void not raised
A14: green at 5fca3889ed113e576b3189e537d193a183491960
A15: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: Refused not raised
A15: green at 5fca3889ed113e576b3189e537d193a183491960
A16: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: AssertionError not raised
A16: green at 5fca3889ed113e576b3189e537d193a183491960
A17: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: Lists differ: [] != ['FSM-LINT FSM-WILDCARD level=warn examine[123 chars]ass']
A17: green at 5fca3889ed113e576b3189e537d193a183491960
A18: red at a2c0f1f62881e492911d53265360bf7540ead1e9: AssertionError: Refused not raised
A18: green at 5fca3889ed113e576b3189e537d193a183491960
```
