# Red-first record: SPEC-665

SPEC-665 (R1 to R16). The SPEC, ADR-687, the tests in `scripts/tests/test_redundant_guards.py`
and the six planted fixtures under `scripts/tests/fixtures/redundant-guards/` were committed first
and alone, over stubs that keep every input but the missing behaviour: no form decides a condition,
the verdict passes everything, no path reads as a language, the Rust test-module reader names
nothing, and neither empty refusal refuses. `LEVEL` was already warn and `LISTED` already empty, so
each red below is its criterion's assertion and never a missing name.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_rust_planted_sites_are_found
A2: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_python_planted_sites_are_found
A3: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_typescript_planted_sites_are_found
A4: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_svelte_planted_sites_are_found
A5: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_swift_planted_sites_are_found
A6: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_shell_planted_sites_are_found
A7: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_the_population_reads_each_language_and_no_test_code
A8: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_the_level_decides_whether_an_unlisted_site_fails_and_a_stale_entry_always_does
A9: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_an_empty_population_is_refused
A10: python3 -m unittest discover -s scripts/tests -p test_redundant_guards.py -k test_every_shipped_source_is_read_and_each_redundant_guard_is_listed
```

The planted reds were read on the box by full test id, each on its first failing assertion. The
repository test reads the whole tree, so its red was read in CI at the first push's `hygiene` job,
and its green at the second push's. The rows commit and this record edit no test.

```red-first
A1: red at b2e4524: AssertionError: Lists differ: [] != [(5, 'same-expression'), (13, 'same-expres[60 chars]on')]
A1: green at 5ce10b6
A2: red at b2e4524: AssertionError: Lists differ: [(6, [39 chars]on'), (32, 'same-expression')] != [(6, [39 chars]on'), (17, 'range'), (24, 'presence'), (32, 'same-expression')]
A2: green at 5ce10b6
A3: red at b2e4524: AssertionError: Lists differ: [] != [(5, 'same-expression'), (14, 'range'), (2[60 chars]on')]
A3: green at 5ce10b6
A4: red at b2e4524: AssertionError: Lists differ: [] != [(39, 'same-expression'), (44, 'range'), (49, 'presence')]
A4: green at 5ce10b6
A5: red at b2e4524: AssertionError: Lists differ: [] != [(5, 'same-expression'), (14, 'range'), (2[60 chars]on')]
A5: green at 5ce10b6
A6: red at b2e4524: AssertionError: Lists differ: [] != [(5, 'same-expression'), (12, 'same-expression')]
A6: green at 5ce10b6
A7: red at b2e4524: AssertionError: None != 'rust' : crates/api/src/routes.rs
A7: green at 5ce10b6
A8: red at b2e4524: AssertionError: Tuples differ: ([], []) != (['WARN redundant-guard crates/api/src/lib.[65 chars], [])
A8: green at 5ce10b6
A9: red at b2e4524: AssertionError: AssertionError not raised
A9: green at 5ce10b6
A10: red at b2e4524: AssertionError: Lists differ: [] != ['python', 'rust', 'shell', 'svelte', 'swift', 'typescript']
A10: green at 5ce10b6
```

The first push's `hygiene` job 130579928985 also failed `test_the_finder_is_defined_once_and_a_planted_copy_is_caught` in `scripts/tests/test_mutation_workflows.py`, a census that reads every test module: it read the red commit `b2e4524`'s stubs `language_of` and `decide`, each a docstring and a bare `return None`, as copies of the finder's `no_wrapper`. That red is the red commit's own and no criterion's; the green commit `5ce10b6` gives both stubs their real bodies, and that census reads its floor again at the second push's `hygiene` job.

`LISTED` stays empty: the census's first full run, `hygiene` job 130895070419, gave no site to list. The green commit `5ce10b6` carries the census, and `git diff --quiet 5ce10b6 HEAD -- scripts/tests/test_redundant_guards.py scripts/tests/fixtures/redundant-guards` reads rc 0.
