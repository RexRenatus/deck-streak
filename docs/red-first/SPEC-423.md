# Red-first record: SPEC-423

SPEC-423 (R1 to R14, A1 to A14). The SPEC, ADR-437, the amendments to SPEC-362, SPEC-422, ADR-373
and ADR-436, the schematic's section 11, the tests of A1 to A14 and the census entries their reads
need were committed first and alone, over stubs that keep every input but the missing behaviour:
`rust_release_ceiling` returns None, `rust_ceiling_refused` returns None and `main` does not call
it, `RUST_MATRIX_LEGS` exists with its `#:` line, and no `RUST_RELEASE_CEILING` exists, which a
test that sets one patches in. Each red below is therefore its criterion's assertion and never a
missing name. The test modules run only in CI, so every red is quoted from the pull request's
checks over that commit, in the job `hygiene`, the job that runs `scripts/tests`.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_only_a_true_release_mark_holds_the_rust_ceiling
A2: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_placeholder_ceiling_sizes_the_release_run_as_a_pull_request_run
A3: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_release_run_sizes_up_to_its_ceiling_and_refuses_one_mutant_past_it
A4: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_a_run_that_is_not_the_release_keeps_the_pull_request_ceiling_at_any_rust_ceiling
A5: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_a_rust_ceiling_outside_its_range_is_refused_by_name
A6: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_rust_lines_read_the_release_ceiling_on_the_release_run_alone
A7: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_rust_release_line_stands_beside_the_python_release_line
A8: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_rust_matrix_of_the_release_run_fits_the_run_jobs
A9: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_verdict_walks_every_leg_past_the_pull_request_ceiling
A10: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_rust_ceiling_cites_the_signed_ruling_only_above_the_pull_request_ceiling
A11: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_python_ceiling_tests_read_the_rust_ceiling_at_its_placeholder
A12: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_runbook_names_the_rust_release_ceiling_its_matrix_and_its_refusal
A13: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_release_run_restates_no_rust_bound_and_assigns_each_once
A14: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py -k test_the_map_judges_the_verdict_scripts_mutants_with_the_rust_ceiling_tests
```

The reds were read in job 116777029844, `hygiene` over the red commit, a job of 1215 s:
`Ran 1285 tests` and `FAILED (failures=12)`. The twelve are the tests the fence above names for
A1 to A8, A10, A11, A12 and A14, every one of them and no other. A9's and A13's tests passed
there, each printing its examined count: `examined 200 legs past the pull request ceiling` for
A9, and `examined 7 values the release run keeps` and `examined 4 functions the release run's
Rust legs reach` for A13. Each failing line is quoted as CI printed it, unittest's own
`[N chars]` cuts included. A2's, A6's and A11's lines are cut here after the first line of the
text they searched, and marked `[cut]`.

Test edits after the red commit, each named here:
- The green commit, 9d8f4509555ddc27a7605ad2c4c0aea92d286189, adds to `verb` in
  `test_release_python_ceiling.py` the hold of `RUST_RELEASE_CEILING` at `LEG_CEILING["ci"]`
  (R10), adds beside each of the four `LEG_CEILING` patches in
  `test_mutation_python_shard_binding.py` one patch holding `RUST_RELEASE_CEILING` at the same
  ceiling (R10), and adds the census entries in `test_ci_workflows.py` that the hold's reads need,
  every old entry kept. It changes no assertion, and no line of `test_release_rust_ceiling.py` or
  `test_dispatch_shards.py`.
- The rows commit, b05914758a00b66faece82dc8201863d20a69a4e, and this record's commit edit no
  test module.

No CI run reads the green commit or the rows commit alone: every green below is read at the
second push's head, which carries the green commit, the rows and this record.

```red-first
A1: red at 92971462330a8888dbe11bc3425954d89bdbdf8c: AssertionError: None != 200 : a true mark under a set ceiling
A1: green at 9d8f4509555ddc27a7605ad2c4c0aea92d286189
A2: red at 92971462330a8888dbe11bc3425954d89bdbdf8c: AssertionError: 'mutation: shards: release run: 2 rust leg(s) of ceiling 200 (RUST_RELEASE_CEILING): 2 in mutation-rust, at most 256 a matrix\n' not found in 'mutation: shards: 3 python shard(s) for 3 listed mutant(s); python legs 3 of ceiling 32\n [cut]
A2: green at 9d8f4509555ddc27a7605ad2c4c0aea92d286189
A3: red at 92971462330a8888dbe11bc3425954d89bdbdf8c: AssertionError: 1 != 0 : mutation: shards: REFUSED: 13600 mutant(s), projected at 1713600 s serially, need more than 190 legs within 10800 s each, the pull request run's ceiling: split the change, since a run is never capped
A3: green at 9d8f4509555ddc27a7605ad2c4c0aea92d286189
A4: red at 92971462330a8888dbe11bc3425954d89bdbdf8c: AssertionError: Tuples differ: (1, None) != (0, '191')
A4: green at 9d8f4509555ddc27a7605ad2c4c0aea92d286189
A5: red at 92971462330a8888dbe11bc3425954d89bdbdf8c: AssertionError: Tuples differ: (0, 'mutation: shards: 3 python shard(s) f[448 chars]0\n') != (1, 'mutation: shards: REFUSED: the releas[92 chars]t\n')
A5: green at 9d8f4509555ddc27a7605ad2c4c0aea92d286189
A6: red at 92971462330a8888dbe11bc3425954d89bdbdf8c: AssertionError: 'mutation: shards: 2 shard(s) for 113 listed mutant(s), projected at 9040 s serially; the slowest at 6701 s of its 10800 s bound; legs 2 of ceiling 200\n' not found in 'mutation: shards: 3 python shard(s) for 3 listed mutant(s); python legs 3 of ceiling 32\n [cut]
A6: green at 9d8f4509555ddc27a7605ad2c4c0aea92d286189
A7: red at 92971462330a8888dbe11bc3425954d89bdbdf8c: AssertionError: 'mutation: shards: 2 shard(s) for 113 listed mutant(s), proj[89 chars]90\n' != 'mutation: shards: release run: 2 rust leg(s) of ceiling 200[63 chars]ix\n'
A7: green at 9d8f4509555ddc27a7605ad2c4c0aea92d286189
A8: red at 92971462330a8888dbe11bc3425954d89bdbdf8c: AssertionError: Tuples differ: (1, None) != (0, '256')
A8: green at 9d8f4509555ddc27a7605ad2c4c0aea92d286189
A9: not red: the base verdict already walks the plan's legs 0 to count - 1 for any count, so the test pins that a count past the pull request ceiling is walked whole; its own negative arm, one leg missing, is its control
A10: red at 92971462330a8888dbe11bc3425954d89bdbdf8c: AssertionError: Lists differ: ['0 ceiling and 1 LEG_CEILING assignment(s), not one'] != []
A10: green at 9d8f4509555ddc27a7605ad2c4c0aea92d286189
A11: red at 92971462330a8888dbe11bc3425954d89bdbdf8c: AssertionError: 'mutation: shards: release run: 2 rust leg(s) of ceiling 200 (RUST_RELEASE_CEILING): 2 in mutation-rust, at most 256 a matrix\n' not found in 'mutation: shards: 3 python shard(s) for 3 listed mutant(s); python legs 3 of ceiling 32\n [cut]
A11: green at 9d8f4509555ddc27a7605ad2c4c0aea92d286189
A12: red at 92971462330a8888dbe11bc3425954d89bdbdf8c: AssertionError: '`RUST_RELEASE_CEILING`' not found in ''
A12: green at 9d8f4509555ddc27a7605ad2c4c0aea92d286189
A13: not red: the base already assigns each kept value once and restates none in a new function
A14: red at 92971462330a8888dbe11bc3425954d89bdbdf8c: AssertionError: 'test_release_rust_ceiling' not found in ['test_mutation_verdict_python_kills', 'test_mutation_python_verdict', 'test_mutation_python_shard_binding', 'test_mutation_verdict', 'test_mutation_equivalent', 'test_memory_cap_verdict', 'test_release_python_ceiling']
A14: green at 9d8f4509555ddc27a7605ad2c4c0aea92d286189
```
