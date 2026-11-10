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
