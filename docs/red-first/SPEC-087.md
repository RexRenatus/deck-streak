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
