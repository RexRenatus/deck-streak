# Red-first record: SPEC-671

SPEC-671 (R1 to R9, eight criteria). The SPEC, ADR-694, the schematic, the planted cases under
`scripts/tests/fixtures/lock-flags/` and the eight tests named below in `scripts/tests/test_ci_lock_flags.py`
were committed first and alone, over stubs that keep every input but the missing behaviour:
`judge` names no site, `population` examines nothing without refusing, and `verdict` passes
everything. `LISTED` and `LEVEL` were already there, so each red below is an
assertion and never a missing name.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_ci_lock_flags.py -k test_an_empty_population_is_void
A2: python3 -m unittest discover -s scripts/tests -p test_ci_lock_flags.py -k test_each_cargo_form_is_a_site_or_clean_as_its_lock_flag_says
A3: python3 -m unittest discover -s scripts/tests -p test_ci_lock_flags.py -k test_each_node_and_apple_and_pip_form_is_a_site_or_clean_as_its_lock_flag_says
A4: python3 -m unittest discover -s scripts/tests -p test_ci_lock_flags.py -k test_a_shell_script_is_read_by_function_and_after_any_wrapper
A5: python3 -m unittest discover -s scripts/tests -p test_ci_lock_flags.py -k test_a_python_script_is_read_by_its_argument_lists
A6: python3 -m unittest discover -s scripts/tests -p test_ci_lock_flags.py -k test_an_unlisted_site_warns_at_warn_and_fails_at_fail
A7: python3 -m unittest discover -s scripts/tests -p test_ci_lock_flags.py -k test_a_stale_entry_fails_at_both_levels
A8: python3 -m unittest discover -s scripts/tests -p test_ci_lock_flags.py -k test_each_site_is_listed_with_its_reason_and_no_entry_is_stale
```

The red criterion: the planted `cargo build -p planted` of `workflow.yml.fixture`, which `judge`'s
cargo arm reports as a site, reads red at the red commit, where the stub names no site, and green
at the green commit. The planted tests read planted text and were read red by name; the repository test reads the tree, so
its red was read in CI, in the `hygiene` job, where the census refused the stub's empty population.

Test edits after the red commit: none. The green commit replaces the stub bodies with the readers and imports `refuse_link_components`;
it changes no test function, planted text or expected value.

A1: red at 395c714: AssertionError: AssertionError not raised
A1: green at fe22725
A2: red at 395c714: AssertionError: Lists differ: [] != [('cargo', 'cargo', 'build', 'site'), ('ca[353 chars]an')]
A2: green at fe22725
A3: red at 395c714: AssertionError: Lists differ: [('node', 'pnpm', 'action-setup run_install', 'site')] != [('node', 'pnpm', 'install', 'site'), ('node', 'pnpm', 'inst[402 chars]te')]
A3: green at fe22725
A4: red at 395c714: AssertionError: Lists differ: [] != [('build', 'cargo', 'build', 'clean'), ('b[153 chars]te')]
A4: green at fe22725
A5: red at 395c714: AssertionError: Lists differ: [] != [('build', 'cargo', 'build', 'site'), ('bu[63 chars]te')]
A5: green at fe22725
A6: red at 395c714: AssertionError: Tuples differ: (0, []) != (2, [])
A6: green at fe22725
A7: red at 395c714: AssertionError: 0 != 1 : warn
A7: green at fe22725
A8: red at 395c714: AssertionError: examined 0 file(s): the population is empty, so nothing was judged (the `hygiene` job 130466085178)
A8: green at fe22725

The repository test's census lines at the green commit were read in the `hygiene` job 130491445522.

The green commit `fe22725` carries the census, and `git diff --quiet fe22725 HEAD` over the module and its planted files reads rc 0.
