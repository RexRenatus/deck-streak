# Red-first record: SPEC-666

SPEC-666 (R1 to R12, nine acceptance criteria). The SPEC, ADR-688, the nine criteria's tests in
`scripts/tests/test_constant_boundaries.py` and the planted trees under
`scripts/tests/fixtures/constant-boundaries/` were committed first and alone, over stubs that keep
every input but the missing behaviour: the path rule classes every file as shipped, each reader
finds no comparison, the resolver resolves no test, the verdict prints nothing and fails nothing,
and the empty population is not refused. The register `scripts/constant-boundaries.json` lands in
the green commit, so each red below is its criterion's assertion and never a missing name.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_each_file_is_classed_by_language_and_test_code_is_read_apart
A2: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_a_comparison_with_no_boundary_test_is_a_site_in_each_language
A3: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_a_test_below_at_and_above_the_constant_makes_a_comparison_clean
A4: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_a_test_at_the_constant_alone_leaves_a_site
A5: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_a_test_the_tree_does_not_define_leaves_its_point_missing
A6: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_a_bare_literal_test_code_and_an_undecided_operand_are_never_sites
A7: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_the_level_decides_whether_a_site_fails_and_a_stale_entry_always_does
A8: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_an_empty_population_is_void
A9: python3 -m unittest discover -s scripts/tests -p test_constant_boundaries.py -k test_the_repository_reads_every_language_and_holds_no_stale_entry
```

Eight reds are read over the planted trees, by name, each on its first assertion; the repository
test reads the whole tree, so its red is read in CI's hygiene job at the red commit, where the
register is absent.

The red and green commit ids, each red's assertion line and the CI job ids are written here at the
build, from the build's own runs.

```red-first
A1: red at 80e6dbb: AssertionError: {'pyt[26 chars]s.py', 'scripts/tests/test_demo_limits.py'], '[307 chars]ts']} != {'pyt[26 chars]s.py'], 'rust': ['crates/demo/src/limits.rs'],[166 chars]ts']}
A1: green at 357928a
A2: red at 80e6dbb: AssertionError: Lists differ: [] != ['WARN constant-boundary crates/demo/src/l[475 chars]est']
A2: green at 357928a
A3: red at 80e6dbb: AssertionError: 'crates/demo/src/limits.rs <= MAX_ITEMS #1' not found in {}
A3: green at 357928a
A4: red at 80e6dbb: AssertionError: Lists differ: [] != ['WARN constant-boundary crates/demo/src/l[499 chars]ove']
A4: green at 357928a
A5: red at 80e6dbb: AssertionError: 'WARN constant-boundary crates/demo/src/limits.rs <= MAX_ITEMS #1: no test above' not found in []
A5: green at 357928a
A6: red at 80e6dbb: AssertionError: 0 != 1 : python
A6: green at 357928a
A7: red at 80e6dbb: AssertionError: 0 != 12
A7: green at 357928a
A8: red at 80e6dbb: AssertionError: AssertionError not raised
A8: green at 357928a
A9: red at 80e6dbb: test_constant_boundaries.py:732: self.assertIn(REGISTER, sources) (hygiene job 130582726573)
A9: green at 357928a
```

The green commit `357928a` carries the census, and `git diff --quiet 357928a HEAD` over the module and its planted files reads rc 0.
