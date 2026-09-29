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
A14: red at 9e913f9: mutation-verdict.py: error: unrecognized arguments: --python-listed
A14: green at 6b5bd84
A15: red at 9e913f9: mutation-verdict.py: error: unrecognized arguments: --python-listed
A15: green at 6b5bd84
A16: red at 9e913f9: mutation-verdict.py: error: argument --class: invalid choice: 'scripts'
A16: green at 6b5bd84
A17: red at 9e913f9: mutation-verdict.py: error: unrecognized arguments: --python-listed
A17: green at 6b5bd84
A18: red at 9e913f9: mutation-verdict.py: error: unrecognized arguments: --python-listed
A18: green at 6b5bd84
A19: red at 9e913f9: AssertionError: 'battery: MISSING mutation-python-shard-7: no report.json' not found
A19: green at 6b5bd84
A20: red at a201fe0: AssertionError: Regex didn't match: 'mutation_python\.py list --plan [^\n]*--out '
A20: green at 015f7f6
A21: red at a201fe0: AssertionError: Regex didn't match: '(?m)^        shard: \[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15\]$'
A21: green at 015f7f6
A22: red at a201fe0: AssertionError: 'mutation_python.py' not found in the builder brief
```
