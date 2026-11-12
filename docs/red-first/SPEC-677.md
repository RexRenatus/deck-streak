# Red-first record: SPEC-677

The tests of every criterion and the planted texts were committed first, over stubs that keep every input
but the missing behaviour: the census finds no file, the marker reader finds no marker, the judge
refuses nothing and the verdict passes everything. a7eeb0e and be56ed0 are the build's
commits, spelt by it.

```red-first
A1: not red: its test reads every test file of the tree, which only the CI hygiene job runs
A2: not red: over the stubs the judge refuses nothing, so the clean listing already reads clean
A3: red at a7eeb0e: AssertionError: Lists differ: [] != ['quarantined flaky.py.fixture::test_somet[51 chars]eed']
A4: red at a7eeb0e: AssertionError: Lists differ: [] != ['quarantined flaky.py.fixture::test_removed: its test is not in the census']
A5: red at a7eeb0e: AssertionError: Lists differ: [] != ['markers.py.fixture:python:expectedFailur[78 chars]ted']
A6: red at a7eeb0e: AssertionError: Lists differ: [] != [('skip', True), ('skip', False), ('skip', True)]
A7: red at a7eeb0e: AssertionError: True is not false
A8: not red: the shared helper already refuses an empty population; it guards the census's call to it
A9: red at a7eeb0e: AssertionError: examined 0 planted file(s): the population is empty, so nothing was judged
A10: red at a7eeb0e: AssertionError: examined 0 planted file(s): the population is empty, so nothing was judged
A3: green at be56ed0
A4: green at be56ed0
A5: green at be56ed0
A6: green at be56ed0
A7: green at be56ed0
A9: green at be56ed0
A10: green at be56ed0
```

Push 1's `hygiene` job (job 130729148768) ran every planted test and the tree test.
The census's population is the tracked tree's test files at the first push's head, and its examined count is the one that job prints; that count lies within the tracked tree's bound at that head.
The green commit `be56ed0` carries the census; the listing commit `0b006b6` changes only `sites` in `scripts/tests/quarantine.json`, and `git diff --quiet 0b006b6 HEAD` over the module, the listing and its planted files reads rc 0.
