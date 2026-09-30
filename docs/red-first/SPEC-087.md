# Red-first record: SPEC-087

The order of work: the red-first tests of the runner (4532d88); a stub runner and the map, so
A1-A12 fail by assertion (5c7fcad); the runner (96eee0b); the verdict's tests beside a verdict that
knew no `scripts` class, no `--python-listed` and no python report (9e913f9); the verdict (6b5bd84);
the workflow and document tests (a201fe0); the workflows (015f7f6); the constants' tests and the
rows `S08701-S08725`, each proved KILLED (3838972, 57e10c2); the documents.

Each criterion was run at its red commit with the SPEC's own fenced command, selecting one test, and
failed by assertion for its own criterion, not by an import error, a missing fixture or an empty
selection. The stub compiled and listed, ran and judged nothing, and the map was empty.

Two disclosures. A20 and A21 read the workflows, which were edited before their test was
committed, so their red was measured by restoring the base `ci.yml` and `mutation-weekly.yml`
beside the committed tests; A22's red is the base documents. A1-A12 were run again after the merge
of `dev` and read green (18 tests, OK).

A15 to A19, re-measured after review. The sentence above holds for A1-A14 and A20-A23 as
committed. At 9e913f9 the verdict's parser refused `--python-listed` and `--class scripts`, so
A15, A17 and A18 stopped in the fixture helper `shard_the_plan` and A16 at its first exit check,
each on a usage error before any assertion of its own criterion; SPEC section 3's stub, which
accepts the flags and reads nothing, was not committed. Their red was therefore measured after
the fact with the tests of 6b5bd84 beside 9e913f9's verdict plus that stub (the diff below), and
each of A15 to A19, and A13, then ran one test and failed by assertion for its own criterion,
with the lines in the block after it.

A19's test also changed between the red commit and the green one, so the criterion was not held
fixed: 9e913f9 asserted `battery: counted 1 of 1 reports whole` and 6b5bd84 asserts `battery:
counted 2 of 2 reports whole`; 9e913f9 asserted `table: VOID mutation-python-shard-3: <outcome>:
<mutant>` and 6b5bd84 asserts `table: VOID mutation-python-shard-1: <outcome>: <mutant>`, because
its fixture now gives each shard its own slice of the listing (`slot=(shard, 16)`) instead of the
whole listing in every shard. The A16 test also gained a `shard_the_plan` call and an empty
`python.json` before 6b5bd84. The A19 line below was read with 6b5bd84's version of the test.

```diff
--- a/scripts/mutation-verdict.py
+++ b/scripts/mutation-verdict.py
-    {"rust": judge_rust, "web": judge_web, "oracle": judge_oracle}[args.klass](verdict, plan, args)
+    {"rust": judge_rust, "web": judge_web, "oracle": judge_oracle, "scripts": lambda v, p, a: None}[args.klass](verdict, plan, args)
-    parser.add_argument("--class", dest="klass", choices=["rust", "web", "oracle"])
+    parser.add_argument("--class", dest="klass", choices=["rust", "web", "oracle", "scripts"])
+    parser.add_argument("--python")
+    parser.add_argument("--python-listed")
+    parser.add_argument("--python-whole")
```

A23, recorded in prose because the probe reads one fenced block: it was red at 6394621, where
`test_every_needed_job_that_uploads_has_a_matching_download_in_the_verdict` failed with
`AssertionError: False is not true : mutation-python uploads mutation-python-shard-0 and the verdict
downloads nothing that matches it`, and `test_the_layout_holds_for_every_count_of_artifacts` failed
with `'T/reports/mutation-python-shard-0/report.json' not found`; both were green at 63b9336, which
adds the verdict's download of `mutation-python-shard-*` and the shard's `python-shards/` output
directory. The number is A23 because A5 was already the runner's sentinel criterion.

Round 2 of review (2026-09-29): the green commits that edit a test file, beyond the red and green
commits named above, each by its short sha.
- 2005f77 adds three tests to `test_mutation_python_lister_kills.py`. Each was red by assertion
  in its own body against its own hand-applied mutant of `scripts/mutation_python.py`, and green
  without it:
  - `break` to `continue` in `Lister.between` (line 178):
    `test_between_stops_at_the_first_token_at_or_past_its_end` failed with
    `AssertionError: Lists differ: ['a', 'b'] != ['a']`;
  - `is not None` to `is None` in `Lister.skipped_nodes` (line 191):
    `test_an_arguments_annotation_is_skipped_as_a_node_of_its_own` failed with
    `AssertionError: Lists differ: [False, False] != [True, True]`;
  - `strict=True` to `strict=False` in `Lister.site` (line 287):
    `test_a_comparison_whose_operators_and_operands_disagree_is_refused` failed with
    `AssertionError: None is not an instance of <class 'ValueError'>`.
- 6b5bd84 also changed, beyond the A16 and A19 changes disclosed above:
  - A15's fixture row id, from `S08799-GUARD` to `S00077-GUARD`. 9e913f9's row lay outside its
    band file `S00000-S00099.json`, so the census refused the fixture before A15's assertion;
  - A19's unviable fixture mutant, from `replace return x + 2 with return None in guard`, which
    names no listed mutant, to `replace return value with return None in guard`; its three held
    outcomes now apply in each shard's slice instead of in shard 0 alone;
  - two of SPEC-039's tests in `test_mutation_verdict.py`, where they assert text this delivery
    changes: A12 (`the_production_classes_are_exact`) reads `scripts/check.py` as `scripts`, not
    `other`; A29 (`a_missing_or_partial_battery_report_fails_by_name`) names the 16 missing
    `mutation-python-shard-<k>` reports and counts `2 of 23` and `examined 23`, not `2 of 7` and
    `examined 7`, and its whole control counts `20 of 20`, not `4 of 4`.
- 015f7f6 adds `mutation-python` to the `mutation-verdict` needs line that
  `test_mutation_workflows.py` asserts.
- 8853ac3 adds `test_a_rehearsal_promises_the_python_shards_it_ran`, red; its green, 69b63f6,
  changed that test's `--shards 0` to `--shards 1`, so the test was not held fixed.
- 424dbb3 adds `size` to the `survivors` needs line that A21's test asserts.
- ba9b956, 5d84ae5 and e123e2e count enumerations through the `examined` helper and change no
  assertion; 3bfa171 also drops `PYTHONPYCACHEPREFIX` from the bytecode row test's environment.
- ea20f09, fddb2be, 2a02cf2, f0e98f1, d1bc0a9 and 7da3f41 add tests only, and 3389443 drops the
  three dead `False` arguments to `table_python`.

Fix round 2 (2026-09-29): the merge of dev `06a881a0` (SPEC-290, PR #443) and the commits after it
that edit a test file.
- d55e792 is the merge. Its auto-merged hunk in `test_mutation_workflows.py` keeps dev's own
  assertions and this delivery's `mutation-python` needs line; no assertion of either side changed.
  At d55e792 `test_the_verdict_step_fails_on_the_legs_check` (SPEC-290 A7) was red by assertion:
  `AssertionError: 99 != 3 : legs refuses, the judges passed`, because the merged step also runs
  `judge --class scripts`, which its recording shim did not know and answered 99.
- c31f056 adds the shim's `scripts` case, answering 0; that test is green and its scenarios and
  assertions are unchanged.
- 7df8833 adds `test_a_plan_of_python_mutants_and_no_rust_mutant_lists_zero`. It was red by
  assertion against a planted `listed=len(mutants) + python count` in a copy of the tree:
  `AssertionError: '1' != '0'`, and green on the real `mutation-verdict.py`.
- 9420f63 adds `test_ci_admits_a_skip_from_the_two_legs_and_from_no_other_need`: every entry of
  `ci`'s needs, read from the YAML, crossed with success, failure, cancelled and skipped, alone and
  beside both admitted skips (96 members). It was red by assertion against a planted
  `mutation-python` in `ci`'s admitted loop, in a copy of the tree:
  `AssertionError: 0 != 1 : mutation-python skipped, alone: ci: mutation-python was not started`,
  and green on the real `ci.yml`.

```red-first
A1: red at 5c7fcad: AssertionError: 'mutation-python: listed 0' != 'mutation-python: listed 29'
A1: green at 96eee0b
A2: red at 5c7fcad: AssertionError: Tuples differ: (0, 'examined 0') != (0, 'examined 1')
A2: green at 96eee0b
A3: red at 5c7fcad: AssertionError: 0 != 2 : examined 0
A3: green at 96eee0b
A4: red at 5c7fcad: AssertionError: 0 != 3 : examined 0
A4: green at 96eee0b
A5: red at 5c7fcad: AssertionError: 0 != 1 : examined 0
A5: green at 96eee0b
A6: red at 5c7fcad: AssertionError: 'survived' != 'killed'
A6: green at 96eee0b
A7: red at 5c7fcad: AssertionError: 0 != 3 : examined 0
A7: green at 96eee0b
A8: red at 5c7fcad: AssertionError: Tuples differ: ('survived', []) != ('unviable', [])
A8: green at 96eee0b
A9: red at 5c7fcad: AssertionError: 0 != 3 : examined 0
A9: green at 96eee0b
A10: red at 5c7fcad: AssertionError: Lists differ: [] != ['+ with - in total', '+ with - in total',
A10: green at 96eee0b
A11: red at 5c7fcad: AssertionError: 0 != 1 : examined 0
A11: green at 96eee0b
A12: red at 5c7fcad: AssertionError: 0 != 3 : examined 0
A12: green at 96eee0b
A13: red at 9e913f9: AssertionError: 'other' != 'scripts'
A13: green at 6b5bd84
A14: red at d8b6517: AssertionError: 8 != 9 : 321 listed
A14: green at b9d43e2
A15: red at 9e913f9: AssertionError: 0 != 3 : mutation: scripts: verdict: ok
A15: green at 6b5bd84
A16: red at 9e913f9: AssertionError: 'EQUIVALENT scripts/guard.py:2:14: replace + with - in guard' not found in 'mutation: scripts: verdict: ok\nexamined 0\n'
A16: green at 6b5bd84
A17: red at 9e913f9: AssertionError: Regex didn't match: '(?m)^examined 9$' not found in 'mutation: scripts: verdict: ok\nexamined 0\n'
A17: green at 6b5bd84
A18: red at 9e913f9: AssertionError: 0 != 1 : mutation: scripts: verdict: ok
A18: green at 6b5bd84
A19: red at 9e913f9: AssertionError: 'battery: MISSING mutation-python-shard-7: no report.json' not found
A19: green at 6b5bd84
A20: red at a201fe0: AssertionError: Regex didn't match: 'mutation_python\.py list --plan [^\n]*--out '
A20: green at 015f7f6
A21: red at a201fe0: AssertionError: Regex didn't match: '(?m)^        shard: \[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15\]$'
A21: green at 015f7f6
A22: red at a201fe0: AssertionError: 'mutation_python.py' not found in the builder brief
A22: green at 1e386cf
A23: red at 6394621: AssertionError: False is not true : mutation-python uploads mutation-python-shard-0 and the verdict downloads nothing that matches it
A23: green at 63b9336
```

Addendum, 2026-09-30 (issue #454, the amendment's A24). The test of A24 was committed
(d632590d) beside one planted fold removal in `ci.yml`, the scripts judge's line
`if [ "$status" -eq 0 ]; then status=$scripts; fi`, because the folds already hold at `dev`, so a
new test alone cannot be red. The whole file ran one test, red by assertion for the planted
removal. Restoring the line (4cb52f6e) turned it green; the green commit edits no test file.

```red-first
A24: red at d632590d: AssertionError: 0 != 1 : judge:scripts alone, exit 1:
A24: green at 4cb52f6e
A25: red at 12a45d84: AssertionError: 1 == 1 : judge:rust piped through | tee "$RUNNER_TEMP/{n}.log" kept its exit 1
A25: green at 9d0daeb4
```

A25 (round 1 of #454). Its test was committed (12a45d84) beside the run helper as it stood, which
ran the step under `bash -eo pipefail`, a stronger shell than the one GitHub resolves for a step
naming none (`bash -e {0}`). Under pipefail a piped command keeps its exit, so the harness
self-test failed by assertion, and so did the census test, whose first cut refused nothing. The
resolver (9d0daeb4) runs the step under the resolved shell and refuses the undrivable capture.

Rows S08766 to S08773 are the companions of the killing test of A24, each proved KILLED by its
full id on a clean committed tree.
