# Red-first record: SPEC-654

SPEC-654 (R1 to R10, A1 to A8). The SPEC, ADR-676, the module's tests of A1 to A8 and the six
planted trees were committed first and alone, over stub bodies that keep every input but the
missing behaviour: the census reads each population but calls no population refusal and classes no
pair, and the verdict passes everything. `LEVEL` and `LISTED` were already there, so each red below
is its criterion's assertion and never a missing name.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_client_types_census.py -k test_an_equal_pair_reads_no_finding
A2: python3 -m unittest discover -s scripts/tests -p test_client_types_census.py -k test_a_drifted_field_is_named_warns_at_warn_and_fails_at_fail
A3: python3 -m unittest discover -s scripts/tests -p test_client_types_census.py -k test_an_unmatched_client_type_and_its_route_are_found
A4: python3 -m unittest discover -s scripts/tests -p test_client_types_census.py -k test_a_listed_finding_reads_listed_and_a_stale_entry_fails_at_both_levels
A5: python3 -m unittest discover -s scripts/tests -p test_client_types_census.py -k test_an_empty_population_is_void
A6: python3 -m unittest discover -s scripts/tests -p test_client_types_census.py -k test_the_tree_holds_todays_listing_at_warn
A7: python3 -m unittest discover -s scripts/tests -p test_client_types_census.py -k test_a_read_inside_an_async_binding_is_paired
A8: python3 -m unittest discover -s scripts/tests -p test_client_types_census.py -k test_a_read_with_a_template_path_is_paired
```

```red-first
A1: red at 64b3fbf1: AssertionError: 0 != 2
A1: green at 6088590c
A2: red at 64b3fbf1: AssertionError: {} != {'GET /api/tally web': ('drifted unread=extra', 'web/app/src/lib/api.ts:4')}
A2: green at 6088590c
A3: red at 64b3fbf1: AssertionError: {} != {'GET /api/elsewhere web': ('unmatched', '[98 chars]:4')}
A3: green at 6088590c
A4: red at 64b3fbf1: AssertionError: Tuples differ: ([], []) != (['LISTED GET /api/tally web: drifted unread=extra (planted)'], [])
A4: green at 6088590c
A5: red at 64b3fbf1: AssertionError: AssertionError not raised
A5: green at 6088590c
A6: red at 64b3fbf1: AssertionError: 0 not greater than 0
A6: green at 6088590c
A7: red at 64b3fbf1: AssertionError: 0 != 1
A7: green at 6088590c
A8: red at 64b3fbf1: AssertionError: 0 != 1
A8: green at 6088590c
```

The green commit 6088590c changed the test module only in its import line and in the bodies of
`census` and `verdict`, which replaced the two stubs; no test, planted tree or expected value
moved between the red and the green.
