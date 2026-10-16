# Red-first record: SPEC-427

SPEC-427 (R1 to R12, A1 to A8). The SPEC, ADR-441, the amendment to SPEC-020, the schematic and
the tests of A1 to A8 in `scripts/tests/test_rust_house_rules.py` were committed first and alone,
over stubs that keep every input but the missing behaviour: the walk returns every
`crates/*/src/**/*.rs` file under its root with no test module removed, the lexer returns the text
unblanked, each rule's matcher refuses nothing, and the empty-population refusal is not made. Each
behaviour's assertion runs before its `examined` call, so each red below is its criterion's
assertion and never an import, a missing fixture, an empty selection or an `examined 0` refusal.
The module walks every shipped source and runs only in CI, so every red is quoted from the pull
request's checks over the red commit, in the job `hygiene`, the job that runs `scripts/tests`.

The reds were read in `hygiene` over the red commit: `Ran 1293 tests` and `FAILED (failures=11)`.
Eight tests failed, and one of them, A7's, failed in four subtests, one per unit. They are the
eight tests the fence of SPEC-427 section 3 names for A1 to A8, every one of them, and no other
test failed, so `scripts/tests/test_ci_workflows.py` and `scripts/tests/test_temp_hygiene.py` read
green over the new module. Each failing line is quoted as CI printed it, and a set comparison's
line is followed by the first site in its difference (RHR is
`test_rust_house_rules.RustHouseRules`):

```text
A1 RHR.test_sqlite_is_opened_only_in_the_kernels_database_base
  AssertionError: Items in the second set but not the first:
  'crates/store/src/lib.rs:3'
A2 RHR.test_no_format_reaches_a_query
  AssertionError: Items in the second set but not the first:
  'crates/query/src/lib.rs:11'
A3 RHR.test_no_secret_named_name_reaches_a_log_macro
  AssertionError: Items in the second set but not the first:
  'crates/log/src/lib.rs:17'
A4 RHR.test_randomness_is_drawn_only_through_getrandom
  AssertionError: Items in the second set but not the first:
  'crates/draw/src/lib.rs:16'
A5 RHR.test_the_population_is_the_shipped_sources_without_test_code
  AssertionError: Items in the first set but not the second:
  'crates/app/src/other_tests.rs'
A6 RHR.test_comments_and_literals_are_not_code_but_a_format_capture_is_read
  AssertionError: Items in the second set but not the first:
  'crates/lexed/src/lib.rs:19'
A7 RHR.test_each_rule_refuses_an_empty_population (unit='shipped source files')
  AssertionError: AssertionError not raised
A7 RHR.test_each_rule_refuses_an_empty_population (unit='query sites')
  AssertionError: AssertionError not raised
A7 RHR.test_each_rule_refuses_an_empty_population (unit='log macro calls')
  AssertionError: AssertionError not raised
A7 RHR.test_each_rule_refuses_an_empty_population (unit='shipped source files and manifests')
  AssertionError: AssertionError not raised
A8 RHR.test_the_repository_holds_every_rule_at_zero
  AssertionError: [] is not true : sqlite-one-place found no opener in crates/kernel/src/db.rs
```

A1 to A4 and A6 are red on the sites their plants hold that the stub's matchers do not refuse, A5
on the test modules the stub's walk keeps, A7 on the refusal the stub does not make, and A8 on its
first assertion, the opener in `crates/kernel/src/db.rs` the stub does not see.

Test edits after the red commit, each named here:
- The green commit, 4a0894ec, is the one commit between the red and the green that changes the
  test module. It replaces the stubs above the tests with SPEC-427 R1 to R11 and changes no test:
  the module from `def planted(`, the fixture helper the tests open with, to its end is byte-equal
  at the red and green commits.
- The rows commit, fd8f3dcb, is the only commit between the green commit and this record's
  commit, and it touches only `scripts/mutation-rows.d/S42700-S42799.json`. This record's commit
  edits no test module, so the module at the head is the green commit's:
  `git diff --quiet 4a0894ec HEAD -- scripts/tests/test_rust_house_rules.py` exits 0.

```red-first
A1: red at 83dbccbdc47aaf3ce160798f369e7bc24bfa4a77: AssertionError: Items in the second set but not the first:
A1: green at 4a0894ec1417c0dd44492b024d4616a2f9ff1c24
A2: red at 83dbccbdc47aaf3ce160798f369e7bc24bfa4a77: AssertionError: Items in the second set but not the first:
A2: green at 4a0894ec1417c0dd44492b024d4616a2f9ff1c24
A3: red at 83dbccbdc47aaf3ce160798f369e7bc24bfa4a77: AssertionError: Items in the second set but not the first:
A3: green at 4a0894ec1417c0dd44492b024d4616a2f9ff1c24
A4: red at 83dbccbdc47aaf3ce160798f369e7bc24bfa4a77: AssertionError: Items in the second set but not the first:
A4: green at 4a0894ec1417c0dd44492b024d4616a2f9ff1c24
A5: red at 83dbccbdc47aaf3ce160798f369e7bc24bfa4a77: AssertionError: Items in the first set but not the second:
A5: green at 4a0894ec1417c0dd44492b024d4616a2f9ff1c24
A6: red at 83dbccbdc47aaf3ce160798f369e7bc24bfa4a77: AssertionError: Items in the second set but not the first:
A6: green at 4a0894ec1417c0dd44492b024d4616a2f9ff1c24
A7: red at 83dbccbdc47aaf3ce160798f369e7bc24bfa4a77: AssertionError: AssertionError not raised
A7: green at 4a0894ec1417c0dd44492b024d4616a2f9ff1c24
A8: red at 83dbccbdc47aaf3ce160798f369e7bc24bfa4a77: AssertionError: [] is not true : sqlite-one-place found no opener in crates/kernel/src/db.rs
A8: green at 4a0894ec1417c0dd44492b024d4616a2f9ff1c24
```

No CI run reads the green commit alone, so each green is read at the second push's head, which
holds the green, rows and record commits and whose checks, read by name, confirm the greens, and
A8's four `examined` lines are read in that run's `hygiene` log.

The `hygiene` job over the second push's head, 698be1d922453300dc4c30bc34fe637b27e249c2, read every
criterion green, and its `python` stage log holds A8's four `examined` lines:

```text
examined 337 shipped source files
examined 40 query sites
examined 202 log macro calls
examined 369 shipped source files and manifests
```
